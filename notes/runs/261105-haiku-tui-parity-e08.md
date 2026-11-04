# 261105 haiku tui-parity E08 (idle permission deny note)

status: done (deliberate n/a, no code change)
branch: rust-port (not pushed)
truth: deprecated/src/main/intt/internal/harness/repl.go:572-600, caller repl.go:431

gate (Go): askPerm takes r.ui branch first (repl.go:580). Idle deny note needs r.ui == nil AND !AutoAllow AND idle.
caller: idleEvent repl.go:431 passes idle=true, only reached while no turn runs on the plain path.
Rust: CORRECTION (261105): plain mode exists (crates/cli/src/tui/mod.rs plain(), HARNESS_PLAIN). Earlier 'no plain mode' claim was wrong. Fixed in 6aa7ab5: plain() sets Config.no_ui=true; can_use_tool Ask with no_ui and !busy denies with the Go note (api.rs can_use_tool arm; test plain_mode_denies_idle_permission_ask_with_notice).
decision: do not port. Adding a no-UI engine flag would be dead code with no caller and would change TUI behavior if misapplied.
changed: plan/tui-parity.md:204 (E08 marked [x], audited deliberate), this note.
check: plan-only edit; no build run needed (no source change). ws: none
risk: none open for E08. Caveat: plain() drains engine events only inside its submit loop, so the idle note prints at the next input line.
