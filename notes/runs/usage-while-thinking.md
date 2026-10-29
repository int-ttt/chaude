# usage-while-thinking
gate: internal/harness/tui.go enter handler, `if m.busy && HasPrefix(line,"/")` returned silently
change: busy + "/usage" -> goroutine usageFetch (one-shot `claude -p /usage --no-session-persistence`, cwd tmp) -> usageSend -> usageMsg popup; busy state/queue untouched
change: other slash while busy -> note busySlashMsg (Korean 존댓말)
change: usageMsg no longer resets gotText when busy
verified: one-shot `claude -p /usage` returns json (checked manually, result field not inspected beyond success)
tests: internal/harness/usagebusy_test.go (3); go vet + go test ./... PASS
doubt: /usage result field in json assumed = text (same as passthrough path); not seen live in TUI
