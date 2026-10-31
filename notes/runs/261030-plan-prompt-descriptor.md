# plan: prompt descriptor (v1)
date: 2026-10-30
status: plan (not implemented)
related: notes/DECISIONS.md [rotate] 2026-10-30 (cap 150k); notes/LESSONS.md 2026-10-29 (rotate must not overwrite pending import snapshot)

## goal
Cut the cost of rotation and child switch. Today: rotate() (repl.go:748) sends a handoffPrompt turn to the old child via makeSnapshotN() (repl.go:719) = one full-context turn (~150k cache read + output + latency, fails when draining). switchChild() (switchchild.go:144-221) uses --resume rec.SessionID = full transcript reloaded = cache write of whole context when cache is cold.
Descriptor = compact, harness-produced (no model call) record per completed user turn; digest of recent descriptors replaces the LLM handoff and the cold --resume.

## options
- A. LLM handoff (current): best fidelity; costs one full-context turn per rotation; can fail mid-drain.
- B. Deterministic descriptor from stream-json events (chosen): zero model cost, deterministic, testable; lossy on in-flight reasoning.
- C. B + short LLM "open items" turn: better fidelity but still pays the full-context turn. Kept as flag, off by default.
choice: B, plus rule: warm cache (parked age < cache TTL) -> keep --resume (cache read is cheap); cold -> fresh child + digest.

## cost effect (per event, 150k context)
- rotation: removes handoff turn = saves ~150k cache-read tokens + ~1-2k output + one round-trip.
- cold switch: --resume writes ~150k tokens to cache; digest writes ~2k -> ~75x fewer write tokens.
- warm switch: unchanged (--resume).

## data format: one JSONL line per completed user turn
path: notes/descriptors/<YYMMDD>-<sid8>.jsonl (sid8 = first 8 chars of child session id)
fields (v1):
{"v":1,"sid":"<session id>","turn":<int>,"ts":"<RFC3339>","model":"haiku|sonnet|opus","class":"<classifier tag>","size":"small|large","prompt":"<first 300 runes of user prompt>","prompt_sha":"<first 8 hex of sha256>","tools":{"<ToolName>":<count>},"files":["<path>", max 10, from Read/Edit/Write tool_use inputs, dedup],"agents":["<subagent_type>", dedup],"result":"<first 300 runes of final assistant text>","ok":<bool, false if error result or interrupted>,"ctx_tokens":<int>,"usage":{"in":<int>,"cw":<int>,"cr":<int>,"out":<int>}}
rules: truncate by runes (Korean-safe), never by bytes; omit empty fields; never store full prompt/answer.

## steps (ordered)
1. new internal/harness/descriptor.go
   - type Descriptor struct (fields above, json tags as above)
   - func newDescriptor(sid, model string, turn int) *Descriptor
   - func (d *Descriptor) observe(ev streamEvent)  // count tool_use names, collect files/agents, capture final text + usage from result event; use the existing stream event type in the package (grep "tool_use" / "result" in internal/harness to find it)
   - func appendDescriptor(dir string, d Descriptor) error  // O_APPEND|O_CREATE, one json line, mkdir -p
   - func loadDescriptors(path string, lastN int) ([]Descriptor, error)
   - func renderDigest(ds []Descriptor, tail string, maxRunes int) string  // default maxRunes 6000
   - func truncRunes(s string, n int) string
2. wire production: at the turn-end point where turnspend.go records per-turn usage (grep "turnspend" / result handling in repl.go), build+append the descriptor. Failure to write = log warning only, never block the turn.
3. digest text (Korean header, model-facing):
   "## 이전 세션 요약 (descriptor v1, 최근 N턴)\n" then one line per descriptor, oldest first:
   "- t<turn> [<class>/<model>] 요청: <prompt> | 도구: <Tool>x<n>,... | 파일: <files, comma> | 결과: <result> | <ok?"ok":"FAIL">"
   then "## 마지막 답변 일부\n<tail, last assistant text, max 1500 runes>"
   then "## 관련 파일\n<union of files over included descriptors>"
   Include newest descriptors first when trimming to maxRunes (drop oldest).
4. rotate() (repl.go:748): new flag -handoff descriptor|llm (default descriptor), registered next to -threshold (repl.go:1060). descriptor mode: snap = renderDigest(loadDescriptors(cur, 12), lastAnswerTail, 6000); do NOT call makeSnapshotN. llm mode: current behaviour unchanged.
   - if r.pendingSnapshot already holds an import snapshot: append digest after it with a blank line; never overwrite (LESSONS 2026-10-29).
5. switchChild() (switchchild.go:171-173): if parked/registry rec found AND time since its last activity < observed cache TTL (use the existing TTL value from the cache-TTL observe code; fallback 3599s) -> --resume as today. Else -> start fresh child, pendingSnapshot = digest of that session's descriptor file. classifier.go:290 behaviour (clear pendingSnapshot when resume set) stays.
6. metrics: add to the rotate/switch metrics line: "handoff":"descriptor|llm|resume", "handoff_runes":<int>.
7. tests in internal/harness/descriptor_test.go:
   - JSONL round trip (append 3, loadDescriptors lastN=2 returns last 2 in order)
   - truncRunes on Korean string does not split runes; result valid UTF-8
   - renderDigest output <= maxRunes and keeps newest turns when trimming
   - observe(): tool counts, file dedup cap 10, agents dedup, ok=false on error result
   - rotate in descriptor mode sends no handoff turn to old child; pending import snapshot preserved and digest appended
   - switchChild: warm -> Resume set; cold -> Resume empty and pendingSnapshot non-empty

## edge cases
- no descriptor file (new or pre-v1 session): fall back to llm handoff once; log "handoff":"llm-fallback".
- interrupted turn (no result event): write descriptor with ok=false, usage zeros.
- multiple children per model (livechild.go): descriptor file is per session id, so no interleaving.
- descriptor dir unwritable: warn, continue; rotation then uses llm fallback.
- secrets: prompt/result capped at 300 runes; files are paths only.

## acceptance
- go build ./... ; go test ./internal/harness/ PASS
- manual: force rotation with -threshold 2000; metrics line shows "handoff":"descriptor"; new child's first message starts with "## 이전 세션 요약"; no extra turn on old child in the transcript.
- digest size <= 6000 runes on a 30-turn session.

## must NOT change
- cap/threshold values (150k), compact-window 400k, .claude/settings.json, idle rotate 3599s, classifier routing, /resume import path (resume.go), CLAUDE.md.

## open question (user)
- 150k cap applies to Opus/Sonnet too (DECISIONS 2026-10-29 said Opus/Sonnet do not rotate by threshold). Confirm or restore Opus/Sonnet to no-rotation.

## [haiku] steps
- haiku-coder: steps 1-3 + tests 1-4 (descriptor.go, descriptor_test.go, turn-end wiring)
- haiku-coder: steps 4-6 + tests 5-6 (repl.go rotate/flag, switchchild.go, metrics)
- haiku-checker: go test ./internal/harness/ -run Descriptor|Rotate|Switch
