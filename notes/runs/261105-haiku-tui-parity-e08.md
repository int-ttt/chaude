# 261105 haiku tui-parity E08 (idle permission deny note)

status: done (deliberate n/a, no code change)
branch: rust-port (not pushed)
truth: deprecated/src/main/intt/internal/harness/repl.go:572-600, caller repl.go:431

gate (Go): askPerm takes r.ui branch first (repl.go:580). Idle deny note needs r.ui == nil AND !AutoAllow AND idle.
caller: idleEvent repl.go:431 passes idle=true, only reached while no turn runs on the plain path.
Rust: no plain mode in crates/cli; engine always has a UI channel (api.rs:2097-2099 comment, can_use_tool arm api.rs:2104-2123).
decision: do not port. Adding a no-UI engine flag would be dead code with no caller and would change TUI behavior if misapplied.
changed: plan/tui-parity.md:204 (E08 marked [x], audited deliberate), this note.
check: plan-only edit; no build run needed (no source change). ws: none
risk: if a plain/no-UI mode is added later, revisit E08 with the gate above (r.ui==nil && !auto_allow).
