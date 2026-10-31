# explore: classifier / category / mode (read-only)
date: 2026-10-29
scope: internal/harness/classifier.go, category.go, mode.go
changed: none (this note only)

## classifier.go
role: router. Sends each user prompt to a one-shot `claude -p` classifier call, parses 5-line answer, picks target session model.
classify: L181. Runs claude -p --model routerModel() --system-prompt routerPrompt() --tools "" (cmd.Dir=TempDir).
parseRouteFull: L118. Fields: topic same|new (else !ok), tier haiku|sonnet|opus, kind lookup|edit|other, worker (kept only if in cands and model haiku and kind edit), task (required, else !ok).
tier fallback: missing/invalid -> legacy size tag (small->haiku) else sonnet; SizeDefaulted=true.
pickModel: L88. cur ""/haiku -> haiku. cur opus -> opus, delegate = (topic new && tier haiku). other cur (sonnet) kept.
pickTarget: L99. ignores kind param.
hints prepended to task: delegateHint (opus main, new+haiku), planHint (model haiku, tier opus), sonnetHint (model haiku, tier sonnet).
routerModel: L48. env CHAUDE_ROUTER_MODEL; default "sonnet" (deprecated per comment); "haiku" allowed; invalid -> warn once, default.
route: L212. isAck or "/" prefix bypasses classifier. classify error -> returns original line.
prompts: classifyPromptSonnet L19 (deprecated, sha-pinned text), classifyPromptHaiku L30 (ordered tier rules: opus first, haiku only for one simple step, else sonnet).

## category.go
categoryRule: L10. system-prompt suffix: end every reply with `[cat: X]`; `[cat-switch: X]` only on clear new category.
catRe: L16. last marker in reply wins. parseCategory: L28.
categoryRotation: L39. rotate only if: marker is cat-switch AND not disabled AND not isAck(prompt) AND label differs (EqualFold).
isAck: L22. empty, ackWords set (ㅇㅇ, 응, 네, ok, yes, thanks, ...), or <=2 runes.
markerFilter: L54. Feed L60 / Flush L129 strip marker lines from stream; hold back trailing "[" partials (markerPrefix L116).

## mode.go
permMode: default, acceptEdits, plan, auto (numModes). Next L22 cycles. parseMode L24 (aliases "accept", "acceptedits").
decideMode: L42. auto -> allow all. acceptEdits -> allow editTools (Edit, Write, MultiEdit, NotebookEdit). plan -> allow readTools, deny others. default -> ask.

## doubts
- route() L224: field "router_prompt" is set to routerModel() (same as router_model). Likely copy error; prompt text not logged.
- pickTarget L99 ignores kind (dead parameter).
- routerModelDefault "sonnet" while Haiku prompt is tuned and flip is "pending user approval" (comments L17, L45).
- not found: tests for these functions were not opened (out of brief).
