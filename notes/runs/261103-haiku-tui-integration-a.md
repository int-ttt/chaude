# 261103-haiku-tui-integration-a
status: done (partial scope, see notes/runs/tui-integration-a.md)
changed: crates/cli/src/tui/mod.rs, crates/cli/src/tui/edit.rs
cause: inline TUI had no full-screen frame model; scroll/chrome/keys not wired
check: cargo fmt --all -- --check -> PASS; cargo clippy --workspace --all-targets -- -D warnings -> PASS; cargo test --workspace -> PASS (144 passed, 0 failed)
risk: alt screen and mouse capture not verified on a real TTY; TermGuard is created after Engine::start so a failed enter leaves the engine running; /status /version /mode still engine-side
