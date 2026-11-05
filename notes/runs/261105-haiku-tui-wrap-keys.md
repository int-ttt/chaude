# 261105 haiku tui-wrap-keys

status: done
commit: 27295b4 fix(tui): hang wrapped output rows under their lead and map Opt/Cmd+Delete
changed: crates/cli/src/tui/mod.rs (+141 -6 across mod.rs and render.rs), crates/cli/src/tui/render.rs (csi_len made pub)
not staged: crates/core/src/api.rs (parallel worker), .claude/*, notes/runs/261104-*, plan/caveman-structure.md, other untracked notes

## output pane hang
- hang_split: leads linked [● 2, > 2, "  ⎿  " 5, "  " 2]; non-linked [● 2, "  ⎿  " 5]; notes no hang
- wrap_row: text wraps at w - hang; continuation rows padded with hang spaces
- narrow pane (w < hang + 2): no hang, as before
- live unfinished assistant line now gets "● " lead (was raw)
- test fix: assistant_link_keeps_style_on_wrapped_rows expected tail "op"; new hang layout at width 20 gives "  mnop" (rows: "● see https://exampl", "  e.com/abcdefghijkl", "  mnop"). Expectation updated.

## keys
- Go truth: input.go DeleteWord (letters/digits/_ incl. Hangul), KillToStart, KillToEnd; tui.go:956-979 KeyBackspace+Alt -> DeleteWord, ^W DeleteWord, ^U KillToStart, ^K KillToEnd
- Opt+Delete (ESC DEL / Alt+BS) -> delete_word (Go word delete, not single char)
- Cmd+Delete: ^U already wired (kill_to_start). Added kitty CSI 127;9u (Backspace+SUPER) -> kill_to_start; only reached if kitty protocol enabled (code never pushes keyboard-enhancement flags)

## checks
- cargo fmt --check -> PASS (rc 0)
- cargo clippy -p chaude-cli --all-targets -- -D warnings -> PASS
- cargo test -p chaude-cli -> PASS (lib 261 passed / 0 failed / 1 ignored; bin 4 passed; doc 0)
- cargo build -p chaude-cli -> PASS; binary target/debug/chaude rebuilt

## doubts
- hang is content-based: a note starting with "● " hangs like tool row
- wrap is hard cell wrap (render::wrap_cells), not Go lipgloss word wrap
- live row lacks md_bold (Go applies mdBold to whole text)
- kitty CSI-u paths inert unless protocol enabled; in practice Cmd+Backspace arrives as ^U

## user verify
1. Run chaude in a narrow pane; long reply/tool/result line: continuation rows aligned under text after "● " / "  ⎿  ".
2. Opt+Delete in input box: deletes previous word.
3. Cmd+Delete in iTerm/Terminal/JetBrains: deletes to line start.
