# usage aggregate metrics (261029)
script: /tmp/agg/agg_metrics.py (python3 -I, foreground). aggregates only, no prompt text.
ts_days: records bucketed by ts date prefix.
sums: numeric fields summed over records where present (null excluded). not de-duplicated.
note: start_ms/end_ms sums are epoch-ms sums, meaningless, ignore.

## 2026-10-28.jsonl
records: 84 (unparsed 0), distinct session ids: 6, ts day: 2026-10-28 (84)
stage counts: turn 55, import 5, category 19, rotate 3, classify 2
model counts (records with model field, 55): haiku 54, opus 1
flags seen on all 84 records: compact_window, drain_timeout_s, handoff_validate, max_drain, threshold; delegation_rule 83; category_rotate 71; conductor 71
fields (record count having field): cache_creation_input_tokens 1, cache_read_input_tokens 1, input_tokens 1, output_tokens 1, ctx 55, model 55, result_ctx 56, result_input_tokens 56, result_output_tokens 56, result_cache_creation_input_tokens 56, result_cache_read_input_tokens 56, cost_usd 56, rotations 54, conductor 47, duration_ms 31, delegations 29, draining 30, source 30, from/to/switch 19 each, attempts/handoff_valid/old_session/outcome/reason/rotate_at_tokens 3 each, from_model 2, routed_model 2, dry_run/files/reco_found/snapshot_chars/turns 5 each, reco_preserved 8
totals:
- cost_usd sum: 24.7821 (over 56 records with cost)
- result_input_tokens sum: 204; result_output_tokens sum: 56400; result_cache_creation_input_tokens: 241139; result_cache_read_input_tokens: 4483939; result_ctx sum: 4725282 (per-result context tokens, summed across turns)
- ctx sum (55 records): 2989832
- rotations sum: 13 (rotations field, 54 records); rotate stage records: 3
- delegations sum: 17; draining sum: 5
- rotate_at_tokens sum: 40465 (3 records)
- top-level input_tokens 2, output_tokens 4, cache_creation 20569, cache_read 0 (single records only)

## 2026-10-29.jsonl
records: 223 (unparsed 0), distinct session ids: 16, ts day: 2026-10-29 (223)
stage counts: turn 72, classify 68, category 66, rotate 12, import 5
model counts (records with model field, 72): opus 13, sonnet 8, haiku 51
  note: classify records carry from_model/routed_model (68), not model
flags seen on all 223 records: category_rotate, compact_window, conductor, delegation_rule, drain_timeout_s, handoff_validate, max_drain, threshold
fields (record count having field): conductor 140, cost_usd 140 (134 non-null), result_* token/ctx fields 140 each, duration_ms 136, session 138, ctx 72, model 72, rotations 72, delegations 68, draining 68, from_model 68, routed_model 68, from/to/switch 66 each, kind 37, reco_preserved 17, attempts/handoff_valid/old_session/outcome/reason/rotate_at_tokens 12 each, source 9, dry_run/files/reco_found/snapshot_chars/turns 5 each
totals:
- cost_usd sum: 54.9803 (over 134 non-null)
- result_input_tokens sum: 424; result_output_tokens sum: 116976; result_cache_creation_input_tokens: 1000460; result_cache_read_input_tokens: 7355601; result_ctx sum: 8356485
- ctx sum (72 records): 3549813
- rotations sum: 168 (72 records); rotate stage records: 12
- delegations sum: 71; draining sum: 0
- rotate_at_tokens sum: 509770 (12 records)
- duration_ms sum: 6839261 (136 records)

## combined (both files)
cost_usd: 79.7604 (USD, sum of non-null cost_usd)
records: 307; rotate stage records: 15; rotations field sum: 181

## settings (/Users/leekwangseok/.claude/settings.json), model/permission/effort keys only
- model: no top-level key present
- effortLevel: medium
- permissions: present (dict); permissions.defaultMode: auto
- autoMode: present (dict, not expanded)
- other top-level keys not reported (env, enabledPlugins, extraKnownMarketplaces, language, theme, agentPushNotifEnabled)

## doubts
- result_* fields and cost_usd may overlap with per-turn fields; sums are not de-duplicated, so totals may double-count if both are summed.
- 2026-10-28 has no ctx-based classify model field; classify records use from_model/routed_model.
- rotations counts are per-record counters, not distinct events; rotate stage count is a separate count.
