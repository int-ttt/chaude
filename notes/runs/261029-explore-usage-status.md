# explore: /usage handling, usage/rate storage, mainStatusText, ctx indicator (read-only)
dir: /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness
status: done
result: PASS (read-only; no code changed)

## /usage flow
- tui.go:645-650 busy path: line=="/usage" -> one-shot usageFetch(m.r.lastCtx) in goroutine -> usageSend(usageMsg). Other slash cmds while busy -> busySlashMsg (tui.go:689).
- tui.go:691 var usageSend = func(r *Repl, msg) { r.ui.send(msg) }
- tui.go:694-705 var usageFetch = func(ctx int) string: exec `claude -p /usage --output-format json --no-session-persistence` in os.TempDir(); on err returns "/usage 조회에 실패했습니다: ..."; parse fail "/usage 결과를 읽지 못했습니다."; returns decorateUsage(str(j["result"]), ctx)
- tui.go:943 idle path routes /usage to passthrough.
- repl.go:763 passthroughCmds includes /usage; repl.go:776-795 passthrough(): runTurn, if /usage -> decorateUsage(t, r.lastCtx); sent to ui as textMsg or r.p; then usageLine if usage non-empty.
- tui.go:425-429 case usageMsg -> m.av=nil, m.usage=text (popup). agentview.go:213-215 usagePopup (title "Usage", "(esc/q/enter close)"); agentview.go:221 usageKey closes.
- usagebar.go:41 decorateUsage: appends renderBar to each line containing N% (pctRe usagebar.go:37), then "\n\nContext (now): [bar] P%  (ctx / 270,000)" (usagebar.go:52-53). ctxBarMax=270_000 (usagebar.go:13), barCells=10 (usagebar.go:15).
- renderBar usagebar.go:18: "[████░░░░░░] 45%", " !" if >100.
- claude raw text format: not in code; only test fixture "Current session  45%" (usagebusy_test.go:31, agentview_test.go:59).

## usage/rate data
- No typed struct. Rate data is map[string]any.
- repl.go:140-141 Repl fields: lastCtx int; rate map[string]any.
- repl.go:217-218 and repl.go:476-477: case rate_limit_event -> r.rate = asMap(ev["rate_limit_info"]).
- repl.go:521-524 usageLine reads only r.rate["unifiedWindows"]["five_hour"]["utilization"] (-> "5h %s", else "?").
- Fields NOT present in code: reset time (resetsAt not referenced), percent as typed field (only utilization via fmt.Sprint), last-refresh time (no timestamp stored for rate or /usage).
- ctx: repl.go:58-59 core.go ctxTokens = input + cache_read + cache_creation. repl.go:518 r.lastCtx = ctx (set in usageLine); repl.go:925 reset to 0 on rotate.
- metrics.go:49-56 usageFields: input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, ctx.

## /usage output format (current)
- popup: "Usage  (esc/q/enter close)" + "\n\n" + m.usage (raw claude text, each % line + bar), then blank line + "Context (now): [bar] P%  (N / 270,000)".
- per-turn usage line (repl.go:530-533): "[ctx N | in N cr N cc N out N | $0.0000 | 5h X | sess ID rot N]"; 5h is "?" when no rate info.

## mainStatusText (tui.go:730-745)
- mainStatusText: base + " · MM mSS" elapsed when busy && turnStart set, + " · lastTool".
- mainStatusBase (tui.go:739): statusText("생각중", model, "메인", trunc(curPrompt first line, 60)); empty summary -> no parens.
- statusText (tui.go ~752): "상태-모델-이름(요약)".
- Rendered at tui.go:313 (fg agents, no text yet) and tui.go:315 (+" (esc to interrupt)").

## context indicator
- tui.go:972-985 statusLine(): " <model> · ctx <lastCtx>/<effectiveThreshold> · rot N" + " · mode X" + " · agents N running (bg N)"; right side "esc interrupt · /rotate · wheel/pgup/pgdn scroll · ctrl+c quit ".
- localcmd.go:39 /status line: "model M · ctx N/T · rotations N · session S" (tui.go:894 same in TUI).

## tests
- usagebar_test.go: TestRenderBar (line 8), TestDecorateUsage (line 28)
- usagebusy_test.go: TestUsageAllowedWhileBusy (10), TestUsageSlashAcceptedWhileBusy (28), TestUsageMsgWhileBusyOpensPopup (51)
- agentview_test.go:59-68 usageMsg popup open/close
- localcmd_test.go:27 "ctx 1,234/" in /help-or-status check
- unit_test.go:32-47 ctxTokens (expects 20591)
- metrics_test.go:61-67 usageFields
- idle_test.go:113-118 "[ctx" usage line
- NOT found: test for rate_limit_event / unifiedWindows / 5h parsing; test for statusLine/mainStatusText directly (grep showed none by name in this scan; not exhaustively verified).

## doubts / not found
- No reset time, no percent field, no last-refresh timestamp anywhere in harness internal/harness.
- Raw /usage text format depends on claude CLI; not defined in repo.
