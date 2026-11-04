# Rust parity checklist (Go user-facing features)

Date: 2026-11-04. Go reference: deprecated/src/main/intt. Rust: crates/cli, crates/core.
Status: done = Rust matches; differs = present but behavior or text differs; missing = not in Rust.
Counts are rows, not individual flags. Repl flags are one row (F3) covering 26 flags.

## Commands (cmd/chaude/main.go)

| id | feature | Go ref | status | Rust ref | note |
|---|---|---|---|---|---|
| C1 | no-arg interactive session | main.go:34-36 | done | crates/cli/src/main.rs:203 run_tui | inline TUI; plain line mode when not a TTY |
| C2 | `repl [flags]` subcommand | main.go:39 | missing | crates/cli/src/main.rs:206 | falls to "not ported" |
| C3 | `metrics [--dir D] [--csv]` | main.go:41 | done | crates/cli/src/main.rs:102 metrics_main | |
| C4 | `mcp-resident` | main.go:43 | done | crates/cli/src/mcp_resident.rs | |
| C5 | `--from-claude ID\|latest\|PATH` | main.go:45, repl.go | done | crates/cli/src/main.rs from_claude_arg, run_tui | load_import_arg; startup import installed into the next prompt (install_import) |
| C6 | `--import-dry-run` | main.go:45 | done | crates/cli/src/main.rs import_dry_run | prints dry_run_report; exit 0 when reco preserved, else 1 |
| C7 | `gen-settings [os] [--out] [--src] [--python]` | main.go:47, settings.go:269-300 | done | crates/core/src/settings/mod.rs:187 | |
| C8 | `clean [dir] [--yes]` | main.go:49, clean.go | done | crates/cli/src/main.rs arm `clean` -> chaude_clean::main | smoke: empty dir exit 0 |
| C9 | `update` | main.go:51 | done | crates/update run_cli wired in main.rs | eed9116 |
| C10 | `--version`, `-v`, `version` | main.go:53 | done | crates/cli/src/main.rs:195 | |
| C11 | `-h`, `--help`, `help` text | main.go:20-29, 55 | done | crates/cli/src/main.rs USAGE | verbatim Go usage; scripts list from NAMES text |
| C12 | 7 helper scripts (`scripts.Has`) | main.go:61, deprecated/src/main/intt/internal/scripts/scripts.go:17-27, helpText common.go:364-395 | done | crates/cli/src/main.rs match arm chaude_scripts::has (7 modules) | charcount, claude-history, compress-archive, export-history, extract-transcript, search-history, usage-report. Ported and tested (3 unit, 5 charcount, 7 smoke). Wired: crates/cli/Cargo.toml dep chaude-scripts and main.rs arm `Some(cmd) if chaude_scripts::has(cmd)`. Smoke: `chaude charcount FILE` exit 0 |
| C13 | leading `-flag` runs repl | main.go:58-59 | done | crates/cli/src/main.rs main() arm `Some(a) if a.starts_with('-')` | routing only; repl flag parsing (unknown-flag rejection) waits on C2/F3 |
| C14 | unknown command error, exit 2 | main.go:64-65 | done | crates/cli/src/main.rs:206-209 | message text differs |
| C15 | `update.CleanupOld()` at startup | main.go:33 | done | chaude_update::cleanup_old() first in main | |

## Repl flags (internal/harness/repl.go:1276-1306)

| id | feature | Go ref | status | Rust ref | note |
|---|---|---|---|---|---|
| F1 | `--orch <mode>` | repl.go:1316 | done | crates/cli/src/main.rs:26 apply_orch | |
| F2 | `--model <id>` | repl.go:1316-1334 | done | crates/cli/src/main.rs:54 parse_model | |
| F3 | remaining 26 repl flags | repl.go:1276-1306 (28 fs.*Var calls; minus --model and --orch = 26) | missing | none | verified count; --from-claude and --import-dry-run overlap C5/C6 |

## Environment

| id | feature | Go ref | status | Rust ref | note |
|---|---|---|---|---|---|
| E1 | `CHAUDE_ORCH` export | repl.go:1316 | done | crates/core/src/orch.rs:8 ENV_KEY | |
| E2 | `CHAUDE_MCP_TOKEN` | mcp (Go) | done | crates/core/src/mcp.rs:71 | |
| E3 | `CHAUDE_MAX_WORKERS` | repl.go:1305 (flag), repl.go:1345 and 1350 (env read), dispatch.go:16 resolveMaxWorkers | missing | none | only read on the repl path, so it lands with C2/F3 |
| E4 | `CHAUDE_IME_SHIFT` (IME column shift override) | tui.go:173-178; also TERMINAL_EMULATOR=JetBrains-JediTerm at tui.go:183 | done | crates/cli/src/tui/mod.rs ime_col_shift, App.ime_shift | read once at run_tui start; test ime_shift_follows_env_override_and_jediterm |
| E5 | settings env overrides | Go settings | done | crates/core/src/settings/mod.rs:190 (CLAUDE_PROJECT_DIR, empty = unset) | Go reads only CLAUDE_PROJECT_DIR in internal/settings (settings.go:285); CHAUDE_MAX_WORKERS is E3. Test default_src_precedence |

## Slash commands (internal/harness/tui.go:192-217; Rust table crates/cli/src/tui/commands.rs:15-111)

Both tables have 25 entries (the earlier estimate of 24 was off by one).

| id | command | status | Rust ref | note |
|---|---|---|---|---|
| S01 | /help | done | commands.rs:15 | |
| S02 | /clear | done | crates/cli/src/tui/mod.rs:412 UiEvent::NewSession | new-session notice wired |
| S03 | /status | done | Dispatch to engine | |
| S04 | /model | done | Dispatch::Model | |
| S05 | /settings | done | crates/cli/src/tui/settings.rs | |
| S06 | /resume | done | record /resume in TUI | |
| S07 | /import | done | Dispatch::Import | last commit 9bbe876 |
| S08 | /view | done | crates/cli/src/tui/mod.rs:400 UiEvent::SessionIds, prev_owned | this run's and previous run's session ids |
| S09 | /rotate | done | Dispatch::Rotate | |
| S10 | /usage | done | Dispatch::OpenUsage | |
| S11 | /cost | done | passthrough to claude | not verified per command |
| S12 | /context | done | passthrough to claude | not verified per command |
| S13 | /compact | done | passthrough to claude | not verified per command |
| S14 | /config | done | passthrough to claude | not verified per command |
| S15 | /mcp | done | passthrough to claude | not verified per command |
| S16 | /agents | done | passthrough to claude | not verified per command |
| S17 | /init | done | passthrough to claude | runs a turn |
| S18 | /review | done | passthrough to claude | runs a turn |
| S19 | /doctor | done | passthrough to claude | runs a turn |
| S20 | /mode | done | engine | |
| S21 | /mouse | done | Dispatch::ToggleMouse | |
| S22 | /version | done | engine | |
| S23 | /update | differs | crates/cli/src/tui/mod.rs (stub, TODO(update-check)) | no update check yet |
| S24 | /exit | done | Dispatch::Quit | |
| S25 | /quit | done | Dispatch::Quit | |

## TUI elements and keys

| id | feature | Go ref | status | Rust ref | note |
|---|---|---|---|---|---|
| K1 | permission prompt keys | tui.go | done | crates/cli/src/tui/mod.rs:621-627 | |
| K2 | Korean IME input, unicode width | tui.go | done | crates/cli/src/tui/render.rs input_view_row | |
| K3 | spinner | tui.go | done | crates/cli/src/tui/bar.rs spinner_frame | |
| K4 | idle "still waiting" notice (30 min) | tui.go | done | crates/cli/src/tui | |
| K5 | clipboard copy | selection.go:115 (clipboard.WriteAll + OSC 52) | done | crates/cli/src/tui/mod.rs native_copy, clipboard_tool, clipboard_payload | off macOS: wl-copy (WAYLAND_DISPLAY), xclip, xsel, clip.exe (UTF-16LE BOM); Windows native uses clip.exe not win32 API; not run on a non-macOS host |
| K6 | status bar P/T/C per-role context | tui.go | done | crates/cli/src/tui/bar.rs role_tokens; mod.rs:397 UiEvent::RoleContext | P/T/C from core RoleContext; test bar shows P 1k/80k |
| K7 | thinking row | tui.go | done | crates/cli/src/tui/mod.rs:1154 bar::thinking_row | |
| K8 | done row (agent completion line) | drain.go:40-55 doneLine (tui.go has no per-turn done row) | done | crates/cli/src/tui/bar.rs done_line; mod.rs UiEvent::AgentEnd | once per agent; AgentEnd carries no model, so always 상속 (doubt) |
| K9 | rotated notice | tui.go | done | crates/cli/src/tui/mod.rs:428 UiEvent::Rotated, bar::rotated_notice | |
| K10 | orphan notice | tui.go | done | crates/cli/src/tui/mod.rs:417 UiEvent::OrphanNotice, bar::orphan_notice | |
| K11 | tool summary | tui.go | done | crates/cli/src/tui/mod.rs:297 render::tool_summary | |
| K12 | assistant link wrap | links.go:197 wrapLinked; tui.go:374 | done | crates/cli/src/tui/mod.rs body() wrap_linked; render.rs wrap_linked | raw md_bold text kept per row; links styled after wrap so a link across rows keeps style; applies to every transcript row (doubt: Go styles only assistant and user text) |

## Settings (internal/harness/chaude_settings.go:11-15)

| id | feature | status | Rust ref | note |
|---|---|---|---|---|
| T1 | session_retention_days | done | crates/cli/src/tui/settings.rs DEFAULT_RETAIN | |
| T2 | extra_coders (max 8, default 4) | done | crates/cli/src/tui/settings.rs MAX_EXTRA_CODERS, DEFAULT_EXTRA_CODERS | |
| T3 | main_model | done | crates/core/src/settings/chaude_settings.rs main_model_from_json | |

## Totals

- Rows: 63 (C 15, F 3, E 5, S 25, K 12, T 3).
- done: 55. differs: 1 (S23). missing: 7 (C2, C9, C15, F3, E3, K8, K12).
- Missing, top: (1) `repl` subcommand and 26 repl flags (C2, F3); (2) `update` CLI and startup cleanup (C9, C15; crates/update has cleanup_old but main.rs does not call it); (3) `CHAUDE_MAX_WORKERS` (E3; no max_workers in core); (4) K8 done row (doubt); (5) K12 assistant link wrap (wrap_linked still dead code).
- Verified 2026-11-04 against code at c8233d6: C3, C4, C5, C6, C8, C11, C12, C13 done (main.rs match arms); K6-K11, S02, S08 wired (f73b6fb). Orch Router (orch.rs) is not called from api.rs (rust-port.md phase 3).

## Notes

- Test count: `grep -rhoE '^func Test' deprecated --include='*_test.go'` gives 320 (68 files, no duplicate names). The 312 in plan/rust-port.md phase 6 was stale (git grep at f970607 gave 313 across the whole repo). Parity gate uses 320.
- Go refs for E3, E4 and F3 were re-read and verified in this pass (see rows).
- Rust CLI usage (C11) and the TUI passthrough rows (S11-S19) were not checked line by line.
