# tui-parity final-1 run note
workstream: tui-parity final-1
branch: rust-port
truth: deprecated/src/main/intt/internal/harness/

batch-1 (rows audited: 15)
rows-same: I27 I28 I29 I30 I31 I32 I33 I34 I35 C01 C02 C03 C04 C05 C26
rows-fixed: none in this batch
rows-diff: none in this batch
C01-check: normalized name|desc compare of Go slashCmds (25) vs Rust SLASH_CMDS (25) -> identical
C02-check: Go suggestions tui.go:982-994 vs Rust suggestions commands.rs:344 -> same guards (/ prefix, no space/tab, not busy, no perm)
C03-check: help text = "%-8s desc" rows + HELP_KEYS, identical to Go tui.go:1057-1061
C04-check: /exit,/quit -> Quit, same as Go quit() tui.go:1196
C05-check: toggle + mouse_note strings identical; capture enable/disable synced in run loop mod.rs:2572
C26-check: BUSY_OK list identical to Go busyOK tui.go:849 (includes /settings, which the plan list omits); busy message identical
remaining-first-half: C07 C08 C09 C14 C15 E01 E02 E03 E04 E05 E06 E07 E08
