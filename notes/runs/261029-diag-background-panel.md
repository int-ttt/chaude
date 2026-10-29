# diag: background agents missing from panel below input (chaude 0.7.1)

## root cause (evidence)
- drain.go:62-63: agentRec.Background = input["run_in_background"].(bool) only.
- Real Agent tool_use now carries NO run_in_background (transcript 17d8f798..jsonl, toolu_01JGYYrL9nk4fAQXXfP9kaep input keys: description, subagent_type, prompt). Agents are async by default.
- Its tool_result = "Async agent launched successfully..." (immediate).
- drain.go:72-77 onToolResult: !Background => Done=true on first tool_result. So the agent is marked done at launch.
- ~/.chaude/agents.json: toolu_01JGYY... started 11:10:40.201, done 11:10:40.632 (0.4s); also 11:06:08.889 -> 11:06:09.513. No entry in agents.json has "background": true (all 14 omit it).
- Result: running() (drain.go:~132) filters !Done -> empty; rows() (tui.go:1163-1180) shows nothing, neither fg nor bg panel. Rendering itself (tui.go:988-995, 1088 `panel + bgPanel + statusLine`) is correct; not a clip/height problem (limit tui.go ~1182 only bites on tiny terminals).
- Completion path also affected: onNotification (drain.go:84) only reacts to <task-notification>, fine once the agent is kept alive; finishOldestBackground (drain.go:116, called repl.go:306) only considers Background==true, so non-flagged agents would never be settled by that fallback.

## fix location (not applied)
- drain.go:62-63 and :75: treat an Agent/Task call as background when run_in_background is true OR absent/not explicitly false (current CC default), or detect by tool_result text prefix "Async agent launched" in onToolResult (drain.go:72) -> set Background=true and do NOT mark Done. Prefer the tool_result-text check (robust to the flag semantics); also make finishOldestBackground/onNotification cover them.
- Tests: panel_test.go/drain_test.go all pass run_in_background:true explicitly, so they never exercised the real shape; add a case with no flag + "Async agent launched" result.

## unverified
- Not run at runtime (no live TUI). Inference that the 0.4s entries are the launch-result case is from timestamps + transcript; longer entries (1-2 min) may be real sync runs or notification-closed.
- Whether a sync Agent call (flag absent, tool_result = real output) exists in the same session: assumed, so the text-prefix check is needed instead of just flipping default.
- Did not read DECISIONS.md body (grep of background/panel only); LEDGER tail shows 0.6.11 moved panel below input, no related decision on flag semantics.
