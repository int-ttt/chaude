// Package harness: terminal REPL over `claude -p` stream-json with session rotation (port of harness/*.py).
package harness

import (
	"bufio"
	"flag"
	"fmt"
	"io"
	"os"
	"os/signal"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"time"

	"chaude/internal/lean"
	"chaude/internal/osenv"
)

const (
	dim = "\033[2m"
	rst = "\033[0m"

	handoffPrompt = "Write a handoff note for a fresh session that will continue this work. No tools. " +
		"Sections: Goal; Decisions; Files created/changed; In-progress work; Open questions; " +
		"User preferences/instructions; Any exact codes, names or numbers the user gave. Be exact and brief."
	preamble = "[Context carried over from a previous session that was rotated to save tokens. " +
		"Treat it as your memory of the conversation so far; do not mention the rotation.]"
	delegationRule = "Default to delegating bounded code changes (implementation steps, small fixes, wiring, tests) to the haiku-coder agent with an exact spec; " +
		"implement yourself only design-level or tightly coupled parts, then verify Haiku's diff. " +
		"Log each delegation (agent, tokens, duration, outcome, retries)."
)

// Args mirrors the argparse flags.
type Args struct {
	Model            string
	Cwd              string
	Threshold        int
	CompactWindow    int
	MaxAutoRotations int // -1 = unlimited (python None)
	Transcript       string
	TailTurns        int
	SnapshotChars    int
	AutoAllow        bool
	ShowRotation     bool
	MetricsDir       string
	AgentsFile       string        // persisted sub-agent state (empty=off)
	NoValidate       bool          // disable handoff validation (safety guard off)
	MaxDrain         int           // max concurrently draining old sessions
	DrainTimeout     time.Duration // give up waiting for a draining session's agents
	Cooldown         time.Duration // min gap after a failed rotation
	FromClaude       string        // import a plain Claude Code transcript: session id | path | latest
	ImportDry        bool          // with FromClaude: print the handoff stats and exit
	NoCategoryRotate bool          // disable category-based rotation (no marker rule, no switch rotation)
	Conductor        bool          // Haiku-fixed main orchestrates; large work: Haiku plans -> Opus/Sonnet review -> Haiku executes
	NoDelegationRule bool          // do not append the haiku delegation rule to the child's system prompt
	ModeName         string        // --mode: permission mode to start in ("" = the one saved for this folder)
}

type kv struct {
	k string
	v any
}

// Transcript is an own append-only JSONL, one thread across any number of claude sessions.
type Transcript struct {
	path  string
	turns [][2]string
}

func newTranscript(path string) *Transcript {
	abs, err := filepath.Abs(path)
	if err != nil {
		abs = path
	}
	_ = os.MkdirAll(filepath.Dir(abs), 0o755)
	return &Transcript{path: path}
}

// add writes {"t":..., "kind":..., <fields in order>} as one line.
func (t *Transcript) add(kind string, fields ...kv) {
	var b strings.Builder
	b.WriteString(fmt.Sprintf(`{"t": %s, "kind": %s`, fmtFloat(float64(time.Now().UnixNano())/1e9), jsonStr(kind)))
	for _, f := range fields {
		b.WriteString(", " + jsonStr(f.k) + ": " + jsonStr(f.v))
	}
	b.WriteString("}\n")
	fh, err := os.OpenFile(t.path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return
	}
	defer fh.Close()
	_, _ = fh.WriteString(b.String())
}

func fmtFloat(f float64) string { return fmt.Sprintf("%.6f", f) }

func commas(n int) string {
	s := fmt.Sprint(n)
	neg := strings.HasPrefix(s, "-")
	if neg {
		s = s[1:]
	}
	var out []byte
	for i, c := range []byte(s) {
		if i > 0 && (len(s)-i)%3 == 0 {
			out = append(out, ',')
		}
		out = append(out, c)
	}
	if neg {
		return "-" + string(out)
	}
	return string(out)
}

func trunc(s string, n int) string {
	r := []rune(s)
	if len(r) > n {
		return string(r[:n])
	}
	return s
}

type Repl struct {
	a               Args
	out             io.Writer
	in              *bufio.Reader
	interactive     bool
	intr            chan struct{}
	tr              *Transcript
	sess            *Session
	pendingSnapshot string
	rotations       int
	files           []string
	lastCtx         int
	rate            map[string]any
	rate0           map[string]any // rate_limit_info at the start of the current task
	rateSeq         int            // bumped per rate_limit_event; a task delta is shown only if one arrived
	rateSeq0        int
	ui              *tuiBridge // non-nil in TUI mode
	seenTools       map[string]bool
	metrics         *Metrics
	tracker         *agentTracker
	agents          *agentStore // on-disk agent state (nil/empty path = off)
	draining        []*drain
	lastReply       string
	lastReco        string
	lastRotFail     time.Time
	notices         []string
	drainMu         sync.Mutex
	cat             string            // current task category from the child's [cat: X] marker
	delegs          int               // Agent/Task tool calls in the current turn
	pool            *workerPool       // Haiku sessions with edit history (direct routing candidates)
	parked          map[string]string // model -> session id left by a direct Haiku switch
	resumeID        string            // session id start() resumes once
	curTask         string            // terse task of the current routed turn
	mode            atomic.Int32      // permMode, set from the UI goroutine
	idle            idleTurn          // child-initiated turn being rendered (idle reader goroutine only)
}

func (r *Repl) p(s string, end string) {
	fmt.Fprint(r.out, s+end)
}

// newSessionFn is a test seam for start().
var newSessionFn = NewSession

func (r *Repl) start() error {
	opts := SessionOpts{Model: r.a.Model, Cwd: r.a.Cwd, CompactWindow: r.a.CompactWindow, Resume: r.resumeID}
	r.resumeID = ""
	rule := ""
	if !r.a.NoDelegationRule {
		rule = delegationRule
	}
	if !r.a.NoCategoryRotate {
		rule = strings.TrimSpace(rule + " " + categoryRule)
	}
	if rule != "" {
		opts.Extra = []string{"--append-system-prompt", rule}
	}
	s, err := newSessionFn(opts)
	if err != nil {
		return err
	}
	r.sess = s
	prev := r.tracker
	r.tracker = newAgentTracker()
	tk := r.tracker
	tk.store = r.agents
	tk.parent = s.ID
	tk.transcript = func() string {
		dir, err := claudeProjectDir(r.a.Cwd)
		id := s.ID()
		if err != nil || id == "" {
			return ""
		}
		return filepath.Join(dir, id+".jsonl")
	}
	if prev != nil { // keep restored (interrupted) agents visible across rotation
		for _, a := range prev.sortedRestored() {
			tk.restore(a.saved("", ""))
		}
	}
	s.StartIdle(func(ev event) { r.idleEvent(s, tk, ev) })
	return nil
}

// idleEvent handles stdout events that arrive while no turn is running (background subagents).
func (r *Repl) idleEvent(s *Session, tk *agentTracker, ev event) {
	tk.onSubEvent(ev)
	switch t := str(ev["type"]); {
	case t == "control_request" && str(asMap(ev["request"])["subtype"]) == "can_use_tool":
		r.tr.add("permission_idle", kv{"tool", asMap(ev["request"])["tool_name"]})
		r.askPerm(s, ev, true)
	case t == "rate_limit_event":
		r.rate, r.rateSeq = asMap(ev["rate_limit_info"]), r.rateSeq+1
	case t == "stream_event" && isMainEvent(ev):
		e := asMap(ev["event"])
		d := asMap(e["delta"])
		if str(e["type"]) == "content_block_delta" && str(d["type"]) == "text_delta" {
			r.idleText(str(d["text"]))
			r.idle.streamed = true
		}
	case t == "assistant":
		m := asMap(ev["message"])
		main := isMainEvent(ev)
		if u := asMap(m["usage"]); main && len(u) > 0 {
			r.idle.usage = u
		}
		for _, bi := range asList(m["content"]) {
			b := asMap(bi)
			switch str(b["type"]) {
			case "tool_use":
				tk.onToolUse(str(b["id"]), str(b["name"]), asMap(b["input"]), time.Now())
			case "text":
				if main && !r.idle.streamed {
					r.idleText(str(b["text"]))
				}
			}
		}
		if main {
			r.idle.active = true
		}
	case t == "user":
		if c, ok := asMap(ev["message"])["content"].(string); ok && tk.onNotification(c) {
			r.idle.sawNotif = true
		}
		for _, bi := range asList(asMap(ev["message"])["content"]) {
			b := asMap(bi)
			switch str(b["type"]) {
			case "tool_result":
				tk.onToolResult(str(b["tool_use_id"]))
			case "text":
				if tk.onNotification(str(b["text"])) {
					r.idle.sawNotif = true
				}
			}
		}
	case t == "result":
		r.idleFinish(s, tk, ev)
	}
}

// idleTurn accumulates a child-initiated main-thread turn (e.g. the reply to a background
// agent's completion notice) that arrives while no user turn is running.
type idleTurn struct {
	reply    strings.Builder
	usage    map[string]any
	t0       time.Time
	active   bool
	streamed bool
	sawNotif bool
}

func isMainEvent(ev event) bool {
	p, ok := ev["parent_tool_use_id"]
	return !ok || p == nil || p == ""
}

func (r *Repl) idleText(txt string) {
	if txt == "" {
		return
	}
	if r.idle.reply.Len() == 0 {
		r.idle.t0 = time.Now()
		if r.ui == nil {
			r.p("\n", "")
		}
	}
	r.idle.reply.WriteString(txt)
	if r.ui != nil {
		r.ui.send(textMsg(txt))
	} else {
		r.p(txt, "")
	}
}

// idleFinish ends a child-initiated turn: logs it like a normal turn and settles the agents counter.
func (r *Repl) idleFinish(s *Session, tk *agentTracker, result event) {
	st := r.idle
	r.idle = idleTurn{}
	reply := st.reply.String()
	if reply == "" && !st.active {
		return // late result of work already shown elsewhere
	}
	if !st.sawNotif {
		tk.finishOldestBackground() // the child only speaks up by itself for a finished background agent
	}
	if reply != "" {
		if r.ui != nil {
			r.ui.send(textEndMsg{})
		} else {
			r.p("", "\n")
		}
	}
	var sid, ru, cost any
	if s.SessionID != "" {
		sid = s.SessionID
	}
	ru, cost = result["usage"], result["total_cost_usd"]
	r.tr.add("assistant", kv{"text", reply}, kv{"session", sid}, kv{"usage", ru},
		kv{"ctx", ctxTokens(st.usage)}, kv{"cost", cost}, kv{"source", "background"})
	r.usageLine(result, st.usage)
	t0 := st.t0
	if t0.IsZero() {
		t0 = time.Now()
	}
	fields := map[string]any{"stage": "turn", "source": "background", "start_ms": t0.UnixMilli(), "end_ms": time.Now().UnixMilli(),
		"model": r.a.Model, "conductor": r.a.Conductor, "ctx": r.lastCtx, "session": s.SessionID, "cost_usd": cost, "rotations": r.rotations}
	for k, v := range usageFields(asMap(ru)) {
		fields["result_"+k] = v
	}
	_ = r.metrics.Log(fields)
}

func (r *Repl) askPermission(req event) { r.askPerm(r.sess, req, false) }

// askPerm answers a can_use_tool request on s. idle = no turn running: on the plain path stdin
// belongs to the prompt loop, so anything that needs asking is denied instead of hanging the child.
func (r *Repl) askPerm(sess *Session, req event, idle bool) {
	rq := asMap(req["request"])
	input := asMap(rq["input"])
	if input == nil {
		input = map[string]any{}
	}
	if v, msg := decideMode(permMode(r.mode.Load()), str(rq["tool_name"])); v != "ask" {
		r.tr.add("permission", kv{"tool", rq["tool_name"]}, kv{"allowed", v == "allow"}, kv{"mode", permMode(r.mode.Load()).String()})
		_ = sess.RespondPermission(str(req["request_id"]), v == "allow", asMap(rq["input"]), msg)
		return
	}
	if r.ui != nil {
		allowed := r.a.AutoAllow || r.ui.ask(nilStr(rq["tool_name"]), toolSummary(input))
		r.tr.add("permission", kv{"tool", rq["tool_name"]}, kv{"allowed", allowed})
		_ = sess.RespondPermission(str(req["request_id"]), allowed, asMap(rq["input"]), "")
		return
	}
	if idle && !r.a.AutoAllow {
		r.p(fmt.Sprintf("\n%s[permission denied while idle] %v %s (use /mode auto or acceptEdits)%s", dim, nilStr(rq["tool_name"]), trunc(jsonStr(input), 120), rst), "\n")
		r.tr.add("permission", kv{"tool", rq["tool_name"]}, kv{"allowed", false}, kv{"idle", true})
		_ = sess.RespondPermission(str(req["request_id"]), false, nil, "denied: no interactive prompt available between turns")
		return
	}
	r.p(fmt.Sprintf("\n%s[permission] %v %s%s", dim, nilStr(rq["tool_name"]), trunc(jsonStr(input), 200), rst), "\n")
	ans := "y"
	if !r.a.AutoAllow {
		r.p("allow? [y/N] ", "")
		line, _ := r.in.ReadString('\n')
		ans = strings.ToLower(strings.TrimSpace(line))
		if ans == "" {
			ans = "n"
		}
	}
	allowed := strings.HasPrefix(ans, "y")
	r.tr.add("permission", kv{"tool", rq["tool_name"]}, kv{"allowed", allowed})
	_ = sess.RespondPermission(str(req["request_id"]), allowed, asMap(rq["input"]), "")
}

func nilStr(v any) string {
	if v == nil {
		return "None"
	}
	return fmt.Sprint(v)
}

// runTurn sends text, streams the reply. Returns reply text, result event (nil if none), last main usage.
func (r *Repl) runTurn(text string, show bool) (string, event, map[string]any) {
	select { // drop a stale Ctrl-C from while idle
	case <-r.intr:
	default:
	}
	_ = r.sess.Send(text)
	var reply strings.Builder
	var mf markerFilter
	emit := func(txt string) {
		if txt == "" {
			return
		}
		if r.ui != nil {
			r.ui.send(textMsg(txt))
		} else {
			r.p(txt, "")
		}
	}
	var lastUsage map[string]any
	var result event
	handle := func(ev event) {
		t := str(ev["type"])
		r.tracker.onSubEvent(ev)
		_, hasParent := ev["parent_tool_use_id"]
		isMain := !hasParent || ev["parent_tool_use_id"] == nil || ev["parent_tool_use_id"] == ""
		switch {
		case t == "stream_event" && isMain:
			e := asMap(ev["event"])
			d := asMap(e["delta"])
			if str(e["type"]) == "content_block_delta" && str(d["type"]) == "text_delta" {
				txt := str(d["text"])
				reply.WriteString(txt)
				if show {
					if r.a.NoCategoryRotate {
						emit(txt)
					} else {
						emit(mf.Feed(txt))
					}
				}
			}
		case t == "assistant" && isMain:
			m := asMap(ev["message"])
			if u := asMap(m["usage"]); len(u) > 0 {
				lastUsage = u
			}
			blocks, _ := m["content"].([]any)
			for _, bi := range blocks {
				b := asMap(bi)
				if str(b["type"]) == "tool_use" {
					if n := str(b["name"]); n == "Agent" || n == "Task" {
						r.delegs++
					}
					r.tracker.onToolUse(str(b["id"]), str(b["name"]), asMap(b["input"]), time.Now())
				}
				if r.ui != nil && show && str(b["type"]) == "tool_use" && !r.seenTools[str(b["id"])] {
					r.seenTools[str(b["id"])] = true
					r.ui.send(toolMsg{name: str(b["name"]), arg: toolSummary(asMap(b["input"]))})
				}
				switch str(b["name"]) {
				case "Write", "Edit", "NotebookEdit":
					if str(b["type"]) != "tool_use" {
						continue
					}
					fp := str(asMap(b["input"])["file_path"])
					if fp != "" && !contains(r.files, fp) {
						r.files = append(r.files, fp)
					}
					if fp != "" && r.a.Model == "haiku" && r.sess != nil {
						r.pool.noteEdit(r.sess.SessionID, r.curTask, fp)
					}
				}
			}
		case t == "user" && isMain:
			for _, bi := range asList(asMap(ev["message"])["content"]) {
				if b := asMap(bi); str(b["type"]) == "tool_result" {
					r.tracker.onToolResult(str(b["tool_use_id"]))
				}
			}
			if r.ui == nil || !show {
				break
			}
			for _, bi := range asList(asMap(ev["message"])["content"]) {
				if b := asMap(bi); str(b["type"]) == "tool_result" {
					r.ui.send(toolResultMsg(resultText(b["content"])))
				}
			}
		case t == "control_request" && str(asMap(ev["request"])["subtype"]) == "can_use_tool":
			r.askPermission(ev)
		case t == "rate_limit_event":
			r.rate, r.rateSeq = asMap(ev["rate_limit_info"]), r.rateSeq+1
		case t == "result":
			result = ev
		case t == "_unparsed":
			r.tr.add("unparsed", kv{"raw", ev["raw"]})
		}
	}
	if r.sess.Events(300*time.Second, r.intr, handle) {
		_ = r.sess.Interrupt()
		r.p("\n"+dim+"[interrupted]"+rst, "\n")
		r.sess.Events(20*time.Second, nil, func(ev event) {
			if ev["type"] == "result" {
				result = ev
			}
		})
	}
	if show && !r.a.NoCategoryRotate {
		emit(mf.Flush())
	}
	if show && r.ui != nil {
		r.ui.send(textEndMsg{})
	} else if show {
		r.p("", "\n")
	}
	return reply.String(), result, lastUsage
}

func contains(l []string, s string) bool {
	for _, x := range l {
		if x == s {
			return true
		}
	}
	return false
}

// rateUtil returns a rate-limit window's utilization in percent (the event gives 0-1). key "fable"
// matches any window whose name contains "fable"; ok=false when the harness has no such data.
func rateUtil(rate map[string]any, key string) (float64, bool) {
	uw := asMap(rate["unifiedWindows"])
	for k, v := range uw {
		if k != key && !(key == "fable" && strings.Contains(strings.ToLower(k), "fable")) {
			continue
		}
		if u, ok := asMap(v)["utilization"].(float64); ok {
			return u * 100, true
		}
	}
	return 0, false
}

func pctStr(rate map[string]any, key string) string {
	if p, ok := rateUtil(rate, key); ok {
		return fmt.Sprintf("%.0f%%", p)
	}
	return "?"
}

func pctDelta(a, b map[string]any, key string) string {
	p0, ok0 := rateUtil(a, key)
	p1, ok1 := rateUtil(b, key)
	if !ok0 || !ok1 {
		return "?"
	}
	if p1 < p0 {
		return "reset"
	}
	return fmt.Sprintf("+%.1f%%", p1-p0)
}

func (r *Repl) usageLine(result event, usage map[string]any) {
	if result == nil {
		return
	}
	ctx := ctxTokens(usage)
	r.lastCtx = ctx
	ru := asMap(result["usage"])
	cost, _ := result["total_cost_usd"].(float64)
	sid := trunc(r.sess.SessionID, 8)
	if r.sess.SessionID == "" {
		sid = "None"
	}
	fable := ""
	if _, ok := rateUtil(r.rate, "fable"); ok {
		fable = " | Fable " + pctStr(r.rate, "fable")
	}
	r.p(fmt.Sprintf("%s[ctx %s | in %d cr %s cc %s out %d | $%.4f | 세션 %s | 주간 %s%s | sess %s rot %d]%s",
		dim, commas(ctx), num(ru["input_tokens"]), commas(num(ru["cache_read_input_tokens"])),
		commas(num(ru["cache_creation_input_tokens"])), num(ru["output_tokens"]), cost,
		pctStr(r.rate, "five_hour"), pctStr(r.rate, "seven_day"), fable, sid, r.rotations, rst), "\n")
}

// taskLine: rate-limit consumption of the task that just ran (window utilization before vs after).
// The event reports whole percents, so a short task shows +0.0%; Fable only when the plan reports such a window.
func (r *Repl) taskLine() {
	if r.rateSeq == r.rateSeq0 {
		r.p(dim+"[이번 작업 소모: 세션 ? | 주간 ?  (사용량 이벤트 없음)]"+rst, "\n")
		return
	}
	fable := ""
	if _, ok := rateUtil(r.rate, "fable"); ok {
		fable = " | Fable " + pctDelta(r.rate0, r.rate, "fable")
	}
	r.p(fmt.Sprintf("%s[이번 작업 소모: 세션 %s | 주간 %s%s]%s", dim, pctDelta(r.rate0, r.rate, "five_hour"),
		pctDelta(r.rate0, r.rate, "seven_day"), fable, rst), "\n")
}

// windowName: Korean label for a rate-limit window key (five_hour, seven_day, seven_day_opus, ...).
func windowName(k string) string {
	switch k {
	case "five_hour":
		return "세션(5시간)"
	case "seven_day":
		return "주간(7일)"
	}
	k = strings.ReplaceAll(k, "seven_day_", "주간 ")
	k = strings.ReplaceAll(k, "five_hour_", "세션 ")
	return k
}

// resetText: "MM-DD HH:MM (N시간 M분 후)" for a window reset time.
func resetText(t time.Time, now time.Time) string {
	d := t.Sub(now)
	s := t.Local().Format("01-02 15:04")
	if d <= 0 {
		return s + " (지남)"
	}
	h, m := int(d.Hours()), int(d.Minutes())%60
	if h >= 24 {
		return fmt.Sprintf("%s (%d일 %d시간 후)", s, h/24, h%24)
	}
	return fmt.Sprintf("%s (%d시간 %d분 후)", s, h, m)
}

// usageText is the `/사용량` answer: subscription windows from the last rate_limit_event the child sent
// (no model call, instant, works while an answer runs) plus the current context size.
func (r *Repl) usageText() string {
	var b strings.Builder
	b.WriteString("사용량 (구독 한도, 마지막 사용량 이벤트 기준)\n")
	uw := asMap(r.rate["unifiedWindows"])
	if len(uw) == 0 {
		b.WriteString("  아직 사용량 이벤트가 없습니다. 한 턴을 보내면 채워집니다.\n")
	}
	keys := make([]string, 0, len(uw))
	for k := range uw {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	now := time.Now()
	for _, k := range keys {
		w := asMap(uw[k])
		u, _ := w["utilization"].(float64)
		line := "  " + windowName(k) + "  " + renderBar(u*100)
		if t, ok := w["resetsAt"].(float64); ok && t > 0 {
			line += "  재설정 " + resetText(time.Unix(int64(t), 0), now)
		}
		if st := str(w["status"]); st != "" && st != "allowed" {
			line += "  [" + st + "]"
		}
		b.WriteString(line + "\n")
	}
	b.WriteString(fmt.Sprintf("  컨텍스트  %s  (%s / %s)", renderBar(float64(r.lastCtx)/ctxBarMax*100), commas(r.lastCtx), commas(ctxBarMax)))
	return b.String()
}

func (r *Repl) buildSnapshot(note string) string {
	n := strings.TrimSpace(note)
	if n == "" {
		n = "(empty)"
	}
	parts := []string{preamble, "## Handoff note (written by previous session)", n}
	if len(r.files) > 0 {
		var l []string
		for _, f := range r.files {
			l = append(l, "- "+f)
		}
		parts = append(parts, "## Files changed", strings.Join(l, "\n"))
	}
	turns := r.tr.turns
	if k := r.a.TailTurns; k > 0 && len(turns) > k {
		turns = turns[len(turns)-k:]
	}
	if len(turns) > 0 {
		parts = append(parts, "## Last turns (verbatim, truncated)")
		for _, t := range turns {
			parts = append(parts, "User: "+trunc(t[0], 1500)+"\nAssistant: "+trunc(t[1], 1500))
		}
	}
	return trunc(strings.Join(parts, "\n\n"), r.a.SnapshotChars)
}

// agentsBusy: delegated agents of the current session that have not finished.
func (r *Repl) agentsBusy() bool { return len(r.tracker.running()) > 0 }

func (r *Repl) rotate(reason string) {
	old := r.sess.SessionID
	t0 := time.Now()
	if r.pendingSnapshot != "" {
		// the current child has not received the pending snapshot (import or previous rotation) yet:
		// a handoff from it would be empty and would replace the snapshot. Restart and keep it.
		r.sess.Close()
		r.rotations++
		r.tr.add("rotate", kv{"n", r.rotations}, kv{"reason", reason}, kv{"old_session", old}, kv{"snapshot_kept", true})
		r.logRotate(reason, old, true, recoPreserved(r.lastReco, r.pendingSnapshot), 0, "snapshot_kept", t0, r.lastCtx)
		if err := r.start(); err != nil {
			fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
		}
		return
	}
	var note string
	var problems []string
	attempts := 0
	for attempts < 2 {
		attempts++
		note, _, _ = r.runTurn(handoffPrompt, false)
		if r.a.NoValidate {
			break
		}
		if problems = validateHandoff(note, r.lastReco, r.files); len(problems) == 0 {
			break
		}
	}
	valid := len(problems) == 0
	if !valid && strings.TrimSpace(note) == "" {
		// deterministic snapshot only (recent turns); if there is nothing to carry, hold the rotation
		if len(r.tr.turns) == 0 {
			r.lastRotFail = time.Now()
			r.logRotate(reason, old, valid, false, attempts, "held", t0, 0)
			r.p(dim+"── rotation held: handoff failed ──"+rst, "\n")
			return
		}
	}
	if !valid {
		note = "(handoff note failed validation: " + strings.Join(problems, "; ") + "; recent turns follow)"
	}
	snap := withReco(r.lastReco, r.buildSnapshot(note))
	drained := false
	if r.agentsBusy() && r.a.MaxDrain > 0 {
		r.sess.StopIdle()
		r.draining = append(r.draining, &drain{sess: r.sess, tr: r.tracker, since: time.Now(), timeout: r.a.DrainTimeout})
		for len(r.draining) > r.a.MaxDrain {
			r.draining[0].sess.Close()
			r.draining = r.draining[1:]
		}
		drained = true
		dr := r.draining[len(r.draining)-1]
		go dr.consume(func(kind string, ev event) {
			r.drainMu.Lock()
			defer r.drainMu.Unlock()
			r.tr.add(kind, kv{"type", str(ev["type"])}, kv{"session", dr.sess.SessionID})
		})
	} else {
		r.sess.Close()
	}
	r.rotations++
	var oldV any
	if old != "" {
		oldV = old
	}
	r.tr.add("rotate", kv{"n", r.rotations}, kv{"reason", reason}, kv{"old_session", oldV}, kv{"snapshot", snap},
		kv{"handoff_valid", valid}, kv{"drained", drained})
	r.pendingSnapshot = snap
	r.logRotate(reason, old, valid, recoPreserved(r.lastReco, snap), attempts, map[bool]string{true: "drained", false: "closed"}[drained], t0, r.lastCtx)
	if err := r.start(); err != nil {
		fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
		return
	}
	if r.a.ShowRotation {
		o := "None"
		if old != "" {
			o = trunc(old, 8)
		}
		r.p(fmt.Sprintf("%s── context rotated #%d (%s; %s → new) ──%s", dim, r.rotations, reason, o, rst), "\n")
	}
}

func (r *Repl) logRotate(reason, old string, valid, reco bool, attempts int, how string, t0 time.Time, at int) {
	_ = r.metrics.Log(map[string]any{"stage": "rotate", "reason": reason, "old_session": old, "handoff_valid": valid,
		"reco_preserved": reco, "attempts": attempts, "outcome": how, "rotate_at_tokens": at,
		"start_ms": t0.UnixMilli(), "end_ms": time.Now().UnixMilli()})
}

// closeDrains ends every draining session (harness exit); unfinished agents stay only in the transcript.
func (r *Repl) closeDrains() {
	for _, d := range r.draining {
		d.sess.Close()
		r.tr.add("drain", kv{"session", d.sess.SessionID}, kv{"timed_out", true}, kv{"at_exit", true})
	}
	r.draining = nil
}

// pollDrains finishes old sessions whose agents are done; queues a notice for the next user message.
func (r *Repl) pollDrains() {
	now := time.Now()
	keep := r.draining[:0]
	for _, d := range r.draining {
		done, to := d.finished(now)
		if !done {
			keep = append(keep, d)
			continue
		}
		for _, a := range d.tr.agents {
			if a.Restored {
				continue // restored from disk: already reported by its own process
			}
			st := "완료"
			if !a.Done || to {
				st = "타임아웃"
			}
			r.notices = append(r.notices, fmt.Sprintf("[agent done] %s · %s · %s · %s", a.ID, a.Task, a.OutPath, st))
		}
		d.sess.Close()
		r.tr.add("drain", kv{"session", d.sess.SessionID}, kv{"timed_out", to})
	}
	r.draining = keep
}

func (r *Repl) handle(line string) {
	r.pollDrains()
	r.tracker.dismissRestored()
	text := line
	if r.a.Conductor {
		text = r.route(line)
	}
	if len(r.notices) > 0 {
		text = strings.Join(r.notices, "\n") + "\n\n" + text
		r.notices = nil
	}
	if r.pendingSnapshot != "" {
		text = r.pendingSnapshot + "\n\n## Current user message\n" + text
		r.pendingSnapshot = ""
	}
	t0 := time.Now()
	r.delegs = 0
	var sid any
	if r.sess.SessionID != "" {
		sid = r.sess.SessionID
	}
	r.tr.add("user", kv{"text", line}, kv{"session", sid})
	r.rate0, r.rateSeq0 = r.rate, r.rateSeq
	reply, result, usage := r.runTurn(text, true)
	r.tr.turns = append(r.tr.turns, [2]string{line, reply})
	r.lastReply = reply
	if rc := extractReco(reply); rc != "" {
		r.lastReco = rc
	}
	sid = nil
	if r.sess.SessionID != "" {
		sid = r.sess.SessionID
	}
	var ru, cost any
	if result != nil {
		ru, cost = result["usage"], result["total_cost_usd"]
	}
	r.tr.add("assistant", kv{"text", reply}, kv{"session", sid}, kv{"usage", ru},
		kv{"ctx", ctxTokens(usage)}, kv{"cost", cost})
	r.usageLine(result, usage)
	if result != nil {
		r.taskLine()
	}
	fields := map[string]any{"stage": "turn", "start_ms": t0.UnixMilli(), "end_ms": time.Now().UnixMilli(),
		"model": r.a.Model, "conductor": r.a.Conductor, "ctx": r.lastCtx, "session": r.sess.SessionID, "draining": len(r.draining),
		"cost_usd": cost, "duration_ms": time.Since(t0).Milliseconds(), "rotations": r.rotations, "delegations": r.delegs}
	for k, v := range usageFields(asMap(ru)) {
		fields["result_"+k] = v // summed over all API calls of the turn
	}
	_ = r.metrics.Log(fields)
	newCat, catRot := categoryRotation(line, reply, r.cat, r.a.NoCategoryRotate)
	if newCat != "" || catRot {
		_ = r.metrics.Log(map[string]any{"stage": "category", "from": r.cat, "to": newCat, "switch": catRot, "session": r.sess.SessionID})
	}
	prevCat := r.cat
	r.cat = newCat
	cooledCat := r.lastRotFail.IsZero() || time.Since(r.lastRotFail) >= r.a.Cooldown
	if catRot && cooledCat {
		r.p(dim+"── category change: "+prevCat+" → "+newCat+" · starting a new session ──"+rst, "\n")
		r.rotate("category " + prevCat + " -> " + newCat)
		return
	}
	autoOK := r.a.MaxAutoRotations < 0 || r.rotations < r.a.MaxAutoRotations
	cooled := r.lastRotFail.IsZero() || time.Since(r.lastRotFail) >= r.a.Cooldown
	if autoOK && cooled && r.lastCtx >= r.a.Threshold {
		r.rotate(fmt.Sprintf("ctx %s >= %s", commas(r.lastCtx), commas(r.a.Threshold)))
	}
}

// passthroughCmds are forwarded verbatim to the child claude (tested: stream-json returns their output as a result).
var passthroughCmds = map[string]bool{"/usage": true, "/cost": true, "/context": true, "/compact": true,
	"/config": true, "/mcp": true, "/agents": true, "/init": true, "/review": true, "/doctor": true}

// unsupportedCmds are interactive-only in the child (-p answers "isn't available in this environment").
var unsupportedCmds = map[string]bool{"/memory": true, "/permissions": true}

// passthrough sends a slash command to the child as-is (no handoff snapshot) and shows its output.
func (r *Repl) passthrough(line string) {
	var sid any
	if r.sess.SessionID != "" {
		sid = r.sess.SessionID
	}
	r.tr.add("user", kv{"text", line}, kv{"session", sid})
	reply, result, usage := r.runTurn(line, true)
	if reply == "" && result != nil {
		if t := strings.TrimRight(str(result["result"]), "\n"); t != "" {
			if strings.Fields(line)[0] == "/usage" {
				t = decorateUsage(t, r.lastCtx)
			}
			if r.ui != nil {
				r.ui.send(textMsg(t))
				r.ui.send(textEndMsg{})
			} else {
				r.p(t, "\n")
			}
		}
	}
	if len(usage) > 0 { // local commands (/usage, /cost, ...) have no model usage
		r.usageLine(result, usage)
	}
}

func (r *Repl) loop() int {
	if err := r.start(); err != nil {
		fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
		return 1
	}
	r.restoreAgents("")
	defer func() {
		r.markInterrupted()
		r.closeDrains()
		if r.sess != nil {
			r.sess.Close()
		}
	}()
	for {
		if r.interactive {
			r.p("\n> ", "")
		}
		line, err := r.in.ReadString('\n')
		if line == "" && err != nil {
			break
		}
		line = strings.TrimRight(line, "\n")
		if strings.TrimSpace(line) == "" {
			continue
		}
		if line == "/quit" {
			break
		}
		if line == "/rotate" {
			r.rotate("manual")
			continue
		}
		if f := strings.Fields(line); f[0] == "/mode" {
			r.modeLine(f[1:])
			continue
		}
		if line == "/사용량" {
			r.p(r.usageText(), "\n")
			continue
		}
		if !r.interactive {
			r.p("> "+line, "\n")
		}
		if f := strings.Fields(line); f[0] == "/resume" {
			r.resumeLine(f[1:]...)
			continue
		} else if f[0] == "/import" {
			a := "latest"
			if len(f) > 1 {
				a = f[1]
			}
			r.importSession(a)
			continue
		}
		if f := strings.Fields(line); passthroughCmds[f[0]] {
			r.passthrough(line)
			continue
		}
		r.handle(line)
	}
	return 0
}

// Main runs the REPL with argparse-equivalent flags (args exclude the "repl" word); returns the exit code.
func Main(args []string) int {
	fs := flag.NewFlagSet("harness", flag.ContinueOnError)
	var a Args
	fs.StringVar(&a.Model, "model", "haiku", "model of the main session: haiku|sonnet|opus|fable (or a full model id)")
	fs.StringVar(&a.Cwd, "cwd", "", "")
	fs.IntVar(&a.Threshold, "threshold", 270_000, "rotate after an answer when context tokens >= this")
	fs.IntVar(&a.CompactWindow, "compact-window", 400_000, "child-only CLAUDE_CODE_AUTO_COMPACT_WINDOW (keep > threshold; 0=off)")
	fs.IntVar(&a.MaxAutoRotations, "max-auto-rotations", -1, "max automatic rotations (default unlimited)")
	fs.StringVar(&a.Transcript, "transcript", filepath.Join(osenv.StateDir(), "transcript.jsonl"), "")
	fs.IntVar(&a.TailTurns, "tail-turns", 4, "")
	fs.IntVar(&a.SnapshotChars, "snapshot-chars", 24000, "")
	fs.BoolVar(&a.AutoAllow, "auto-allow", false, "allow all permission prompts (testing)")
	fs.BoolVar(&a.ShowRotation, "show-rotation", false, "print a one-line rotation marker")
	fs.StringVar(&a.MetricsDir, "metrics-dir", filepath.Join("notes", "metrics"), "per-turn JSONL metrics dir (empty=off)")
	fs.BoolVar(&a.NoValidate, "no-handoff-validate", false, "disable handoff validation (safety guard)")
	fs.IntVar(&a.MaxDrain, "max-drain", 3, "max concurrently draining old sessions (0=never drain)")
	fs.DurationVar(&a.DrainTimeout, "drain-timeout", 60*time.Minute, "give up waiting for draining agents")
	fs.DurationVar(&a.Cooldown, "rotate-cooldown", 2*time.Minute, "min gap after a failed rotation")
	fs.BoolVar(&a.NoCategoryRotate, "no-category-rotate", false, "disable category-based session rotation")
	fs.BoolVar(&a.NoDelegationRule, "no-delegation-rule", false, "do not append the haiku delegation rule to the child's system prompt")
	fs.BoolVar(&a.Conductor, "conductor", true, "conductor mode: Haiku-fixed main; a one-shot Sonnet classifier tags size and adds the plan->review->execute hint to large tasks; a prompt naming its model (opus로, fable로) goes to that model directly")
	fs.StringVar(&a.AgentsFile, "agents-file", defaultAgentsFile(), "persisted sub-agent state, restored on restart and /resume (empty=off)")
	fs.StringVar(&a.ModeName, "mode", "", "permission mode to start in: default|acceptEdits|plan|auto (default: the mode last used in this folder)")
	fs.StringVar(&a.FromClaude, "from-claude", "", "start from a plain Claude Code transcript: session id, .jsonl path or latest")
	fs.BoolVar(&a.ImportDry, "import-dry-run", false, "with --from-claude: build the handoff, print size and 추천 status, do not launch claude")
	if _, err := lean.ParseInterspersed(fs, args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	r := &Repl{a: a, out: os.Stdout, in: bufio.NewReaderSize(os.Stdin, 1<<20), intr: make(chan struct{}, 1)}
	startMode := loadMode(a.Cwd)
	if a.ModeName != "" {
		pm, ok := parseMode(a.ModeName)
		if !ok {
			fmt.Fprintln(os.Stderr, "chaude: unknown --mode", a.ModeName, "(default | acceptEdits | plan | auto)")
			return 2
		}
		startMode = pm
	}
	r.mode.Store(int32(startMode))
	if st, err := os.Stdin.Stat(); err == nil {
		r.interactive = st.Mode()&os.ModeCharDevice != 0
	}
	r.tr = newTranscript(a.Transcript)
	r.agents = newAgentStore(a.AgentsFile)
	r.pool = newWorkerPool()
	r.agents.interruptRunning(false) // agents left 'running' by a crashed/killed harness
	r.metrics = newMetrics(a.MetricsDir, map[string]any{"threshold": a.Threshold, "handoff_validate": !a.NoValidate,
		"max_drain": a.MaxDrain, "drain_timeout_s": int(a.DrainTimeout.Seconds()), "compact_window": a.CompactWindow,
		"delegation_rule": !a.NoDelegationRule, "category_rotate": !a.NoCategoryRotate, "conductor": a.Conductor})
	r.seenTools = map[string]bool{}
	if a.FromClaude != "" {
		if code, done := r.importFromClaude(); done {
			return code
		}
	}
	sig := make(chan os.Signal, 1)
	signal.Notify(sig, os.Interrupt, syscall.SIGTERM)
	go func() {
		for range sig {
			select {
			case r.intr <- struct{}{}:
			default:
			}
		}
	}()
	if r.interactive && os.Getenv("HARNESS_PLAIN") == "" {
		return r.runTUI()
	}
	return r.loop()
}

// clear drops the conversation: new session, no handoff snapshot.
func (r *Repl) clear() {
	r.sess.Close()
	r.pendingSnapshot, r.files, r.lastCtx = "", nil, 0
	r.tr.turns = nil
	r.tr.add("clear")
	if err := r.start(); err != nil {
		r.p(dim+"cannot start claude: "+err.Error()+rst, "\n")
		return
	}
	r.p(dim+"── new session ──"+rst, "\n")
}

// setConductor toggles classifier routing (a one-shot Sonnet call per prompt picks the model of the session that answers).
func (r *Repl) setConductor(on bool) {
	r.a.Conductor = on
	r.p(fmt.Sprintf("conductor: %v (classifier routes each prompt to haiku|sonnet|opus|fable)", on), "\n")
}

// modeLine handles "/mode [name]" on the plain path (no arg shows, "next" cycles).
func (r *Repl) modeLine(args []string) {
	cur := permMode(r.mode.Load())
	if len(args) > 0 && (args[0] == "conductor" || args[0] == "conductor-off") {
		r.setConductor(args[0] == "conductor")
		return
	}
	if len(args) == 0 {
		r.p("mode "+cur.String()+"  (conductor="+fmt.Sprint(r.a.Conductor)+"; default | acceptEdits | plan | auto)", "\n")
		return
	}
	p, ok := cur.Next(), true
	if args[0] != "next" {
		p, ok = parseMode(args[0])
	}
	if !ok {
		r.p("unknown mode "+args[0]+" (default | acceptEdits | plan | auto)", "\n")
		return
	}
	r.mode.Store(int32(p))
	saveMode(r.a.Cwd, p)
	r.p("mode: "+p.String(), "\n")
}
