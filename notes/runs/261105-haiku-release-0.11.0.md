# 261105 haiku-release 0.11.0
workstream: release-0.11.0
status: done
commit: 83ef2e6 on rust-port (release: 0.11.0 (Rust switch))
tag: v0.11.0 -> 83ef2e6 (annotated) pushed; 22123fce5f856bafa76aaaf53dbaa6d2f25276d3

## files committed (6)
- Cargo.toml: workspace version 0.10.2 -> 0.11.0
- Cargo.lock: 6 chaude-* entries -> 0.11.0 (cargo build --workspace)
- CHANGELOG.md: "## Unreleased" -> "## 0.11.0 - 2026-11-05", empty Unreleased kept above
- README.md, README.ko.md: CHAUDE_VERSION=v0.11.0 (install section)
- notes/OPEN.md: [release] released 0.11.0; [deprecate] 0.11.0 = release #3 after 0.10.0 (v0.10.1, v0.10.2, v0.11.0)

## push
- master 5d4949e..83ef2e6 (ff, local master ref moved via update-ref, no checkout)
- rust-port 01c49f9..83ef2e6 (ff)
- tag v0.11.0 pushed

## checks
- cargo fmt --all -- --check: exit 0
- cargo clippy --workspace --all-targets -- -D warnings: exit 0
- cargo test --workspace: exit 0, passed=719 failed=0 ignored=3 (log /tmp/chaude-test-0.11.0.log)
- note: "kill: 67364: No such process" printed during tests/engine.rs run; test still passed

## workflow
- release.yml run 42044784194: success (rust-build x4 success, go-release skipped, release success)
- GitHub release v0.11.0: not draft, not prerelease, published 2026-11-05T14:26:56Z
- assets: chaude_0.11.0_darwin_arm64.tar.gz, chaude_0.11.0_linux_amd64.tar.gz, chaude_0.11.0_linux_arm64.tar.gz, chaude_0.11.0_windows_amd64.zip, checksums.txt

## doubts / not done
- commit attribution: brief said Claude Opus 5.5; system attribution says Claude Haiku 5.5. Used Haiku 5.5. Amend if Opus wanted.
- not touched (not in brief): deprecated/src/main/intt/cmd/chaude/main.go `var version = "0.10.2"` (Go, kept for 5 releases); docs/claude-lean/README.md line 3 "**버전 0.10.2**". bump-version.sh would update both.
- unrelated dirty tree left untouched (many untracked notes/runs, .claude/*, plan/caveman-structure.md).
- worktree /private/tmp/wt-width2 and others untouched.
