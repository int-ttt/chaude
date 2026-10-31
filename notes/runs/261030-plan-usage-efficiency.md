# plan: usage efficiency (identical input-output) - 261030
goal: same user input -> same output, fewer tokens/usage. Every item names measured metric + expected effect; none changes output.
sources: notes/runs/261030-usage-eff-measure.txt (eff.py, all transcripts, assumed prices opus 5/25/cw6.25/cr0.5, sonnet 3/15/3.75/0.3, haiku 1/5/1.25/0.1 per MTok), notes/runs/261030-usage-m3.txt (measure.py), notes/runs/261030-usage-ur-reconcile.txt.

## measured basis
- est_usd_all: 135.58 = cache_write 48.00 (35.4%) + cache_read 35.50 + output 52.04
- subagent_share: 31 sessions covered by metrics: main-only 11.34 vs main+subagents 82.89 -> subagents ~71.55 (~86%)
- spawns_all: 612 (scout 229, sonnet-analyst 181, haiku-coder 59, opus-architect 36, batch-worker 32, runner 31)
- first_msg_cache_write (cold spawn): scout median 2352 tok, sonnet-analyst median 2736 / mean 4197, opus-architect mean 6571 / median 7308, coder median 3487
- first_msg_cw_usd: 6.33 (13.2% of cache_write usd); remaining cache_write 41.67 = context growth inside runs + cache expiry rewrites
- short_spawns (<=3 msgs): 189, cache_write 4.22 usd
- by_type_10-29 (measure.py): sonnet-analyst 20.68, opus-architect 13.40 of 49.25 usd (m2 run; m0 was 49.58) (~69%)
- main_sessions: cache_write 3.93M tok, cache_read 64.86M tok (read/write 16.5x), output 0.80M
- harness cost_usd is cumulative per session: naive turn sum 467.18 vs per-session-max 43.34 (10-29..10-30)

## recommendations (ranked by measured share)
R1 subagent context growth (largest: subagents ~86%, non-first cache_write 41.67 usd)
- form: scouts/coders grep first, read line ranges only; RETURN caps kept (<=4000 chars); parent never re-asks for text it already holds.
- metric: per-type sum cache_write and cache_read per spawn (eff.py section b).
- expected: cache_write and cache_read per spawn fall roughly in proportion to bytes not read; output same because the same facts are returned.
R2 Opus/Sonnet own-turn tokens (sonnet-analyst + opus-architect ~69% of 10-29 usd)
- form: Opus/Sonnet agents send only deterministic subtasks (verbatim read, verbatim save, command run, gofmt/vet/test) to Haiku; they keep the decisions. Do not move reasoning to Haiku (that would change output).
- metric: usd by agent type per day; turns per opus/sonnet spawn.
- expected: fewer Opus/Sonnet output+read tokens per task; deterministic outputs are identical by construction.
R3 batch independent scout briefs into one spawn
- form: when 2+ read-only briefs have no dependency, send them as one numbered brief to one haiku-scout.
- metric: spawns per turn; first_msg cache_write (scout median 2352 tok).
- expected: saves (k-1) x ~2.4k cache_write tok + one parent tool round per merge; small in usd (haiku) but lowers spawn count and parent context.
R4 short spawns (189 spawns <=3 msgs, 4.22 usd cache_write)
- form: fold a short follow-up (one grep, one check) into the next brief of the same workstream instead of a new spawn.
- metric: count of spawns with <=3 msgs and their cache_write usd.
- expected: up to 4.22 usd-equivalent less cache_write over the measured period at the same output.
R5 prompt prefix stability (cache hits)
- form: keep CLAUDE.md and .claude/agents/*.md byte-stable during a work window; batch edits to them; hooks put dynamic text (time, git status) after the stable prefix.
- metric: main read/write ratio (now 16.5x); first_msg cache_write per agent type vs its .md size.
- expected: fewer cold rewrites after edits; no output change since content is unchanged.
R6 opus-architect cold prompt (mean 6571 vs scout 2352 tok first write)
- form: candidate trim of opus-architect.md duplication. NOT implemented: changing instructions may change behavior; needs an A/B on fixed tasks.
- metric: first_msg cache_write of opus-architect.
- expected: ~4k tok x 6.25/MTok ~= 0.025 usd per spawn (36 spawns ~= 0.9 usd).
R7 measurement in harness (implemented now)
- form: stage=turn row gets sub_msgs, sub_input_tokens, sub_output_tokens, sub_cache_creation_input_tokens, sub_cache_read_input_tokens, spawns, spawn_types (subagent assistant usage seen in the stream, deduped by message.id).
- metric: these fields per turn; usd_per_pct from stage=usage rows.
- expected: R1-R5 effects become measurable from notes/metrics without transcript price estimates. No output change (logging only).

## implementation (R7 only)
- internal/harness/turnspend.go: type turnSpend {sub map[string]map[string]any (msg id -> usage); spawns int; types map[string]int}; func newTurnSpend() *turnSpend; func (s *turnSpend) onAssistant(ev event) - if !isMainEvent(ev): store message.usage by message.id (keep last); for every tool_use block named Agent or Task (main or sub): spawns++, types[input.subagent_type or "general-purpose"]++; func (s *turnSpend) fields() map[string]any - sums; omit all keys when zero activity.
- repl.go main turn loop: create per turn, call onAssistant on each assistant event, merge fields() into the turn row before r.metrics.Log.
- internal/harness/turnspend_test.go: dedupe by id, main usage ignored, spawn counting, empty -> no keys.
- scripts: none.

## acceptance
- gofmt -l internal/harness empty; go vet ./internal/harness/...; go test ./internal/harness/... -count=1 PASS.
## must not change
- user-visible output, routing, prompts, agent definitions, existing metric keys.
