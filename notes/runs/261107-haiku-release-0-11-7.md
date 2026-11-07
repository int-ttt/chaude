# 261107 haiku release 0.11.7

status: in progress (commit/push/tag/CI pending at write time)
branch: rust-port (base da0a4d5 release 0.11.6), master ff target
procedure: notes/runs/261106-haiku-release-0-11-6.md

## code (uncommitted before this run, now in release)
- tui/mod.rs tui(): status bar seeded from Config.perm_mode (set_start_mode), test status_bar_starts_on_the_engine_start_mode
- tui/mod.rs plain(): ctx.mode = start_mode from cfg.perm_mode (was hardcoded PermMode::Default)
- core api.rs: Config.perm_mode (default Auto); actor() mode = perm_mode
- tests/engine.rs: configured_start_mode_is_the_engine_mode
- tui/resume_pick.rs: MAX_ROWS 30 cap removed; test groups_beyond_thirty_stay_reachable
- core settings/project.rs: hook_python order py -3, python (no python3); tests windows_guard_has_no_python3, main_guard_powershell_*
- core settings/mod.rs: windows_repo_settings_have_no_python3
- cargo fmt --all applied (ran with file arg; rustfmt formatted resume_pick.rs test asserts)

## version
- Cargo.toml:7, Cargo.lock (6 chaude-* entries), README.md:73, README.ko.md:73, docs/claude-lean/README.md:3 -> 0.11.7
- CHANGELOG.md: 0.11.7 section EN+KO (3 bullets, Windows re-apply note); Unreleased empty
- README.md / README.ko.md: Windows py -3 re-apply bullet under Features

## checks
- cargo fmt --all --check: exit 0
- cargo clippy --workspace --all-targets -- -D warnings: exit 0
- cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings: exit 0
- cargo test --workspace --no-fail-fast: exit 0 (engine 107 passed, 0 failed; all result lines ok, 1 ignored pre-existing)
- cargo build --release --target aarch64-apple-darwin: exit 0; target/.../chaude --version -> chaude 0.11.7

## mac deploy
- /usr/local/bin/chaude is root:wheel (currently 0.11.6). sudo -n true -> "sudo: a password is required" (exit 1)
- NOT deployed. exact cmd when allowed: sudo install -m 755 target/aarch64-apple-darwin/release/chaude /usr/local/bin/chaude && /usr/local/bin/chaude --version (expect chaude 0.11.7)

## doubts
- Excluded from commit: .claude/settings.json, plan/caveman-structure.md, notes/runs/* other than this file and 261104-haiku-cli-resume-picker.md, record/*, and other untracked notes.
- project.rs also carries main_guard_* PowerShell tests (different workstream hunk in same file); committed with the file.
- Windows hook fix re-apply: existing Windows .claude/settings.json keeps python3 until regenerated (documented in CHANGELOG/README).
