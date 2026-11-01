---
name: correction
description: Handle user corrections and pushback (messages starting with 아니, 근데, 아, 그게 아니라, 아니면, 그냥, or repeating an instruction with -니깐/-라고). Classify, route the fix to the cheapest agent, fix only the delta, prevent repeats.
---

# Handling corrections (아니 / 근데 / 아 …)

Never open with an apology or a recap. Never redo the whole output. Opus classifies and routes; subagents do all fixing (the main thread has no work tools).

| Type | Signals | Route |
|---|---|---|
| A. Scope/target misread | 아니 그거 말고, 각 ~마다, 전부, ~라니깐, ~라고 | Reread the user's ORIGINAL instruction. Write `이해: <target>에 <action>` (one line). Continue the SAME agent with the user's exact words + that line. |
| B. Question taken as an order | ~할 수 있나?, ~아니야?, ~어때? | Answer (≤5 lines), no action. Facts needed → `haiku-researcher`/`haiku-scout`. If something was already done, say so in one line and offer to revert. |
| C. Fact correction | 아 ~가 아니라 ~다, A가 아니라 B야 | `/fact <old> → <new>` (haiku-batch-worker): FACTS.md + every file. Report files fixed. |
| D. Change missing | 안 바뀐 것 같은데, ~했어?, 바꾼거 맞아? | `/check` (haiku-checker) → send FAIL items to the same haiku-drafter/haiku-coder → report per target. |
| E. Doubt/pushback (근데 …) | 근데 ~ 아니야?, ~같은데, 이상한데 | Opus judges. Need evidence → haiku-scout/haiku-checker (or `sonnet-analyst` for analysis results) first. Verdict first: `맞습니다 / 일부 맞습니다 / 아닙니다` + reason ≤3 lines. Don't flip just because of pushback. Change only if yes or the user insists. |
| F. Direction change | 아니면 ~, 그냥 ~해, A로 가자 | `haiku-batch-worker` appends one line to `notes/DECISIONS.md` (or include it in the next haiku-drafter brief). Drop the old plan; re-brief. |
| G. Preference/tone | 너무 어려워, 딱딱하게, 내가 쓴 것처럼 | Relay to the haiku-drafter: record the rule in DECISIONS.md, then revise. |

## Same item corrected twice or more
Stop delegating. Opus replies with a requirement checklist (≤5 bullets, user's own words) and asks for a one-word confirmation (`ㅇㅇ` / wrong item number). Then send one brief; if the corrected work needed judgment, Opus decides it first (Haiku main: `opus-architect`).

## Reply after a correction
`이해: …` (A/C/F only) → agent's changed parts only → files touched → (C/D) every place fixed.
