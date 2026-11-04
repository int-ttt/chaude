# TUI parity checklist: Go (bubbletea) → Rust

Source: `deprecated/src/main/intt/internal/harness/` (paths below are relative to this dir unless prefixed).
Format: `- [ ] <ID> <behavior>  (<go file:line>) · test: <Go test or none> · core: <engine feature>`
Status: all items unchecked (`[ ]`); this file is a checklist, not a verdict.
Read basis: tui.go (full), input.go, links.go, selection.go, history.go, agentview.go, sessview.go, settings_picker.go, usagebar.go (full or key ranges), repl.go (runTurn, event loop, permission, interrupt, main loop), tui_test names via grep. Items marked `test: none` have no Go test found by name.

Counts (filled at end): see the "Counts" section.

## Go dependencies (go.mod)

- github.com/charmbracelet/bubbletea v1.3.4: Program, Msg, KeyMsg, MouseMsg, WithAltScreen, WithMouseCellMotion, WithOutput (tui.go:1378-1385)
- github.com/charmbracelet/bubbles v0.20.0: spinner (`m.sp`), viewport (`m.vp`)
- github.com/charmbracelet/lipgloss v1.1.0: styles, Color, SetHasDarkBackground (tui.go:1384), reverse video, RoundedBorder
- github.com/charmbracelet/x/ansi v0.8.0: Truncate, Cut, StringWidth, Strip, SetClipboard (OSC 52) (selection.go:117, tui.go:1429)
- github.com/mattn/go-runewidth v0.0.16: CJK cell width (input.go:22-30, usagebar.go:35)
- github.com/atotto/clipboard v0.1.4: native clipboard (selection.go:115)
- github.com/muesli/termenv v0.16.0: terminal capability (indirect use via lipgloss)
- Indirect: rivo/uniseg, mattn/go-isatty, go-localereader, muesli/cancelreader, lucasb-eyer/go-colorful, x/term, x/cellbuf, colorprofile, go-osc52/v2, golang.org/x/sys, golang.org/x/text
- Stdlib: regexp (links, cavemanStatusRe tui.go:97, pctRe usagebar.go:37), os/signal + syscall (repl.go:1384-1388), sync/atomic (cursorWriter tui.go:226-236)

## Layout / regions

- [x] L01 Altscreen full-screen layout: scroll viewport on top, queue/panel rows, input box, status line (View tui.go:1222; runTUI tui.go:1378) · test: none · core: none (UI only) · audited (fixed): chrome order bar, queue, input box, fg panel, bg rows, status (mod.rs:1334); layout body_h from chrome (mod.rs:1925); tests queue_rows_sit_dimmed_above_the_input_box, foreground_rows_sit_below_the_box_and_list_running_agents_only, background_rows_follow_the_foreground_rows
- [x] L02 Viewport height is recomputed from terminal size and the panel/queue/input heights each frame (layout tui.go:419; WindowSizeMsg tui.go:447) · test: TestPanelScrollWindow (panel_test.go:367) · core: none · audited (same): layout() recomputes body_h from chrome each frame (mod.rs:1772-1797)
- [x] L03 Before the first WindowSizeMsg the view shows "starting…" (View tui.go:1223) · test: none · core: none · audited (deliberate): layout() returns starting… only for zero size (mod.rs:1928; test empty_size_shows_the_starting_frame). Startup reads terminal::size() before the first frame, so the pre-size frame is unreachable at runtime; not wired
- [x] L04 Message blocks (user/asst/tool/result/note) render in the viewport in order (render tui.go:348-383) · test: TestToolResultCavemanHidden (tui_caveman_test.go:28) · core: transcript events · audited (same): transcript Vec rendered in order (mod.rs:119, body 1219)
- [x] L05 Welcome banner with model and rotate threshold on start (welcome tui.go:295-314) · test: none · core: effectiveThreshold · audited (deliberate): welcome text matches (welcome_rows mod.rs:1657, tui.go:295-311). Box inner width clamp(4,64) vs Go min 20: Go overflows terminals under ~22 cols, Rust fits; test welcome_box_fits_narrow_terminals
- [x] L06 Bottom-of-screen background rows (sub-agent rows) sit below the input box (bgLines tui.go:1443; test) · test: TestBackgroundRowsBelowInputBox (panel_test.go:133) · core: agent tracker · audited (fixed): bg rows = agent_rows with background && !done, drawn after the fg panel above the status row (agent_lines mod.rs:1391, agent_row_line mod.rs:1902); test background_rows_follow_the_foreground_rows
- [x] L07 Foreground panel rows above the input (panelLines tui.go:1440) with "more" row when truncated · test: TestPanelMoreRowAndSmallHeight (panel_test.go:100) · core: agent tracker · audited (fixed): fg rows = !background && !done with bar::agent_status_text (agent_row_line mod.rs:1902); overflow row "… 1-n/N (마우스 휠로 스크롤)" (agent_lines mod.rs:1391); tests foreground_rows_sit_below_the_box_and_list_running_agents_only, panel_shows_a_scroll_row_when_agents_do_not_fit
- [x] L08 Small terminal heights do not overflow the frame (panel/bg/queue) · test: TestBackgroundSmallHeightNoOverflow (panel_test.go:153), TestQueueSmallHeightNoOverflow (panel_test.go:289) · core: none · audited (fixed): room bounded by MIN_VIEW_ROWS=3 (mod.rs:64), caps MAX_QUEUE_ROWS=5 (mod.rs:60) and MAX_PANEL_ROWS=4 (mod.rs:62); test panels_and_queue_never_overflow_small_heights
- [x] L09 Slash suggestion popup above input, capped at maxPop=8 rows (suggestions tui.go:974; maxPop tui.go:989) · test: none · core: none · audited (same): SLASH_MAX_ROWS=8 (overlay.rs:9), menu_max in layout (mod.rs:1778)
- [x] L10 Input box height grows with content up to maxInputRows=5 (tui.go:988; Height input.go:202) · test: none · core: none · audited (same): MAX_INPUT_ROWS=5 (mod.rs:58), input_height (mod.rs:1503); tui.go:996
- [x] L11 Queue lines dimmed above input while busy, "… +N 대기" when many (queueLines tui.go:921-939) · test: TestQueueLinesDimAboveInput (panel_test.go:254) · core: QueueTexts (core 8f1dba7, api.rs) · audited (fixed): queue_lines (mod.rs:1360) renders QueueTexts dimmed above input, overflow "  … +N 대기"; tests queue_rows_sit_dimmed_above_the_input_box, queue_overflow_row_counts_the_hidden_prompts. Gap: Go slash popup height not subtracted from queue room
- [x] L12 Status line at bottom (statusLine tui.go:1193) · test: none · core: none · audited (same): bar::rows + status row at bottom (mod.rs:1280-1298); text compared in B group
- [x] L13 Overlay screens replace the viewport: agent view, usage view, session view, settings, rewind, record picker (View tui.go:1222-1362) · test: TestUsageOpenNotInline (usageview_test.go:94) · core: none · audited (same): overlays replace body rows only (mod.rs:1782-1790), see A03
- [x] L14 Cursor row tracking so the real terminal cursor lands in the input box (syncCursor tui.go:999; cursorWriter tui.go:226-260) · test: TestCursorRowMatchesPanelAfterTick (panel_test.go:79), TestCursorRowIncludesBackgroundRows (panel_test.go:301) · core: none · audited (fixed): cursor() (mod.rs:1963) subtracts fg+bg+2 below-box rows; test cursor_stays_on_the_input_line_above_the_agent_rows

## Rendering (colors, wrapping, links, markdown, CJK width)

- [x] R01 Palette: orange `#D97757` for brand accent, grey `#888888` dim (tui.go:252-256) · test: none · core: none → done: render.rs:10-12 ORANGE_ON 217,119,87 / GREY_ON 136
- [x] R02 Dark background assumed; no OSC background query at startup (tui.go:1384) · test: none · core: none → done: no OSC 11 background query in crates/cli (grep)
- [x] R03 Markdown bold `**x**` rendered with bold SGR (mdBold tui.go:316) · test: TestMdBold (bold_test.go:5) · core: none → done: render.rs:372 md_bold; mod.rs:1918; test render.rs:1325
- [x] R04 Caveman status blocks detected by regex `^status:\s*(done|partial|fail|blocked)\b` and hidden/collapsed in tool results (cavemanStatusRe tui.go:97; isCavemanBlock tui.go:100) · test: TestIsCavemanBlock (tui_caveman_test.go:9), TestToolResultCavemanHidden (tui_caveman_test.go:28) · core: none → done: render.rs:425 is_caveman_block, render.rs:442 tool_result_text hides it; test render.rs:1338
- [x] R05 Tool result text summarized with "… (+N lines)" beyond the first line (tui.go:485) · test: none · core: tool_result content → done: render.rs:442-455 tool_result_text "… (+N lines)"; mod.rs:358
- [x] R06 Tool summary line from tool_use input (toolSummary tui.go:67; resultText tui.go:86) · test: none · core: tool_use events → done: render.rs:458 tool_summary; mod.rs ToolInput handler; test render.rs:1362
- [x] R07 Permission prompt note "Allow <tool>(<arg>)?  [y] yes  [n] no" (tui.go:499) · test: none · core: control_request can_use_tool → done: render.rs:490 perm_note; mod.rs:530
- [x] R08 Links: URL, Windows path, dot path, relative path, absolute path detected and colored `linkColorOn = \x1b[38;2;95;175;255m` (links.go:13-40, linkSpans links.go:59, markLinks links.go:84) · test: TestLinkSpansDetects (links_test.go:18), TestLinkSpansProseNegatives (links_test.go:40), TestLinkSpansOverlapAndBoundary (links_test.go:54) · core: none → done: render.rs:18-20 LINK_COLOR_ON; link_spans 802, mark_links 807
- [x] R09 Link text stays visible text-equal after linkify (TestLinkifyPreservesVisibleText links_test.go:65) · test: TestLinkifyPreservesVisibleText (links_test.go:65) · core: none → done: test render.rs:1224 linkify_keeps_visible_text_and_width
- [x] R10 Links are not styled inside fenced code blocks (wrapLinked links.go:197) · test: TestWrapLinkedSkipsFence (links_test.go:82) · core: none → done: wrap_linked render.rs:896; test render.rs:1235
- [x] R11 Link color survives soft wrap across rows using private SGR marks 9001/9002/9003 (restyle links.go:128) · test: TestWrapLinkedColorsWholeTokens (links_wrap_test.go:88) · core: none → done: restyle render.rs:846; wrap_linked render.rs:896; test render.rs:1247
- [x] R12 Wrap by display cell width, not rune count (wrap in input rows input.go:149; ansi.Truncate tui.go:1429) · test: TestInputWrapWide (input_test.go:30) · core: none → done: wrap_cells render.rs:340 used mod.rs:1612 (cell width)
- [x] R13 Mixed-width (CJK + ASCII) width measure (TestInputWidthMixed input_test.go:9) · test: TestInputWidthMixed (input_test.go:9) · core: none → done: render.rs:139-277 cell width; test render.rs:1401 mixed_width_measure
- [x] R14 Ellipsis truncation with "…" for long lines (open: trunc_ellipsis render.rs:317 has no production caller; only tests) (ansiTrunc tui.go:1429; trunc in repl) · test: none · core: none · fixed: rewind.rs:lines() rows wrapped by trunc_ellipsis (w-1 cells + "…"); render.rs allow removed · test: rewind draw_100x30_window_follows_selection · core: none
- [x] R15 Usage bar: 10 cells, width measured with runewidth (renderBar usagebar.go:18; barWidth usagebar.go:35) · test: TestRenderBar (usagebar_test.go:8) · core: none → done: render.rs:530 render_bar 10 cells; bar_width render.rs:543 is a dead_code helper only
- [x] R16 Context usage decoration against ctxBarMax=150000 tokens (decorateUsage usagebar.go:41) · test: TestDecorateUsage (usagebar_test.go:28) · core: usage/context numbers → done: render.rs:566 decorate_usage; mod.rs:468
- [x] R17 Usage percent text parsed by `(\d+(?:\.\d+)?)%` and bar inserted (pctRe usagebar.go:37) · test: TestDecorateUsage (usagebar_test.go:28) · core: claude /usage output → done: render.rs:548 pct_value; used by decorate_usage
- [x] R18 Dim/bold/reset SGR used for notes and status (dimSt tui.go:256; repl.go dim/rst constants) · test: none · core: none → done: render.rs:13-17 DIM_ON/RESET/BOLD; used in agentview.rs, rewind.rs
- [x] R19 Import transcript rebuilt as blocks on --from-claude (importBlocks tui.go:283; "(N earlier turns not shown)" at 100 cap tui.go:286) · test: TestImportShowsTurnsInTUI (resumeshift_test.go:42) · core: transcript import → done: render.rs:508 import_blocks, cap 100 at render.rs:514; mod.rs:971
- [x] R20 Selection highlight overlay in view by content coordinates (highlightView selection.go:92; highlightLine selection.go:81) · test: TestSelectionMultiLineAndReversedOrder (selection_test.go:37) · core: none → done: TextSel render.rs:959; highlight_line render.rs:1017; mod.rs:1330-1373

## Input editor (keys, multiline, paste, history, IME)

- [x] I01 Printable runes inserted at cursor (KeyRunes tui.go:943; Insert input.go:40) · test: TestInputNewline (input_test.go:52) · core: none → done: mod.rs:748 insert_char; edit.rs:44
- [x] I02 Control chars filtered out of insert; tab converted to space (Insert input.go:40-62) · test: none · core: none → done: edit.rs:263 filter_char; test edit.rs:455
- [x] I03 Space key inserts (tui.go:947) · test: none · core: none → done: mod.rs:748; test edit.rs:467
- [x] I04 Backspace / Ctrl+H (tui.go:949; Backspace input.go:64) · test: TestTUIAltBackspaceAndCtrlU (input_test.go:109) · core: none → done: mod.rs:784; test edit.rs:440
- [x] I05 Delete / Ctrl+D inside editor (tui.go:955; Delete input.go:71) · test: none · core: none → done: mod.rs:789 Delete; Ctrl+D quits in Go (tui.go:682) and Rust (mod.rs:715), so no editor delete on Ctrl+D in either
- [x] I06 Left/Right move one rune (tui.go:957-959; Left/Right input.go:117-127) · test: none · core: none → done: mod.rs:793-800; edit.rs:129-133
- [x] I07 Home / Ctrl+A and End / Ctrl+E per logical line (tui.go:961-963; Home/End input.go:129-139) · test: none · core: none → done: mod.rs:801-816; edit.rs:140-150; test edit.rs:475
- [x] I08 Ctrl+W deletes previous word, punctuation-aware (tui.go:965; DeleteWord input.go:81) · test: TestInputDeleteWordPunct (input_test.go:91) · core: none → done: mod.rs:728; edit.rs:90; test edit.rs:528
- [x] I09 Ctrl+K kills to end of line (tui.go:967; KillToEnd input.go:141) · test: none · core: none → done: mod.rs:736; edit.rs:124; test edit.rs:515
- [x] I10 Ctrl+U kills to start of line; terminal cmd+delete sends ^U (tui.go:969; KillToStart input.go:101) · test: TestInputKillToStart (input_test.go:72), TestTUIAltBackspaceAndCtrlU (input_test.go:109) · core: none → done: mod.rs:732; edit.rs:110; test edit.rs:499
- [x] I11 Alt+Enter and Ctrl+J insert newline (tui.go:747) · test: none · core: none → done: mod.rs:716 (ctrl+j), mod.rs:753 (alt+enter)
- [x] I12 Trailing backslash then Enter inserts newline instead of submit (tui.go:776-783) · test: none · core: none → done: edit.rs:67 backslash_newline; mod.rs:870; test edit.rs:579
- [x] I13 Enter submits trimmed non-empty line; empty line ignored (tui.go:776-800) · test: none · core: none → done: mod.rs:871-875 submit; empty ignored
- [x] I14 Up/Down move across soft-wrapped rows before history (tui.go:718-742; MoveRow input.go:185) · test: TestUpMovesCompleterWhenNotAtTop (history_file_test.go:109) · core: none → done: mod.rs:851-861 move_row first; edit.rs:151
- [x] I15 Up/Down history recall when at top/bottom row (tui.go:718-742; inputHistory Prev/Next history.go:89-111) · test: TestUpDownHistoryWithSlashCompleter (history_file_test.go:77) · core: none → done: mod.rs:851-861; edit.rs:225,241
- [x] I16 Slash completer owns Up/Down while open and not browsing a recalled prompt (tui.go:718-742) · test: TestUpDownHistoryWithSlashCompleter (history_file_test.go:77) · core: none → done: mod.rs:831-841 (fixed: browsing guard, top falls to history, wrap); edit.rs browsing()
- [x] I17 Tab completes the selected slash suggestion and appends a space (tui.go:712-717) · test: none · core: none → done: mod.rs:842-849; commands.rs:313 complete appends space
- [x] I18 Shift+Tab cycles permission mode (tui.go:744; setMode tui.go:1182) · test: none · core: permission modes (default/acceptEdits/plan/auto) → done: mod.rs:863 BackTab -> CycleMode; mode set in core (not verified here)
- [x] I19 Multiline paste: bracketed paste delivers runes; newlines inserted, not submitted (KeyRunes path tui.go:943 + Insert) · test: none (verify) · core: none → done: mod.rs paste path inserts via insert_str; edit.rs:52; test edit.rs:490
- [x] I20 History: max 500 entries, consecutive duplicates dropped (maxHistory history.go:18; Add history.go:78) · test: TestHistoryDedupeConsecutive (history_file_test.go:127) · core: none → done: edit.rs:14 MAX_HISTORY=500; edit.rs:296 push_history; test edit.rs:611
- [x] I21 History persisted to `<cwd>/.chaude/history.jsonl` (historyFileFor history.go:22) · test: TestHistoryFileFor (history_file_test.go:48), TestHistoryFileRoundTrip (history_file_test.go:12) · core: none → done: edit.rs:286 history_file_for; test edit.rs:660
- [x] I22 History load rules (skip bad lines, cap) (loadInputHistory history.go:35) · test: TestHistoryFileLoadRules (history_file_test.go:26) · core: none → done: edit.rs:307 read_history; test edit.rs:684
- [x] I23 History save is tmp file + rename (saveInputHistory history.go:57) · test: none · core: none → done: edit.rs:321-340 write_history tmp+rename
- [x] I24 A new model loads previously recorded history (recordHistory tui.go:273; loadHistory tui.go:266) · test: TestRecordHistoryPersistsAndNewModelLoads (history_file_test.go:54) · core: none → done: edit.rs:215 load_history; test edit.rs:638
- [x] I25 Esc+Esc on empty input opens rewind picker within 600ms (tui.go:696-711) · test: none · core: transcript turns (rewind unit = user turn) → done: mod.rs:760-783 double Esc 600ms
- [x] I26 Rewind picker: Open/Move/Cancel/Confirm (rewindState history.go:124-152) · test: none · core: transcript turns · audited: rewind.rs:36-98 matches history.go:124-152 (Open filter, wrap Move, Cancel, Confirm cut); wiring mod.rs:638-654, open mod.rs:774
- [ ] I27 Rewind excludes slash commands from the unit list (history.go:124) · test: none · core: transcript turns
- [ ] I28 Rewind with no turns shows note "nothing to rewind to" (tui.go:700-703) · test: none · core: none
- [ ] I29 IME preedit: real terminal cursor placed in the input row via cursor up/col escape each frame (cursorWriter tui.go:232-260; imeColShift tui.go:177) · test: TestJediTermIMEColumnShift (resumeshift_test.go:28) · core: none
- [ ] I30 Wide-char cursor column computed with runewidth (cells input.go:24) · test: TestInputWrapWide (input_test.go:30) · core: none
- [ ] I31 Typing while scrolled keeps the scroll position (TestTypingKeepsScroll agentview_test.go:103) · test: TestTypingKeepsScroll (agentview_test.go:103) · core: none
- [ ] I32 Placeholder text when empty (placeholder tui.go:879; View input.go:211) · test: none · core: none
- [ ] I33 Input cursor drawn as reverse-video cell (View input.go:211) · test: none · core: none
- [ ] I34 Ctrl+C with input and not busy quits (tui.go:685-690; quit tui.go:1188) · test: none · core: none
- [ ] I35 Ctrl+D quits (tui.go:691) · test: none · core: none

## Scrolling

- [x] S01 Follow tail: viewport sticks to bottom unless user is held (refresh tui.go:394; held field tui.go:~60) · test: TestScrollBottomOnInputAndUpdate (agentview_test.go:130) · core: none  · audited (same): start_row() takes the tail while follow is set (mod.rs:1467-1471 follow_tail/start_row; Go refresh tui.go:394-411)
- [x] S02 Holding: wheel or PgUp/PgDn/Ctrl+B/Ctrl+F or Shift+Up/Down sets held when not at bottom (tui.go:751-763) · test: TestWheelScrollHoldsPosition (agentview_test.go:73) · core: none  · audited (same): wheel and PgUp/PgDn/Ctrl+B/Ctrl+F/Shift+Up/Down go through scroll()/scroll_to() which set follow only at the bottom (mod.rs:2312-2313, 738-745, 815-829, 1450-1468)
- [x] S03 New output while held sets unseen indicator, does not move the view (refresh tui.go:394; notes/runs/scroll-bottom.md) · test: TestScrollBottomOnInputAndUpdate (agentview_test.go:130) · core: none  · audited (same): content while !follow sets unseen, view unchanged; top bar shows NEW_MSG (mod.rs:547, 1318; test new_output_while_held_keeps_position_and_shows_the_indicator mod.rs:2707)
- [x] S04 Input typed or Update event at bottom re-follows tail per scroll-bottom rule (notes/runs/scroll-bottom.md; Update tui.go:444) · test: TestScrollBottomOnInputAndUpdate (agentview_test.go:130) · core: none  · audited (same): follow_tail() on submit/append paths (mod.rs:893, 973, 1032, 1074); a held view is only released by reaching max row (scroll_to mod.rs:1458-1466)
- [x] S05 Home/End jump top/bottom when input empty; otherwise edit cursor (tui.go:764-774) · test: none · core: none  · audited (same): Home/End jump only when editor is empty, else edit cursor (mod.rs:799-814)
- [x] S06 Mouse wheel scrolls the viewport (MouseMsg tui.go:526-535) · test: TestWheelScrollHoldsPosition (agentview_test.go:73) · core: none  · audited (same): ScrollUp/Down ±WHEEL_LINES=3 (mod.rs:2312-2313, const mod.rs:61; Go tui.go:526-535 viewport wheel)
- [x] S07 Mouse off (/mouse) passes selection to terminal; on = wheel scroll (/mouse slash tui.go:1063; selectMouse selection.go:124) · test: none · core: none  · audited (same): ToggleMouse flips app.mouse and EnableMouseCapture/DisableMouseCapture follow it (mod.rs:912-914, 2335-2341; mouse_note commands.rs:231; Go tui.go:1071-1078)
- [x] S08 Mouse drag selection by content coordinates, reversed order handled (selectMouse selection.go:124; ordered selection.go:28) · test: TestSelectionMultiLineAndReversedOrder (selection_test.go:37) · core: none  · audited (same): select_press/drag/release in body coords, TextSel ordered (mod.rs:1357-1390; test mod.rs:3307 reversed order)
- [x] S09 Selection anchor stays on content after append (selection_test.go:13) · test: TestSelectionAnchorStaysOnContentAfterAppend (selection_test.go:13) · core: none  · audited (same): body_at() maps to content row start+y, so anchor survives appends (mod.rs:1400-1409)
- [x] S10 Mouse release copies selection to clipboard; status "복사됨 (N자)" (copySelected selection.go:150; copyToClipboard selection.go:115) · test: TestReleaseCopiesSelection (selection_test.go:50) · core: none  · audited (same): select_release returns text, copy_done sets 복사됨 (N자) with chars().count() like RuneCount (mod.rs:1381-1421; Go selection.go:150-167)
- [x] S11 Clipboard: native first, then OSC 52 for SSH (copyToClipboard selection.go:115-120) · test: none · core: none  · audited (fixed): copy_text now runs native copy first, then writes OSC 52 even on native error, returns native error only (mod.rs:1676-1681; Go selection.go:115-119). test: none (native copy not mockable; ordering by code)
- [x] S12 Agent view scroll: up/down/pgup/pgdn/home/end (avKey agentview.go:135) · test: TestAgentViewClickReadOnlyClose (agentview_test.go:21) · core: sub-agent log events  · fixed: wheel ±3 via AgentView::wheel (agentview.rs; mod.rs overlay_wheel); PgUp/PgDn page = live body_h (mod.rs av key path, Go vp.Height-2 avKey agentview.go:135-150). test: wheel_scrolls_the_log_by_three_like_go; no PgUp test yet
- [x] S13 Usage view scroll bounds, wheel and refresh keep scroll (usageview_test.go:24, :66) · test: TestUsageViewScrollBounds (usageview_test.go:24), TestUsageViewWheelAndRefreshKeepScroll (usageview_test.go:66) · core: none  · audited (same): usage scroll_by/scroll_to clamp (usage.rs:89-116), wheel via scroll() usage branch (mod.rs:1447-1452); tests usage.rs wheel_and_refresh_keep_scroll
- [x] S14 Session view list wheel moves cursor (wheelSel sessview.go:116) · test: TestViewWheelMovesSessionCursor (sessview_test.go:120) · core: none  · audited (same): overlay_wheel -> SessView::wheel list cursor, wraps (mod.rs:1431-1445; sessview.rs:292-296, test wheel_moves_session_cursor_and_wraps)
- [x] S15 Record picker wheel and Enter (resume_picker_test.go:33) · test: TestRecordPickerKeysWheelEnter (resume_picker_test.go:33) · core: record store  · audited (same): records.rs:325 wheel_sel over projects; Enter/keys in records.rs:330 (test picker_keys_wheel_enter_follow_go)
- [x] S16 Panel wheel scroll over agent panel (panelWheel agentview.go:355) · test: TestPanelScrollWindow (panel_test.go:367) · core: none  · fixed: panel_wheel (mod.rs) moves pan_off one row per notch over the panels, clamped to running count (Go agentview.go:368); overlay_wheel takes the row. test: wheel_over_the_panels_scrolls_the_agent_window, wheel_above_the_panels_is_not_taken_by_them
- [x] S17 "… a-b/n (마우스 휠로 스크롤)" range indicator in agent list (tui.go:1490) · test: none · core: none  · fixed: agent_lines (mod.rs) draws bar::scroll_range_text, #[allow(dead_code)] removed (bar.rs). test: panel_shows_a_scroll_row_when_agents_do_not_fit, wheel_over_the_panels_scrolls_the_agent_window (2-4/5)

## Status bar, spinner, notices

- [ ] B01 Status line: state, model, "rot N", "agents N running (bg N)" (statusLine tui.go:1193-1210) · test: none · core: rotation count, agent tracker
- [x] B02 Main status text while busy with elapsed "· Mm SSs" (mainStatusText tui.go:888-898) · test: none · core: turn start time  · fixed: ToolStart sets last_tool (40 runes, mod.rs), Busy(true) clears it, busy_row passes it to bar::elapsed_suffix (Go tui.go:900-906). test: busy_row_shows_the_last_tool_and_a_new_turn_clears_it; slash menu room (popH): slash_menu_takes_room_from_the_queue
- [x] B03 Spinner while busy (sp Tick Init tui.go:280) · test: none · core: none  · audited (same): bar.rs:192 spinner_frame; tick at mod.rs:593
- [x] B04 Status text prefixed by agent name; format (TestAgentStatusTextPrefixedName panel_test.go:314; TestAgentStatusFormat panel_test.go:340; agentStatusText tui.go:1527) · test: TestAgentStatusFormat (panel_test.go:340) · core: agent events  · audited (same): bar.rs:269-292 agent_status_text + tests
- [x] B05 Agent state dot: state mapping (agentState tui.go:1505; agentDot tui.go:1520) · test: TestStalledOutputFileKeepsAgentRunningUntilDoneSignal (panel_test.go:169) · core: agent done signal  · audited (same): bar.rs:225 agent_state; dot wiring in agent rows
- [x] B06 Model prefix hidden when already in name (hasModelPrefix tui.go:1546) · test: none · core: none  · audited (same): bar.rs:269-292 has_model_prefix + tests
- [ ] B07 Thinking indicator only for foreground session (TestThinkingOnlyForForeground panel_test.go:120; TestSessionPanelAndThinking panel_test.go:17) · test: TestThinkingOnlyForForeground (panel_test.go:120) · core: thinking deltas
- [ ] B08 Idle tick refreshes panel when background agent finishes (TestIdleTickRefreshesWhenBackgroundFinishes panel_test.go:55) · test: TestIdleTickRefreshesWhenBackgroundFinishes (panel_test.go:55) · core: agent done
- [ ] B09 Background finish notification finishes oldest background row (TestUserEventNotificationFinishesBackground panel_test.go:384) · test: TestUserEventNotificationFinishesBackground (panel_test.go:384) · core: task-notification user events
- [x] B10 Done line for a finished agent (doneLine agentview/tui note; markDoneLocked drain.go/agentview.go:59) · test: none · core: agent done  · audited (same): bar.rs:305 done line; mod.rs:1229-1232
- [x] B11 Stage notices "· main 처리중", "· agent <type> 호출", "· agent <type> 완료", "· main 완료" (repl.go:~790-800; stage resident.go:255) · test: none · core: resident child events  · audited (same): mod.rs:485-488 Stage -> note; stage_text mod.rs:1753-1764 (· main 처리중/· agent X 호출/완료)
- [x] B12 Non-stage notices via addNotice / noteMsg (resident.go:277; drain.go:96) · test: none · core: drain/agent notices  · audited (same): mod.rs:489-494 error note block (Go addNotice/noteMsg)
- [x] B13 Orphan notice at start: "[orphan] claude pid N … still running" (reconcileStart repl.go:1400-1410 area) · test: none · core: registry reconcile  · audited (same): bar.rs:319 orphan_notice; mod.rs:467
- [x] B14 Budget stop notice "[budget] … task stopped" (repl.go:839) · test: none · core: budget + total_cost_usd  · audited (same): bar.rs:327 budget_notice; mod.rs:556
- [x] B15 Context-rotated notice "── context rotated #N (reason; …) ──" (repl.go:1019) · test: TestLiveRotationCarriesContext (acceptance_test.go:21) · core: rotate/handoff  · audited (same): bar.rs:331-337 rotated_notice; mod.rs:472
- [x] B16 "── new session ──" on /clear (clear repl.go:1409-1412) · test: none · core: session start  · audited (same): bar.rs:340-341 NEW_SESSION_NOTICE; mod.rs:454
- [x] B17 Quiet-wait notice "[no output for <d>; still waiting, Esc/Ctrl+C to interrupt]" once per quiet stretch with no open tool (repl.go:788; quiet closure repl.go:~775-790) · test: TestTurnQuietNoticeKeepsWaiting (turn_wait_test.go:63) · core: result/events stream  · audited (same): bar.rs:346 quiet_notice; mod.rs:590
- [ ] B18 Long-running Agent/Task tool call is waited for with no cap (repl.go:~800 comment; TestTurnWaitsForOpenAgentToolResult turn_wait_test.go:38) · test: TestTurnWaitsForOpenAgentToolResult (turn_wait_test.go:38) · core: tool_use/tool_result pairing
- [ ] B19 Stalled output file keeps agent running until done signal (panel_test.go:169) · test: TestStalledOutputFileKeepsAgentRunningUntilDoneSignal (panel_test.go:169) · core: output file / done signal
- [ ] B20 Usage popup and budget flows: /usage allowed while busy (usagebusy_test.go:10-51) · test: TestUsageAllowedWhileBusy (usagebusy_test.go:10), TestUsageMsgWhileBusyOpensPopup (usagebusy_test.go:51) · core: one-shot claude call
- [ ] B21 Queue: input submitted while busy is queued, not sent (TestQueueWhileBusyNotSent panel_test.go:218) · test: TestQueueWhileBusyNotSent (panel_test.go:218) · core: none
- [ ] B22 Queue sent on done, in order, merged with "\n\n" (mergePrompts tui.go:876; TestQueueSentOnDoneInOrder panel_test.go:235) · test: TestQueueSentOnDoneInOrder (panel_test.go:235) · core: turn done
- [ ] B23 Background-only agents do not queue input (TestBackgroundOnlyDoesNotQueue panel_test.go:280) · test: TestBackgroundOnlyDoesNotQueue (panel_test.go:280) · core: agent tracker
- [ ] B24 Esc during foreground agent clears foreground panel (TestEscInterruptClearsForegroundPanel panel_test.go:38) · test: TestEscInterruptClearsForegroundPanel (panel_test.go:38) · core: interrupt
- [x] B25 Copy note and transient status line (copyNote field tui.go:~70; copySelected selection.go:150) · test: none · core: none  · audited (same): mod.rs:1504 복사됨 (N자); test mod.rs:3613 asserts 5자; Go selection.go:161-166

## Slash commands

- [ ] C01 Slash list (`slashCmds`) with descriptions: help, clear, status, model, settings, resume, import, view, rotate, usage, cost, context, compact, config, mcp, agents, init, review, doctor, mode, mouse, version, update, exit, quit (tui.go:192-230) · test: none · core: see per-command rows
- [ ] C02 Suggestions filter by prefix as user types (suggestions tui.go:974) · test: none · core: none
- [ ] C03 /help shows command list and key help (runSlash tui.go:1057) · test: none · core: none
- [ ] C04 /exit and /quit exit (tui.go:1055) · test: none · core: none
- [ ] C05 /mouse toggles wheel/select mode (tui.go:1063) · test: none · core: none
- [ ] C06 /status shows model, ctx tokens, rotations, session (tui.go:1071-1079) · test: none · core: lastCtx, rotations, session id
- [ ] C07 /model shows or switches; switch rotates with new model (tui.go:1080; submit path runTUI repl.go:1385-1395) · test: none · core: rotate with model
- [ ] C08 /settings opens picker: retention {0,7,30,90,365}, extra coders 0..settingsMaxExtraCoders, main model opus/sonnet/fable; saved on esc, applies next session (settings_picker.go:17-117; tui.go:1090) · test: none · core: settings file write
- [ ] C09 /settings save message "설정 저장: <path> · ..." (settings_picker.go; closeSettings settings_picker.go:47) · test: none · core: settings file
- [ ] C10 /resume lists record topics; /resume [n|slug]; /resume claude [q] lists Claude Code sessions (tui.go:1092-1130) · test: TestRecordPickCandidates (resume_picker_test.go:11) · core: record store, Claude session list
- [ ] C11 Record picker keys and wheel (TestRecordPickerKeysWheelEnter resume_picker_test.go:33) · test: TestRecordPickerKeysWheelEnter (resume_picker_test.go:33) · core: record store
- [ ] C12 /view [id-prefix] opens session browser; ambiguous and no-match messages (openSessView sessview.go:73; tui.go:1131) · test: TestViewArgOpensSessionDirectly (sessview_test.go:83), TestViewListsOnlyOwnSessions (sessview_test.go:97) · core: session list (own sessions only)
- [ ] C13 Session browser: level 1 list, level 2 prompts; Enter/←/→/esc (svKey sessview.go:132) · test: TestViewSessionSelectionAndPromptNavigation (sessview_test.go:53) · core: session prompts
- [ ] C14 Session browser esc returns to normal screen (TestViewEscBacksOutToNormalScreen sessview_test.go:35) · test: TestViewEscBacksOutToNormalScreen (sessview_test.go:35) · core: none
- [ ] C15 Session list active marker "● " within sessActiveWindow=10m (sessLines sessview.go:169; sessActiveWindow sessview.go:13) · test: none · core: session last-active time
- [ ] C16 Session prompt pane "Session <id>  prompt n/N" and "… (+N lines)" (svPromptLines sessview.go:194) · test: none · core: session prompts
- [ ] C17 /import [id|latest] imports session (tui.go:1139; sendImport tui.go:1043) · test: TestImportShowsTurnsInTUI (resumeshift_test.go:42) · core: import from Claude session
- [x] C18 /mode shows or sets permission mode; shift+tab cycles (tui.go:1145) · test: none · core: permMode · audit: same: shift+tab wired mod.rs:895 -> CycleMode (tui.go:752); stale 'Not wired yet' comment commands.rs:163
- [ ] C19 /version shows chaude and Claude Code versions (tui.go:1156) · test: TestVersionNoteShowsChaudeVersion (busyslash_test.go:56) · core: claude --version
- [ ] C20 /update checks and installs newer chaude; runs off UI goroutine; result as note (tui.go:1158) · test: none · core: none (binary replace)
- [ ] C21 /usage opens full-screen usage view, one-shot claude call, never a turn (tui.go:1161; usageFetch tui.go:849) · test: TestUsageSlashAcceptedWhileBusy (usagebusy_test.go:28), TestUsageViewEscCloses (usageview_test.go:86) · core: one-shot claude usage
- [ ] C22 /clear: fresh session with handoff snapshot (clearWithHandoff repl.go:1400-1415 area) · test: TestRotateKeepsPendingSnapshot (resumeshift_test.go:65) · core: handoff snapshot
- [ ] C23 /rotate: hand off to fresh session (runTUI submit repl.go:1366-1385) · test: TestLiveRotationCarriesContext (acceptance_test.go:21) · core: rotate/handoff
- [ ] C24 /cost, /context, /compact, /config, /mcp, /agents, /init, /review, /doctor: passthrough to Claude child (passthroughCmds; passthrough) · test: none · core: passthrough to claude child
- [ ] C25 /init, /review, /doctor run as a conversation turn (tui.go:1163 area) · test: none · core: runTurn
- [ ] C26 Busy-allowed slash set: /usage, /version, /help, /status, /mode, /mouse, /view (busyOK tui.go:841; busySlashMsg tui.go:844 Korean) · test: TestBusySlashAllowedAndRefused (busyslash_test.go:12) · core: none
- [ ] C27 Other slash commands while busy are refused with busySlashMsg (tui.go:844; enter path tui.go:~800-815) · test: TestBusySlashAllowedAndRefused (busyslash_test.go:12) · core: none
- [ ] C28 Unknown slash command handling (runSlash default) · test: none · core: none
- [x] C29 Agent view open via ctrl+t; cycle with tab/shift+tab/left/right; close with esc/ctrl+c/q (openAgentKey agentview.go:113; avKey agentview.go:135) · test: TestAgentViewClickReadOnlyClose (agentview_test.go:21) · core: sub-agent log events · audit: same: avKey agentview.rs:229-250 matches agentview.go:135-158
- [ ] C30 Agent view click on panel row opens agent (panelClick agentview.go:160; click agentview.go:172) · test: TestAgentViewClickReadOnlyClose (agentview_test.go:21) · core: agent tracker
- [x] C31 Agent view read-only (log lines, cap maxAgentLog=400) (agentview.go:13; agentViewLines agentview.go:182) · test: TestAgentViewClickReadOnlyClose (agentview_test.go:21) · core: sub-agent log events · audit: same: MAX_AGENT_LOG 16 = maxAgentLog 400 (agentview.go:13); read-only keys match avKey
- [ ] C32 Usage popup open/close (TestUsagePopupOpenClose agentview_test.go:57) · test: TestUsagePopupOpenClose (agentview_test.go:57) · core: none
- [x] C33 Settings and session/agent/usage overlays close on esc and do not leak keys to input (key ordering tui.go:589-680) · test: none · core: none · audit: fixed: perm prompt now read first in on_key (mod.rs:609), before overlays (tui.go key() perm first)

## Engine events shown (tool_use, agents, errors, interrupts)

- [ ] E01 Assistant text streams into viewport: textMsg per delta, textEndMsg at end (repl.go:510-520, 529; runTurn put repl.go:~720-730) · test: TestLiveStreamingMultipleDeltas (acceptance_test.go:81) · core: stream-json assistant deltas
- [ ] E02 Main-agent tool_use shown once as toolMsg (name + summary), deduped by id (repl.go:~729-735 seenTools) · test: none · core: tool_use blocks
- [ ] E03 tool_result shown as toolResultMsg (repl.go:761; user event branch ~line 757-765) · test: TestToolResultCavemanHidden (tui_caveman_test.go:28) · core: tool_result blocks
- [ ] E04 Agent/Task tool calls counted as delegations and tracked with label (subagentLabel; repl.go:~715-725; tracker.onToolUse) · test: TestTurnWaitsForOpenAgentToolResult (turn_wait_test.go:38) · core: Agent/Task tool_use
- [ ] E05 Sub-agent events feed agent log (onSubEvent agentview.go:19; cap 400 agentview.go:13) · test: none · core: sub-agent event stream
- [ ] E06 Permission request: decided by mode (decideMode); shown and asked via ui.ask in TUI (repl.go:575-600; tui.go:50) · test: none · core: control_request can_use_tool
- [ ] E07 Permission "y"/"enter" allows, "n"/"esc" denies (tui.go:592-598) · test: none · core: RespondPermission
- [ ] E08 Permission idle denial note "[permission denied while idle] … (use /mode auto or acceptEdits)" (repl.go:605-608 area; plain path repl.go:~603) · test: none · core: idle child permission
- [ ] E09 AutoAllow flag bypasses prompt (repl.go:~585) · test: none · core: none
- [ ] E10 Rate-limit events update the rate state (setRate repl.go:~1095 path; control rate_limit_event) · test: none · core: rate_limit_event
- [ ] E11 Unparsed event recorded to transcript (_unparsed repl.go:~1098) · test: none · core: stream parser
- [ ] E12 Result event carries total_cost_usd, charged to budget (repl.go:~1034-1038 area; budget.charge) · test: none · core: result cost
- [ ] E13 Mid-turn budget overrun: interrupts child once, budget stop notice (repl.go:~680-690; budgetHit) · test: none · core: budget + assistant usage
- [x] E14 Error/timeout: child process error surfaces as note (repl.go error paths; r.p) · test: none · core: process errors · fixed: mod.rs UiEvent::Error is dim note block + blank row (Go note rendering) · test: error_event_is_a_dim_note_then_blank_row · core: none
- [x] E15 Esc or Ctrl+C while busy calls interrupt: finish foreground panel and send intr to runTurn (interrupt tui.go:581; repl.go:637 `<-r.intr`, :711 Interrupt) · test: TestEscInterruptClearsForegroundPanel (panel_test.go:38) · core: Session.Interrupt · fixed: mod.rs finish_foreground() on Action::Interrupt and Answer{interrupt}; late core AgentEnd dropped via finished_fg; not on Wake::Signal · test: interrupt_finishes_foreground_agents_only · core: none
- [ ] E16 After interrupt: "[interrupted]" then waits up to 20s for result (repl.go:~800-805) · test: none · core: Interrupt + result
- [ ] E17 Stale Ctrl+C dropped at start of runTurn (repl.go:~690-695 select default) · test: none · core: none
- [x] E18 Ctrl+C with no turn running quits (tui.go:600-606) · fixed: mod.rs:704 idle Ctrl+C quits even with text · test: ctrl_c_interrupts_when_busy_and_quits_when_idle_even_with_text (mod.rs:2970) · core: none
- [x] E19 OS SIGINT/SIGTERM forwarded to intr channel (repl.go:1384-1388) · fixed: mod.rs forward_signals + Wake::Signal arm in tui loop · test: sigterm_is_forwarded_as_wake_signal (mod.rs) · core: none · note: plain line mode does not forward (Go forwards before the TTY check)
- [ ] E20 Interrupt marks running agents interrupted on exit (markInterrupted repl.go:1207; interruptRunning agentview.go:314) · test: none · core: agent registry
- [ ] E21 Idle child-initiated turn: reply shown with idle timers and finish (idleFinish repl.go:~530-545) · test: none · core: idle turn events
- [ ] E22 Agents counter in status: "agents N running (bg M)" (statusLine tui.go:1203-1205) · test: TestAgentStatusFormat (panel_test.go:340) · core: agent tracker
- [x] E23 Agent done notice and background row finish (TestUserEventNotificationFinishesBackground panel_test.go:384) · test: TestUserEventNotificationFinishesBackground (panel_test.go:384) · core: task-notification · audited (same for string content): bg AgentEnd removes row + done notice (mod.rs agent_done) · test: background_agent_end_removes_row_and_notes_done · diff (core-owned, not edited): api.rs:1948-1955 handles only Content::Text, Go onUserEvent also reads text blocks
- [ ] E24 Restored agents from prior run shown and dismissed (sortedRestored agentview.go:326; dismissRestored agentview.go:346) · test: none · core: agents file restore · differs: core agent_store.rs:780-792 has sorted_restored/dismiss_restored, but no engine startup restore and no UiEvent carries restored agents; TUI agent_recs (mod.rs:1105) has no restored view. Core-owned, not changed.
- [ ] E25 Resident coders: "stage" notices for residents (resident.go:255-277) · test: none · core: resident children
- [ ] E26 Live rotation carries context and keeps pending snapshot (TestRotateKeepsPendingSnapshot resumeshift_test.go:65; TestLiveRotationCarriesContext acceptance_test.go:21) · test: TestLiveRotationCarriesContext (acceptance_test.go:21) · core: rotate/handoff/snapshot
- [ ] E27 Drain of old child before rotate (drain.go; TestDrain in drain_test.go, not opened) · test: drain_test.go (not read) · core: drain
- [ ] E28 Waiting for an open tool call; no cap on turn wait (TestTurnWaitsForOpenAgentToolResult turn_wait_test.go:38) · test: TestTurnWaitsForOpenAgentToolResult (turn_wait_test.go:38) · core: tool open set

## Startup and exit

- [ ] X01 Start child process, print error to stderr and exit 1 on failure (runTUI repl.go:1364-1368; loop repl.go:1270-1273) · test: none · core: session start
- [ ] X02 Interactive detection: stdin is a char device (repl.go:~1393) · test: none · core: none
- [x] X03 TUI only when interactive and HARNESS_PLAIN unset; else plain loop (repl.go:1386-1388) · fixed: mod.rs:1984 HARNESS_PLAIN check in run · test: multiproc_s_test.go (not read) · core: none
- [x] X04 Project settings ensured at start (settings.EnsureProjectSettings repl.go:~1374) · fixed: main.rs:138 ensure_project_settings before tui::run · test: none · core: settings
- [ ] X05 Start: agent store, client wiring, worker pool, reconcileStart, metrics (repl.go:~1376-1395) · test: none · core: registry, metrics
- [ ] X06 --from-claude import at startup (importFromClaude) shows transcript · test: TestImportShowsTurnsInTUI (resumeshift_test.go:42) · core: import
- [ ] X07 Startup shows welcome banner (welcome tui.go:295) · test: none · core: none
- [ ] X08 Program runs with altscreen, mouse cell motion, custom output writer (runTUI tui.go:1378) · test: none · core: none
- [ ] X09 On exit: save exit handoff, mark interrupted, close drains, close session, close children (deferred runTUI tui.go:1366-1377; loop repl.go:1271-1280) · test: TestRecordHistoryPersistsAndNewModelLoads (history_file_test.go:54) (partial) · core: exit handoff, registry
- [ ] X10 Exit save input history to historyFileFor (history.go:57) · test: TestHistoryFileRoundTrip (history_file_test.go:12) · core: none
- [ ] X11 /quit and /exit run quit (tui.go:1188) · test: none · core: none
- [ ] X12 Submit goroutine: per-line dispatch (rotate, idle-rotate, rotate-deferred, clear, passthrough, import, model switch, default handle + recordSave) then doneMsg (runTUI repl.go:1366-1393) · test: none · core: session commands
- [ ] X13 Program error printed as "harness: …", exit 1 (runTUI tui.go:1408-1412) · test: none · core: none

## Inline vs altscreen

- [ ] A01 TUI is altscreen only (tea.WithAltScreen tui.go:1378); no inline mode in the Go TUI · test: none · core: none
- [ ] A02 Plain path (HARNESS_PLAIN or non-TTY stdin) is line-oriented, not a TUI; output goes to stdout with r.p/dim (loop repl.go:1270+) · test: multiproc_s_test.go (not read) · core: plain loop
- [x] A03 Overlays (usage/agent/session) render full-screen, not inline (TestUsageOpenNotInline usageview_test.go:94) · test: TestUsageOpenNotInline (usageview_test.go:94) · core: none · audited: overlays replace body rows only, chrome and input kept (mod.rs:1782-1790), same as tui.go:1349-1354
- [x] A04 Esc from full-screen overlay returns to normal screen (TestViewEscBacksOutToNormalScreen sessview_test.go:35) · test: TestViewEscBacksOutToNormalScreen (sessview_test.go:35) · core: none · audited: usage.rs:108, sessview.rs:337, agentview mod.rs:658-665 all return to normal view

## Counts

Items per group (this file, `- [ ]` lines):
- L layout/regions: 14
- R rendering: 20
- I input editor: 35
- S scrolling: 17
- B status/spinner/notices: 25
- C slash commands: 33
- E engine events: 28
- X startup/exit: 13
- A altscreen/inline: 4
- Total: 189

Tests with no item mapping yet: TestDrain*, multiproc_s_test.go internals, drain_test.go, mode_test.go, bold_test.go (mapped), acceptance_test.go (mapped), input_test.go (mapped). Not opened: drain_test.go, mode_test.go, multiproc_s_test.go, resident tests.

## Core engine features needed (cross-reference)

- stream-json assistant/user/result events, tool_use/tool_result (E01-E03, R05-R06)
- control_request can_use_tool and RespondPermission (E06-E09, R07)
- Session.Interrupt, intr channel (E15-E17)
- budget: usage per assistant message, total_cost_usd (E12-E13)
- rate_limit_event (E10)
- rotate/handoff/snapshot, clear with handoff (B15, C22-C23, E26)
- transcript turns and rewind (I25-I27)
- record store and history (C10-C11, I21)
- session list and prompts for /view and /import (C12-C17)
- agent tracker: Agent/Task tool_use, sub-agent events, done signals, restored agents (B04-B09, E04-E05, E22-E24)
- resident children and stage notices (B11, E25)
- settings file read/write (C08-C09, X04)
- one-shot claude calls for /usage and /version (C19, C21, B20)
- passthrough to claude child for /cost /context /compact etc. (C24)
- registry reconcile, orphan notice (B13, X05)
- exit handoff and interrupt marking (X09, E20)

## Open gaps (not yet read)

- drain.go beyond line 96; drain_test.go; mode_test.go; multiproc_s_test.go; resident tests
- repl.go lines 540-575 and 840-1080 (passthrough, rotate notice body, budget), not read in detail
- exact line numbers for some items marked "~" are approximate (from grep hits and partial reads); verify before porting
