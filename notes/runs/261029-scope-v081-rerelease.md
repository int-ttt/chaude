found: tag v0.8.1 already exists -> 1af0f16 (HEAD), GitHub release exists (per brief)
pending changes (tracked): .claude/hooks/haiku-context-guard.py, .claude/settings.json (Bash deny), notes/FACTS.md, notes/LEDGER.md, notes/metrics/2026-10-29.jsonl
not done: CHANGELOG edit, build/vet/test, review, commit (stopped per rule 6: tag exists)
open decision: how to ship the Bash-deny hook change
options:
 A: commit on master, no tag move; ship as v0.8.2 (new tag + release). recommended, no overwrite.
 B: commit only, leave unreleased until next version.
 C: move/delete v0.8.1 tag (forbidden by brief; needs explicit user approval).
