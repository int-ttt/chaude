# 261031-plan-interactive-repl
status: plan only, no source edits yet
user_request: "현재 구조 확인 후, claude -p 기반 구조를 새 git 브랜치에서 REPL 연결(interactive) 방식으로도 구현; 설계 판단 필요"
goal: keep resident `claude -p` stream-json child (core.go:145-169 NewSession) as DEFAULT; add opt-in second backend that drives the unmodified interactive `claude` through a pty, on a new branch.
prior: notes/runs/261031-plan-io-architecture-review.md:14 recommends AGAINST replacing the stream-json child with an interactive TUI child (loses cost/permission/notification signals, see 261030-plan-interactive-child-review.md:32). This plan does NOT replace it: new backend behind a flag, default unchanged, and signal recovery (transcript + hooks) is gated by a Phase 0 spike. DECISIONS.md L47 conflict must be resolved by the user before Phase 1.

## constraints (terms)
- binary: unmodified `claude` via osenv.ResolveBin(o.ClaudeBin); no patching, no injected code, no API proxy.
- auth: user's own subscription login as stored by claude itself. Go never reads ~/.claude/.credentials.json, the macOS keychain item, or any token env var; adds no auth env. Env stays withCacheTTL(cleanEnv(o.CompactWindow), o.CacheTTL) exactly as core.go:155.
- Go reads only: pty output bytes, the session transcript JSONL (local conversation data, path taken from hook payload transcript_path), and hook payloads sent by the harness's own hook command.
- input is keystrokes into the pty (same as a user typing). Documented features only: --session-id, --resume, --settings hooks.

## current structure (facts)
- S1 core.go:145-169: claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model M [--permission-prompt-tool stdio] [--resume id] Extra; StdinPipe/StdoutPipe; Session{cmd, stdin, lines chan string(256)}; Send core.go:192 writes {"type":"user","message":{...}}; RespondPermission core.go:205.
- Session API used by Repl: Send, Events, Interrupt, StopIdle, RespondPermission, Close, field SessionID. Session id captured from event session_id (core.go:251-252, 299-300); no --session-id flag today.
- children.go:32 `type closer interface{ Close() }`; liveChild.Sess is closer.
- one-shot -p sites stay -p in both backends: classifier.go:188, usagepoll.go:54, tui.go:703 (/usage), tui.go:953 (--version).
- go.mod: no pty library.

## options
A. pty + screen scraping (vt emulator parses TUI for text and turn end). single channel, but breaks on every claude UI change, ANSI/IME column issues (LESSONS 2026-10-29), no usage numbers. REJECT.
B. pty for input + transcript JSONL tail + hooks for structured events. Input = bracketed paste + CR. Output = transcript entries (assistant/user/tool_result/usage). Turn end, permission, session start = hooks calling back the harness over a unix socket. CHOSEN.
C. attach only: interactive claude on the user's terminal (raw passthrough), harness only watches transcript for metrics and rotation advice. Simplest, but no Send, so rotate/park/drain/orchestration impossible. Kept as optional Phase 4 (subset of B).
why B: only B keeps Repl features (rotate 150k, park-live, drain, idle-rotate 3599s) with zero Repl changes, by translating to the existing stream-json wire format (see seam). A is too fragile; C alone loses automation.

## seam (key design)
- Do not add a new child interface across Repl. Instead give Session a transport: stream backend = current stdin/stdout pipes; pty backend = adapter that (a) translates outgoing stream-json lines into pty keystrokes / hook replies, (b) emits stream-json-compatible lines into Session.lines from transcript + hooks.
- Result: Events parsing, setID, idle reader, drain, children, rotate all reused unchanged.
- transport interface (core.go, unexported): `type transport interface { writeLine(b []byte) error; closeInput() error; signal(sig os.Signal) error; wait() error; pid() int }`. Implementer copies exact usage from existing Session.write/Close/Interrupt/StopIdle; keep every method signature Repl uses unchanged.
- outgoing translation (pty): type=user -> "\x1b[200~"+text+"\x1b[201~"+"\r"; control_request subtype=interrupt -> "\x1b"; control_response for a permission -> reply to the pending PreToolUse hook request; anything else -> drop + debug log.
- incoming synthesis (pty): SessionStart hook -> {"type":"system","subtype":"init","session_id":...,"model":...}; transcript assistant/user entries -> {"type":"assistant"|"user","message":<entry.message>,"session_id":...,"parent_tool_use_id":...}; Stop hook -> {"type":"result","subtype":"success","session_id":...,"usage":<last assistant usage>,"total_cost_usd":0,"is_error":false,"backend":"pty"}; PreToolUse hook -> {"type":"control_request","request_id":<id>,"request":{"subtype":"can_use_tool","tool_name":...,"input":...}}. Exact key shapes must match what core.go Events already parses (implementer checks against testdata stream-json captures).
- no partial text deltas (transcript holds whole messages): TUI shows message-level output only for pty backend.

## branch strategy
- current branch interactive-child-compare has large uncommitted WIP (orch.go, turnspend.go, usagepoll.go, hooks, agents). User decides; recommended: commit it as 0.8.2 on interactive-child-compare first.
- new branch `interactive-repl-pty` from that commit. main untouched.
- commit order: (1) transport seam, no behavior change (cherry-pickable to main alone); (2) fake-claude test harness; (3) pty backend files; (4) flag wiring; (5) optional attach mode.
- merge gate: `go test ./...` green with default backend; pty real e2e behind build tag; sonnet-reviewer review of the diff.

## session lifecycle (pty backend)
- start: uuid v4 generated in Go. argv: `claude --session-id <uuid> --model M --settings <tmp hooks settings.json> [Extra]`; when o.Resume != "": `claude --resume <id> --model M --settings ...` (no --session-id; real id comes from SessionStart payload). NO -p. Dir=o.Cwd, Env as core.go:155. pty size 200x50 (managed).
- ready: wait for SessionStart hook for this process <= 20s. Timeout -> kill, return error "pty child not ready"; Repl prints notice "인터랙티브 자식이 시작되지 않았습니다. 폴더 신뢰 확인 등 첫 실행 대화상자가 있으면 claude를 한 번 직접 실행해 주십시오." and falls back to stream backend.
- transcript path: SessionStart payload transcript_path; fallback glob ~/.claude/projects/*/<uuid>.jsonl. Tail from offset 0 for new sessions, from EOF for --resume.
- turn: Send -> paste+CR. Turn end = Stop hook with matching session_id. Interrupt -> Esc.
- idle / background agents: tailer keeps running between turns; task-notification entries become user lines -> existing idle reader and agentTracker.onNotification.
- rotate / idle-rotate / park: unchanged Repl logic (new child started, old parked or drained).
- close: write "/exit\r", wait 15s (same as Session.Close), SIGTERM, then Kill; remove socket and tmp settings.
- crash: pty EOF -> close lines -> existing dead-child path.
- cost: interactive has no total_cost_usd. budget disabled for pty backend; metrics records backend=pty so cost stats are not mixed.

## hooks protocol
- tmp settings (osenv.StateDir()/pty/<uuid>.settings.json): hooks SessionStart, Stop, PreToolUse(matcher "*", timeout 600; omitted when o.NoPermission), Notification; each command = `<abs chaude> hook --sock <sockpath> --event <Name>`.
- socket path short (macOS 104-byte limit): /tmp/chaude-<first 8 hex of uuid>.sock, mode 0600.
- `chaude hook` (new subcommand, cmd/chaude/main.go): read stdin JSON, send one line {"event":Name,"payload":<stdin>} to socket, read one response line, print response JSON to stdout, exit 0. Socket missing -> print nothing, exit 0 (never block claude).
- PreToolUse response: {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"|"deny","permissionDecisionReason":"<text>"}}. No answer within 590s -> deny.
- user/project hooks in .claude/settings.json still run (merge); same as today's -p child.

## files / ordered steps
Phase 0 spike (gate; no harness code)
- S0.0 haiku-researcher: official docs for interactive --session-id, --settings hook merge, PreToolUse output schema, hook timeout, SessionStart/Stop payload fields (session_id, transcript_path). Save notes/research/claude-hooks-interactive.md with URLs.
- S0.1-S0.6 haiku-coder writes tests/spike/ptyprobe/main.go (build tag spike, uses creack/pty); haiku-runner runs it against real claude (haiku model, one short turn):
  S0.1 --session-id accepted interactively; transcript file created; flush latency per message.
  S0.2 SessionStart/Stop/PreToolUse fire via --settings with session_id and transcript_path.
  S0.3 bracketed paste multi-line + Korean submits exactly once.
  S0.4 transcript assistant entry keys vs stream-json assistant event keys (diff); usage present.
  S0.5 background Task completion appears in transcript while idle.
  S0.6 "/exit\r" ends process; Esc interrupts a running turn.
  record `claude --version`. Save notes/runs/261031-spike-pty-results.md (key: value per check). Copy 1 anonymized transcript to internal/harness/testdata/pty_transcript_basic.jsonl.
- GATE: S0.1, S0.2, S0.4 must PASS, else stop and report to user (do not build Phase 2).
Phase 1 seam (no behavior change)
- core.go: add transport interface + stdioTransport wrapping current cmd/StdinPipe/StdoutPipe; Session uses it; argv and env untouched.
- session options struct: add `Backend string` ("" or "stream" = current, "pty").
- check: go test ./internal/harness/... all PASS, identical to before.
Phase 2 test harness first
- internal/harness/testdata/fakeclaude.sh: reads pty input lines, appends user/assistant entries to the transcript path, invokes the hook command for SessionStart/Stop/PreToolUse, exits on /exit. Used via o.ClaudeBin.
Phase 3 pty backend
- go.mod: add github.com/creack/pty (latest v1.1.x).
- internal/harness/ptychild.go: startPty(o) (transport, error); argv build; ready wait; close sequence.
- internal/harness/transcripttail.go: poll 100ms (no fsnotify), partial-line buffering, entry -> stream-json line conversion.
- internal/harness/hooksock.go: unix socket server, request/response matching by tool_use_id, pending permission map.
- cmd/chaude/main.go: `hook` subcommand.
- repl flag `--child-backend stream|pty` (default stream) + env HARNESS_CHILD_BACKEND; Repl passes into options only.
Phase 4 optional attach mode
- `--child-backend pty-attach`: raw copy pty<->user terminal (reuse a term package already in go.sum; no new dep if possible), Ctrl-] detaches to harness; harness only tails transcript for usage line and rotation advice.

## tests
- unit: transcripttail_test.go golden (testdata/pty_transcript_basic.jsonl -> expected stream-json lines); hooksock_test.go allow/deny roundtrip, timeout -> deny, socket missing -> hook exits 0.
- integration (no real claude): ptychild_test.go with fakeclaude.sh: start, Send "hello", receive assistant + result, Interrupt, Close leaves no process.
- regression: go test ./... with default backend unchanged.
- real e2e: `go test -tags pty_e2e -run TestPtyRealClaude ./internal/harness/` (manual, user's login, haiku, 1 turn).
- acceptance (manual): `chaude repl --child-backend pty`: reply to "ok라고만 답해", usage line shows ctx, forced rotate works, Ctrl-C interrupts, exit leaves no claude process (pgrep).

## edge cases
- first-run dialogs (folder trust, onboarding, update banner) block the pty -> ready timeout + fallback notice.
- --resume may yield a new session id -> always take id from SessionStart payload.
- SubagentStop must not end the turn; only Stop with matching session_id.
- very long paste may be collapsed by claude UI; transcript still has full text.
- multiple children in same cwd: separated by session id / socket.
- terminal resize irrelevant in managed mode (fixed size).
- claude version change may alter transcript format -> compare against version recorded in spike; warn on mismatch.

## must NOT change
- default backend (stream) behavior, core.go argv/env for stream, one-shot -p sites, existing tests, CLAUDE.md, settings.json, existing DECISIONS entries (append a new one only after user decision). Never read credentials/tokens.

## agents
- Phase 0: haiku-researcher, haiku-coder, haiku-runner. Phase 1 and 3: sonnet-analyst (design-level multi-file) delegating mechanical edits to haiku-coder. Verify: haiku-checker per step; sonnet-reviewer before merge.

## open questions (user only)
- 브랜치 전 현재 미커밋 작업을 0.8.2로 커밋할지 여부
- 브랜치 이름 interactive-repl-pty 사용 여부
- attach 모드(Phase 4) 필요 여부
