# plan: orch A/B real run (M0,B,C,D,E,F x T1~T5 x 2)
user: "전부 구현한 뒤 테스트해라"; coordinator: worktree for same start commit, do not overwrite uncommitted changes.
owner: haiku-runner (long job). prereq: impl verified (notes/runs/261030-plan-orch-impl.md).
spend: real model usd; Opus mains in B/C/D/F. stop and report if cumulative usd > 15.
setup (per run, never touch main tree /Users/leekwangseok/PycharmProjects/claude-harness except notes/runs outputs):
 1 build binary from main tree: go build -o /tmp/orchab/chaude ./cmd/... (find the main package of chaude; record path).
 2 fresh worktree per run: git -C <repo> worktree add --detach /tmp/orchab/wt-<mode>-<task>-<n> HEAD. remove after run: git -C <repo> worktree remove --force <wt>. (only worktree commands; no checkout/stash/reset in main tree)
 3 overlay: rsync -a <repo>/.claude/ <wt>/.claude/ (new hooks + agents); copy <repo>/notes/DECISIONS.md to <wt>/notes/ (T1 answer source).
 4 T3 only: seed bug in <wt>/internal/harness/orch.go: in orchDefaultModel change `"opus"` -> `"sonet"` (one line); check = go test ./internal/harness -run TestOrchDefaultModel PASS after run.
preflight: check whether chaude runs non-interactively (piped stdin / HARNESS_PLAIN=1 / one-shot flag). record result.
 - if yes: run `CHAUDE_ORCH` via flag: /tmp/orchab/chaude --orch <mode> with prompt on stdin, cwd=<wt>.
 - if no: B,C,D,E via scripts/ab_headless.py arm: model = orchDefaultModel(mode) (m0: haiku), extra_args ["--append-system-prompt", orchRule(mode) text from internal/harness/orch.go], env CHAUDE_ORCH=<mode>, cwd=<wt>; F = NOT RUN (needs harness REPL routing) and say so.
modes: m0, b, c, d, e, f. runs: 2 per (mode, task): run1 cold, run2 warm (within 5 min). order per task: m0,b,c,d,f,e.
tasks (prompt verbatim):
 T1 "rotation threshold 값은?" -> PASS if answer contains 150k or 150,000.
 T2 "usagepoll.go와 ratestatus.go 관계 설명" -> haiku-checker rubric: 3 named facts present.
 T3 "internal/harness 의 TestOrchDefaultModel 이 실패한다. 고쳐라." -> PASS if test passes in <wt>.
 T4 "오케스트레이션 안 B와 D 중 무엇을 기본으로 할지 2-3개 선택지와 트레이드오프, 선택과 이유를 제시하라" -> rubric: options, trade-offs, choice+why.
 T5 "다음 텍스트를 notes/runs/abtest-t5.md 에 그대로 저장: 가나다 123 abc" -> PASS if file content equals text (diff empty).
record per run (jsonl): notes/runs/261030-orch-abtest.jsonl fields: mode, task, run, usd (total_cost_usd), model_usage, spawn_count, is_error, pass, secs, wt_commit.
pass judging: haiku-checker blind to mode (give only task, rubric, answer).
report: notes/runs/261030-orch-abtest-report.md: per mode usd total, PASS count, B<=0.85xM0 & PASS>=M0 check, C<=0.85xB check; Korean summary 3 lines.
must NOT: edit main tree code, commit, push, change settings.json/CLAUDE.md.
