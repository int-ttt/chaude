# claude-squeeze (jongwooish/claude-squeeze) research table

| item | value | source |
|---|---|---|
| description | "Keep long command output out of Claude's context. Token saver for Claude Code. Rust, zero dependencies." | https://api.github.com/repos/jongwooish/claude-squeeze |
| license | MIT | https://api.github.com/repos/jongwooish/claude-squeeze ; https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| stars / forks / open issues | 1 / 0 / 1 (snapshot at fetch time) | https://api.github.com/repos/jongwooish/claude-squeeze |
| created | 2026-11-03T05:19:29Z | https://api.github.com/repos/jongwooish/claude-squeeze |
| last push | 2026-11-10T06:59:53Z (last commit 2026-11-10 06:59:52 UTC "docs: what to do when Windows, macOS or Linux blocks the release files") | https://api.github.com/repos/jongwooish/claude-squeeze ; https://api.github.com/repos/jongwooish/claude-squeeze/commits?per_page=5 |
| releases | v0.1.0 Sentinel (2026-11-03, 7 assets); v0.2.0 squeeze-Auditor (2026-11-10, 13 assets); neither prerelease | https://api.github.com/repos/jongwooish/claude-squeeze/releases |
| version / crate | squeeze 0.2.0, rust-version 1.80, no [dependencies] | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/v0.2-Auditor/v0.2.0_squeeze-auditor/Cargo.toml |
| language | Rust | https://api.github.com/repos/jongwooish/claude-squeeze |
| token savings (replay) | 93.8% total, 83,325 -> 5,128 est. tokens over 7 logs; per log 86.9%–96.8%; short npm output 0% | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| error retention claim | 50 of 50 distinct errors/warnings kept in fixtures | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| measurement method | saved logs replayed; tokens = bytes / 4; one log synthetic; not a live A/B test with Claude Code (listed as planned) | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| mechanisms | Bash pre-tool hook denies noisy commands (npm install, go test, pytest) -> hint to use `squeeze run`; read guard redirects reads >400 lines to outline; compression keeps first 8 + last 20 lines + errors/warnings with 2 lines context; dedupe with counts; repeated-read blocker; budget pause ~150K tokens; handoff note ~120K tokens; fixed-cost audit of CLAUDE.md/skills/agents/MCP; idle MCP auto-off after 14 days; Strong/Balanced/Light modes | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| not claimed | model routing, subagents, caching (README contrasts itself with wrappers that rotate sessions/switch models) | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| install | installers (Win .exe, macOS .pkg, Debian .deb; not code-signed); curl script (macOS/Linux), irm script (Windows), `cargo install` from git (Rust 1.80+); SHA-256 checksums | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| usage | `squeeze setup` then `claude`; `squeeze auditor`; run, compress, clip, report, bench, doctor, init, mcp | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| requirements | macOS/Linux/Windows; Claude Code for hooks; Claude desktop app for desktop tools | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| limitations (stated) | token counts estimates; claude.ai/mobile only manual `squeeze clip` until v0.3; desktop integration cannot see/change chats; dashboard does not show official plan usage; installers unsigned; 0.x so CLI/config may change; Pro plan shares one limit between Claude Code and claude.ai; unofficial, not affiliated with Anthropic | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/README/README.md |
| contributor rules | stdlib only; hooks never approve tool calls, fail open; auditor binds 127.0.0.1 with per-launch token; Claude credentials never read | https://raw.githubusercontent.com/jongwooish/claude-squeeze/main/CLAUDE.md |

## Unverified / not found
- Root README.md: 404 at raw root; README is at README/README.md (6 languages).
- Last commit date from repo HTML page: not shown; taken from API.
- Live Claude Code A/B savings: not measured by the project (stated as planned).
- Cargo.toml of CARGO workspace has no [package] section (workspace-only).
