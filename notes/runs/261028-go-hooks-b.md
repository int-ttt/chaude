# go hooks-b result
files: internal/hooks/{correction_guard,context_alert,agent_model_pin,main_thread_guard,haiku_context_guard,ledger,history_export}.go
checks: gofmt clean, go vet ./internal/hooks clean, go build ./cmd/harness ok
differential: python3 original vs go (temp CLAUDE_PROJECT_DIR each), compare exit code + stdout (JSON semantic) + all written files (ledger timestamps normalized)
result: 86/88 pass (88 cases)
cases: correction-guard 22 (rules A-F, generic 아니, 2nd/3rd consecutive, reset, garbage state, sid sanitizing, no sid, lean-off flag, CLAUDE_LEAN=0, bad json);
  agent-model-pin 22 (builtin deny, no type, haiku/sonnet model strip, claude-code-guide pin variants, custom agent, other tool, agent_id deny/allow, lean-off, bad json);
  main-thread-guard 12; haiku-context-guard 11; context-alert 10 (under/over limit, unset, state skip/+25k, missing transcript, lean-off);
  ledger 18 (model shortening haiku/sonnet-4/opus-4.7/fable/odd/<synthetic>, ping skip, short input, list content, empty msg, 300-char cut, explicit transcript, cfg fallback, 300-line rotation, no trailing newline)
known diffs (intentional, 2 FAIL): python crashes exit 1 (traceback) on bad stdin JSON in context-alert and ledger, and on non-int LEAN_RESET_AT; go exits 0 silently (plan: never crash).
not diffed: history-export (needs scripts "export-history" from another agent; currently go prints 'unknown script' to stderr, exit 0). Python also skipped when scripts/export_history.py missing; go always calls scripts.Run.
minor: python \w/\s unicode vs go ASCII/unicode-letter classes in sid regex; splitlines on exotic separators ( ) not replicated.
