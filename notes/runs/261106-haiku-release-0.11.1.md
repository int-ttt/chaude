task: release chaude 0.11.1 (same procedure as 0.11.0)
status: in progress
checks:
- cargo test --workspace: PASS (725 passed, 0 failed)
- cargo clippy --all-targets: PASS (0 warnings)
- cargo fmt --all -- --check: PASS
files:
- Cargo.toml, Cargo.lock (chaude-* 0.11.0 -> 0.11.1)
- README.md, README.ko.md (CHAUDE_VERSION=v0.11.1), README reorder
- CHANGELOG.md (## 0.11.1 - 2026-11-06, EN + KO lines)
- notes/OPEN.md (0.11.1 = release #4 after 0.10.0; delete deprecated/ in #6)
- crates/cli/src/tui/mod.rs (SendMessage-resumed agents in list, yellow running rows)
