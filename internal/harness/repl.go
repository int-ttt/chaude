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
	"strconv"
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

	// rotation thresholds: opus uses opusThreshold; every other model (haiku, unknown, sonnet) uses -threshold, default haikuThreshold
	haikuThreshold = 80_000
	opusThreshold  = 200_000

	handoffPrompt = "Write a handoff note for a fresh session that will continue this work. No tools. " +
		"Sections: Goal; Decisions; Files created/changed; In-progress work; Open questions; " +
		"User preferences/instructions; Any exact codes, names or numbers the user gave. Be exact and brief."
	preamble = "[Context carried over from a previous session that was rotated to save tokens. " +
		"Treat it as your memory of the conversation so far; do not mention the rotation.]"
	delegationRule = "Default to delegating bounded code changes (implementation steps, small fixes, wiring, tests) to the haiku-coder agent with an exact spec; " +
		"implement yourself only design-level or tightly coupled parts, then verify Haiku's diff. " +
		"Log each delegation (agent, tokens, duration, outcome, retries)."
)

const forwardRule = "[forward] You are the Haiku main: you receive the user's input first. Forward design, judgment, diagnosis, planning and multi-step implementation to the opus-architect agent in ONE foreground call; the brief carries the user's original text verbatim plus a short summary, in caveman B format (docs/claude-lean/CAVEMAN_FORMAT.md 3.1). Simple lookups go to haiku-scout, repeat/loop requests to haiku-prompter. Relay the result to the user in Korean 존댓말; do not decide design yourself."

// forwardRuleFor returns the forward rule for a child on model; an Opus main already has the judgment, so no rule.
func forwardRuleFor(model string) string {
	if strings.Contains(strings.ToLower(model), "opus") {
		return ""
	}
	return forwardRule
}

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
	NoDelegationRule bool          // do not append the haiku delegation rule to the child's system prompt
	CacheTTL         string        // prompt cache TTL requested from claude: auto|1h|5m (see withCacheTTL)
	IdleRotate       time.Duration // idle time after which a child is treated as cold; 0 = off
	ClientsFile      string        // per-(cwd,model) client registry; empty = off
	MaxWorkers       int           // cap on concurrent live worker sessions (see resolveMaxWorkers)
	TaskBudgetUSD    float64       // per-task USD cap (--task-budget-usd); 0 = off
	Orch             string        // orchestration mode m0|b|c|d|e|f (A/B test)
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
	rateMu          sync.Mutex // guards rate, lastFiveHour, fiveHourSeen, rawRateLogged
	rateStr         atomic.Value
	lastFiveHour    float64    // last five_hour utilization logged by setRate
	fiveHourSeen    bool       // lastFiveHour is valid
	rawRateLogged   bool       // rate_raw diagnostic row already written
	usageMu         sync.Mutex // guards usagePollAt, usagePolling
	usagePollAt     time.Time  // last /usage poll start (pollUsage)
	usagePolling    bool       // a /usage poll is in flight
	costMu          sync.Mutex // guards lastCostSess, lastCost
	lastCostSess    string     // session of the last turn cost (turnCostUSD)
	lastCost        float64    // cumulative total_cost_usd of lastCostSess
	ui              *tuiBridge // non-nil in TUI mode
	seenTools       map[string]bool
	spend           *turnSpend // subagent usage and spawns of the current turn
	metrics         *Metrics
	tracker         *agentTracker
	agents          *agentStore // on-disk agent state (nil/empty path = off)
	draining        []*drain
	lastReply       string
	lastReco        string
	lastRotFail     time.Time
	notices         []string
	drainMu         sync.Mutex
	cat             string              // current task category from the child's [cat: X] marker
	delegs          int                 // Agent/Task tool calls in the current turn
	pool            *workerPool         // Haiku sessions with edit history (edit history)
	parked          map[string]string   // model -> session id left by a direct Haiku switch
	resumeID        string              // session id start() resumes once
	curTask         string              // terse task of the current routed turn
	mode            atomic.Int32        // permMode, set from the UI goroutine
	idle            idleTurn            // child-initiated turn being rendered (idle reader goroutine only)
	reg             *clientRegistry     // per-(cwd,model) client registry; nil = off (old routing)
	children        *childSet           // live child processes kept per (cwd,model); nil = off
	resident        []residentSlot      // --resident preset slots; nil = off
	residentUp      bool                // spawnResident ran (once, after the first successful start)
	idleRotate      time.Duration       // idle deadline base (--idle-rotate); 0 = off
	budget          *budget             // per-task USD ledger; nil = off
	ttl             string              // prompt cache TTL requested (--cache-ttl)
	lastAPI         time.Time           // single clock for the active child: last API message_start
	ttlSeen         string              // cache TTL observed from usage ("" = unknown)
	idleFiredFor    time.Time           // lastAPI value an idle rotate already fired for
	cur             *liveChild          // record of the active child (idle callback identity; set by start/bindChild)
	clockMu         sync.Mutex          // guards lastAPI, ttlSeen, idleFiredFor (idle goroutine writes, UI reads)
	noticeMu        sync.Mutex          // guards notices
	ws              map[string]*wsState // mode f: per-workspace Haiku sessions
	curWS           string              // mode f: workspace id of the current routed turn ("" = main)
	orchMain        string              // mode f: main model for untagged lines
}

// wireClients sets the registry and child set from Args. Empty ClientsFile
// leaves both nil so routing keeps the pre-multiproc path.
func (r *Repl) wireClients() {
	r.idleRotate = r.a.IdleRotate
	if r.a.TaskBudgetUSD > 0 {
		r.budget = newBudget(r.a.TaskBudgetUSD)
	}
	r.ttl = r.a.CacheTTL
	if r.a.ClientsFile == "" {
		return
	}
	r.reg = newClientRegistry(r.a.ClientsFile)
	r.children = newChildSet()
}

func (r *Repl) p(s string, end string) {
	fmt.Fprint(r.out, s+end)
}

func (r *Repl) setRate(info map[string]any) {
	r.rateStr.Store(fmtRateStatus(info, time.Now()))
	r.rateMu.Lock()
	r.rate = info
	raw := info != nil && !r.rawRateLogged
	r.rawRateLogged = r.rawRateLogged || raw
	v, ok := fiveHourUtil(info)
	changed := ok && (!r.fiveHourSeen || v != r.lastFiveHour)
	if changed {
		r.lastFiveHour, r.fiveHourSeen = v, true
	}
	r.rateMu.Unlock()
	if raw { // diagnostic: key names only, never values
		_ = r.metrics.Log(rateRawRow(info))
	}
	if changed {
		_ = r.metrics.Log(map[string]any{"stage": "rate", "five_hour": v})
	}
}

// newSessionFn is a test seam for start().
var newSessionFn = NewSession

func (r *Repl) start() error {
	opts := SessionOpts{Model: r.a.Model, Cwd: r.a.Cwd, CompactWindow: r.a.CompactWindow, Resume: r.resumeID, CacheTTL: r.a.CacheTTL}
	r.resumeID = ""
	rule := ""
	if !r.a.NoDelegationRule {
		rule = delegationRule
	}
	if fr := forwardRuleFor(r.a.Model); fr != "" {
		rule = strings.TrimSpace(rule + " " + fr)
	}
	if !r.a.NoCategoryRotate {
		rule = strings.TrimSpace(rule + " " + categoryRule)
	}
	if orr := orchRule(r.a.Orch); orr != "" {
		rule = strings.TrimSpace(rule + " " + orr)
	}
	if rule != "" {
		opts.Extra = append(opts.Extra, "--append-system-prompt", rule)
	}
	if r.a.TaskBudgetUSD > 0 { // CLI backstop; the ledger is primary
		opts.Extra = append(opts.Extra, "--max-budget-usd", strconv.FormatFloat(r.a.TaskBudgetUSD, 'f', -1, 64))
	}
	s, err := newSessionFn(opts)
	if err != nil {
		return err
	}
	r.sess = s
	r.clockMu.Lock()
	r.lastAPI = time.Time{} // new child: no API call yet (idleDue needs a turn)
	r.clockMu.Unlock()
	prev := r.tracker
	r.tracker = newAgentTracker()
	tk := r.tracker
	tk.notify = r.agentNotice
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
	self := &liveChild{Sess: s, Tracker: tk, Model: r.a.Model}
	r.cur = self
	s.StartIdle(func(ev event) {
		if self.parked.Load() {
			r.parkedIdleEvent(self, ev)
			return
		}
		r.idleEvent(s, tk, ev)
	})
	if len(r.resident) > 0 && !r.residentUp {
		r.residentUp = true
		if err := r.spawnResident(); err != nil {
			r.p(dim+"chaude: resident slots: "+err.Error()+rst, "\n")
		}
	}
	return nil
}

// idleEvent handles stdout events that arrive while no turn is running (background subagents).
func (r *Repl) idleEvent(s *Session, tk *agentTracker, ev event) {
	if r.idle.spend == nil {
		r.idle.spend = newTurnSpend()
	}
	r.idle.spend.onEvent(ev)
	tk.onSubEvent(ev)
	r.touchMain(ev, time.Now())
	switch t := str(ev["type"]); {
	case t == "control_request" && str(asMap(ev["request"])["subtype"]) == "can_use_tool":
		r.tr.add("permission_idle", kv{"tool", asMap(ev["request"])["tool_name"]})
		r.askPerm(s, ev, true)
	case t == "rate_limit_event":
		r.setRate(asMap(ev["rate_limit_info"]))
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
				tk.onToolResultText(str(b["tool_use_id"]), resultText(b["content"]))
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
	spend    *turnSpend // subagent usage and spawns seen while this turn runs
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
	if tc, ok := cost.(float64); ok {
		r.budget.charge(s, tc)
	}
	r.tr.add("assistant", kv{"text", reply}, kv{"session", sid}, kv{"usage", ru},
		kv{"ctx", ctxTokens(st.usage)}, kv{"cost", cost}, kv{"source", "background"})
	r.usageLine(result, st.usage)
	t0 := st.t0
	if t0.IsZero() {
		t0 = time.Now()
	}
	fields := map[string]any{"stage": "turn", "source": "background", "start_ms": t0.UnixMilli(), "end_ms": time.Now().UnixMilli(),
		"model": r.a.Model, "ctx": r.lastCtx, "session": s.SessionID, "cost_usd": cost, "rotations": r.rotations}
	if d, ok := r.turnCostUSD(s.SessionID, cost); ok {
		fields["cost_turn_usd"] = d
	}
	for k, v := range usageFields(asMap(ru)) {
		fields["result_"+k] = v
	}
	if mu, ok := asMap(result)["modelUsage"].(map[string]any); ok && len(mu) > 0 {
		fields["result_model_usage"] = mu
	}
	if st.spend != nil {
		for k, v := range st.spend.fields() {
			fields[k] = v
		}
	}
	_ = r.metrics.Log(fields)
	r.pollUsage()
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
	if !r.budget.allow(r.estNext()) {
		r.budgetStop()
		return "", nil, nil
	}
	r.spend = newTurnSpend()
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
	budgetHit := false
	handle := func(ev event) {
		r.spend.onEvent(ev)
		t := str(ev["type"])
		r.tracker.onSubEvent(ev)
		r.touchMain(ev, time.Now())
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
				if r.budget.over(midTurnEstimate(r.a.Model, u)) && !budgetHit {
					budgetHit = true
					_ = r.sess.Interrupt()
				}
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
					r.recordDesignPath(r.a.Model, fp)
					if fp != "" && r.a.Model == "haiku" && r.sess != nil {
						r.pool.noteEdit(r.sess.SessionID, r.curTask, fp)
					}
				}
			}
		case t == "user" && isMain:
			r.tracker.onUserEvent(ev) // tool_results + <task-notification>s arriving mid-turn
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
			r.setRate(asMap(ev["rate_limit_info"]))
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
	if result != nil {
		if tc, ok := result["total_cost_usd"].(float64); ok {
			r.budget.charge(r.sess, tc)
		}
	}
	if budgetHit {
		r.budgetStop()
	}
	return reply.String(), result, lastUsage
}

// estNext is the planned cost of the next call on the active child (0 when budget is off).
func (r *Repl) estNext() float64 {
	if r.budget == nil {
		return 0
	}
	dl, ttl := r.warmDeadline(), r.effTTL()
	r.clockMu.Lock()
	warm := !r.lastAPI.IsZero() && time.Since(r.lastAPI) < dl
	r.clockMu.Unlock()
	return estimate(r.a.Model, r.lastCtx, warm, ttl)
}

// budgetStop reports that the task cap was reached.
func (r *Repl) budgetStop() {
	r.p(dim+fmt.Sprintf("[budget] $%.4f / $%.4f reached, task stopped", r.budget.Spent(), r.budget.cap)+rst, "\n")
	_ = r.metrics.Log(map[string]any{"stage": "budget", "spent_usd": r.budget.Spent(), "cap_usd": r.budget.cap, "model": r.a.Model})
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
	r.rateMu.Lock()
	rate := r.rate
	r.rateMu.Unlock()
	if w := asMap(asMap(rate["unifiedWindows"])["five_hour"]); w != nil {
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

// snapState is the outcome of makeSnapshot.
type snapState int

const (
	snapOK      snapState = iota // handoff note written and valid
	snapHeld                     // handoff failed and there is nothing to carry: hold the rotation
	snapInvalid                  // handoff failed validation; the snapshot carries a failure note and recent turns
)

// makeSnapshot builds the handoff snapshot from the active child (handoff turn, validation, recommendation).
// It neither closes nor starts anything.
func (r *Repl) makeSnapshot(reason string) (string, snapState) {
	snap, st, _ := r.makeSnapshotN(reason)
	return snap, st
}

// makeSnapshotN is makeSnapshot plus the number of handoff attempts (for rotate's log).
func (r *Repl) makeSnapshotN(reason string) (snap string, st snapState, attempts int) {
	_ = reason
	var note string
	var problems []string
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
	if !valid && strings.TrimSpace(note) == "" && len(r.tr.turns) == 0 {
		return "", snapHeld, attempts // deterministic snapshot only (recent turns); nothing to carry
	}
	if !valid {
		note = "(handoff note failed validation: " + strings.Join(problems, "; ") + "; recent turns follow)"
	}
	snap = withReco(r.lastReco, r.buildSnapshot(note))
	if valid {
		return snap, snapOK, attempts
	}
	return snap, snapInvalid, attempts
}

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
	snap, st, attempts := r.makeSnapshotN(reason)
	if st == snapHeld {
		r.lastRotFail = time.Now()
		r.logRotate(reason, old, false, false, attempts, "held", t0, 0)
		r.p(dim+"── rotation held: handoff failed ──"+rst, "\n")
		return
	}
	valid := st == snapOK
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
			r.addNotice(fmt.Sprintf("[agent done] %s · %s · %s · %s", a.ID, a.Task, a.OutPath, st))
		}
		d.sess.Close()
		r.tr.add("drain", kv{"session", d.sess.SessionID}, kv{"timed_out", to})
	}
	r.draining = keep
}

// effectiveThreshold is the context-token count that triggers auto-rotation for the current model.
func (r *Repl) effectiveThreshold() int { return r.thresholdFor(r.a.Model) }

func (r *Repl) thresholdFor(model string) int {
	if strings.Contains(model, "opus") {
		return opusThreshold
	}
	return r.a.Threshold
}

// shouldRotate reports whether the context token count has reached the rotation threshold.
func shouldRotate(ctx, thr int) bool { return ctx >= thr }

func (r *Repl) handle(line string) {
	r.pollDrains()
	r.tracker.dismissRestored()
	r.budget.reset()
	if !r.budget.allow(0) {
		r.budgetStop()
		return
	}
	text := line
	if ns := r.takeNotices(); len(ns) > 0 {
		text = strings.Join(ns, "\n") + "\n\n" + text
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
	if strings.TrimSpace(reply) == "" && result != nil && r.ui != nil {
		if t := str(result["result"]); t != "" {
			r.ui.send(textMsg(t))
			r.ui.send(textEndMsg{})
		}
	}
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
	r.wsRecord(reply)
	r.pollUsage()
	fields := map[string]any{"stage": "turn", "start_ms": t0.UnixMilli(), "end_ms": time.Now().UnixMilli(),
		"model": r.a.Model, "ctx": r.lastCtx, "session": r.sess.SessionID, "draining": len(r.draining),
		"cost_usd": cost, "duration_ms": time.Since(t0).Milliseconds(), "rotations": r.rotations, "delegations": r.delegs}
	if d, ok := r.turnCostUSD(r.sess.SessionID, cost); ok {
		fields["cost_turn_usd"] = d // cost_usd is cumulative per session; this is the turn's own share
	}
	for k, v := range usageFields(asMap(ru)) {
		fields["result_"+k] = v // summed over all API calls of the turn
	}
	if mu, ok := asMap(result)["modelUsage"].(map[string]any); ok && len(mu) > 0 {
		fields["result_model_usage"] = mu
	}
	for k, v := range r.spend.fields() {
		fields[k] = v
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
	thr := r.effectiveThreshold()
	if autoOK && cooled && shouldRotate(r.lastCtx, thr) {
		r.rotate(fmt.Sprintf("ctx %s >= %s", commas(r.lastCtx), commas(thr)))
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
		r.closeAllChildren()
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
		if line == "/quit" || line == "/exit" {
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
		if !r.interactive {
			r.p("> "+line, "\n")
		}
		if txt, ok := localCmd(r, line); ok {
			r.p(txt, "\n")
			continue
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

// parseCacheTTL validates the --cache-ttl value: auto, 1h or 5m.
func parseCacheTTL(s string) (string, error) {
	switch s {
	case "auto", "1h", "5m":
		return s, nil
	}
	return "", fmt.Errorf("--cache-ttl must be auto, 1h or 5m (got %q)", s)
}

// Main runs the REPL with argparse-equivalent flags (args exclude the "repl" word); returns the exit code.
func Main(args []string) int {
	fs := flag.NewFlagSet("harness", flag.ContinueOnError)
	var a Args
	fs.StringVar(&a.Model, "model", "haiku", "")
	fs.StringVar(&a.Cwd, "cwd", "", "")
	fs.IntVar(&a.Threshold, "threshold", haikuThreshold, "rotate after an answer when context tokens >= this")
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
	fs.StringVar(&a.AgentsFile, "agents-file", defaultAgentsFile(), "persisted sub-agent state, restored on restart and /resume (empty=off)")
	fs.StringVar(&a.FromClaude, "from-claude", "", "start from a plain Claude Code transcript: session id, .jsonl path or latest")
	fs.BoolVar(&a.ImportDry, "import-dry-run", false, "with --from-claude: build the handoff, print size and 추천 status, do not launch claude")
	fs.StringVar(&a.CacheTTL, "cache-ttl", "1h", "prompt cache TTL requested from claude: auto, 1h or 5m")
	fs.Float64Var(&a.TaskBudgetUSD, "task-budget-usd", 0, "per-task USD cap (0=off); also passed to claude as --max-budget-usd backstop")
	fs.DurationVar(&a.IdleRotate, "idle-rotate", 3599*time.Second, "idle time after which a child is treated as cold (0=off); idle rotation runs in the TUI only")
	fs.StringVar(&a.ClientsFile, "clients-file", defaultClientsFile(), "per-(cwd,model) client registry (empty=off)")
	fs.IntVar(&a.MaxWorkers, "max-workers", DefaultMaxWorkers, "cap on concurrent live worker sessions (env CHAUDE_MAX_WORKERS when unset)")
	fs.StringVar(&a.Orch, "orch", "m0", "orchestration mode m0|b|c|d|e|f (A/B test)")
	var resident, residentMainModel string
	fs.StringVar(&resident, "resident", "off", "resident slot preset off|min|fixed (env CHAUDE_RESIDENT when unset)")
	fs.StringVar(&residentMainModel, "resident-main-model", "", "main slot model when --resident is not off (default: fixed=opus, min=haiku)")
	if _, err := lean.ParseInterspersed(fs, args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	mode, err := parseOrch(a.Orch)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 2
	}
	a.Orch = mode
	modelSet := false
	fs.Visit(func(f *flag.Flag) {
		if f.Name == "model" {
			modelSet = true
		}
	})
	if dm := orchDefaultModel(mode); dm != "" && !modelSet {
		a.Model = dm
	}
	applyOrchEnv(mode)
	if _, err := parseCacheTTL(a.CacheTTL); err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 2
	}
	maxWorkersSet := false
	fs.Visit(func(f *flag.Flag) {
		if f.Name == "max-workers" {
			maxWorkersSet = true
		}
	})
	mw, err := resolveMaxWorkers(maxWorkersSet, a.MaxWorkers, os.Getenv("CHAUDE_MAX_WORKERS"))
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 2
	}
	residentSet := false
	fs.Visit(func(f *flag.Flag) {
		if f.Name == "resident" {
			residentSet = true
		}
	})
	resName := resident
	if env := strings.TrimSpace(os.Getenv("CHAUDE_RESIDENT")); !residentSet && env != "" {
		resName = env
	}
	slots, residentCap, err := residentPreset(resName, residentMainModel)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 2
	}
	if residentCap > 0 && !maxWorkersSet && strings.TrimSpace(os.Getenv("CHAUDE_MAX_WORKERS")) == "" {
		mw = residentCap
	}
	if len(slots) > 0 {
		a.Model = slots[0].Model
	}
	a.MaxWorkers = mw
	r := &Repl{a: a, out: os.Stdout, in: bufio.NewReaderSize(os.Stdin, 1<<20), intr: make(chan struct{}, 1), resident: slots}
	if st, err := os.Stdin.Stat(); err == nil {
		r.interactive = st.Mode()&os.ModeCharDevice != 0
	}
	r.tr = newTranscript(a.Transcript)
	r.agents = newAgentStore(a.AgentsFile)
	r.wireClients()
	r.pool = newWorkerPool()
	r.agents.interruptRunning(false) // agents left 'running' by a crashed/killed harness
	r.reconcileStart()
	r.metrics = newMetrics(a.MetricsDir, map[string]any{"threshold": a.Threshold, "handoff_validate": !a.NoValidate,
		"max_drain": a.MaxDrain, "drain_timeout_s": int(a.DrainTimeout.Seconds()), "compact_window": a.CompactWindow,
		"delegation_rule": !a.NoDelegationRule, "category_rotate": !a.NoCategoryRotate})
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
	r.endAllParked("clear") // parked children and their pre-clear session ids must not come back
	r.parked = map[string]string{}
	r.sess.Close()
	r.reg.MarkEnded(r.a.Cwd, r.a.Model, "clear", time.Now())
	r.pendingSnapshot, r.files, r.lastCtx = "", nil, 0
	r.tr.turns = nil
	r.tr.add("clear")
	if err := r.start(); err != nil {
		r.p(dim+"cannot start claude: "+err.Error()+rst, "\n")
		return
	}
	r.p(dim+"── new session ──"+rst, "\n")
}

// clearWithHandoff is /clear in the TUI: the next session gets a handoff snapshot of the conversation
// that is dropped (taken before anything is closed). A held handoff leaves the new session empty.
func (r *Repl) clearWithHandoff() {
	snap := ""
	if r.sess != nil && r.pendingSnapshot == "" && len(r.tr.turns) > 0 {
		if s, st := r.makeSnapshot("clear"); st != snapHeld {
			snap = s
		}
	}
	r.clear()
	if snap != "" {
		r.pendingSnapshot = snap
	}
}

// reconcileStart repairs registry rows left by killed harnesses and shows orphaned children (never kills them).
func (r *Repl) reconcileStart() {
	for _, o := range r.reg.reconcile(time.Now(), r.warmDeadline()) {
		r.addNotice(fmt.Sprintf("[orphan] claude pid %d (%s, %s) still running after its harness died; left running", o.ChildPID, o.Model, o.Cwd))
	}
}

// modeLine handles "/mode [name]" on the plain path (no arg shows, "next" cycles).
func (r *Repl) modeLine(args []string) {
	cur := permMode(r.mode.Load())
	if len(args) == 0 {
		r.p("mode "+cur.String()+"  (default | acceptEdits | plan | auto)", "\n")
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
	r.p("mode: "+p.String(), "\n")
}
