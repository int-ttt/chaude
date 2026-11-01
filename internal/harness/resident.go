package harness

import (
	_ "embed"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"
	"unicode/utf8"
)

// prompterRule is the resident prompter's rules: its system prompt (Minimal slot, no tools).
//
//go:embed rules/resident-prompter.md
var prompterRule string

// residentMaxWorkers is the worker cap used when --max-workers and CHAUDE_MAX_WORKERS are unset.
// The resident slots are parked children and count against admit(); with the default 3 a spawn
// would evict them.
const residentMaxWorkers = 8

// beginBusy marks a resident child as handling a command: it shows as one running-list row until endBusy.
func (c *liveChild) beginBusy(prompt string, now time.Time) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.busy = &agentRec{ID: c.Role, Name: c.Role, Model: c.Model, Prompt: firstLine(prompt), Started: now}
}

// endBusy clears the command marker; the child drops out of the running list.
func (c *liveChild) endBusy() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.busy = nil
}

// busyRec returns the running-list row of a busy child, nil when idle.
func (c *liveChild) busyRec() *agentRec {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.busy
}

// residentBusy: parked resident children that are handling a command, oldest first. Idle ones are not listed.
func (r *Repl) residentBusy() []*agentRec {
	if r.children == nil {
		return nil
	}
	var l []*agentRec
	r.children.mu.Lock()
	for _, c := range r.children.m {
		if c == nil {
			continue
		}
		if b := c.busyRec(); b != nil {
			l = append(l, b)
		}
	}
	r.children.mu.Unlock()
	sort.Slice(l, func(i, j int) bool {
		if l[i].Started.Equal(l[j].Started) {
			return l[i].Name < l[j].Name
		}
		return l[i].Started.Before(l[j].Started)
	})
	return l
}

// residentSlot is one named child of the resident set. All slots are spawned with the REPL and parked.
// The "main" slot is the active child.
type residentSlot struct {
	Role      string
	Model     string
	Agent     string // agent rules file name (haiku-tui, ...); "" for none
	Rule      string // inline rules body; used instead of Agent when set
	Minimal   bool   // no tools, MCP or skills; the rules replace the default system prompt
	Tools     string // non-empty: --tools <Tools>
	StrictMCP bool   // --strict-mcp-config
	NoSlash   bool   // --disable-slash-commands
}

// residentSlots returns the four resident slots: main (the active child, on mainModel), prompter, tui
// and coder (all haiku). The coder is one resident session, kept for long context. The prompter and tui
// are minimal: no tools, no MCP, no skills.
func residentSlots(mainModel string) []residentSlot {
	return []residentSlot{
		{Role: "main", Model: mainModel},
		{Role: "prompter", Model: "haiku", Rule: prompterRule, Minimal: true},
		{Role: "tui", Model: "haiku", Agent: "haiku-tui", Minimal: true},
		{Role: "coder", Model: "haiku", Agent: "haiku-coder", Tools: "Read,Edit,Write,Bash,Grep,Glob", StrictMCP: true, NoSlash: true},
	}
}

// residentSlotByRole finds the slot with role in r.resident.
func (r *Repl) residentSlotByRole(role string) (residentSlot, bool) {
	for _, s := range r.resident {
		if s.Role == role {
			return s, true
		}
	}
	return residentSlot{}, false
}

// spawnSlot starts a claude child for slot through admit() and newSessionFn, and parks it under
// its (cwd, model, role) key.
func (r *Repl) spawnSlot(slot residentSlot) (*liveChild, error) {
	if r.children == nil {
		return nil, errors.New("resident slots need the client registry (--clients-file)")
	}
	if err := r.admit(); err != nil {
		return nil, err
	}
	agent := slot.Agent
	if agent != "" && slot.Rule == "" {
		if _, err := agentBody(r.a.Cwd, agent); err != nil {
			r.notice("에이전트 규칙 없음(" + agent + "): 기본 세션으로 시작")
			agent = ""
		}
	}
	s, err := newSessionFn(SessionOpts{Model: slot.Model, Cwd: r.a.Cwd, CompactWindow: r.a.CompactWindow, CacheTTL: r.a.CacheTTL,
		Agent: agent, Rule: slot.Rule, Minimal: slot.Minimal, Tools: slot.Tools, StrictMCP: slot.StrictMCP, NoSlash: slot.NoSlash})
	if err != nil {
		return nil, err
	}
	c := &liveChild{Sess: s, Tracker: newAgentTracker(), Model: slot.Model, Role: slot.Role}
	c.setLastAPI(time.Now())
	c.parked.Store(true)
	s.StartIdle(func(ev event) { r.parkedIdleEvent(c, ev) }) // parked children have no turn: read stdout here
	r.children.Park(childKey(r.a.Cwd, slot.Model, slot.Role), c)
	return c, nil
}

// residentRoleKey maps a resident role to its status-line slot: prompter P, tui T, coder C.
// "" for roles without a slot (main).
func residentRoleKey(role string) string {
	switch role {
	case "prompter":
		return "P"
	case "tui":
		return "T"
	case "coder":
		return "C"
	}
	return ""
}

// residentChild returns the parked child of resident slot role, left in the set. False when absent or dead.
func (r *Repl) residentChild(role string) (*liveChild, bool) {
	slot, ok := r.residentSlotByRole(role)
	if !ok || r.children == nil {
		return nil, false
	}
	r.children.mu.Lock()
	c := r.children.m[childKey(r.a.Cwd, slot.Model, role)]
	r.children.mu.Unlock()
	return c, c != nil && !childDead(c)
}

// sendResident writes msg to the parked resident child of role as one user message and marks it busy
// with label until its result. The returned channel gets the reply text of that result (buffered, one value).
func (r *Repl) sendResident(role, msg, label string) (<-chan string, error) {
	c, ok := r.residentChild(role)
	if !ok {
		return nil, fmt.Errorf("no live resident %q", role)
	}
	s, ok := c.Sess.(*Session)
	if !ok || s == nil {
		return nil, fmt.Errorf("resident %q has no session", role)
	}
	ch := make(chan string, 1)
	c.mu.Lock()
	c.await = ch
	c.mu.Unlock()
	c.beginBusy(label, time.Now()) // before the write: the result may arrive right after it
	if err := s.sendFrame(msg); err != nil {
		c.endBusy()
		c.mu.Lock()
		c.await = nil
		c.mu.Unlock()
		return nil, err
	}
	return ch, nil
}

// agentBody returns the rules of agent name: the body of <cwd>/.claude/agents/<name>.md, else of
// ~/.claude/agents/<name>.md, without its YAML frontmatter. An error when neither file yields a body.
func agentBody(cwd, name string) (string, error) {
	paths := []string{filepath.Join(cwd, ".claude", "agents", name+".md")}
	if home, err := os.UserHomeDir(); err == nil {
		paths = append(paths, filepath.Join(home, ".claude", "agents", name+".md"))
	}
	for _, p := range paths {
		b, err := os.ReadFile(p)
		if err != nil {
			continue
		}
		if body := stripFrontmatter(string(b)); body != "" {
			return body, nil
		}
	}
	return "", fmt.Errorf("no agent file %q", name)
}

// stripFrontmatter drops the lines up to and including the second `---` line when the first line is `---`.
func stripFrontmatter(s string) string {
	lines := strings.SplitAfter(s, "\n")
	if len(lines) == 0 || strings.TrimSpace(lines[0]) != "---" {
		return strings.TrimSpace(s)
	}
	for i := 1; i < len(lines); i++ {
		if strings.TrimSpace(lines[i]) == "---" {
			return strings.TrimSpace(strings.Join(lines[i+1:], ""))
		}
	}
	return strings.TrimSpace(s)
}

// stage shows one dim progress line for a pipeline stage: a stage note in the TUI, a dim line on stdout
// when headless. Safe to call from any goroutine (the TUI send is).
func (r *Repl) stage(s string) {
	if r.ui != nil {
		r.ui.send(stageMsg(s))
		return
	}
	r.p(dim+s+rst, "\n")
}

// prompterTimeout bounds the wait for the resident prompter's summary; on timeout the input goes unchanged.
var prompterTimeout = 60 * time.Second

// tuiTimeout bounds the wait for the resident tui's Korean answer; on timeout the main reply is shown as is.
var tuiTimeout = 120 * time.Second

// notice shows a pipeline failure: a note line in the TUI, a dim line on stdout when headless.
func (r *Repl) notice(s string) {
	if r.ui != nil {
		r.ui.send(noteMsg(s))
		return
	}
	r.p(dim+s+rst, "\n")
}

// viaPrompter returns the text Opus main receives: the resident prompter's summary, and the original
// input when the prompter asks for it or needOriginal(text) says the summary could lose meaning. Without a
// live prompter, or when it does not answer in time, text is returned unchanged and a notice says why.
// With no residents configured the text is returned without a notice.
func (r *Repl) viaPrompter(text string) string {
	if len(r.resident) == 0 {
		return text
	}
	r.stage("· prompter 처리중")
	ch, err := r.sendResident("prompter", text, text)
	if err != nil {
		r.notice("prompter 실패(오류: " + err.Error() + "): 원문 그대로 메인에 보냄")
		return text
	}
	select {
	case reply := <-ch:
		if reply == "" {
			r.notice("prompter 실패(빈 응답): 원문 그대로 메인에 보냄")
			return text
		}
		flag, sum := parsePrompterReply(reply)
		if sum == "" {
			return text
		}
		r.stage("· prompter 완료")
		out := "[prompter summary]\n" + sum
		if flag || needOriginal(text) {
			out += "\n\n[original]\n" + text
		}
		return out
	case <-time.After(prompterTimeout):
		r.notice(fmt.Sprintf("prompter 실패(%g초 시간초과): 원문 그대로 메인에 보냄", prompterTimeout.Seconds()))
		return text
	}
}

// parsePrompterReply splits the prompter reply: a first non-empty line NEED_ORIGINAL (flag set) or OK is
// dropped; any other reply is the summary whole. The summary is trimmed; "" when nothing is left.
func parsePrompterReply(reply string) (bool, string) {
	s := strings.TrimSpace(reply)
	head, rest, _ := strings.Cut(s, "\n")
	switch strings.TrimSpace(head) {
	case "NEED_ORIGINAL":
		return true, strings.TrimSpace(rest)
	case "OK":
		return false, strings.TrimSpace(rest)
	}
	return false, s
}

var (
	needPathRe  = regexp.MustCompile(`(?:[~.]?/?[\w.-]+/[\w./-]+)|\b[\w-]+\.(?:go|py|md|js|ts|tsx|json|ya?ml|toml|sh|txt|rs|java|c|h|cpp|html|css)\b`)
	needDigitRe = regexp.MustCompile(`[0-9]`)
	needQuotes  = "`\"“”「『"
	needWords   = []string{"아니", "근데", "말고", "정정", "다시 말", "번역", "translate", "문구", "표현", "말투", "어투", "wording", "문장", "그대로", "원문"}
)

// needOriginal reports whether input is too exact for a summary alone: quotes or code, many lines or
// long text, digits, file paths, a correction, or a translation/wording request.
func needOriginal(s string) bool {
	if strings.ContainsAny(s, needQuotes) || strings.Count(s, "\n") >= 3 || utf8.RuneCountInString(s) > 400 {
		return true
	}
	if needDigitRe.MatchString(s) || needPathRe.MatchString(s) {
		return true
	}
	lower := strings.ToLower(s)
	for _, w := range needWords {
		if strings.Contains(lower, w) {
			return true
		}
	}
	return false
}

// viaTUI turns the main reply into the Korean answer the user sees, through the resident tui (haiku-tui).
// On failure the main reply is shown as is, with a notice. An empty reply gives an empty answer.
func (r *Repl) viaTUI(reply string) string {
	if reply == "" {
		return ""
	}
	prompt := "Opus 메인 반환입니다. relay 필드가 있으면 relay를, 없으면 summary_ko(없으면 본문)를 근거로 사용자에게 보일 한국어 존댓말 답변만 출력하세요. 새 사실을 더하지 마세요.\n\n" + reply
	r.stage("· tui 번역중")
	ch, err := r.sendResident("tui", prompt, "tui")
	if err != nil {
		r.notice("haiku-tui 실패(오류: " + err.Error() + "): 메인 원문 표시")
		return reply
	}
	select {
	case out := <-ch:
		if out == "" {
			r.notice("haiku-tui 실패(빈 응답): 메인 원문 표시")
			return reply
		}
		r.stage("· tui 완료")
		return out
	case <-time.After(tuiTimeout):
		r.notice(fmt.Sprintf("haiku-tui 실패(%g초 시간초과): 메인 원문 표시", tuiTimeout.Seconds()))
		return reply
	}
}

// spawnResident starts the non-main slots and parks them. Failures are collected; the other slots are still tried.
func (r *Repl) spawnResident() error {
	var errs []error
	for _, slot := range r.resident {
		if slot.Role == "main" {
			continue
		}
		if _, err := r.spawnSlot(slot); err != nil {
			errs = append(errs, fmt.Errorf("%s: %w", slot.Role, err))
		}
	}
	return errors.Join(errs...)
}
