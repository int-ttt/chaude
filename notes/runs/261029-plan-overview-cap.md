# Plan: adaptive overview length cap

date: 261029
status: plan only, document only, no code or rule edits
user (Korean, verbatim): "개요 길이 상한(현재 Haiku 3,000자, Opus·Sonnet 6,000자 고정)을 상황에 따라 유기적으로 조정되게 변경, 최대 10000자."
inputs: notes/DECISIONS.md (last rows), CLAUDE.md (line 27 thresholds), notes/runs/261029-plan-hybrid-design.md (rule 6, open item 3)
facts file: notes/FACTS.md has no entries, nothing cited from it

## (a) Current fixed caps and where they are recorded
- Haiku overview: 3,000 chars. Opus and Sonnet overview: 6,000 chars. Both fixed.
- recorded in: notes/DECISIONS.md:41 (last row, 2026-10-29 user decision, "no existing code limit").
- CLAUDE.md: no cap stated (checked by grep for 3000/6000/overview). Not recorded there.
- notes/runs/261029-plan-hybrid-design.md: cap marked "사용자 확인 필요" at rule 6 (line 47) and open item 3 (line 93). The hybrid plan does not carry the 3,000/6,000 values.
- Related thresholds used for scaling (CLAUDE.md:27, DECISIONS:41): rotation threshold T = Haiku main 80k; Opus and Sonnet main 270k. Auto-compaction window 150k (CLAUDE.md:27).

## (b) Proposed adaptive rule
Inputs per overview write:
- base = 3,000 (Haiku main) or 6,000 (Opus or Sonnet main).
- ctx = current session context size, same unit as T (see open question 1).
- T = rotation threshold of the current main model (Haiku 80,000; Opus and Sonnet 270,000).

Formula:
- cap = clamp( base × (0.5 + ctx / T), FLOOR, CEIL )
- CEIL = 10,000 chars (user maximum).
- FLOOR = 1,000 chars (chosen here, not from the user; guards against very small or negative values; see open question 3).
- cap is rounded to the nearest 100 chars.
- ctx unknown: use cap = base (no scaling). Stated here as a fallback, not a user decision.

Shape:
- at ctx = 0, cap = 0.5 × base.
- at ctx = T, cap = 1.5 × base.
- above T: the main session is rotated at T, so this range only applies if rotation does not happen; cap keeps growing until CEIL.
- Direction (cap grows with context) is a proposal; see open question 2.

## (c) Where the rule is applied
- placeholder: to be located. Pointer: notes/runs/261029-scout-overview-cap.md (exists; scout found no code or CLAUDE.md rule for the cap, only DECISIONS.md:41). Its finding: no Go code writes the overview; Haiku writes it via planHint (classifier.go:31), so a code-level cap needs a hook in that writer, which does not exist yet. Exact file and line for the rule are not decided here.
- candidate place once located: hybrid plan rule 6 (notes/runs/261029-plan-hybrid-design.md:47) replaces "Length cap = 사용자 확인 필요" with the formula in (b).
- not applied to: chat replies (no fixed cap today; CLAUDE.md:21 says length follows content), agent result summaries (DECISIONS:36).

## (d) Acceptance: formula with 3 example inputs
Formula: cap = clamp( base × (0.5 + ctx / T), 1,000, 10,000 ), rounded to 100.

| # | model | base | ctx | T | 0.5 + ctx/T | raw | cap (chars) |
|---|-------|------|-----|---|-------------|-----|-------------|
| 1 | Haiku | 3,000 | 20,000 | 80,000 | 0.75 | 2,250 | 2,300 |
| 2 | Opus | 6,000 | 270,000 | 270,000 | 1.5 | 9,000 | 9,000 |
| 3 | Sonnet | 6,000 | 400,000 | 270,000 | 1.98 | 11,889 | 10,000 (ceiling) |

Check for #1: 3,000 × 0.75 = 2,250, rounded to nearest 100 = 2,300 (2,250 rounds up under round-half-up; state the rounding rule if this matters: 사용자 확인 필요).
Acceptance: a test passes if each row gives the cap above, and an overview file over its cap fails the check (measure with the project charcount).

## (e) Open questions for the user
1. ctx unit: tokens (as shown in the status line and T values) or characters? Overview cap is in chars, so the ratio needs one unit. 사용자 확인 필요
2. Direction: plan has cap growing with ctx (more context, longer overview allowed). Or should cap shrink as ctx grows (save tokens)? 사용자 확인 필요
3. FLOOR = 1,000 chars is my choice. Confirm or give a value. 사용자 확인 필요
4. Rounding rule (nearest 100, half up) and whether the 10,000 ceiling is per overview file. 사용자 확인 필요
5. Does "Opus·Sonnet 6,000자" apply to both main and subagent overviews, or main only? 사용자 확인 필요
6. Location of the rule (CLAUDE.md, agent definition, or hook). Placeholder in (c) until decided. 사용자 확인 필요
