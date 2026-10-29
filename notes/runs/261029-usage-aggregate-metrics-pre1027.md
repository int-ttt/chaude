# Usage aggregate: metrics pre-2026-10-27 (261029)

task: aggregate harness metrics, filename date < 2026-10-27 only
status: PASS (no qualifying files; zero-record aggregate)

## files included
- (none)
files_skipped (date >= cutoff or non-date name):
- 2026-10-28.jsonl
- 2026-10-29.jsonl

## record count
- records: 0

## date span
- (no records)

## per-date counts (by ts date, UTC)
- (none)

## per-model counts
- (none)

## per-route counts
- (none)

## per-tier counts
- (none)

## per-stage counts (context)
- (none)

## numeric totals (sum over records that have the field)
- input_tokens: uncached input tokens per turn record
- output_tokens: generated output tokens per turn record
- cache_creation_input_tokens: tokens written to prompt cache
- cache_read_input_tokens: tokens read from prompt cache
- ctx: context size reported for the record (summed, not a cost)
- (no cost/token/cache numeric fields found)

## settings keys (~/.claude/settings.json, not date-filtered; values only for these keys)
- effortLevel: "medium"
- permissions: {"defaultMode": "auto"}

## notes
- 2026-10-28.jsonl and 2026-10-29.jsonl are both >= 2026-10-27 and were excluded.
- no raw prompt text, env, tokens or credentials are included.
