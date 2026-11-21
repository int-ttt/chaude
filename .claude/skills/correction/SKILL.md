---
name: correction
description: Handle user corrections and pushback (messages starting with 아니 "no", 근데 "but", 아 "oh", 그게 아니라 "that's not it", 아니면 "or else", 그냥 "just", or repeating an instruction with -니깐/-라고 "I already said"; English: "no", "but", "oh", "that's not it", "or else", "just", "I already said"). Classify, route the fix to the cheapest agent, fix only the delta, prevent repeats. Row C applies a fact correction everywhere (A가 아니라 B야, "it's B, not A"); row D verifies a change reached every target (바꾼거 맞아?, 안 바뀐 것 같은데, "did you change it?", "seems unchanged").
---

# Handling corrections (아니 / 근데 / 아 … · no / but / oh …)

Think and write in English. Only haiku-output writes user-facing text, in the CHAUDE_LANG language (ko or en).

Never open with an apology or a recap. Never redo the whole output. Opus classifies and routes; subagents do all fixing (the main thread has no work tools).

| Type | Signals | Route |
|---|---|---|
| A. Scope/target misread | 아니 그거 말고 ("no, not that"; EN: "no, not that one", "not that"), 각 ~마다 ("each ~"; EN: "each ~"), 전부 ("all"; EN: "all of them"), ~라니깐 / ~라고 ("I said ~"; EN: "I said ~") | Reread the user's ORIGINAL instruction. Write `Understanding: <action> for <target>` (one line). Continue the SAME agent with the user's exact words + that line. |
| B. Question taken as an order | ~할 수 있나? ("can you ~?"; EN: "can you ~?", "could you ~?"), ~아니야? ("isn't it ~?"; EN: "isn't it ~?"), ~어때? ("how about ~?"; EN: "how about ~?") | Answer (≤5 lines), no action. Facts needed → `haiku-researcher`/`haiku-scout`. If something was already done, say so in one line and offer to revert. |
| C. Fact correction | 아 ~가 아니라 ~다 ("oh, it's ~, not ~"), A가 아니라 B야 ("it's B, not A"; EN: "oh, it's ~", "it's B not A") | Procedure C below (`haiku-batch-worker`): FACTS.md + every file. Report files fixed. |
| D. Change missing | 안 바뀐 것 같은데 ("seems unchanged"; EN: "seems unchanged", "not applied"), 바꾼거 맞아? ("did you change it?"; EN: "did you change it?"), ~어디 갔 / 빠졌 ("where did ~ go", "is ~ missing"; EN: "where did ~ go", "missing") | Procedure D below (`haiku-checker`) → send FAIL items to the same haiku-drafter/haiku-coder → report per target. |
| E. Doubt/pushback (근데 …) | 근데 ~ 아니야? ("but isn't it ~?"; EN: "but isn't it ~?", "but I thought ~"), ~같은데 ("seems ~"; EN: "seems ~"), 이상한데 ("feels off"; EN: "feels off", "looks wrong") | Opus judges. Need evidence → haiku-scout/haiku-checker first. Verdict first: `Correct / Partly correct / Not correct` + reason ≤3 lines. Don't flip just because of pushback. Change only if yes or the user insists. |
| F. Direction change | 아니면 ~ ("or else ~"; EN: "or else ~", "otherwise ~"), 그냥 ~해 ("just ~"; EN: "just ~", "just do ~"), A로 가자 ("let's go with A"; EN: "let's go with A") | `haiku-batch-worker` appends one line to `notes/DECISIONS.md` (or include it in the next haiku-drafter brief). Drop the old plan; re-brief. |
| G. Preference/tone | 너무 어려워 ("too hard"), 딱딱하게 ("too stiff"), 내가 쓴 것처럼 ("like I wrote it"; EN: "too hard", "too stiff", "sound like me") | Relay to the haiku-drafter: record the rule in DECISIONS.md, then revise. |

Questions alone (no correction marker) are not corrections: answer them as usual.

## Procedure C: fact correction (`haiku-batch-worker`)
Input: `<old> → <new> [context]`.
1. Update or add the line in `notes/FACTS.md` (format: `- [area] fact (source: user)`).
2. `grep -rn` the old value across documents (exclude .git, logs).
3. Replace only where the context matches this fact; list ambiguous occurrences unchanged.
Return ≤5 lines: files changed with counts, ambiguous occurrences (file:line).

## Procedure D: change check (`haiku-checker`)
Input: `<what changed> [target paths]`.
1. If targets are not named, find every file that should contain this content with `grep -rl` over the document folders (exclude .git, logs) and check each.
2. Also run char-limit checks for any section with a known limit in `notes/DECISIONS.md`.
3. Return ≤5 lines: FAIL items first (file:line), then "N others PASS". Full table → `notes/runs/`.

## Same item corrected twice or more
Stop delegating. Opus replies with a requirement checklist (≤5 bullets, user's own words) and asks for a one-word confirmation (`ㅇㅇ` = yes / wrong item number). Then send one brief; if the corrected work needed judgment, Opus decides it first (Haiku main: `opus-architect`).

## Reply after a correction
`Understanding: …` (A/C/F only) → agent's changed parts only → files touched → (C/D) every place fixed.
