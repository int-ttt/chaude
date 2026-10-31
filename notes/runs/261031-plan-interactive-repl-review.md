# 261031-plan-interactive-repl-review
plan: notes/runs/261031-plan-interactive-repl.md
verdict: sound with changes below; do not start Phase 1 before the spike gate and the user's DECISIONS L47 decision.

## improvements
1. ordering: move S0.0 (docs research) strictly before writing the probe; probe flags depend on confirmed hook schema.
2. version pin: add a startup check in ptychild.go comparing `claude --version` to the spike-recorded version; on mismatch print notice and continue (transcript format is not a public contract).
3. parity risk: project hooks (.claude/settings.json: style-adapter, main-thread-guard, correction-guard) run inside the interactive child too; spike S0.2 must also log which project hooks fire and whether output differs from the -p child.
4. metrics: tag every metrics/ledger record with backend so pty runs (cost 0) do not skew cost reports; add to Phase 3 acceptance.
5. permission hook: if PreToolUse "ask" semantics differ from stdio prompt tool (e.g. claude still shows its own prompt), fall back to running the pty child with the project's permission settings and document it; add as spike check S0.7.
6. Stop hook missing (older claude): fail start with explicit error, never guess turn end from idle timers.
7. dependency: prefer a term package already in go.sum for attach raw mode; check before adding golang.org/x/term.
8. seam commit: add a test asserting stream argv is byte-identical before/after Phase 1 (guards core.go:145-153).
9. cleanup: tmp settings and socket must be removed on every exit path, including harness panic (defer + signal handler); add pgrep check to integration test.
10. scope control: Phase 2 integration tests must pass on fakeclaude before any real-claude run, to keep subscription usage minimal.
