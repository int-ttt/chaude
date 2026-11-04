task: rust-port phase 5 GUI placeholder
branch: rust-port
status: done
changed:
  - Cargo.toml (removed exclude = ["crates/gui"] and its comment)
  - crates/gui/Cargo.toml (chaude-core path dep)
  - crates/gui/src/lib.rs (event_kind + 2 unit tests)
  - Cargo.lock (+chaude-gui entry)
  - plan/rust-port.md (phase 5 status)
checks:
  - cargo build --workspace -> exit 0
  - cargo test -p chaude-gui -> exit 0 (2 passed)
  - cargo fmt --check -> exit 0
  - cargo clippy -p chaude-gui --all-targets -- -D warnings -> exit 0
decisions:
  - Plan names Tauri for phase 5; brief asked for placeholder with no UI framework deps. Tauri not added.
  - default-members unchanged (core, cli); gui builds via --workspace.
not touched: crates/core, crates/cli, unrelated dirty files (.claude/hooks, notes/runs/*, crates/core/tests/engine.rs).
open: Tauri UI work, .dmg / .msi packaging.
