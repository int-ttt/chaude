# review: 261029-plan-overview-cap.md
date: 261029
reviewer: opus-architect (improvements only; plan not executed)
verdict: formula shape OK, but plan misses existing code (handoff note path) and the fact that ctx >= T at every rotation, so most of the adaptive range is dead. 9 improvements.

## facts found (code, not in scout file)
- f1 internal/harness/repl.go:27 `opusSonnetThreshold = 270_000`; repl.go:689 `effectiveThreshold()` returns 270k for opus/sonnet, else `a.Threshold` (flag default 80_000, repl.go:860). T already exists in code.
- f2 repl.go:517-518 `ctx := ctxTokens(usage); r.lastCtx = ctx` -> ctx is tokens from the last result usage (= "result_ctx"). Same unit as T. repl.go:921 resets lastCtx to 0 on new session.
- f3 repl.go:757 rotation fires only when `r.lastCtx >= thr` (after an answer). So at handoff-write time ctx >= T always (can overshoot by one turn).
- f4 repl.go:29 `handoffPrompt` (code-generated handoff note, "Be exact and brief", no length number). handoff.go:47 `validateHandoff` already rejects notes < 200 runes (utf8.RuneCountInString), retried via attempts (repl.go:633). A code hook for an upper cap already has a natural place.
- f5 repl.go:559 snapshot = trunc(join(parts), a.SnapshotChars), default 24000 (repl.go:865). Handoff note is one part of that snapshot.
- f6 classifier.go:31 planHint tells Haiku to write notes/runs/<YYMMDD>-plan-<topic>.md; no length rule. This is the only path for the plan/overview file (hybrid rule 6).

## improvements
1. Define "overview" (DECISION for user). Two different artifacts exist: (A) handoff note written at rotation (f4, code), (B) Haiku plan/overview file per workstream (f6, hybrid rule 6). DECISIONS:41 sits under [rotate], which points to (A).
   plan edit: add to (a): "overview = (A) handoff note at rotation and/or (B) plan file in notes/runs/. 사용자 확인 필요 (DECISION). Default proposal: both, same formula." Replace open question 6 with this.

2. Dead adaptive range for (A) (fact f3). At rotation ctx >= T, so cap for (A) is always >= 1.5 x base: Haiku >= 4,500, Opus/Sonnet >= 9,000 (hits 10,000 at ctx ~= 315k). Scaling below T only matters for (B), written mid-session.
   plan edit: in (b) Shape replace the "above T ... only if rotation does not happen" line with: "(A) handoff: ctx >= T by construction (repl.go:757), so cap = 1.5 x base up to CEIL. (B) plan file: ctx in [0, T), full range applies." Add acceptance row: Haiku, ctx 80,000 -> 4,500.

3. Haiku never reaches the user's 10,000 max (DECISION). With base 3,000, cap = 10,000 needs ctx = 2.83 x T = 226k, which the 80k rotation prevents; effective Haiku max ~= 4,500-4,800. If the user meant "both models may reach 10,000", the formula must change, e.g. cap = base + (CEIL - base) x min(ctx/T, 1) (Haiku 3,000->10,000, Opus 6,000->10,000 over [0, T]).
   plan edit: add open question: "Haiku 최대치가 약 4,500자에 머무는 것이 의도인지, 아니면 두 모델 모두 T에서 10,000자에 도달해야 하는지. 사용자 확인 필요 (DECISION)." Keep the current formula as option 1, the linear-to-CEIL one as option 2, with a 3-row table each.

4. Direction (DECISION, recommend grow). Purpose of the overview is to replace history the reader will not see; more context -> more to carry -> longer note. Shrinking saves tokens only on the reader side, and the reader (new session) starts near ctx 0, so the saving is small (<= 10,000 chars ~= 3-5k tokens once). Shrinking also contradicts the user's "최대 10,000자" (a max only makes sense if it grows).
   plan edit: open question 2 keeps the user decision but states the recommendation and reason: "추천: 증가. 이유: 개요는 새 세션이 못 보는 이력을 대신함; 축소 시 절약은 1회 3-5k 토큰 이하."

5. ctx source (fact f2, removes open question 1). ctx = r.lastCtx in tokens, taken from the last result usage of the session being summarized, read before the handoff prompt is sent (the handoff prompt itself adds tokens). Do not use the new session's ctx (~0 after reset, repl.go:921). Unit question is moot: ctx/T is a ratio, both in tokens.
   plan edit: (b) "ctx = r.lastCtx (tokens, last result usage of the session that writes the overview, measured before the handoff prompt)". Fallback: "ctx <= 0 -> cap = base" (0 means reset/unknown, repl.go:921). Delete open question 1.

6. FLOOR 1,000 is unreachable (fact, plus DECISION on value). ctx >= 0 gives min 0.5 x base = 1,500 (Haiku), 3,000 (Opus/Sonnet); FLOOR only matters if base changes. Lower bound already enforced: validateHandoff < 200 runes = invalid.
   plan edit: (b) "FLOOR = 1,000: safety only, inactive with current bases (min reachable 1,500). Must stay > 200 (validateHandoff min)." Open question 3 becomes: "FLOOR 1,000 유지(안전장치) 또는 제거. 사용자 확인 필요 (DECISION); 추천: 유지."

7. Fill (c) location: code, not docs only (recommend; DECISION only for (B) enforcement strength).
   - add `func overviewCap(base, ctx, thr int) int` in internal/harness/handoff.go (pure, table-testable); base from model of the session that writes the note (3,000 haiku / 6,000 opus,sonnet); thr = r.effectiveThreshold().
   - (A): append "Max <N> characters." to handoffPrompt at send time (repl.go:29 becomes a format or suffix); add problem "too long: <n> > <cap>" in validateHandoff (rune count, same as the 200 check) so the existing retry path handles it. After final failed attempt: keep note, truncate to cap (DECISION: truncate vs accept as-is; 추천: truncate, logged in metrics).
   - (B): planHint (classifier.go:31) carries the computed cap number in text; no hard check (Haiku writes the file outside Go). Docs: one line in DECISIONS.md replacing the fixed caps (by main session, not this plan).
   plan edit: replace (c) placeholder with the list above; open question 6 -> "강제 수준: (A) 코드 검증+재시도, (B) 안내문만. 사용자 확인 필요 (DECISION)."

8. Units and rounding (decide in plan, not user). "chars" = Unicode code points (utf8.RuneCountInString), matching validateHandoff; Korean text otherwise triples in bytes. Rounding: round down to 100 (integer math `raw/100*100`, no half-up ambiguity). Row 1 becomes 2,200.
   plan edit: (b) add "char = rune"; replace "rounded to the nearest 100" with "floored to a multiple of 100"; fix table row 1 to 2,200; delete open question 4 rounding part (keep "per file" note: cap applies per note/file).

9. Fit inside the snapshot budget (fact f5). Snapshot is truncated at 24,000 chars after joining parts; a 10,000-char note plus reco and other parts can push the tail (Open questions / exact codes) out.
   plan edit: edge cases: "CEIL 10,000 must stay <= SnapshotChars - other parts; if SnapshotChars < CEIL + 4,000, clamp cap to SnapshotChars - 4,000. Acceptance: snapshot with a 10,000-rune note keeps the note's last line."

## acceptance additions (replace (d) table)
| # | model | base | ctx | T | cap |
|---|-------|------|-----|---|-----|
| 1 | haiku | 3,000 | 20,000 | 80,000 | 2,200 |
| 2 | haiku | 3,000 | 80,000 | 80,000 | 4,500 |
| 3 | opus | 6,000 | 270,000 | 270,000 | 9,000 |
| 4 | sonnet | 6,000 | 400,000 | 270,000 | 10,000 |
| 5 | haiku | 3,000 | 0 | 80,000 | 3,000 (fallback base) |
check: `go test ./internal/harness -run 'TestOverviewCap|TestValidateHandoff'` (table test for overviewCap; validateHandoff "too long" case).

## ordering note
- user decisions first: 1 (scope), 3 (formula option), 4 (direction), 6 (floor), 7 (enforcement/truncate). Then haiku-coder implements 7-9 (handoff.go, repl.go:29/validate call, classifier.go:31, test file). Do not touch opusSonnetThreshold, Threshold flag default, SnapshotChars default.
- open conflict to flag (not this plan's scope): DECISIONS row 40 says Opus/Sonnet mains do not switch by threshold; row 41 and code (repl.go:27) rotate them at 270k. If row 40 wins, (A) never fires for Opus/Sonnet and only (B) applies to them.
