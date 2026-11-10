---
name: status
description: Report progress of background jobs from logs/*.status. Use for 진행상황 (progress), 끝났어? (finished?), 상태 (status); English: progress, status, done yet?, is it finished?, what's the status.
argument-hint: "[job name]"
context: fork
agent: haiku-runner
model: haiku
background: false
---

Think and write in English. Only haiku-tui writes user-facing text, in the CHAUDE_LANG language (ko or en).

Report status for job: $ARGUMENTS (if empty: every job in `logs/*.status`).
Read the status files and `tail -n 20` of each log only.
≤5 lines: done/total, failures by item, ETA, running or stopped.
