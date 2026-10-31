# review: 261031-plan-repl-vs-print.md
verdict: 승인 (조건부) — 결정 자체는 유지, 아래 개선 반영 후 실행

## improvements
- ordering: step7 (DECISIONS.md 기록) should run first or right after user approval, not last; steps 3-4 depend on it being the recorded direction
- gate: step3 must be skipped entirely if step1 result is 불가; step4 is independent of step1 and can run in parallel with step1/2
- missing step: registry update on conversation_reset event (new session_id after /clear) — verify existing code handles it before step3; add haiku-scout grep `conversation_reset` in internal/harness/*.go
- missing check: step1 cmd C reuse test can pollute the session; run it with a second fresh uuid copy, not the one used for cmd B
- risk: step6 adds a new module dependency (creack/pty) -> go.mod/go.sum change; keep optional and ask user before adding
- risk: step1/2 run real claude calls on the subscription -> small usage; run while other sessions idle so transcripts are easy to isolate (/usage is account total)
- agent choice: step5 analysis should go to sonnet-analyst directly with haiku-scout excerpts as input (already stated); do not hand to opus
- acceptance gap: add `go vet ./internal/harness/...` alongside go test
- reconsider opt2 only if: a confirmed opt1 hard limit is recorded (e.g. U4 shows hooks cannot be applied in -p, or U5 shows the 15k gap cannot be reduced and matters for cost)
