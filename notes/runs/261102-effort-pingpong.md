date: 2026-11-02
task: 핑퐁 벤치 effort 재측정 (harness 메인 Opus unset / low / medium)
status: done
method: docs/pingpong-bench.md 방식 그대로. 2 runs x 3 prompts per level, harness only. plain 재사용.
harness-copy: /tmp/bench-harness .claude/settings.json effortLevel 변경 (unset=키 삭제). 저장소 .claude/settings.json 미변경 (medium 유지)
runner: /tmp/pp-effort/run.sh (resumable), status logs/pingpong-effort.status, log logs/pingpong-effort.log
raw: notes/runs/pingpong-raw/effort-{level}-r{n}-p{p}.json, games-effort-{level}-r{n}/pong.py
aggregate: /tmp/pp-effort/agg.py
result-unset: cost 0.861 total 1,272,448 turns 5.0 dur 140.8s
result-low: cost 0.689 total 1,083,215 turns 5.0 dur 110.8s
result-medium: cost 0.786 total 1,078,181 turns 4.5 dur 93.2s
plain-ref: cost 1.379 total 787,628 turns 22 dur 120.2s
calls: 54 claude calls total (18 runs x 3), errors 0
doubts: n=2 per level; cache state noise (same low 0.79 old vs 0.69 new); permission_denials 3-7 per run (unset/low), medium r1 6 r2 0; score-display check regex only, medium-r2 has no score|점수 text; trailer model Haiku per system rule
