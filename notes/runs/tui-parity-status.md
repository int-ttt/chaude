# TUI parity status: Go (bubbletea) vs Rust (crates/cli/src/tui, crates/core/src)

Source checklist: plan/tui-parity.md (189 items, IDs L, R, I, S, B, C, E, X, A). Extra Go-only features: N01-N15 (Go refs below).
Status: done | partial | missing | engine-missing (core lacks the feature; TUI side not judged).
Evidence: Rust file:line in crates/cli/src/tui unless prefixed core/. "helper only" = function exists with unit tests and no production call site in mod.rs.
Read-only audit; no code changed.

## Summary counts

| Group | Items | done | partial | missing | engine-missing |
|---|---|---|---|---|---|
| L | 14 | 3 | 6 | 5 | 0 |
| R | 20 | 5 | 14 | 1 | 0 |
| I | 35 | 19 | 9 | 7 | 0 |
| S | 17 | 6 | 1 | 10 | 0 |
| B | 25 | 1 | 16 | 7 | 1 |
| C | 33 | 7 | 12 | 14 | 0 |
| E | 28 | 7 | 11 | 7 | 3 |
| X | 13 | 3 | 9 | 1 | 0 |
| A | 4 | 1 | 1 | 2 | 0 |
| N | 15 | 0 | 3 | 12 | 0 |
| **spec total** | 189 | 52 | 79 | 54 | 4 |
| **Go-only extra (N)** | 15 | 0 | 3 | 12 | 0 |

Group names: L layout, R rendering, I input, S scrolling/mouse/selection, B status/notices, C slash, E engine events, X startup/exit, A altscreen, N Go-only extras.

## Top 10 missing user-visible features (ranked)

1. Usage full-screen view and popup (C21, C32, S13, A03, N15) - Go tui.go:1161, usageview; Rust has UiEvent::Usage but no view
2. Settings picker (C08, C09, N12) - Go settings_picker.go; no Rust overlay
3. Session browser /view (C12-C16, S14) - Go sessview.go:73; no Rust overlay
4. Record picker /resume (C10, C11, S15) - Go tui.go:1092; Dispatch::Resume forwarded only
5. Permission y/n answer keys (E07, E08, E06) - Go tui.go:592-598; answer_permission not called by TUI
6. Agent view Ctrl+T and panel click (C29-C31, S12, S16, A04) - Go agentview.go:135-172
7. Rewind picker Esc Esc (I25-I28) - Go tui.go:696-711, history.go:124
8. Mouse drag selection and clipboard copy (S08-S11, B25) - Go selection.go:92-150
9. Rate status and budget stop display (N01, E10, E13, B14) - Go ratestatus.go:33, repl.go:839; RateLimit ignored mod.rs:279, BudgetStop unhandled
10. Shift+Tab permission mode cycle and Ctrl+D quit (I18, I35, C18) - Go tui.go:744, 691

Next tier: welcome banner (L05, X07), stage notices (B11), restored agents (E24), queue lines (L11), IME/cursor (I29), engine-missing drain/resident/orphan (E25, E27, B13).

## Per-group tables

Columns: ID | status | Rust evidence | item text (from plan).

### L - Layout

| ID | status | Rust evidence |
|---|---|---|
| L01 | partial | mod.rs:730 frame; no panel/queue rows |
| L02 | partial | mod.rs:730 viewport height; no panel/queue heights |
| L03 | missing | no starting state |
| L04 | done | mod.rs:185-240 apply blocks in order |
| L05 | missing | no welcome banner |
| L06 | partial | mod.rs:196 bg_agents tracked, not drawn below input |
| L07 | partial | mod.rs:69 agents list; no panel rows |
| L08 | missing | no panel/bg/queue rows to clip |
| L09 | done | overlay.rs slash_popup; SLASH_MAX_ROWS=8 overlay.rs:11 |
| L10 | missing | input height not grown (no evidence) |
| L11 | partial | mod.rs:227 Queued sets bar.queued; no dimmed queue lines |
| L12 | done | bar.rs:116 rows |
| L13 | missing | overlay.rs only slash popup |
| L14 | partial | mod.rs:828 Show cursor; no IME row tracking |

### R - Rendering

| ID | status | Rust evidence |
|---|---|---|
| R01 | done | GREY_ON/FG_OFF in render.rs and mod.rs |
| R02 | done | no OSC background query in cli/src |
| R03 | partial | md_bold render.rs:149 unused by mod.rs |
| R04 | partial | is_caveman_block render.rs:202 helper only |
| R05 | done | mod.rs:185-192 ToolResult "(+N lines)" |
| R06 | partial | tool_summary render.rs:235 not called from mod.rs |
| R07 | partial | mod.rs:273 perm_note note only, no y/n prompt line |
| R08 | partial | linkify/link_spans render.rs:575-668 helper only |
| R09 | partial | linkify test only (render.rs:848) |
| R10 | partial | wrap_linked render.rs:674 helper only |
| R11 | partial | restyle render.rs:619 helper only |
| R12 | partial | wrap_cells render.rs:117 not wired in mod.rs |
| R13 | partial | cell_width render.rs:68 tests only |
| R14 | done | trunc_chars mod.rs:187 |
| R15 | partial | render_bar render.rs:305 helper only |
| R16 | partial | decorate_usage render.rs:340 helper only |
| R17 | partial | pct_value render.rs:322 helper only |
| R18 | done | GREY_ON/FG_OFF mod.rs:272 |
| R19 | partial | import_blocks render.rs:283 helper only |
| R20 | missing | no selection highlight |

### I - Input editor

| ID | status | Rust evidence |
|---|---|---|
| I01 | done | on_key Char insert mod.rs:354-360 area |
| I02 | done | KeyCode::Char if !ctrl mod.rs:340; Editor insert_str |
| I03 | done | KeyCode::Char space path mod.rs:340 |
| I04 | partial | Backspace mod.rs:358; Ctrl+H not mapped |
| I05 | partial | Delete mod.rs:363; Ctrl+D not mapped |
| I06 | done | Left/Right mod.rs:367-371 |
| I07 | done | Home/End mod.rs:375-387; Ctrl+A/E 317-321 |
| I08 | done | Ctrl+W mod.rs:325; punctuation rule unverified |
| I09 | done | Ctrl+K mod.rs:333 |
| I10 | done | Ctrl+U mod.rs:329 |
| I11 | partial | Ctrl+J mod.rs:313; Alt+Enter unverified |
| I12 | partial | backslash-enter unverified |
| I13 | done | Enter submit path mod.rs:442-490 |
| I14 | done | Up/Down move_row edit.rs:153 used by mod.rs |
| I15 | done | history_prev/next mod.rs:425-431 edit.rs:227/243 |
| I16 | done | suggest Up/Down mod.rs:407-411 |
| I17 | done | Tab completes suggestion mod.rs:416 |
| I18 | missing | no BackTab handling |
| I19 | done | Event::Paste mod.rs:953 |
| I20 | done | edit.rs history load/add |
| I21 | done | history_file_for edit.rs:288 |
| I22 | done | load_history edit.rs:217 |
| I23 | partial | write_history edit.rs:323; atomic rename unverified |
| I24 | done | load_history at startup mod.rs |
| I25 | missing | Esc idle is Continue mod.rs:357 |
| I26 | missing | no rewind picker |
| I27 | missing | no rewind picker |
| I28 | missing | no rewind picker |
| I29 | partial | cursor Show mod.rs:828; no preedit handling |
| I30 | partial | cell-width cursor col unverified |
| I31 | partial | follow flag mod.rs:778 |
| I32 | missing | placeholder not found in cli/src/tui |
| I33 | partial | cursor drawn as terminal cursor, not reverse cell |
| I34 | done | Ctrl+C mod.rs:303-306 |
| I35 | missing | Ctrl+D not mapped |

### S - Scrolling, mouse, selection

| ID | status | Rust evidence |
|---|---|---|
| S01 | done | follow mod.rs:778 |
| S02 | partial | PgUp/PgDn/Ctrl+B/F, Shift+Up/Down mod.rs:391-403; no held state |
| S03 | done | unseen mod.rs:283 |
| S04 | done | unseen cleared mod.rs:572-578 |
| S05 | done | Home/End when empty mod.rs:375-387 |
| S06 | done | mod.rs:954-956 |
| S07 | done | ToggleMouse mod.rs:478-480; mouse_note commands.rs:233 |
| S08 | missing | no drag selection |
| S09 | missing | no selection |
| S10 | missing | no clipboard copy |
| S11 | missing | no clipboard |
| S12 | missing | no agent view |
| S13 | missing | no usage view |
| S14 | missing | no session view |
| S15 | missing | no record picker |
| S16 | missing | no panel wheel |
| S17 | missing | no agent range indicator |

### B - Status, spinner, notices

| ID | status | Rust evidence |
|---|---|---|
| B01 | partial | bar.rs:116 rows; rot/agents partial |
| B02 | partial | bar.rs elapsed helper unverified |
| B03 | partial | spinner unverified |
| B04 | partial | agent_status_text bar.rs:218 helper only |
| B05 | partial | AgentState bar.rs:17 helper only |
| B06 | missing | no hasModelPrefix |
| B07 | missing | no foreground-only thinking |
| B08 | partial | AgentEnd mod.rs:204 handled; idle tick unverified |
| B09 | partial | AgentEnd mod.rs:204 only |
| B10 | partial | done_line bar.rs:241 helper only |
| B11 | missing | stage_agent_call has 0 call sites |
| B12 | partial | addNotice path not wired; notices helpers only |
| B13 | engine-missing | core orphan 0 matches; bar.rs:271 helper only |
| B14 | missing | BudgetStop not handled in mod.rs; budget_notice bar.rs:278 helper only |
| B15 | partial | rotated_notice bar.rs:283 helper only |
| B16 | missing | no new-session notice |
| B17 | partial | quiet_notice bar.rs:296 helper only |
| B18 | partial | open-tool wait; core interrupt_grace api.rs:164 only |
| B19 | partial | no stalled-output logic in cli |
| B20 | partial | /usage forwarded; Usage event mod.rs not shown |
| B21 | done | UiEvent::Queued mod.rs:227; core queue |
| B22 | partial | core queue; merge on done unverified in TUI |
| B23 | missing | no background-only queue rule |
| B24 | partial | Esc interrupt mod.rs:356 |
| B25 | missing | no copy note |

### C - Slash commands

| ID | status | Rust evidence |
|---|---|---|
| C01 | done | SLASH_CMDS commands.rs:14-111 (25 entries) |
| C02 | done | suggest prefix filter |
| C03 | done | help_text commands.rs |
| C04 | done | Quit dispatch mod.rs:477 |
| C05 | done | ToggleMouse mod.rs:478 |
| C06 | partial | status_line commands.rs; forwarded text |
| C07 | partial | Dispatch::Model forwarded to engine |
| C08 | missing | no settings picker |
| C09 | missing | no settings save note |
| C10 | partial | Dispatch::Resume forwarded; no record picker |
| C11 | missing | no record picker |
| C12 | missing | no session browser |
| C13 | missing | no session browser |
| C14 | missing | no session browser |
| C15 | missing | no session browser |
| C16 | missing | no session browser |
| C17 | partial | Dispatch::Import forwarded as text |
| C18 | partial | SetMode forwarded; Shift+Tab missing |
| C19 | partial | /version note path commands.rs |
| C20 | missing | Dispatch::Update forwarded; no update note |
| C21 | partial | UiEvent::Usage exists; no full-screen usage view |
| C22 | partial | Dispatch::Clear forwarded |
| C23 | partial | Dispatch::Rotate forwarded |
| C24 | partial | Dispatch::Passthrough forwarded |
| C25 | partial | passthrough forwarded as text |
| C26 | done | BUSY_OK commands.rs:119-126 |
| C27 | done | BUSY_SLASH_MSG commands.rs:130 |
| C28 | partial | unknown slash forwarded to engine |
| C29 | missing | no ctrl+t agent view |
| C30 | missing | no agent view click |
| C31 | missing | no agent log view |
| C32 | missing | no usage popup |
| C33 | missing | no overlays |

### E - Engine events

| ID | status | Rust evidence |
|---|---|---|
| E01 | done | TextDelta apply mod.rs |
| E02 | done | ToolUse apply mod.rs |
| E03 | done | ToolResult mod.rs:185 |
| E04 | partial | AgentStart mod.rs:197; delegation count TurnStats |
| E05 | partial | agent events; no sub-agent log |
| E06 | partial | PermissionRequest note mod.rs:273; no ask |
| E07 | missing | no y/n answer key; answer_permission unused |
| E08 | missing | no idle denial note |
| E09 | partial | auto_allow is core; TUI unaware |
| E10 | missing | RateLimit ignored mod.rs:279 |
| E11 | partial | core parse; TUI unverified |
| E12 | partial | core budget; TUI BudgetStatus unhandled |
| E13 | missing | BudgetStop unhandled in TUI |
| E14 | done | UiEvent::Error mod.rs:234 |
| E15 | done | Esc/Ctrl+C interrupt mod.rs:303-357 |
| E16 | done | Interrupted note mod.rs:~216; grace api.rs:164 |
| E17 | partial | core interrupt flag |
| E18 | done | Ctrl+C quits when not busy mod.rs:303-306 |
| E19 | missing | no SIGINT/SIGTERM handler |
| E20 | missing | no interrupt marking on exit |
| E21 | partial | idle turn renders as normal turn |
| E22 | partial | bar agents counter bar.rs |
| E23 | partial | AgentEnd only |
| E24 | missing | no restored agents |
| E25 | engine-missing | core resident 0 matches |
| E26 | partial | core rotate 2 matches; TUI rot counter |
| E27 | engine-missing | core drain 0 matches |
| E28 | engine-missing | core has no open-tool wait cap (interrupt_grace only) |

### X - Startup and exit

| ID | status | Rust evidence |
|---|---|---|
| X01 | done | main start path |
| X02 | partial | TTY check unverified in cli |
| X03 | partial | TUI vs plain gate unverified |
| X04 | partial | core settings 49 matches; not wired on start in TUI |
| X05 | partial | core startup partial |
| X06 | partial | import forwarded; no import transcript |
| X07 | missing | no welcome banner |
| X08 | done | EnableMouseCapture, altscreen mod.rs:864-881 |
| X09 | partial | exit path partial |
| X10 | partial | write_history on exit unverified |
| X11 | done | /quit /exit Quit dispatch |
| X12 | partial | dispatch forwarded per line; no per-line handlers |
| X13 | partial | error print path unverified |

### A - Altscreen

| ID | status | Rust evidence |
|---|---|---|
| A01 | done | altscreen only mod.rs:864-881 |
| A02 | partial | plain path unverified |
| A03 | missing | no overlays |
| A04 | missing | no overlays |

### N - Go-only extras (not in spec; Go file:line in evidence)

| ID | status | Rust evidence |
|---|---|---|
| N01 | missing | Go rate status line (ratestatus.go:33; tui.go:1195); Rust mod.rs:279 ignores RateLimit |
| N02 | missing | `chaude metrics` CLI (metricscmd.go:14); no metrics command in cli |
| N03 | missing | metrics JSONL log (metrics.go:19); core metrics 4 matches, no TUI use |
| N04 | missing | background usage poll (usagepoll.go:71); no poll |
| N05 | missing | deferred rotate while bg agents run (rotate_defer.go:31) |
| N06 | missing | deferred rotate timeout (rotate_defer.go:59) |
| N07 | missing | bg handoff lines (rotate_defer.go:79); repl.go:885 |
| N08 | missing | --orch m0..f A/B modes (orchws.go:78; repl.go:1305) |
| N09 | missing | import viewBuilder subagent lines (importview.go:34) |
| N10 | missing | plain-loop /help /status /update (localcmd.go:13-39) |
| N11 | partial | passthrough/unsupported lists (repl.go:1167-1171); Rust commands.rs:134,138 forwards |
| N12 | partial | settings load/save (chaude_settings.go:32-94); core settings only, no TUI picker |
| N13 | partial | model switch (switchchild.go:149; session_switch.go:10); Dispatch::Model forwarded |
| N14 | missing | idle rotate sweep (idlesweep.go:32) |
| N15 | missing | usage fetch timeout/copy note in usage view (tui.go:849); no usage view |

## Doubts

- Many "partial" marks are helper-only (function with tests, no mod.rs call). Verify with grep before acting.
- Items marked missing without a grep (L03, L05, L10, X07, I32) are unverified absences.
- Sub-items with "~" in plan line numbers were not re-checked.
- Go-only N items: line refs from the lead brief; N05-N08, N14 not opened in depth.

Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
