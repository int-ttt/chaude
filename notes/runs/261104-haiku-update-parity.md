# 261104 haiku update parity (group 3, update/)
task: map 16 Go update tests (plan/rust-test-parity-p3.md rows 214-229) to Rust tests in crates/update
status: done
src_change: crates/update/src/cleanup.rs -> pub fn remove_old(exe: &Path) seam (Go removeOld); cleanup_old calls it. no other src edit.
new_file: crates/update/tests/parity_update.rs (16 tests, offline, Fetch fake, no network, no httptest)
covered: 16 (all added; 0 existing reused, 0 ignored)
ignored: 0
notable:
  - TestUnknownCurrentVersionIsRefused: Rust has no ErrUnknownVersion; returns Error::Failed "not a release version", refused before download. behaviour matches, note in plan row.
  - TestAssetNamePlatforms: asset_name is pub(crate); checked via run() "not found in release" text; darwin/amd64 via "Intel Mac".
  - TestNotWritableChangesNothing: unix only; skips if probe write succeeds. ran for real here (uid 501, not root), not skipped.
  - TestUpdateWindowsRenamesOldAndCleanupRemovesIt: runs goos=windows on darwin host (pure fs path).
plan_diff: 16 rows changed, Totals and section header still say "wait: update" (not touched per brief).
checks:
  - cargo test -p chaude-update --offline --test parity_update -> 16 passed, 0 failed
  - cargo test -p chaude-update --offline -> 15 unit + 16 parity passed, 0 failed
  - cargo clippy -p chaude-update --all-targets --offline -- -D warnings -> clean
risk: none known. parity tests depend on Fake matching update.rs URL shape (api_base + /assets/{name}).
