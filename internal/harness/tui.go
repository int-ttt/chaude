package harness

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"sort"
	"strings"
	"sync/atomic"
	"time"

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
	doneMsg       struct{}
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

type block struct{ kind, text string } // kind: user | asst | tool | result | note

type tuiModel struct {
	r        *Repl
	submit   chan string
	w, h     int
	vp       viewport.Model
	in       inputBox
	inH      int
	sp       spinner.Model
	blocks   []block
	busy     bool
	gotText  bool
	perm     *permMsg
	pick     []sessionInfo // non-nil while the /resume picker is open
	pickSel  int
	ready    bool
	quitting bool
	sel      int
	curCol   atomic.Int32
	curUp    atomic.Int32
	popH     int
	unseen   bool
	sig      int
	mouse    bool
}

// slash commands (Claude Code style)
type slashCmd struct{ name, desc string }

var slashCmds = []slashCmd{
	{"/help", "show commands and keys"},
	{"/clear", "start a fresh session (no handoff)"},
	{"/status", "model, context tokens, session"},
	{"/model", "show or switch model: /model <name>"},
	{"/resume", "list Claude Code sessions and import one"},
	{"/import", "import a session: /import [id|latest]"},
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
	{"/mouse", "toggle mouse wheel scroll (off = select text)"},
	{"/version", "harness and claude versions"},
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
	sp.Spinner = spinner.Spinner{Frames: []string{"✻", "✽", "✶", "✳", "✢", "·"}, FPS: time.Second / 8}
	sp.Style = lipgloss.NewStyle().Foreground(orange)
	return &tuiModel{r: r, submit: submit, sp: sp, mouse: true}
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
		dimSt.Render(fmt.Sprintf("  model: %s · rotate at %s tokens", m.r.a.Model, commas(m.r.a.Threshold)))
	return lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(orange).Padding(0, 1).Width(w).Render(body)
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
			out = append(out, indent(wrap(b.text, 2), lipgloss.NewStyle().Foreground(orange).Render("● "), "  "), "")
		case "tool":
			out = append(out, indent(wrap(b.text, 2), lipgloss.NewStyle().Foreground(lipgloss.Color("#5FAF87")).Render("● ")+"", "  "))
		case "result":
			out = append(out, dimSt.Render(indent(wrap(b.text, 5), "  ⎿  ", "     ")), "")
		case "note":
			out = append(out, dimSt.Render(wrap(b.text, 0)), "")
		}
	}
	if m.busy && !m.gotText && m.perm == nil {
		out = append(out, m.sp.View()+dimSt.Render(" Working… (esc to interrupt)"))
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
	m.vp.SetContent(m.render())
	if atBottom {
		m.vp.GotoBottom()
		m.unseen = false
	} else if sig != m.sig {
		m.unseen = true
	}
	m.sig = sig
}

func (m *tuiModel) layout() {
	m.fit()
	m.inH = m.in.Height(maxInputRows)
	vh := m.h - m.inH - 2 - 1 - m.popH // input box + status line + suggestions
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
	if n := len(m.blocks); n > 0 && m.blocks[n-1].kind == "asst" && m.gotText {
		m.blocks[n-1].text += s
		return
	}
	m.blocks = append(m.blocks, block{"asst", s})
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
		if m.busy {
			m.refresh()
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
		m.blocks = append(m.blocks, block{"tool", t})
		m.refresh()
	case toolResultMsg:
		s := strings.TrimSpace(string(v))
		lines := strings.Split(s, "\n")
		t := trunc(lines[0], 160)
		if len(lines) > 1 {
			t += fmt.Sprintf(" … (+%d lines)", len(lines)-1)
		}
		m.blocks = append(m.blocks, block{"result", t})
		m.refresh()
	case noteMsg:
		m.gotText = false
		m.blocks = append(m.blocks, block{"note", string(v)})
		m.refresh()
	case permMsg:
		m.perm = &v
		m.blocks = append(m.blocks, block{"note", fmt.Sprintf("Allow %s(%s)?  [y] yes  [n] no", v.tool, v.arg)})
		m.refresh()
	case doneMsg:
		m.busy, m.gotText = false, false
		m.refresh()
	case tea.MouseMsg:
		var c tea.Cmd
		m.vp, c = m.vp.Update(v)
		cmds = append(cmds, c)
	case tea.KeyMsg:
		return m.key(v)
	}
	if !m.quitting {
	}
	m.after()
	return m, tea.Batch(cmds...)
}

func (m *tuiModel) interrupt() {
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
			m.blocks = append(m.blocks, block{"note", "allowed"})
			m.perm = nil
		case "n", "esc":
			m.perm.reply <- false
			m.blocks = append(m.blocks, block{"note", "denied"})
			m.perm = nil
		case "ctrl+c":
			m.perm.reply <- false
			m.perm = nil
			m.interrupt()
		}
		m.refresh()
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
			id := m.pick[m.pickSel].ID
			m.pick = nil
			m.busy = true
			m.blocks = append(m.blocks, block{"user", "/import " + id})
			m.vp.GotoBottom()
			m.refresh()
			m.submit <- "/import " + id
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
	case "esc":
		if m.busy {
			m.interrupt()
		}
		return m, nil
	case "tab":
		if sg := m.suggestions(); len(sg) > 0 {
			m.in.SetValue(sg[min(m.sel, len(sg)-1)].name + " ")
			m.after()
		}
		return m, nil
	case "up", "down":
		if n := len(m.suggestions()); n > 0 {
			if k.String() == "up" {
				m.sel = (m.sel + n - 1) % n
			} else {
				m.sel = (m.sel + 1) % n
			}
		} else if k.String() == "up" {
			m.in.MoveRow(-1)
		} else {
			m.in.MoveRow(1)
		}
		m.after()
		return m, nil
	case "alt+enter", "ctrl+j":
		m.in.Insert([]rune{'\n'})
		m.after()
		return m, nil
	case "pgup", "pgdown", "ctrl+b", "ctrl+f":
		var c tea.Cmd
		m.vp, c = m.vp.Update(k)
		return m, c
	case "shift+up":
		m.vp.LineUp(1)
		return m, nil
	case "shift+down":
		m.vp.LineDown(1)
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
		if m.busy {
			return m, nil
		}
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
		m.in.SetValue("")
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
		m.blocks = append(m.blocks, block{"user", line})
		m.busy = true
		m.vp.GotoBottom()
		m.refresh()
		m.submit <- line
		return m, nil
	}
	m.sel = 0
	m.edit(k)
	m.after()
	return m, nil
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
		m.in.Backspace()
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
	case tea.KeyCtrlU:
		m.in.SetValue("")
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
	m.curCol.Store(int32(1 + 2 + col + 1))    // border + prompt "> " + text; 1-based column
	m.curUp.Store(int32(m.inH - 1 - vis + 2)) // rows below cursor in box + bottom border + status line
}

func (m *tuiModel) after() {
	m.fit()
	m.syncCursor()
	if n := min(len(m.suggestions()), maxPop); n != m.popH || m.in.Height(maxInputRows) != m.inH {
		m.popH = n
		m.layout()
		m.syncCursor()
	}
}

func (m *tuiModel) runSlash(line string) (tea.Model, tea.Cmd) {
	f := strings.Fields(line)
	note := func(s string) { m.blocks = append(m.blocks, block{"note", s}); m.refresh() }
	switch f[0] {
	case "/quit", "/exit":
		return m.quit()
	case "/help":
		var l []string
		for _, c := range slashCmds {
			l = append(l, fmt.Sprintf("%-8s %s", c.name, c.desc))
		}
		note(strings.Join(l, "\n") + "\nesc interrupt · y/n permission · pgup/pgdn scroll · tab complete")
	case "/mouse":
		m.mouse = !m.mouse
		if m.mouse {
			note("mouse wheel scroll ON (shift+drag selects text in most terminals)")
			return m, tea.EnableMouseCellMotion
		}
		note("mouse OFF: text selection works; scroll with pgup/pgdn, shift+up/down")
		return m, tea.DisableMouse
	case "/status":
		sid := m.r.sess.SessionID
		if sid == "" {
			sid = "-"
		}
		note(fmt.Sprintf("model %s · ctx %s/%s · rotations %d · session %s", m.r.a.Model, commas(m.r.lastCtx), commas(m.r.a.Threshold), m.r.rotations, trunc(sid, 8)))
	case "/model":
		if len(f) < 2 {
			note("model " + m.r.a.Model + "  (switch: /model <name>, hands off context to the new session)")
			break
		}
		m.busy = true
		m.blocks = append(m.blocks, block{"user", line})
		m.vp.GotoBottom()
		m.refresh()
		m.submit <- line
	case "/resume":
		l, msg := m.r.sessionList()
		if msg != "" {
			note(msg)
			break
		}
		if len(l) > 30 {
			l = l[:30]
		}
		m.pick, m.pickSel = l, 0
	case "/import":
		m.busy = true
		m.blocks = append(m.blocks, block{"user", line})
		m.vp.GotoBottom()
		m.refresh()
		m.submit <- line
	case "/version":
		v, _ := exec.Command("claude", "--version").Output()
		note("harness (Go REPL) · claude " + strings.TrimSpace(string(v)))
	case "/clear", "/rotate", "/usage", "/cost", "/context", "/compact", "/config", "/mcp", "/agents", "/init", "/review", "/doctor":
		if f[0] == "/clear" {
			m.blocks = nil
		}
		m.busy = true
		m.blocks = append(m.blocks, block{"user", line})
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

func (m *tuiModel) quit() (tea.Model, tea.Cmd) {
	m.quitting = true
	return m, tea.Quit
}

func (m *tuiModel) statusLine() string {
	ctx := m.r.lastCtx
	left := fmt.Sprintf(" %s · ctx %s/%s · rot %d", m.r.a.Model, commas(ctx), commas(m.r.a.Threshold), m.r.rotations)
	right := "esc interrupt · /rotate · wheel/pgup/pgdn scroll · ctrl+c quit "
	if m.perm != nil {
		right = "y allow · n deny "
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
	bc := grey
	if m.perm != nil {
		bc = orange
	}
	box := lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(bc).Width(m.w - 2).Render(m.in.View(maxInputRows, "Try \"how does <file> work?\"", dimSt))
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
	vv := m.vp.View()
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
	if m.unseen && m.vp.AtBottom() {
		m.unseen = false
	}
	if m.unseen {
		ls := strings.Split(vv, "\n")
		tag := lipgloss.NewStyle().Foreground(orange).Render("↓ 새 메시지 (End: 맨 아래로)")
		ls[len(ls)-1] = strings.Repeat(" ", max(0, m.w-lipgloss.Width(tag)-1)) + tag
		vv = strings.Join(ls, "\n")
	}
	return vv + "\n" + pop + box + "\n" + m.statusLine()
}

// runTUI is the interactive entry: same Repl, rendered with bubbletea in the alt screen.
func (r *Repl) runTUI() int {
	if err := r.start(); err != nil {
		fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
		return 1
	}
	defer func() {
		if r.sess != nil {
			r.closeDrains()
			r.sess.Close()
		}
	}()
	submit := make(chan string)
	m := newTUIModel(r, submit)
	cw := &cursorWriter{File: os.Stdout, w: os.Stdout, col: &m.curCol, up: &m.curUp}
	lipgloss.SetHasDarkBackground(true) // skip the slow OSC background query at startup
	p := tea.NewProgram(m, tea.WithAltScreen(), tea.WithMouseCellMotion(), tea.WithOutput(cw))
	r.ui = &tuiBridge{p}
	r.out = tuiWriter{r.ui}
	go func() {
		for line := range submit {
			f := strings.Fields(line)
			switch {
			case line == "/rotate":
				r.rotate("manual")
			case line == "/clear":
				r.clear()
			case passthroughCmds[f[0]]:
				r.passthrough(line)
			case f[0] == "/import":
				a := "latest"
				if len(f) > 1 {
					a = f[1]
				}
				r.importSession(a)
			case f[0] == "/model" && len(f) > 1:
				r.a.Model = f[1]
				r.rotate("model " + f[1])
			default:
				r.handle(line)
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
