# Agent scheduler plan (2026-11-15)

Goal: a long agent job runs as a sequence of steps from a task file. The resident coder runs each step, gates it, and commits at each checkpoint, without a main round-trip per step.

## Design (main, final)
- Task file: `notes/tasks/<ws>.md`. Steps: title, files, gate cmd, commit msg, status `todo|done|failed`, commit hash, notes.
- Checkpoint = end of a step: the agent runs the gate, commits, and updates that step in the task file.
- Scheduler (core, step 2) dispatches the next `todo` step to the resident coder without main round-trips.
- At each checkpoint, if the agent context is over 50K, the next step starts in a fresh session given only the task file and the step.
- Gate fail: one retry in a fresh session with the failure log. If it fails again, stop and report.
- Safety stays: 80K rotation and `maxTurns: 60`.
- Step 3: TUI agentview shows step n/m. GUI later.
- Open tasks are exempt from notes GC.

## File format
```
# Task: <title>

## <step title>
- files: path/a.rs, path/b.rs
- gate: <command>
- commit: <commit message>
- status: todo
- hash:
> <failure log lines, added by mark_failed>
```
- A step starts at `## `. Unknown non-blank lines in a step are notes and are kept as written.
- Field order on write: files, gate, commit, status, hash, then notes.
- A missing status reads as `todo`. An unknown status is an error.
- Only a `todo` step can be marked done or failed.
- Failure logs are written as `> ` lines, so they cannot start a new step or a field.
- The workspace id is letters, digits, `-` or `_`.

## Steps
1. [done in code] Core module `crates/core/src/tasks.rs`: parse and render the task file, `next_todo`, `mark_done`, `mark_failed`, load and save, unit tests. Checkpoint rule in the `haiku-coder` and `haiku-batch-worker` definitions (`crates/core/src/settings/agents/` and `.claude/agents/`): "if the brief names a task file, do only the first todo step, run its gate, commit, update that step, hand back."
2. Scheduler in core: pick the next todo step, dispatch it to the resident coder, read the checkpoint result, apply the context and retry rules above.
3. TUI agentview shows step n/m. GUI later.

## Gate (every step)
- `python3 scripts/squeeze-run.py -- cargo fmt --all -- --check`
- `python3 scripts/squeeze-run.py -- cargo clippy --all-targets -- -D warnings`
- `python3 scripts/squeeze-run.py -- cargo test -p chaude-core -p chaude-cli`
