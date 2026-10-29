# Usage aggregate: transcript records before 2026-10-27 (UTC)

source: /Users/leekwangseok/.claude/projects/**/*.jsonl (all files, incl. subagents/)
filter: timestamp < 2026-10-27T00:00:00Z; records without parsable timestamp excluded
dedupe: records with a repeated uuid counted once
scope: aggregates only; no prompt text or credentials

## Scope
- jsonl files scanned: 537
- raw lines: 49,441; bad json lines: 0; no timestamp: 11419; at/after cutoff (excluded): 31,396
- included records (after uuid dedupe): 6,626
- min timestamp: 2026-09-08T01:20:14.590000+00:00; max timestamp: 2026-10-24T08:54:22.466000+00:00
- distinct sessions (sessionId): 7; projects: 3

## Records per date (UTC)
- 2026-09-08: 1,349
- 2026-09-09: 985
- 2026-09-10: 461
- 2026-09-11: 380
- 2026-09-12: 128
- 2026-09-13: 173
- 2026-09-16: 66
- 2026-09-21: 12
- 2026-09-24: 617
- 2026-09-28: 273
- 2026-09-30: 5
- 2026-10-02: 1,225
- 2026-10-03: 354
- 2026-10-06: 157
- 2026-10-07: 211
- 2026-10-09: 22
- 2026-10-10: 172
- 2026-10-24: 36

## Records per hour of day (UTC, 00-23)
- 00: 261
- 01: 390
- 02: 668
- 03: 219
- 04: 316
- 05: 353
- 06: 383
- 07: 606
- 08: 164
- 09: 9
- 10: 0
- 11: 0
- 12: 141
- 13: 743
- 14: 1,118
- 15: 853
- 16: 228
- 17: 11
- 18: 54
- 19: 105
- 20: 0
- 21: 0
- 22: 0
- 23: 4

## Per project (dir name under projects/)
count = records; sessions = distinct sessionId
- -Users-leekwangseok-PycharmProjects-VocalAnalyze: sessions=4, records=6,289
- -Users-leekwangseok-Desktop-------2-PycharmProjects-UnitV2M12: sessions=2, records=301
- -Users-leekwangseok: sessions=1, records=36

## Record type counts (record field `type`)
- assistant: 2,799
- attachment: 1,729
- user: 1,605
- system: 398
- queue-operation: 86
- file-history-delta: 9

## Records per model (message.model; count = records carrying a model)
- claude-opus-5: 1,330
- claude-sonnet-5: 1,155
- claude-fable-5: 217
- claude-sonnet-5-5: 79
- claude-opus-5-5: 9
- <synthetic>: 9

## tool_use blocks per tool name (count = tool_use content blocks)
- Bash: 1,069
- ScheduleWakeup: 24
- Edit: 18
- Write: 8
- Read: 7
- ToolSearch: 6
- mcp__claude_ai_Google_Docs__read_doc: 4
- mcp__claude_ai_Google_Docs__update_doc: 4
- TaskStop: 3
- WebFetch: 3
- AskUserQuestion: 3
- Skill: 2
- WebSearch: 2
- mcp__claude_ai_Google_Drive__read_file_content: 2
- total: 1,155

## Token totals (sum of message.usage fields over included records)
input = input_tokens; output = output_tokens; cache_read = cache_read_input_tokens; cache_creation = cache_creation_input_tokens
- input: 5,575
- output: 2,554,029
- cache_read: 1,208,570,617
- cache_creation: 42,335,338

## Token totals per model
- claude-opus-5: input=2,639, output=1,224,606, cache_read=500,932,267, cache_creation=12,220,784
- claude-sonnet-5: input=2,312, output=1,020,628, cache_read=488,675,769, cache_creation=19,425,125
- claude-fable-5: input=434, output=236,257, cache_read=160,890,704, cache_creation=6,680,985
- claude-sonnet-5-5: input=172, output=69,809, cache_read=57,733,307, cache_creation=3,969,191
- claude-opus-5-5: input=18, output=2,729, cache_read=338,570, cache_creation=39,253
- <synthetic>: input=0, output=0, cache_read=0, cache_creation=0

## Per-session distribution
records per session: median=9, max=6,274, n=7
total tokens per session (input+output+cache_read+cache_creation): median=22,005,191, max=1,231,079,798, n=3
output tokens per session: median=221,662, max=2,329,638, n=3
input tokens per session: median=244, max=5,313, n=3
cache_read tokens per session: median=20,652,890, max=1,187,579,157, n=3
cache_creation tokens per session: median=1,130,395, max=41,165,690, n=3

