# Open items
<!-- One line each. Newest at bottom. Close by deleting the line and noting the commit in DECISIONS.md. -->
<!-- format: - [area] item (owner/trigger) -->
- [deprecate] deprecated/ (Go) kept through the 5 releases after 0.10.0; delete in the 6th release after 0.10.0 (trigger: release count after 0.10.0)
- [i18n] README.ko.md deprecation notice and CHANGELOG "## Unreleased" entries (0.10.0 prep) are English; Korean translation pending by haiku-tui before release
- [rust] phases 3-6 open (harness logic into core, TUI beyond T1, GUI placeholder, parity + switch release to Rust): plan/rust-port.md
- [release] release.yml still builds the Go binary only; Rust release path not written
- [install] platform_setup/install.* (dev build from source) and root install.* (release installer) are different scripts, not duplicates; both kept
- [docs] notes/runs/*, notes/LEDGER.md keep old rust/ and tools/ paths as history; not rewritten
