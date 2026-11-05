# Open items
<!-- One line each. Newest at bottom. Close by deleting the line and noting the commit in DECISIONS.md. -->
<!-- format: - [area] item (owner/trigger) -->
- [deprecate] deprecated/ (Go) kept through the 5 releases after 0.10.0; delete in the 6th release after 0.10.0 (trigger: release count after 0.10.0). Count: 0.10.1 and 0.10.2 are #1 and #2; 0.11.0 is release #3 after 0.10.0 (tags v0.10.1, v0.10.2, v0.11.0); 2 more to go (#4, #5), delete in #6.
- [i18n] done: README.ko.md synced with README.md (Rust release assets, Go deprecation notice) and CHANGELOG "## Unreleased" Korean lines added (261105)
- [rust] phases 3-6 open (harness logic into core, TUI beyond T1, GUI placeholder, parity + switch release to Rust): plan/rust-port.md
- [release] released 0.11.0 (261105, Rust switch, replaces planned 0.10.3; tag v0.11.0). release.yml Rust path (commit 93d1405; Go job disabled, if: false)
- [install] platform_setup/install.* (dev build from source) and root install.* (release installer) are different scripts, not duplicates; both kept
- [docs] notes/runs/*, notes/LEDGER.md keep old rust/ and tools/ paths as history; not rewritten
