# Transcript model into core (S4, 0.13.0 GUI parity)

Problem: TUI transcript = pre-rendered ANSI Vec<String> with row indexes (crates/cli/src/tui/mod.rs ~315-866, ~60 push sites); rewind/import/record-picker use row indexes. GUI Entry already in core (590d622).

## [haiku] steps
1. Widen core Entry (crates/core/src/ui_state.rs) to cover every TUI transcript kind (user, assistant text, tool call/result, notes, errors, status lines). Add Transcript { entries: Vec<Entry> } with push helpers. No TUI change yet. Tests per kind.
2. TUI pushes Entry into core Transcript at every push site; render lines from entries with a per-entry line cache (invalidate on width change). Output must be byte-identical; add snapshot tests on a few sessions before switching.
3. Rewind, import and record picker use entry indexes instead of row indexes; keep a row->entry map for scroll/selection. GUI renders from the same Transcript.

Each step: cargo test core/cli/gui PASS, clippy --workspace --all-targets + fmt clean, CHANGELOG Unreleased en+ko line, one local commit, no push. Fresh haiku-coder per step.
