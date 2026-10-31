# 261029 explore: running chaude-related processes (read-only)
status: done (read-only; no process killed or signaled; no repo source edited; only this file written)
date: 2026-10-29 22:56

## 1. processes (ps -axo pid,ppid,etime,command, filtered)
- pid 61618 ppid 9704 etime 12:16 `chaude`
  purpose: chaude harness main process (REPL/TUI) running in the zsh shell of GoLand terminal.
- pid 61653 ppid 61618 etime 12:06 `/Users/leekwangseok/.local/bin/claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model haiku --permission-prompt-tool stdio --append-system-prompt "Default to delegating bounded code ..."`
  purpose: child Claude Code worker (haiku model, stream-json pipe) spawned by chaude 61618; one of the per-model live children (v0.8.0).
- pid 51677 ppid 9181 etime 01:06:57 `/Users/leekwangseok/.local/bin/claude`
  purpose: interactive Claude Code session; cwd = repo (lsof). Not a chaude child (no chaude in its ancestry). Relevance by path only.

## 2. parent chain (context, not counted)
- pid 9181 ppid 1 etime 01-01:26:45 GoLand IDE (`goland`). Its children: 9556 fsnotifier, 9586 cef_server (JCEF), 9704 zsh, 51677 claude, 41768 zsh.
- pid 9704 ppid 9181 etime 01-01:25:53 `/bin/zsh --login -i`; parent of chaude 61618.
- pid 41768 ppid 9181 etime 22:02:27 `/bin/zsh --login -i`; no chaude/claude child; cwd not checked.
- no `go run`, `go build`, or `go test` process found. The `go` match was GoLand only.
- Google Chrome and Crashpad processes matched the filter and were excluded as unrelated.

## 3. agent-task status and logs
- logs/*.status: none exist.
- logs/ exists (205 entries): hidden .corr-<uuid> and .lean-ver-<uuid> marker files (1 and 5 bytes), plus history/ (INDEX.txt, per-turn 20261028-HHMM-<id>-01.txt files, latest mod Oct 29 00:29).
- agent store: ~/.chaude/agents.json exists (151392 bytes, mod 2026-10-29 22:55, mode 600). Also ~/.chaude/clients.json. Path per internal/harness/agentstore.go:255.
- not inspected: contents of agents.json (state values not read).

## 4. notes/runs: in-progress or no final status
- 261029-haiku-coder-multiproc-integration.md: `status: partial`. acceptance line lists build/vet/test; `uncommitted: yes`. Only explicit in-progress marker found.
- 261029-haiku-runner-release-v080-push.md (22:39): no status line; doubts listed (main.go version string, /usage budget charge). Looks unfinished or push-blocked; not confirmed.
- Recent read-only explore/analysis runs with no status line (22:35-22:48): 261029-explore-child-files.md, 261029-explore-handoff-resume-drain.md, 261029-explore-classifier-category-mode.md, 261029-explore-harness-3files.md, 261029-analysis-structure-and-roadmap.md. Content looks complete (findings present) but uses no status header. Treated as done, unverified.
- Files with `status: done` or `verdict:` in recent set (not in progress): 261029-haiku-coder-release-v080.md (done, commit+tag, NOT pushed), 261029-test-effort-haiku-vs-sonnet.md, 261029-test-router-30-prod-args.md, 261029-review-v080-final.md (PASS).
- Plans marked `plan only`: not running jobs.

## 5. uncertainty
- pid 51677 is only path-linked to the repo (cwd); its purpose is inferred.
- No status line in a run file does not prove it is still running; grep-based check only.
- ps snapshot taken at 22:55:59; processes may end after that.
