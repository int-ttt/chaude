task: mode f ws routing (orchws.go, orchws_test.go, classifier.go route, repl.go struct+handle)
status: done, PASS
changed:
  - internal/harness/orchws.go (new, 134 lines): wsRe, parseWS, wsState, consts, wsNeedsRotate, wsHandoff, orchSwitch, orchFresh, routeOrch, wsRecord
  - internal/harness/orchws_test.go (new, 92 lines): TestParseWSCases, TestWSNeedsRotateCases, TestWSHandoffRuneCap, TestRouteOrchBReturnsLine
  - internal/harness/classifier.go:215-217 route(): early return for Orch != "" && != "m0" -> r.routeOrch(line)
  - internal/harness/repl.go:190-193 Repl fields ws, curWS, orchMain (gofmt realigned block 172-193)
  - internal/harness/repl.go:918 handle(): r.wsRecord(reply) after r.usageLine(result, usage)
fresh-session path: non-children -> switchSession(model, "") (it always closes and restarts; no equality guard). children mode (reg && children) -> routeChildren(model,"","") (no-op when model already active; switchChild returns nil when target==a.Model and resumeID==""). Gap in children mode for forced same-model fresh haiku.
checks:
  gofmt -l internal/ -> empty (after gofmt -w repl.go; only struct block changed)
  go build ./... -> ok, no output
  go vet ./internal/harness/ -> exit 0
  go test ./internal/harness/ -run 'Orch|WS|Ws' -count=1 -v -> 7 PASS (TestOrchParseMode, TestOrchDefaultModel, TestOrchRule, TestParseWSCases, TestWSNeedsRotateCases, TestWSHandoffRuneCap, TestRouteOrchBReturnsLine); ok chaude/internal/harness 0.442s
doubts:
  - mode f routeOrch with a session (resume/new) not unit-tested (needs claude child).
  - curWS is not cleared by isAck or "/" lines (route returns early), so a following slash-command reply may be recorded to the last ws.
  - route() now skips classifier for modes b-e (intended per brief).
  - children mode: orchFresh cannot force same-model haiku restart.
