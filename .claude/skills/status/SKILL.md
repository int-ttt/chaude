---
name: status
description: Report progress of background jobs from logs/*.status. Use for 진행상황, 끝났어?, 상태.
argument-hint: "[job name]"
context: fork
agent: haiku-runner
model: haiku
background: false
---

Report status for job: $ARGUMENTS (if empty: every job in `logs/*.status`).
Read the status files and `tail -n 20` of each log only.
≤5 lines: done/total, failures by item, ETA, running or stopped.
