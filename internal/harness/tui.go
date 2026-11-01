package harness

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"sync/atomic"
	"time"

	"chaude/internal/osenv"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
)

// Claude Code-style screen: welcome box, scrolling conversation, bordered input, status line.
// The REPL logic (sessions, rotation, streaming) is unchanged; this file only renders it.

type (
	textMsg       string
	textEndMsg    struct{}
	toolMsg       struct{ name, arg string }
	toolResultMsg string
	noteMsg       string
	stageMsg      string // dim pipeline progress line ("· prompter 처리중"); shown as a note, keeps the spinner state
	doneMsg       struct{}
	importedMsg   struct{ turns [][2]string } // /import or /resume loaded a session: show its turns
	permMsg       struct {
		tool, arg string
		reply     chan bool
	}
)

// tuiBridge lets the REPL worker goroutine talk to the bubbletea program.
type tuiBridge struct{ p *tea.Program }

func (b *tuiBridge) send(m tea.Msg) { b.p.Send(m) }

func (b *tuiBridge) ask(tool, arg string) bool {
	ch := make(chan bool, 1)
	b.p.Send(permMsg{tool, arg, ch})
	return <-ch
}

// tuiWriter turns the REPL's dim status lines (usage, rotation, interrupted) into note blocks.
type tuiWriter struct{ b *tuiBridge }

func (w tuiWriter) Write(p []byte) (int, error) {
	s := strings.NewReplacer(dim, "", rst, "").Replace(string(p))
	if s = strings.TrimSpace(s); s != "" {
		w.b.send(noteMsg(s))
	}
	return len(p), nil
}

func toolSummary(in map[string]any) string {
	for _, k := range []string{"command", "file_path", "pattern", "path", "url", "query", "description", "prompt"} {
		if v := str(in[k]); v != "" {
			return trunc(strings.ReplaceAll(v, "\n", " "), 100)
		}
	}
	if len(in) == 0 {
		return ""
	}
	keys := make([]string, 0, len(in))
	for k := range in {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return trunc(jsonStr(in), 100)
}

func asList(v any) []any { l, _ := v.([]any); return l }

func resultText(v any) string {
	if s, ok := v.(string); ok {
		return s
	}
	var parts []string
	for _, x := range asList(v) {
		parts = append(parts, str(asMap(x)["text"]))
	}
	return strings.Join(parts, "\n")
}

var cavemanStatusRe = regexp.MustCompile(`^status:\s*(done|partial|fail|blocked)\b`)

// isCavemanBlock reports whether s is an agent caveman return (3.6 header: a status line and a reason line).
func isCavemanBlock(s string) bool {
	hasStatus, hasReason := false, false
	for _, ln := range strings.Split(s, "\n") {
		ln = strings.TrimSpace(ln)
		if cavemanStatusRe.MatchString(ln) {
			hasStatus = true
		}
		if strings.HasPrefix(ln, "reason:") {
			hasReason = true
		}
	}
	return hasStatus && hasReason
}

type block struct {
	kind, text string
	fromRecord bool // loaded by /resume: never merged with streamed text or re-recorded
} // kind: user | asst | tool | result | note

type tuiModel struct {
	r         *Repl
	submit    chan string
	w, h      int
	vp        viewport.Model
	in        inputBox
	inH       int
	sp        spinner.Model
	blocks    []block
	busy      bool
	turnStart time.Time // when the running turn started (busy status elapsed time)
	lastTool  string    // last tool call of the running turn, truncated to 40 chars
	gotText   bool
	perm      *permMsg
	pick      []sessionGroup // non-nil while the /resume picker is open
	pickSel   int
	rpick     []recordProject // non-nil while the record /resume picker is open
	rpickSel  int
	stPick    *settingsPick // non-nil while the /settings picker is open
	ready     bool
	quitting  bool
	sel       int
	curCol    atomic.Int32
	curUp     atomic.Int32
	popH      int
	panelH    int // foreground rows below the input box
	bgH       int // background rows below the input box, above the status line
	panOff    int // first agent row shown in the (scrollable) agent panels below the input box
	lastRun   int // running agent count at the last refresh (idle tick redraw)
	unseen    bool
	held      bool // user scrolled the transcript away from the tail; refresh keeps the offset
	sig       int
	mouse     bool
	hist      inputHistory
	histPath  string // <project>/.chaude/history.jsonl; "" = not persisted
	rw        rewindState
	lastEsc   time.Time
	queue     []string // prompts typed while a foreground answer runs; merged into one turn when it ends
	curPrompt string   // prompt of the running turn (summary in the status row)
	queueH    int
	av        *agentView         // non-nil while the read-only agent view is open
	sv        *sessView          // non-nil while /view is open (session list, then one session's prompts)
	svH       int                // session rows below the input box (/view list)
	usage     string             // non-empty while the /usage popup is open
	imeShift  int32              // column offset for terminals that draw IME preedit off the cursor (JediTerm: -1)
	lines     []string           // transcript lines of the last refresh (viewport content, split on \n)
	selc      textSel            // drag selection in content coordinates (survives streaming output and scrolling)
	dragging  bool               // left button held inside the transcript
	clip      func(string) error // clipboard writer for a release copy (nil = copyToClipboard)
	copyNote  string             // status note after a copy ("복사됨 (N자)"), cleared after 2s
	copySeq   int                // identifies the current copyNote so an older timer does not clear it
}

// imeColShift: JediTerm (JetBrains) draws uncommitted IME text at cursorX+1 cells
// (TerminalPanel.drawInputMethodUncommitedChars), so a Hangul syllable being composed shows one cell
// right of where it lands. Parking the real cursor one cell left puts the preedit on the right cell.
// CHAUDE_IME_SHIFT=<int> overrides (e.g. 0 for terminals without the offset).
func imeColShift() int32 {
	if v := strings.TrimSpace(os.Getenv("CHAUDE_IME_SHIFT")); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return int32(n)
		}
	}
	if os.Getenv("TERMINAL_EMULATOR") == "JetBrains-JediTerm" {
		return -1
	}
	return 0
}

// slash commands (Claude Code style)
type slashCmd struct{ name, desc string }

var slashCmds = []slashCmd{
	{"/help", "show commands and keys"},
	{"/clear", "start a fresh session (handoff note carried over)"},
	{"/status", "model, context tokens, session"},
	{"/model", "show or switch model: /model <name>"},
	{"/settings", "session retention, resident coder slots, main model (saved on esc; applies on next session start)"},
	{"/resume", "list record topics, or show one: /resume [n|slug] (/resume claude [q]: Claude Code sessions, filtered by q)"},
	{"/import", "import a session: /import [id|latest]"},
	{"/view", "browse Claude Code sessions (↑/↓ enter, ←/→ prompts, esc back): /view [id-prefix]"},
	{"/rotate", "hand off to a fresh session"},
	{"/usage", "plan usage and limits (from claude)"},
	{"/cost", "usage / cost (from claude)"},
	{"/context", "context usage by category (from claude)"},
	{"/compact", "compact the child's context (from claude)"},
	{"/config", "show/set config: /config key=value (from claude)"},
	{"/mcp", "MCP server status (from claude)"},
	{"/agents", "subagents info (from claude)"},
	{"/init", "analyze repo, propose CLAUDE.md (runs a turn)"},
	{"/review", "review current changes (runs a turn)"},
	{"/doctor", "check installation and config (runs a turn)"},
	{"/mode", "permission mode: default|acceptEdits|plan|auto (shift+tab cycles)"},
	{"/mouse", "toggle mouse wheel scroll (off = select text)"},
	{"/version", "chaude and Claude Code versions"},
	{"/exit", "exit"},
	{"/quit", "exit"},
}

// cursorWriter wraps the terminal output so the REAL terminal cursor (where the OS IME draws
// Korean/CJK preedit text) sits in the input box instead of at the end of the bottom status line.
// Each frame is followed by "cursor up N, column C"; before the next frame the cursor is moved back down.
type cursorWriter struct {
	*os.File // satisfies term.File (Fd/Read/Close) so bubbletea still reads the window size
	w        io.Writer
	col      *atomic.Int32 // 1-based column in the input row; 0 = off
	up       *atomic.Int32 // rows from the last view row up to the input row
	moved    bool
	upWas    int32
}

func (c *cursorWriter) Write(p []byte) (int, error) {
	var pre, post string
	if c.moved {
		pre = fmt.Sprintf("\x1b[%dB", c.upWas)
		c.moved = false
	}
	if col, up := c.col.Load(), c.up.Load(); col > 0 && len(p) > 0 {
		post = fmt.Sprintf("\x1b[%dA\x1b[%dG", up, col)
		c.moved, c.upWas = true, up
	}
	if _, err := io.WriteString(c.w, pre); err != nil {
		return 0, err
	}
	n, err := c.w.Write(p)
	if err == nil && post != "" {
		_, err = io.WriteString(c.w, post)
	}
	return n, err
}

var (
	orange = lipgloss.Color("#D97757")
	grey   = lipgloss.Color("#888888")
	dimSt  = lipgloss.NewStyle().Foreground(grey)
)

func newTUIModel(r *Repl, submit chan string) *tuiModel {
	sp := spinner.New()
	sp.Spinner = spinner.Spinner{Frames: []string{"✻", "✽", "✶", "✲", "✢", "·"}, FPS: time.Second / 8}
	sp.Style = lipgloss.NewStyle().Foreground(orange)
	return &tuiModel{r: r, submit: submit, sp: sp, mouse: true, imeShift: imeColShift()}
}

// loadHistory restores the recall list from <cwd>/.chaude/history.jsonl and enables saving there.
func (m *tuiModel) loadHistory(cwd string) {
	m.histPath = historyFileFor(cwd)
	m.hist.items = loadInputHistory(m.histPath)
	m.hist.idx = len(m.hist.items)
}

// recordHistory adds a submitted prompt to the recall list and saves it to the project history file.
func (m *tuiModel) recordHistory(line string) {
	m.hist.Add(line)
	if m.histPath != "" {
		_ = saveInputHistory(m.histPath, m.hist.items) // best effort: recall still works in memory
	}
}

func (m *tuiModel) Init() tea.Cmd { return m.sp.Tick }

func (m *tuiModel) welcome() string {
	cwd := m.r.a.Cwd
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	w := m.w - 2
	if w > 64 {
		w = 64
	}
	if w < 20 {
		w = 20
	}
	body := lipgloss.NewStyle().Foreground(orange).Bold(true).Render("✻ Welcome to Claude Harness!") + "\n\n" +
		dimSt.Render("  /help for help, /rotate to rotate session, /quit to exit") + "\n\n" +
		dimSt.Render("  cwd: "+trunc(cwd, w-12)) + "\n" +
		dimSt.Render(fmt.Sprintf("  model: %s · rotate at %s tokens", m.r.a.Model, commas(m.r.effectiveThreshold())))
	return lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(orange).Padding(0, 1).Width(w).Render(body)
}

// mdBold turns paired **bold** spans on one line into ANSI bold (asterisks removed).
// Unpaired markers stay verbatim; lines inside ``` fences are untouched.
func mdBold(s string) string {
	lines := strings.Split(s, "\n")
	fence := false
	for i, ln := range lines {
		if strings.HasPrefix(strings.TrimSpace(ln), "```") {
			fence = !fence
			continue
		}
		if fence || !strings.Contains(ln, "**") {
			continue
		}
		var sb strings.Builder
		rest := ln
		for {
			a := strings.Index(rest, "**")
			if a < 0 {
				break
			}
			b := strings.Index(rest[a+2:], "**")
			if b <= 0 { // no closing pair, or empty "****"
				break
			}
			sb.WriteString(rest[:a])
			sb.WriteString("\x1b[1m" + rest[a+2:a+2+b] + "\x1b[22m")
			rest = rest[a+2+b+2:]
		}
		sb.WriteString(rest)
		lines[i] = sb.String()
	}
	return strings.Join(lines, "\n")
}

func (m *tuiModel) render() string {
	cw := m.w
	if cw < 10 {
		cw = 10
	}
	wrap := func(s string, indent int) string {
		return lipgloss.NewStyle().Width(cw - indent).Render(s)
	}
	indent := func(s string, pre string, rest string) string {
		lines := strings.Split(s, "\n")
		for i := range lines {
			if i == 0 {
				lines[i] = pre + lines[i]
			} else {
				lines[i] = rest + lines[i]
			}
		}
		return strings.Join(lines, "\n")
	}
	var out []string
	out = append(out, m.welcome(), "")
	for _, b := range m.blocks {
		switch b.kind {
		case "user":
			out = append(out, lipgloss.NewStyle().Foreground(grey).Render(indent(wrap(b.text, 2), "> ", "  ")), "")
		case "asst":
			out = append(out, indent(wrap(mdBold(b.text), 2), lipgloss.NewStyle().Foreground(orange).Render("● "), "  "), "")
		case "tool":
			out = append(out, indent(wrap(b.text, 2), lipgloss.NewStyle().Foreground(lipgloss.Color("#5FAF87")).Render("● ")+"", "  "))
		case "result":
			out = append(out, dimSt.Render(indent(wrap(b.text, 5), "  ⎿  ", "     ")), "")
		case "note":
			out = append(out, dimSt.Render(wrap(b.text, 0)), "")
		}
	}
	fg, _ := m.r.tracker.counts()
	opus := m.r.tracker.opusRunning()
	res := len(m.r.residentBusy()) > 0
	if (fg > 0 || opus || res) && (opus || res || !m.gotText) && m.perm == nil {
		out = append(out, m.sp.View()+dimSt.Render(" "+m.mainStatusText()))
	} else if m.busy && !m.gotText && m.perm == nil {
		out = append(out, m.sp.View()+dimSt.Render(" "+m.mainStatusText()+" (esc to interrupt)"))
	}
	return strings.Join(out, "\n")
}

func (m *tuiModel) refresh() {
	if !m.ready {
		return
	}
	atBottom := m.vp.AtBottom()
	sig := len(m.blocks)
	if sig > 0 {
		sig = sig*100003 + len(m.blocks[sig-1].text)
	}
	m.lastRun = m.runningN()
	body := m.render()
	m.lines = strings.Split(body, "\n")
	m.vp.SetContent(body)
	if m.held && !atBottom {
		if sig != m.sig {
			m.unseen = true
		}
	} else {
		m.held = false
		m.vp.GotoBottom() // follow the tail unless the user is scrolling
		m.unseen = false
	}
	m.sig = sig
}

func (m *tuiModel) layout() {
	m.fit()
	m.inH = m.in.Height(maxInputRows)
	vh := m.h - m.inH - 2 - 1 - m.popH - m.panelH - m.bgH - m.queueH - m.svH // input box + status line + suggestions
	if vh < 1 {
		vh = 1
	}
	if !m.ready {
		m.vp = viewport.New(m.w, vh)
		m.ready = true
	} else {
		m.vp.Width, m.vp.Height = m.w, vh
	}
	m.refresh()
}

func (m *tuiModel) appendAsst(s string) {
	if n := len(m.blocks); n > 0 && m.blocks[n-1].kind == "asst" && m.gotText && !m.blocks[n-1].fromRecord {
		m.blocks[n-1].text += s
		return
	}
	m.blocks = append(m.blocks, block{"asst", s, false})
	m.gotText = true
}

func (m *tuiModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd
	switch v := msg.(type) {
	case tea.WindowSizeMsg:
		m.w, m.h = v.Width, v.Height
		m.layout()
	case spinner.TickMsg:
		var c tea.Cmd
		m.sp, c = m.sp.Update(v)
		cmds = append(cmds, c)
		if n := m.runningN(); m.busy || n != m.lastRun {
			m.refresh()
		}
		if !m.busy && m.perm == nil && m.r.sweepIdle(time.Now()) {
			m.busy, m.turnStart = true, time.Now()
			m.submit <- "/idle-rotate" // rotate on the sender goroutine, like a user prompt
		}
		if !m.busy && m.perm == nil && m.r.deferredDue(time.Now()) {
			m.busy, m.turnStart = true, time.Now()
			m.submit <- "/rotate-deferred" // held rotation: background agents finished or timed out
		}
	case textMsg:
		m.appendAsst(string(v))
		m.refresh()
	case textEndMsg:
		m.gotText = false
	case toolMsg:
		m.gotText = false
		t := v.name
		if v.arg != "" {
			t += "(" + v.arg + ")"
		}
		m.lastTool = trunc(t, 40)
		m.blocks = append(m.blocks, block{"tool", t, false})
		m.refresh()
	case toolResultMsg:
		s := strings.TrimSpace(string(v))
		if !isCavemanBlock(s) { // agent caveman return: hidden from the TUI transcript
			lines := strings.Split(s, "\n")
			t := trunc(lines[0], 160)
			if len(lines) > 1 {
				t += fmt.Sprintf(" … (+%d lines)", len(lines)-1)
			}
			m.blocks = append(m.blocks, block{"result", t, false})
		}
		m.refresh()
	case noteMsg:
		m.gotText = false
		m.blocks = append(m.blocks, block{"note", string(v), false})
		m.refresh()
	case stageMsg:
		m.blocks = append(m.blocks, block{"note", string(v), false})
		m.refresh()
	case permMsg:
		m.perm, m.av, m.usage = &v, nil, ""
		m.blocks = append(m.blocks, block{"note", fmt.Sprintf("Allow %s(%s)?  [y] yes  [n] no", v.tool, v.arg), false})
		m.refresh()
	case importedMsg:
		m.gotText = false
		m.blocks = nil
		turns := v.turns
		if len(turns) > 100 {
			m.blocks = append(m.blocks, block{"note", fmt.Sprintf("(%d earlier turns not shown)", len(turns)-100), false})
			turns = turns[len(turns)-100:]
		}
		for _, t := range turns {
			m.blocks = append(m.blocks, block{"user", t[0], false}, block{"asst", t[1], false})
		}
		m.refresh()
		m.vp.GotoBottom()
	case usageMsg:
		if !m.busy {
			m.gotText = false
		}
		m.av, m.usage = nil, strings.TrimRight(string(v), "\n")
	case doneMsg:
		m.busy, m.gotText = false, false
		m.r.tracker.finishForeground() // turn over: no foreground agent can still be running (esc/error)
		if len(m.queue) > 0 {
			next := mergePrompts(m.queue) // everything typed meanwhile goes out as ONE turn
			m.queue = nil
			m.sendPrompt(next)
			m.after()
			return m, nil
		}
		m.refresh()
	case tea.MouseMsg:
		if m.usage != "" {
			break
		}
		m.click(v)
		if m.stPick != nil {
			wheelSel(&m.stPick.sel, settingsRows, v.Button)
			break
		}
		if m.rpick != nil {
			wheelSel(&m.rpickSel, len(m.rpick), v.Button)
			break
		}
		if m.sv != nil && m.sv.turns == nil && len(m.sv.list) > 0 { // /view session list: same cursor picker
			wheelSel(&m.sv.sel, len(m.sv.list), v.Button)
			break
		}
		if m.av == nil && m.perm == nil && m.panelWheel(v) {
			break
		}
		if m.av != nil {
			if v.Button == tea.MouseButtonWheelUp {
				m.av.off += 3
			} else if v.Button == tea.MouseButtonWheelDown {
				m.av.off = max(0, m.av.off-3)
			}
			break
		}
		if ok, c := m.selectMouse(v); ok {
			cmds = append(cmds, c)
			break
		}
		var c tea.Cmd
		m.vp, c = m.vp.Update(v)
		m.held = !m.vp.AtBottom()
		cmds = append(cmds, c)
	case copyNoteMsg:
		if v.seq == m.copySeq {
			m.copyNote = ""
		}
	case tea.KeyMsg:
		return m.key(v)
	}
	if !m.quitting {
	}
	m.after()
	return m, tea.Batch(cmds...)
}

func (m *tuiModel) interrupt() {
	m.r.tracker.finishForeground()
	select {
	case m.r.intr <- struct{}{}:
	default:
	}
}

func (m *tuiModel) key(k tea.KeyMsg) (tea.Model, tea.Cmd) {
	if m.perm != nil {
		switch strings.ToLower(k.String()) {
		case "y", "enter":
			m.perm.reply <- true
			m.blocks = append(m.blocks, block{"note", "allowed", false})
			m.perm = nil
		case "n", "esc":
			m.perm.reply <- false
			m.blocks = append(m.blocks, block{"note", "denied", false})
			m.perm = nil
		case "ctrl+c":
			m.perm.reply <- false
			m.perm = nil
			m.interrupt()
		}
		m.refresh()
		return m, nil
	}
	if m.usage != "" {
		m.usageKey(k)
		return m, nil
	}
	if m.av != nil {
		m.avKey(k)
		return m, nil
	}
	if m.sv != nil {
		m.svKey(k)
		return m, nil
	}
	if m.rw.open {
		switch k.String() {
		case "up", "k":
			m.rw.Move(-1)
		case "down", "j":
			m.rw.Move(1)
		case "esc", "ctrl+c", "q":
			m.rw.Cancel()
		case "enter":
			if cut, text, ok := m.rw.Confirm(m.blocks); ok {
				m.blocks = m.blocks[:cut]
				m.in.SetValue(text)
				m.blocks = append(m.blocks, block{"note", "rewound to this prompt (view + input only; the child session keeps its context. Use /clear or /rotate to reset it)", false})
				m.vp.GotoBottom()
				m.refresh()
				m.after()
			}
		}
		return m, nil
	}
	if m.stPick != nil {
		m.settingsKey(k.String())
		return m, nil
	}
	if m.rpick != nil {
		n := len(m.rpick)
		switch k.String() {
		case "up", "k":
			m.rpickSel = (m.rpickSel + n - 1) % n
		case "down", "j":
			m.rpickSel = (m.rpickSel + 1) % n
		case "esc", "ctrl+c", "q":
			m.rpick = nil
		case "enter":
			p := m.rpick[m.rpickSel]
			m.rpick = nil
			m.loadRecordPick(p)
		default:
			if d := k.String(); len(d) == 1 && d[0] >= '1' && d[0] <= '9' && int(d[0]-'1') < n {
				m.rpickSel = int(d[0] - '1')
			}
		}
		return m, nil
	}
	if m.pick != nil {
		n := len(m.pick)
		switch k.String() {
		case "up", "k":
			m.pickSel = (m.pickSel + n - 1) % n
		case "down", "j":
			m.pickSel = (m.pickSel + 1) % n
		case "esc", "ctrl+c", "q":
			m.pick = nil
		case "enter":
			key := m.pick[m.pickSel].key()
			m.pick = nil
			m.sendImport(key)
		default:
			if d := k.String(); len(d) == 1 && d[0] >= '1' && d[0] <= '9' && int(d[0]-'1') < n {
				m.pickSel = int(d[0] - '1')
			}
		}
		return m, nil
	}
	switch k.String() {
	case "ctrl+c":
		if m.busy {
			m.interrupt()
			return m, nil
		}
		return m.quit()
	case "ctrl+d":
		return m.quit()
	case "ctrl+t":
		m.openAgentKey()
		return m, nil
	case "esc":
		if m.busy {
			m.interrupt()
			return m, nil
		}
		// esc esc on an empty input opens the rewind picker
		if m.in.Value() == "" && time.Since(m.lastEsc) < 600*time.Millisecond {
			m.lastEsc = time.Time{}
			if !m.rw.Open(m.blocks) {
				m.blocks = append(m.blocks, block{"note", "nothing to rewind to", false})
				m.refresh()
			}
			return m, nil
		}
		m.lastEsc = time.Now()
		return m, nil
	case "tab":
		if sg := m.suggestions(); len(sg) > 0 {
			m.in.SetValue(sg[min(m.sel, len(sg)-1)].name + " ")
			m.after()
		}
		return m, nil
	case "up", "down":
		// The slash completer owns Up/Down only while open and no recalled prompt is shown.
		// Up at its top item and any Up/Down on a recalled prompt go to history.
		n := len(m.suggestions())
		switch {
		case n > 0 && !m.hist.Browsing() && k.String() == "down":
			m.sel = (m.sel + 1) % n
		case n > 0 && !m.hist.Browsing() && m.sel > 0:
			m.sel--
		case k.String() == "up":
			if !m.in.MoveRow(-1) {
				if v, ok := m.hist.Prev(m.in.Value()); ok {
					m.in.SetValue(v)
					m.sel = 0
				}
			}
		default:
			if !m.in.MoveRow(1) && m.hist.Browsing() {
				if v, ok := m.hist.Next(); ok {
					m.in.SetValue(v)
					m.sel = 0
				}
			}
		}
		m.after()
		return m, nil
	case "shift+tab":
		m.setMode(permMode(m.r.mode.Load()).Next())
		return m, nil
	case "alt+enter", "ctrl+j":
		m.in.Insert([]rune{'\n'})
		m.after()
		return m, nil
	case "pgup", "pgdown", "ctrl+b", "ctrl+f":
		var c tea.Cmd
		m.vp, c = m.vp.Update(k)
		m.held = !m.vp.AtBottom()
		return m, c
	case "shift+up":
		m.vp.LineUp(1)
		m.held = !m.vp.AtBottom()
		return m, nil
	case "shift+down":
		m.vp.LineDown(1)
		m.held = !m.vp.AtBottom()
		return m, nil
	case "home", "end":
		if m.in.Value() == "" {
			if k.String() == "home" {
				m.vp.GotoTop()
			} else {
				m.vp.GotoBottom()
			}
			return m, nil
		}
		m.edit(k)
		m.after()
		return m, nil
	case "enter":
		if v := m.in.val; len(v) > 0 && m.in.pos == len(v) && v[len(v)-1] == '\\' {
			m.in.Backspace() // trailing "\" + enter = newline, like Claude Code
			m.in.Insert([]rune{'\n'})
			m.after()
			return m, nil
		}
		line := strings.TrimSpace(m.in.Value())
		if line == "" {
			return m, nil
		}
		if m.busy {
			if strings.HasPrefix(line, "/") {
				if line == "/usage" { // allowed while thinking: separate one-shot claude call, popup
					m.in.SetValue("")
					m.recordHistory(line)
					m.after()
					go func() { usageSend(m.r, usageMsg(usageFetch(m.r.lastCtx))) }()
					return m, nil
				}
				if f := strings.Fields(line); busyOK[f[0]] { // read-only / UI-only: no conversation or session change
					m.in.SetValue("")
					m.recordHistory(line)
					m.after()
					return m.runSlash(line)
				}
				m.blocks = append(m.blocks, block{"note", busySlashMsg, false})
				m.refresh()
				return m, nil
			}
			m.in.SetValue("")
			m.recordHistory(line)
			m.queue = append(m.queue, line)
			m.vp.GotoBottom()
			m.after()
			return m, nil
		}
		m.in.SetValue("")
		m.recordHistory(line)
		if strings.HasPrefix(line, "/") {
			if sg := m.suggestions(); len(sg) > 0 && sg[min(m.sel, len(sg)-1)].name != line {
				c := sg[min(m.sel, len(sg)-1)].name
				if c == "/model" {
					m.in.SetValue(c + " ")
					m.after()
					return m, nil
				}
				line = c
			}
			return m.runSlash(line)
		}
		m.sendPrompt(line)
		return m, nil
	}
	m.sel = 0
	if !m.vp.AtBottom() {
		m.held = true // typing keeps the output scroll position away from the tail
	}
	m.edit(k)
	m.after()
	return m, nil
}

// busyOK: slash commands that run while an answer is in flight (no conversation or session change).
// /usage is handled before this check (separate one-shot claude call).
var busyOK = map[string]bool{"/usage": true, "/version": true, "/help": true, "/status": true, "/mode": true, "/mouse": true, "/view": true, "/settings": true}

// busySlashMsg: shown when a conversation-affecting slash command is typed while an answer runs.
const busySlashMsg = "생각 중에는 /usage, /version, /help, /status, /mode, /mouse, /view 만 사용할 수 있습니다. 답변이 끝난 뒤 다시 입력해 주세요."

var usageSend = func(r *Repl, msg tea.Msg) { r.ui.send(msg) }

// usageFetch runs /usage in a separate one-shot claude process so it never touches the busy session.
var usageFetch = func(ctx int) string {
	cmd := exec.Command(osenv.ResolveBin("claude"), "-p", "/usage", "--output-format", "json", "--no-session-persistence")
	cmd.Env = cleanEnv(0)
	cmd.Dir = os.TempDir()
	out, err := cmd.Output()
	if err != nil {
		return "/usage 조회에 실패했습니다: " + err.Error()
	}
	var j map[string]any
	if json.Unmarshal(out, &j) != nil || strings.TrimSpace(str(j["result"])) == "" {
		return "/usage 결과를 읽지 못했습니다."
	}
	return decorateUsage(str(j["result"]), ctx)
}

// sendPrompt shows the prompt in the transcript and hands it to the sender goroutine.
func (m *tuiModel) sendPrompt(line string) {
	m.blocks = append(m.blocks, block{"user", line, false})
	m.curPrompt = line
	m.busy, m.turnStart, m.lastTool = true, time.Now(), ""
	m.held = false
	m.vp.GotoBottom()
	m.refresh()
	m.submit <- line
}

// mergePrompts joins prompts that arrived while busy into one message (blank line between).
func mergePrompts(q []string) string { return strings.Join(q, "\n\n") }

// placeholder: idle hint in the empty input box.
func (m *tuiModel) placeholder() string {
	if m.busy {
		return "생각중 - 추가 입력은 한 번에 합쳐 처리"
	}
	return "대기 - 프롬프트 입력"
}

// mainStatusText: the main session's row in the same `상태-모델-세션이름(요약)` form as agents,
// plus ` · <elapsed> · <lastTool>` while a turn is running.
func (m *tuiModel) mainStatusText() string {
	s := m.mainStatusBase()
	if m.busy && !m.turnStart.IsZero() {
		d := time.Since(m.turnStart)
		s += fmt.Sprintf(" · %dm%02ds", int(d.Minutes()), int(d.Seconds())%60)
		if m.lastTool != "" {
			s += " · " + m.lastTool
		}
	}
	return s
}

func (m *tuiModel) mainStatusBase() string {
	sum := strings.TrimSpace(m.curPrompt)
	if i := strings.IndexByte(sum, '\n'); i >= 0 {
		sum = strings.TrimSpace(sum[:i])
	}
	if sum == "" {
		return statusText("생각중", m.r.a.Model, "메인", "")
	}
	return statusText("생각중", m.r.a.Model, "메인", trunc(sum, 60))
}

// statusText formats `상태-모델-이름(요약)`; summary part omitted when empty.
func statusText(state, model, name, sum string) string {
	s := state + "-" + model + "-" + name
	if sum != "" {
		s += "(" + sum + ")"
	}
	return s
}

// queueLines renders waiting prompts (dim, "대기" prefix) shown right above the input box.
func (m *tuiModel) queueLines() []string {
	maxShow := 5
	if m.h > 0 { // keep at least minViewRows for the transcript
		maxShow = min(maxShow, m.h-m.inH-2-1-m.popH-minViewRows)
	}
	if maxShow <= 0 {
		return nil
	}
	var out []string
	for i, q := range m.queue {
		if i == maxShow-1 && len(m.queue) > maxShow {
			out = append(out, dimSt.Render(fmt.Sprintf("  … +%d 대기", len(m.queue)-i)))
			break
		}
		out = append(out, ansiTrunc(dimSt.Render("  대기 "+strings.ReplaceAll(q, "\n", " ")), m.w))
	}
	return out
}

// edit applies a plain editing key to the input box.
func (m *tuiModel) edit(k tea.KeyMsg) {
	switch k.Type {
	case tea.KeyRunes:
		if !k.Alt {
			m.in.Insert(k.Runes)
		}
	case tea.KeySpace:
		m.in.Insert([]rune{' '})
	case tea.KeyBackspace, tea.KeyCtrlH:
		if k.Alt { // opt+delete
			m.in.DeleteWord()
		} else {
			m.in.Backspace()
		}
	case tea.KeyDelete, tea.KeyCtrlD:
		m.in.Delete()
	case tea.KeyLeft:
		m.in.Left()
	case tea.KeyRight:
		m.in.Right()
	case tea.KeyHome, tea.KeyCtrlA:
		m.in.Home()
	case tea.KeyEnd, tea.KeyCtrlE:
		m.in.End()
	case tea.KeyCtrlW:
		m.in.DeleteWord()
	case tea.KeyCtrlK:
		m.in.KillToEnd()
	case tea.KeyCtrlU: // cmd+delete (terminal sends ^U)
		m.in.KillToStart()
	}
}

func (m *tuiModel) suggestions() []slashCmd {
	v := m.in.Value()
	if !strings.HasPrefix(v, "/") || strings.ContainsAny(v, " \t") || m.busy || m.perm != nil {
		return nil
	}
	var out []slashCmd
	for _, c := range slashCmds {
		if strings.HasPrefix(c.name, v) {
			out = append(out, c)
		}
	}
	return out
}

const maxInputRows = 5
const maxPop = 8

func (m *tuiModel) fit() {
	w := m.w - 6
	if w < 4 {
		w = 4
	}
	m.in.width = w
}

func (m *tuiModel) syncCursor() {
	if !m.ready || m.perm != nil {
		m.curCol.Store(0)
		return
	}
	row, col := m.in.Cursor()
	if len(m.in.val) == 0 {
		row, col = 0, 0
	}
	m.in.View(maxInputRows, "", dimSt) // keeps m.in.top current
	vis := row - m.in.top
	m.curCol.Store(max(1, int32(1+2+col+1)+m.imeShift))          // border + prompt "> " + text; 1-based column
	m.curUp.Store(int32(m.inH - 1 - vis + 2 + m.panelH + m.bgH)) // rows below cursor in box + bottom border + status line
}

func (m *tuiModel) after() {
	m.fit()
	m.syncCursor()
	if pn, bn, qn, sn := len(m.panelLines()), len(m.bgLines()), len(m.queueLines()), len(m.sessLines()); pn != m.panelH || bn != m.bgH || qn != m.queueH || sn != m.svH {
		m.panelH, m.bgH, m.queueH, m.svH = pn, bn, qn, sn
		m.layout()
		m.syncCursor()
	}
	if n := min(len(m.suggestions()), maxPop); n != m.popH || m.in.Height(maxInputRows) != m.inH {
		m.popH = n
		m.layout()
		m.syncCursor()
	}
}

// sendImport loads a /resume group (key: see sessionGroup.key) as the running session, showing the command.
// loadRecordPick appends one record project's blocks to the transcript (Enter in the record /resume picker).
func (m *tuiModel) loadRecordPick(p recordProject) {
	entries, err := loadRecordProject(p.Dir)
	if err != nil {
		m.blocks = append(m.blocks, block{"note", err.Error(), false})
	}
	for _, e := range entries {
		m.blocks = append(m.blocks, block{e.Kind, e.Text, true})
	}
	m.vp.GotoBottom()
	m.refresh()
}

func (m *tuiModel) sendImport(key string) {
	m.busy, m.turnStart, m.lastTool = true, time.Now(), ""
	m.blocks = append(m.blocks, block{"user", "/import " + key, false})
	m.vp.GotoBottom()
	m.refresh()
	m.submit <- "/import " + key
}

func (m *tuiModel) runSlash(line string) (tea.Model, tea.Cmd) {
	f := strings.Fields(line)
	note := func(s string) { m.blocks = append(m.blocks, block{"note", s, false}); m.refresh() }
	switch f[0] {
	case "/quit", "/exit":
		return m.quit()
	case "/help":
		var l []string
		for _, c := range slashCmds {
			l = append(l, fmt.Sprintf("%-8s %s", c.name, c.desc))
		}
		note(strings.Join(l, "\n") + "\nesc esc rewind · up/down history · esc interrupt · y/n permission · pgup/pgdn scroll · tab complete")
	case "/mouse":
		m.mouse = !m.mouse
		if m.mouse {
			note("mouse wheel scroll ON (shift+drag selects text in most terminals)")
			return m, tea.EnableMouseCellMotion
		}
		note("mouse OFF: text selection works; scroll with pgup/pgdn, shift+up/down")
		return m, tea.DisableMouse
	case "/status":
		sid := ""
		if m.r.sess != nil {
			sid = m.r.sess.SessionID
		}
		if sid == "" {
			sid = "-"
		}
		note(fmt.Sprintf("model %s · ctx %s/%s · rotations %d · session %s", m.r.a.Model, commas(m.r.lastCtx), commas(m.r.effectiveThreshold()), m.r.rotations, trunc(sid, 8)))
	case "/model":
		if len(f) < 2 {
			note("model " + m.r.a.Model + "  (switch: /model <name>, hands off context to the new session)")
			break
		}
		m.busy, m.turnStart, m.lastTool = true, time.Now(), ""
		m.blocks = append(m.blocks, block{"user", line, false})
		m.vp.GotoBottom()
		m.refresh()
		m.submit <- line
	case "/settings":
		m.openSettings()
	case "/resume":
		if terms, record := resumeTarget(m.r.a.Cwd, f[1:]); !record {
			gs, msg := m.r.groupList(terms...)
			if msg != "" {
				note(msg)
				break
			}
			if len(gs) == 1 {
				m.sendImport(gs[0].key())
				break
			}
			if len(gs) > 30 {
				gs = gs[:30]
			}
			m.pick, m.pickSel = gs, 0
			break
		}
		ps, err := recordPick(recordDir(m.r.a.Cwd), strings.Join(f[1:], " "))
		if err != nil {
			note(err.Error())
			break
		}
		if ps != nil {
			m.rpick, m.rpickSel = ps, 0
			break
		}
		lines, blocks, err := recordResume(recordDir(m.r.a.Cwd), strings.Join(f[1:], " "))
		if err != nil {
			note(err.Error())
			break
		}
		for _, l := range lines {
			m.blocks = append(m.blocks, block{"note", l, true})
		}
		for _, b := range blocks {
			m.blocks = append(m.blocks, block{b.Kind, b.Text, true})
		}
		m.vp.GotoBottom()
		m.refresh()
	case "/view":
		arg := ""
		if len(f) > 1 {
			arg = f[1]
		}
		if msg := m.openSessView(arg); msg != "" {
			note(msg)
		}
	case "/import":
		m.busy, m.turnStart, m.lastTool = true, time.Now(), ""
		m.blocks = append(m.blocks, block{"user", line, false})
		m.vp.GotoBottom()
		m.refresh()
		m.submit <- line
	case "/mode":
		if len(f) < 2 {
			note("mode " + permMode(m.r.mode.Load()).String() + "  (default | acceptEdits | plan | auto; shift+tab cycles)")
			break
		}
		p, ok := parseMode(f[1])
		if !ok {
			note("unknown mode " + f[1] + " (default | acceptEdits | plan | auto)")
			break
		}
		m.setMode(p)
	case "/version":
		note("chaude " + ChaudeVersion + " · Claude Code " + claudeVersion())
	case "/clear", "/rotate", "/usage", "/cost", "/context", "/compact", "/config", "/mcp", "/agents", "/init", "/review", "/doctor":
		if f[0] == "/clear" {
			m.blocks = nil
		}
		m.busy, m.turnStart, m.lastTool = true, time.Now(), ""
		m.blocks = append(m.blocks, block{"user", line, false})
		m.vp.GotoBottom()
		m.refresh()
		m.submit <- line
	default:
		if unsupportedCmds[f[0]] {
			note(f[0] + " is interactive-only in claude and not supported in this REPL")
			break
		}
		note("unknown command " + f[0] + " (try /help)")
	}
	return m, nil
}

func (m *tuiModel) setMode(p permMode) {
	m.r.mode.Store(int32(p))
	m.blocks = append(m.blocks, block{"note", "mode: " + p.String(), false})
	m.refresh()
}

func (m *tuiModel) quit() (tea.Model, tea.Cmd) {
	m.quitting = true
	return m, tea.Quit
}

func (m *tuiModel) statusLine() string {
	left := " " + ctxBar(m.r.lastCtx, m.r.effectiveThreshold(), m.r.roleCtxSnap(), m.r.tracker.countRunningCoders())
	if s, _ := m.r.rateStr.Load().(string); s != "" {
		left += " · " + s
	}
	left += fmt.Sprintf(" · rot %d", m.r.rotations)
	if pm := permMode(m.r.mode.Load()); pm != modeDefault {
		left += " · mode " + pm.String()
	}
	if fg, bg := m.r.tracker.counts(); fg+bg > 0 {
		left += fmt.Sprintf(" · agents %d running", fg+bg)
		if bg > 0 {
			left += fmt.Sprintf(" (bg %d)", bg)
		}
	}
	right := "esc interrupt · /rotate · wheel/pgup/pgdn scroll · ctrl+c quit "
	if m.perm != nil {
		right = "y allow · n deny "
	}
	if m.copyNote != "" {
		left += " · " + m.copyNote
	}
	gap := m.w - lipgloss.Width(left) - lipgloss.Width(right)
	if gap < 1 {
		return dimSt.Render(trunc(left, m.w))
	}
	return dimSt.Render(left + strings.Repeat(" ", gap) + right)
}

func (m *tuiModel) View() string {
	if !m.ready {
		return "starting…"
	}
	ql := m.queueLines()
	prevQ := m.queueH
	m.queueH = len(ql) // panels size themselves around the queue rows of this frame
	pl := m.panelLines()
	panel := ""
	if len(pl) > 0 {
		panel = strings.Join(pl, "\n") + "\n"
	}
	bl := m.bgLines()
	bgPanel := ""
	if len(bl) > 0 {
		bgPanel = strings.Join(bl, "\n") + "\n"
	}
	sl := m.sessLines()
	svPanel := ""
	if len(sl) > 0 {
		svPanel = strings.Join(sl, "\n") + "\n"
	}
	changed := len(pl) != m.panelH || len(bl) != m.bgH || len(ql) != prevQ
	m.panelH, m.bgH, m.queueH, m.svH = len(pl), len(bl), len(ql), len(sl) // computed in the frame it is drawn: layout height always matches
	if vh := max(1, m.h-m.inH-2-1-m.popH-m.panelH-m.bgH-m.queueH-m.svH); m.h > 0 && vh != m.vp.Height {
		atBottom := m.vp.AtBottom()
		m.vp.Height = vh
		if atBottom {
			m.vp.GotoBottom() // shrinking at the bottom must not cut off the last line
		}
	}
	if changed {
		m.syncCursor() // panel rows changed between Update and View: keep cursor row in step
	}
	bc := grey
	if m.perm != nil {
		bc = orange
	}
	box := lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(bc).Width(m.w - 2).Render(m.in.View(maxInputRows, m.placeholder(), dimSt))
	pop := ""
	if sg := m.suggestions(); len(sg) > 0 {
		var l []string
		sel := min(m.sel, len(sg)-1)
		from := 0
		if sel >= maxPop {
			from = sel - maxPop + 1
		}
		for i := from; i < len(sg) && i < from+maxPop; i++ {
			c := sg[i]
			st := dimSt
			if i == sel {
				st = lipgloss.NewStyle().Foreground(orange)
			}
			l = append(l, st.Render(fmt.Sprintf("  %-9s %s", c.name, c.desc)))
		}
		pop = strings.Join(l, "\n") + "\n"
	}
	vv := highlightView(m.vp.View(), m.lines, m.vp.YOffset, m.selc)
	if m.rw.open {
		hh := m.vp.Height
		l := []string{lipgloss.NewStyle().Foreground(orange).Render("Rewind to a prompt  (↑/↓ select · enter rewind · esc cancel)")}
		from := max(0, min(m.rw.sel-hh/2, len(m.rw.turns)-(hh-2)))
		for i := from; i < len(m.rw.turns) && len(l) < hh-1; i++ {
			t := fmt.Sprintf("%2d) %s", i+1, trunc(strings.ReplaceAll(m.blocks[m.rw.turns[i]].text, "\n", " "), 100))
			if i == m.rw.sel {
				t = lipgloss.NewStyle().Foreground(orange).Render("> " + t)
			} else {
				t = dimSt.Render("  " + t)
			}
			l = append(l, ansiTrunc(t, m.w))
		}
		for len(l) < hh {
			l = append(l, "")
		}
		vv = strings.Join(l, "\n")
	}
	if m.pick != nil {
		hh := m.vp.Height
		from := max(0, min(m.pickSel-hh/2, len(m.pick)-(hh-2)))
		l := []string{lipgloss.NewStyle().Foreground(orange).Render("Resume a session  (↑/↓ or 1-9 select · enter import · esc cancel)")}
		for i := from; i < len(m.pick) && len(l) < hh-1; i++ {
			t := fmt.Sprintf("%2d) %s", i+1, m.pick[i].label())
			if i == m.pickSel {
				t = lipgloss.NewStyle().Foreground(orange).Render("> " + t)
			} else {
				t = dimSt.Render("  " + t)
			}
			l = append(l, ansiTrunc(t, m.w))
		}
		for len(l) < hh {
			l = append(l, "")
		}
		vv = strings.Join(l, "\n")
	}
	if m.stPick != nil {
		vv = m.settingsLines(m.vp.Height)
	}
	if m.rpick != nil {
		hh := m.vp.Height
		from := max(0, min(m.rpickSel-hh/2, len(m.rpick)-(hh-2)))
		l := []string{lipgloss.NewStyle().Foreground(orange).Render("Resume a project  (↑/↓ wheel or 1-9 select · enter load · esc cancel)")}
		for i := from; i < len(m.rpick) && len(l) < hh-1; i++ {
			p := m.rpick[i]
			t := fmt.Sprintf("%2d) %s · 파일 %d개 · %s", i+1, p.Name, p.Files, p.Last.Format("2006-01-02"))
			if i == m.rpickSel {
				t = lipgloss.NewStyle().Foreground(orange).Render("> " + t)
			} else {
				t = dimSt.Render("  " + t)
			}
			l = append(l, ansiTrunc(t, m.w))
		}
		for len(l) < hh {
			l = append(l, "")
		}
		vv = strings.Join(l, "\n")
	}
	if m.sv != nil && m.sv.turns != nil {
		vv = m.svPromptLines(m.vp.Height)
	}
	if m.av != nil {
		vv = m.agentViewLines(m.vp.Height)
	}
	if m.usage != "" {
		vv = m.usagePopup(m.vp.Height)
	}
	if m.unseen && m.vp.AtBottom() {
		m.unseen = false
	}
	if m.unseen && m.av == nil && m.usage == "" && m.sv == nil {
		ls := strings.Split(vv, "\n")
		tag := lipgloss.NewStyle().Foreground(orange).Render("↓ 새 메시지 (End: 맨 아래로)")
		ls[len(ls)-1] = strings.Repeat(" ", max(0, m.w-lipgloss.Width(tag)-1)) + tag
		vv = strings.Join(ls, "\n")
	}
	qv := ""
	if len(ql) > 0 {
		qv = strings.Join(ql, "\n") + "\n"
	}
	return vv + "\n" + pop + qv + box + "\n" + panel + bgPanel + svPanel + m.statusLine()
}

// runTUI is the interactive entry: same Repl, rendered with bubbletea in the alt screen.
func (r *Repl) runTUI() int {
	if err := r.start(); err != nil {
		fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
		return 1
	}
	defer func() {
		r.saveExitHandoff()
		if r.sess != nil {
			r.closeDrains()
			r.sess.Close()
		}
		r.closeAllChildren()
	}()
	submit := make(chan string)
	m := newTUIModel(r, submit)
	m.loadHistory(r.a.Cwd)
	cw := &cursorWriter{File: os.Stdout, w: os.Stdout, col: &m.curCol, up: &m.curUp}
	lipgloss.SetHasDarkBackground(true) // skip the slow OSC background query at startup
	go claudeVersion()                  // warm the /version cache off the UI goroutine
	p := tea.NewProgram(m, tea.WithAltScreen(), tea.WithMouseCellMotion(), tea.WithOutput(cw))
	r.ui = &tuiBridge{p}
	r.out = tuiWriter{r.ui}
	go func() {
		for line := range submit {
			f := strings.Fields(line)
			switch {
			case line == "/rotate":
				r.rotate("manual")
			case line == "/idle-rotate":
				r.idleRotateNow()
			case line == "/rotate-deferred":
				r.runDeferredRotate()
			case line == "/clear":
				r.clearWithHandoff()
			case passthroughCmds[f[0]]:
				r.passthrough(line)
			case f[0] == "/import":
				a := "latest"
				if len(f) > 1 {
					a = f[1]
				}
				r.importSession(a)
			case f[0] == "/model" && len(f) > 1:
				r.endParked(f[1], "model-switch") // rotate starts a fresh child under this key
				r.a.Model = f[1]
				r.rotate("model " + f[1])
			default:
				r.handle(line)
				r.recordSave(line)
			}
			p.Send(doneMsg{})
		}
	}()
	_, err := p.Run()
	close(submit)
	if err != nil {
		fmt.Fprintln(os.Stderr, "harness:", err)
		return 1
	}
	return 0
}

func ansiTrunc(s string, w int) string { return ansi.Truncate(s, w, "…") }

const (
	maxPanel    = 4
	minViewRows = 3
)

// runningN: running agents plus busy resident sessions (the running list's refresh key).
func (m *tuiModel) runningN() int { return len(m.r.tracker.running()) + len(m.r.residentBusy()) }

// panelLines: one row per running foreground delegated session (below the input box).
func (m *tuiModel) panelLines() []string { return m.rows(false, 0) }

// bgLines: one row per running background agent, drawn below the input box (`상태 - 이름`).
func (m *tuiModel) bgLines() []string { return m.rows(true, len(m.panelLines())) }

// rows: description and first prompt line of running agents of one kind; `used` rows are already
// taken by the other panel. Finished agents drop out because the tracker marks them done.
func (m *tuiModel) rows(bg bool, used int) []string { r, _ := m.rowsA(bg, used); return r }

// rowsA also returns the agent behind each row (nil for the "+N more" row), for click mapping.
func (m *tuiModel) rowsA(bg bool, used int) ([]string, []*agentRec) {
	var l []*agentRec
	for _, a := range m.r.tracker.sortedRunning() {
		if a.Background == bg {
			l = append(l, a)
		}
	}
	if !bg { // resident sessions handling a command, idle ones are not listed
		l = append(l, m.r.residentBusy()...)
	}
	for _, a := range m.r.tracker.sortedRestored() {
		if a.Background == bg {
			l = append(l, a)
		}
	}
	if len(l) == 0 || m.w < 10 {
		return nil, nil
	}
	limit := maxPanel
	if m.h > 0 { // keep at least minViewRows for the transcript
		limit = min(limit, m.h-m.inH-2-1-m.popH-m.queueH-minViewRows-used)
	}
	if limit <= 0 {
		return nil, nil
	}
	var out []string
	var ag []*agentRec
	win := l
	off := 0
	if len(l) > limit { // limit-1 agent rows + one scroll indicator row
		n := limit - 1
		off = max(0, min(m.panOff, len(l)-n))
		win = l[off : off+n]
	}
	for _, a := range win {
		line := agentStatusText(a, m.r.a.Model)
		out = append(out, ansiTrunc("  "+agentDot(agentState(a))+dimSt.Render(" "+strings.ReplaceAll(line, "\n", " ")), m.w))
		ag = append(ag, a)
	}
	if len(l) > limit {
		out = append(out, dimSt.Render(fmt.Sprintf("  … %d-%d/%d (마우스 휠로 스크롤)", off+1, off+len(win), len(l))))
		ag = append(ag, nil)
	}
	return out, ag
}

// agent states shown in the status dot.
const (
	stRunning = "실행 중"
	stDone    = "완료"
	stFailed  = "실패"
	stQueued  = "대기"
	stInterr  = "중단"
)

func agentState(a *agentRec) string {
	switch {
	case a.Failed:
		return stFailed
	case a.Interrupted:
		return stInterr
	case a.Done:
		return stDone
	case a.Started.IsZero():
		return stQueued
	}
	return stRunning
}

// agentDot: colored ● per state (running yellow, done green, failed red, queued gray).
func agentDot(state string) string {
	c := map[string]string{stRunning: "220", stDone: "42", stFailed: "196", stQueued: "245", stInterr: "208"}[state]
	return lipgloss.NewStyle().Foreground(lipgloss.Color(c)).Render("●")
}

// agentStatusText: `상태-모델-세션이름(작업 요약 1문장)`; summary = Agent call's description
// (else the first prompt line), model falls back to the session model.
func agentStatusText(a *agentRec, sessionModel string) string {
	model, name, sum := a.Model, a.Name, a.Task
	if model == "" {
		model = sessionModel
	}
	if name == "" {
		name = "agent"
	}
	if sum == "" {
		sum = a.Prompt
	}
	if hasModelPrefix(name) {
		return agentState(a) + "-" + name + "(" + sum + ")"
	}
	return agentState(a) + "-" + model + "-" + name + "(" + sum + ")"
}

// hasModelPrefix: name already starts with a model id (e.g. subagent_type "haiku-scout"), so the
// model segment would double it.
func hasModelPrefix(name string) bool {
	for _, p := range []string{"haiku-", "opus-", "sonnet-", "fable-"} {
		if strings.HasPrefix(name, p) {
			return true
		}
	}
	return false
}
