# 261104 haiku test-parity-p3 final
task: test parity group 3 + report-only checks
branch: rust-port (no push)
scope: crates/core/tests/* new files, plan/rust-test-parity-p3.md; no crates/*/src edits

## p3 counts (table-wide, plan/rust-test-parity-p3.md)
rows: 91
covered: 77 (10 new this run: TestTurnWaits existing engine.rs test + 9 in parity_harness_p3_spend.rs)
ignored: 13 (stubs in parity_harness_p3_ignored.rs, panic "not ported", #[ignore = reason])
missing: 0
n-a: 1 (TestUsageLive)

## test runs
cargo test -p chaude-core --test parity_harness_p3_spend -> 9 passed, 0 failed, 0 ignored
cargo test -p chaude-core --test parity_harness_p3_ignored -> 0 passed, 0 failed, 13 ignored
cargo test -p chaude-core --test engine slow_agent_result -> 1 passed

## checks (report only)
check1 TUI /resume list: partial
  cap 30 match: deprecated/src/main/intt/internal/harness/tui.go:1111-1112 vs crates/cli/src/tui/resume_pick.rs:18 (MAX_ROWS=30), :47 truncate
  plain path: Go resume.go:129-130 caps 20; Rust not compared in this run
  label text not compared byte-for-byte
  cargo test -p chaude-cli resume -> 8 passed
check2 extra coder default 4: match
  Go deprecated/src/main/intt/internal/harness/chaude_settings.go:20 (settingsDefaultExtraCoders = 4)
  Rust crates/core/src/api.rs:378 (extra_coders: 4)
check3 import on normal-mode start: match
  Go tui.go:1388-1389 startView; importclaude.go:207
  Rust crates/cli/src/tui/mod.rs:2009 (tui startup import), :2136 (plain); test mod.rs:3138
check4 update vs real network: partial
  read-only GET https://api.github.com/repos/int-ttt/chaude/releases/latest -> HTTP 200, tag v0.10.2
  assets: darwin_arm64, linux_amd64, linux_arm64, windows_amd64.zip, checksums.txt
  chaude update NOT run (installs when newer; no dry-run flag)
  Rust crates/update/src/update.rs:186-205 run_cli installs; Go deprecated/src/main/intt/internal/update/update.go:292-308 installs

## doubts
- api.rs under active edit by other coder (lib build intermittently broken); not staged
- background agents a2bf149adbd2bd810, a902c3184737faf09 still running
