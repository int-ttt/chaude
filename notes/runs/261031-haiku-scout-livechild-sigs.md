# scout: livechild/repl/drain/orch signatures (grep -n)
source: internal/harness/{livechild,repl,drain,orch}.go; liveChild struct is in children.go

internal/harness/orch.go:11:func parseOrch(s string) (string, error) {
internal/harness/orch.go:23:func orchDefaultModel(mode string) string {
internal/harness/orch.go:34:func applyOrchEnv(mode string) {
internal/harness/orch.go:49:func orchRule(mode string) string {
internal/harness/livechild.go:10:func (r *Repl) touchMain(ev event, now time.Time) {
internal/harness/livechild.go:31:func (r *Repl) parkedIdleEvent(c *liveChild, ev event) {
internal/harness/livechild.go:94:func (r *Repl) addNotice(s string) {
internal/harness/livechild.go:100:func (r *Repl) takeNotices() []string {
internal/harness/livechild.go:108:func (r *Repl) closeAllChildren() {
internal/harness/livechild.go:116:func (r *Repl) unbindActive() *liveChild {
internal/harness/livechild.go:147:func (r *Repl) bindChild(c *liveChild) {
internal/harness/drain.go:16:type agentRec struct {
internal/harness/drain.go:37:func (a *agentRec) doneLine() string {
internal/harness/drain.go:57:func markDoneLocked(a *agentRec) string {
internal/harness/drain.go:66:type agentTracker struct {
internal/harness/drain.go:76:func newAgentTracker() *agentTracker { return &agentTracker{agents: map[string]*agentRec{}} }
internal/harness/drain.go:79:func (t *agentTracker) emit(lines []string) {
internal/harness/drain.go:91:func (r *Repl) agentNotice(s string) {
internal/harness/drain.go:97:func (t *agentTracker) onToolUse(id, name string, input map[string]any, now time.Time) {
internal/harness/drain.go:122:func (t *agentTracker) onToolResult(id string) { t.onToolResultText(id, "") }
internal/harness/drain.go:126:func (t *agentTracker) onToolResultText(id, text string) {
internal/harness/drain.go:144:func (t *agentTracker) onNotification(text string) bool {
internal/harness/drain.go:170:func (t *agentTracker) onUserEvent(ev event) bool {
internal/harness/drain.go:191:func (t *agentTracker) finishForeground() {
internal/harness/drain.go:208:func (t *agentTracker) finishOldestBackground() {
internal/harness/drain.go:227:func (t *agentTracker) running() []*agentRec {
internal/harness/drain.go:240:type drain struct {
internal/harness/drain.go:247:func (d *drain) finished(now time.Time) (done, timedOut bool) {
internal/harness/drain.go:259:func (d *drain) consume(log func(kind string, ev event)) {
internal/harness/drain.go:272:func (t *agentTracker) counts() (fg, bg int) {
internal/harness/drain.go:283:func firstLine(s string) string {
internal/harness/drain.go:293:func (t *agentTracker) sortedRunning() []*agentRec {
internal/harness/repl.go:41:type Args struct {
internal/harness/repl.go:71:type kv struct {
internal/harness/repl.go:77:type Transcript struct {
internal/harness/repl.go:82:func newTranscript(path string) *Transcript {
internal/harness/repl.go:92:func (t *Transcript) add(kind string, fields ...kv) {
internal/harness/repl.go:107:func fmtFloat(f float64) string { return fmt.Sprintf("%.6f", f) }
internal/harness/repl.go:109:func commas(n int) string {
internal/harness/repl.go:128:func trunc(s string, n int) string {
internal/harness/repl.go:136:type Repl struct {
internal/harness/repl.go:198:func (r *Repl) wireClients() {
internal/harness/repl.go:211:func (r *Repl) p(s string, end string) {
internal/harness/repl.go:215:func (r *Repl) setRate(info map[string]any) {
internal/harness/repl.go:238:func (r *Repl) start() error {
internal/harness/repl.go:297:func (r *Repl) idleEvent(s *Session, tk *agentTracker, ev event) {
internal/harness/repl.go:359:type idleTurn struct {
internal/harness/repl.go:369:func isMainEvent(ev event) bool {
internal/harness/repl.go:374:func (r *Repl) idleText(txt string) {
internal/harness/repl.go:393:func (r *Repl) idleFinish(s *Session, tk *agentTracker, result event) {
internal/harness/repl.go:445:func (r *Repl) askPermission(req event) { r.askPerm(r.sess, req, false) }
internal/harness/repl.go:449:func (r *Repl) askPerm(sess *Session, req event, idle bool) {
internal/harness/repl.go:487:func nilStr(v any) string {
internal/harness/repl.go:495:func (r *Repl) runTurn(text string, show bool) (string, event, map[string]any) {
internal/harness/repl.go:629:func (r *Repl) estNext() float64 {
internal/harness/repl.go:641:func (r *Repl) budgetStop() {
internal/harness/repl.go:646:func contains(l []string, s string) bool {
internal/harness/repl.go:655:func (r *Repl) usageLine(result event, usage map[string]any) {
internal/harness/repl.go:681:func (r *Repl) buildSnapshot(note string) string {
internal/harness/repl.go:708:func (r *Repl) agentsBusy() bool { return len(r.tracker.running()) > 0 }
internal/harness/repl.go:711:type snapState int
internal/harness/repl.go:721:func (r *Repl) makeSnapshot(reason string) (string, snapState) {
internal/harness/repl.go:727:func (r *Repl) makeSnapshotN(reason string) (snap string, st snapState, attempts int) {
internal/harness/repl.go:755:func (r *Repl) rotate(reason string) {
internal/harness/repl.go:818:func (r *Repl) logRotate(reason, old string, valid, reco bool, attempts int, how string, t0 time.Time, at int) {
internal/harness/repl.go:825:func (r *Repl) closeDrains() {
internal/harness/repl.go:834:func (r *Repl) pollDrains() {
internal/harness/repl.go:860:func (r *Repl) effectiveThreshold() int { return r.thresholdFor(r.a.Model) }
internal/harness/repl.go:862:func (r *Repl) thresholdFor(model string) int {
internal/harness/repl.go:869:func (r *Repl) handle(line string) {
internal/harness/repl.go:964:func (r *Repl) passthrough(line string) {
internal/harness/repl.go:989:func (r *Repl) loop() int {
internal/harness/repl.go:1054:func parseCacheTTL(s string) (string, error) {
internal/harness/repl.go:1063:func Main(args []string) int {
internal/harness/repl.go:1167:func (r *Repl) clear() {
internal/harness/repl.go:1184:func (r *Repl) clearWithHandoff() {
internal/harness/repl.go:1198:func (r *Repl) reconcileStart() {
internal/harness/repl.go:1205:func (r *Repl) setConductor(on bool) {
internal/harness/repl.go:1211:func (r *Repl) modeLine(args []string) {

## liveChild (children.go:35-56)
type liveChild struct {
	Sess     closer
	Tracker  *agentTracker
	Model    string
	Role     string
	LastAPI  time.Time
	LastCtx  int
	Files    []string
	TurnFrom int

	// Repl state with no other home while the child is parked (saved by unbindActive, restored by bindChild).
	PendingSnapshot string
	LastRotFail     time.Time
	TTLSeen         string
	Cat             string

	mu       sync.Mutex  // guards LastAPI and the parked reply buffer
	parked   atomic.Bool // true from unbindActive until bindChild; the idle callback branches on it
	reply    strings.Builder
	stream   bool // the parked reply arrived as text_delta events
	sawNotif bool // a <task-notification> was seen since the last result
}

## comment lines idle/3599/rotate/resume/session
internal/harness/livechild.go:30:// parkedIdleEvent handles events from a child that is parked-live: no rendering, no permission prompt.
internal/harness/livechild.go:81:			c.Tracker.finishOldestBackground() // same fallback as idleFinish: notice not visible on stdout
internal/harness/livechild.go:115:// The caller parks or closes the returned child. Returns nil when there is no active session.
internal/harness/livechild.go:124:	// Sess/Tracker/Model are set when the child is created (start) and are read by the idle callback: not rewritten here.
internal/harness/livechild.go:125:	c.parked.Store(true) // first: the idle callback must stop rendering before state moves
internal/harness/drain.go:20:	Model       string // `model` input of the Agent call (empty: inherits the session model)
internal/harness/drain.go:21:	Name        string // `name` input, else subagent_type: the session name shown in the panel
internal/harness/drain.go:65:// agentTracker records delegations of one session.
internal/harness/drain.go:71:	parent     func() string // session id of the child that owns this tracker
internal/harness/drain.go:239:// drain is an old session kept alive until its agents finish (or time out).
internal/harness/drain.go:292:// sortedRunning: running agents, oldest first (stable order for the session panel).
internal/harness/repl.go:1:// Package harness: terminal REPL over `claude -p` stream-json with session rotation (port of harness/*.py).
internal/harness/repl.go:55:	MaxDrain         int           // max concurrently draining old sessions
internal/harness/repl.go:56:	DrainTimeout     time.Duration // give up waiting for a draining session's agents
internal/harness/repl.go:58:	FromClaude       string        // import a plain Claude Code transcript: session id | path | latest
internal/harness/repl.go:64:	IdleRotate       time.Duration // idle time after which a child is treated as cold; 0 = off
internal/harness/repl.go:66:	MaxWorkers       int           // cap on concurrent live worker sessions (see resolveMaxWorkers)
internal/harness/repl.go:76:// Transcript is an own append-only JSONL, one thread across any number of claude sessions.
internal/harness/repl.go:158:	lastCostSess    string     // session of the last turn cost (turnCostUSD)
internal/harness/repl.go:174:	pool            *workerPool         // Haiku sessions with edit history (direct routing candidates)
internal/harness/repl.go:175:	parked          map[string]string   // model -> session id left by a direct Haiku switch
internal/harness/repl.go:176:	resumeID        string              // session id start() resumes once
internal/harness/repl.go:179:	idle            idleTurn            // child-initiated turn being rendered (idle reader goroutine only)
internal/harness/repl.go:182:	idleRotate      time.Duration       // idle deadline base (--idle-rotate); 0 = off
internal/harness/repl.go:187:	idleFiredFor    time.Time           // lastAPI value an idle rotate already fired for
internal/harness/repl.go:188:	cur             *liveChild          // record of the active child (idle callback identity; set by start/bindChild)
internal/harness/repl.go:189:	clockMu         sync.Mutex          // guards lastAPI, ttlSeen, idleFiredFor (idle goroutine writes, UI reads)
internal/harness/repl.go:191:	ws              map[string]*wsState // mode f: per-workspace Haiku sessions
internal/harness/repl.go:235:// newSessionFn is a test seam for start().
internal/harness/repl.go:263:	r.lastAPI = time.Time{} // new child: no API call yet (idleDue needs a turn)
internal/harness/repl.go:296:// idleEvent handles stdout events that arrive while no turn is running (background subagents).
internal/harness/repl.go:357:// idleTurn accumulates a child-initiated main-thread turn (e.g. the reply to a background
internal/harness/repl.go:392:// idleFinish ends a child-initiated turn: logs it like a normal turn and settles the agents counter.
internal/harness/repl.go:447:// askPerm answers a can_use_tool request on s. idle = no turn running: on the plain path stdin
internal/harness/repl.go:496:	select { // drop a stale Ctrl-C from while idle
internal/harness/repl.go:707:// agentsBusy: delegated agents of the current session that have not finished.
internal/harness/repl.go:726:// makeSnapshotN is makeSnapshot plus the number of handoff attempts (for rotate's log).
internal/harness/repl.go:824:// closeDrains ends every draining session (harness exit); unfinished agents stay only in the transcript.
internal/harness/repl.go:833:// pollDrains finishes old sessions whose agents are done; queues a notice for the next user message.
internal/harness/repl.go:924:		fields["cost_turn_usd"] = d // cost_usd is cumulative per session; this is the turn's own share
internal/harness/repl.go:1166:// clear drops the conversation: new session, no handoff snapshot.
internal/harness/repl.go:1168:	r.endAllParked("clear") // parked children and their pre-clear session ids must not come back
internal/harness/repl.go:1182:// clearWithHandoff is /clear in the TUI: the next session gets a handoff snapshot of the conversation
internal/harness/repl.go:1183:// that is dropped (taken before anything is closed). A held handoff leaves the new session empty.
internal/harness/repl.go:1204:// setConductor toggles classifier routing (a one-shot Sonnet call per prompt picks the model of the session that answers).

## go.mod require
require (
	github.com/charmbracelet/bubbles v0.20.0
	github.com/charmbracelet/bubbletea v1.3.4
	github.com/charmbracelet/lipgloss v1.1.0
	github.com/charmbracelet/x/ansi v0.8.0
	github.com/mattn/go-runewidth v0.0.16
	github.com/muesli/termenv v0.16.0
)

require (
	github.com/aymanbagabas/go-osc52/v2 v2.0.1 // indirect
	github.com/charmbracelet/colorprofile v0.2.3-0.20250311203215-f60798e515dc // indirect
	github.com/charmbracelet/x/cellbuf v0.0.13-0.20250311204145-2c3ea96c31dd // indirect
	github.com/charmbracelet/x/term v0.2.1 // indirect
	github.com/erikgeiser/coninput v0.0.0-20211004153227-1c3628e74d0f // indirect
	github.com/lucasb-eyer/go-colorful v1.2.0 // indirect
	github.com/mattn/go-isatty v0.0.20 // indirect
	github.com/mattn/go-localereader v0.0.1 // indirect
	github.com/muesli/ansi v0.0.0-20230316100256-276c6243b2f6 // indirect
	github.com/muesli/cancelreader v0.2.2 // indirect
	github.com/rivo/uniseg v0.4.7 // indirect
	github.com/xo/terminfo v0.0.0-20220910002029-abceb7e1c41e // indirect
	golang.org/x/sync v0.11.0 // indirect
	golang.org/x/sys v0.30.0 // indirect
	golang.org/x/text v0.3.8 // indirect
)
