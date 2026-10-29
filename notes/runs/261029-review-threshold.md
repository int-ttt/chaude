# review: threshold change (261029)
verdict: FIX (minor; code PASS)

## checks
- rotation threshold: PASS. repl.go:756 handle() uses effectiveThreshold(); opus/sonnet 270_000 (const repl.go:27), else a.Threshold; flag default 80_000 (repl.go:860).
- UI helper: PASS. tui.go:237, 878, 959 all use effectiveThreshold(); no remaining raw a.Threshold in UI.
- unrelated edits: none in code. CLAUDE.md:10 adds sonnet-analyst no-direct-file-access line (unrelated to threshold; has DECISIONS row 38 [routing], so documented).
- CLAUDE.md:27 vs DECISIONS: consistent with row 41 [rotate] (Haiku 80k, Opus/Sonnet 270k).

## issues
1. med | notes/DECISIONS.md:40 | [main-model] says Opus/Sonnet "do not switch by threshold"; conflicts with row 41 and code (270k rotation). OPEN, not resolved per brief; user must pick (fix row 40 wording or drop 270k).
2. low | README.md:24,35 | still says default threshold 270000 for haiku; now 80000 default, opus/sonnet 270000. Fix: update both lines.
3. low | repl.go:893 | metrics logs a.Threshold only (80000) even for opus/sonnet; use effectiveThreshold() if metrics should reflect actual.
4. low | repl.go:690 | model match is exact "opus"/"sonnet"; full model IDs or aliases (e.g. "opus[1m]") fall to 80k. Check /model accepts only those names (tui.go:1129 sets f[1] unvalidated).
5. info | repl.go:860 | compact-window 400k still > 270k; OK.
6. info | no tests added for effectiveThreshold.
