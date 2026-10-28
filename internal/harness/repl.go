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
	"strings"
	"sync"
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
	NoValidate       bool          // disable handoff validation (safety guard off)
	MaxDrain         int           // max concurrently draining old sessions
	DrainTimeout     time.Duration // give up waiting for a draining session's agents
	Cooldown         time.Duration // min gap after a failed rotation
	FromClaude       string        // import a plain Claude Code transcript: session id | path | latest
	ImportDry        bool          // with FromClaude: print the handoff stats and exit
	NoDelegationRule bool          // do not append the haiku delegation rule to the child's system prompt
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
	ui              *tuiBridge // non-nil in TUI mode
	seenTools       map[string]bool
	metrics         *Metrics
	tracker         *agentTracker
	draining        []*drain
	lastReply       string
	lastReco        string
	lastRotFail     time.Time
	notices         []string
	drainMu         sync.Mutex
	delegs          int // Agent/Task tool calls in the current turn
}

func (r *Repl) p(s string, end string) {
	fmt.Fprint(r.out, s+end)
}

func (r *Repl) start() error {
	opts := SessionOpts{Model: r.a.Model, Cwd: r.a.Cwd, CompactWindow: r.a.CompactWindow}
	if !r.a.NoDelegationRule {
		opts.Extra = []string{"--append-system-prompt", delegationRule}
	}
	s, err := NewSession(opts)
	if err != nil {
		return err
	}
	r.sess = s
	r.tracker = newAgentTracker()
	return nil
}

func (r *Repl) askPermission(req event) {
	rq := asMap(req["request"])
	input := asMap(rq["input"])
	if input == nil {
		input = map[string]any{}
	}
	if r.ui != nil {
		allowed := r.a.AutoAllow || r.ui.ask(nilStr(rq["tool_name"]), toolSummary(input))
		r.tr.add("permission", kv{"tool", rq["tool_name"]}, kv{"allowed", allowed})
		_ = r.sess.RespondPermission(str(req["request_id"]), allowed, asMap(rq["input"]), "")
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
	_ = r.sess.RespondPermission(str(req["request_id"]), allowed, asMap(rq["input"]), "")
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
	var lastUsage map[string]any
	var result event
	handle := func(ev event) {
		t := str(ev["type"])
		_, hasParent := ev["parent_tool_use_id"]
		isMain := !hasParent || ev["parent_tool_use_id"] == nil || ev["parent_tool_use_id"] == ""
		switch {
		case t == "stream_event" && isMain:
			e := asMap(ev["event"])
			d := asMap(e["delta"])
			if str(e["type"]) == "content_block_delta" && str(d["type"]) == "text_delta" {
				txt := str(d["text"])
				reply.WriteString(txt)
				if show && r.ui != nil {
					r.ui.send(textMsg(txt))
				} else if show {
					r.p(txt, "")
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
			r.rate = asMap(ev["rate_limit_info"])
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

func (r *Repl) usageLine(result event, usage map[string]any) {
	if result == nil {
		return
	}
	ctx := ctxTokens(usage)
	r.lastCtx = ctx
	ru := asMap(result["usage"])
	five := "?"
	if w := asMap(asMap(r.rate["unifiedWindows"])["five_hour"]); w != nil {
		if u, ok := w["utilization"]; ok && u != nil {
			five = fmt.Sprint(u)
		}
	}
	cost, _ := result["total_cost_usd"].(float64)
	sid := trunc(r.sess.SessionID, 8)
	if r.sess.SessionID == "" {
		sid = "None"
	}
	r.p(fmt.Sprintf("%s[ctx %s | in %d cr %s cc %s out %d | $%.4f | 5h %s | sess %s rot %d]%s",
		dim, commas(ctx), num(ru["input_tokens"]), commas(num(ru["cache_read_input_tokens"])),
		commas(num(ru["cache_creation_input_tokens"])), num(ru["output_tokens"]), cost, five, sid, r.rotations, rst), "\n")
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
	text := line
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
	fields := map[string]any{"stage": "turn", "start_ms": t0.UnixMilli(), "end_ms": time.Now().UnixMilli(),
		"model": r.a.Model, "ctx": r.lastCtx, "session": r.sess.SessionID, "draining": len(r.draining),
		"cost_usd": cost, "duration_ms": time.Since(t0).Milliseconds(), "rotations": r.rotations, "delegations": r.delegs}
	for k, v := range usageFields(asMap(ru)) {
		fields["result_"+k] = v // summed over all API calls of the turn
	}
	_ = r.metrics.Log(fields)
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
var unsupportedCmds = map[string]bool{"/memory": true, "/permissions": true, "/resume": true}

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
	defer func() {
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
		if !r.interactive {
			r.p("> "+line, "\n")
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
	fs.StringVar(&a.Model, "model", "haiku", "")
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
	fs.BoolVar(&a.NoDelegationRule, "no-delegation-rule", false, "do not append the haiku delegation rule to the child's system prompt")
	fs.StringVar(&a.FromClaude, "from-claude", "", "start from a plain Claude Code transcript: session id, .jsonl path or latest")
	fs.BoolVar(&a.ImportDry, "import-dry-run", false, "with --from-claude: build the handoff, print size and 추천 status, do not launch claude")
	if _, err := lean.ParseInterspersed(fs, args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	r := &Repl{a: a, out: os.Stdout, in: bufio.NewReaderSize(os.Stdin, 1<<20), intr: make(chan struct{}, 1)}
	if st, err := os.Stdin.Stat(); err == nil {
		r.interactive = st.Mode()&os.ModeCharDevice != 0
	}
	r.tr = newTranscript(a.Transcript)
	r.metrics = newMetrics(a.MetricsDir, map[string]any{"threshold": a.Threshold, "handoff_validate": !a.NoValidate,
		"max_drain": a.MaxDrain, "drain_timeout_s": int(a.DrainTimeout.Seconds()), "compact_window": a.CompactWindow,
		"delegation_rule": !a.NoDelegationRule})
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
