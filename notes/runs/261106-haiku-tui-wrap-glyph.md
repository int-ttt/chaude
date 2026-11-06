workstream: tui-wrap-glyph
status: done
cause: no "└"/"╰" in agent rows. Agent row lead is AGENT_PREFIX "  │ " (mod.rs:77) on first row only; wrap_row (mod.rs:2116) did not know that lead, so continuation rows got pad "" (no glyph). Bottom wrapped row was blank, not │.
before: wrapped agent row, e.g. width 8 "  │ aaaa" / "aaaaaa" (rows 2+ have no glyph)
after: "  │ aaaa" / "  │ aaaa" / "  │ aa" (every row │)
changed: crates/cli/src/tui/mod.rs: hang_split leads += (AGENT_PREFIX,4); wrap_row pad = AGENT_PREFIX when head is agent lead; new test agent_wrapped_rows_keep_bar
check: cargo test -p chaude-cli -> 272 passed, 0 failed, 1 ignored (lib); 4 passed (bin)
check: cargo clippy -p chaude-cli -> clean
check: cargo fmt --check -p chaude-cli -> exit 0
risk: linked agent-variant "  " lead ordering unchanged; AGENT_PREFIX placed before "  " in linked list. Not committed.
