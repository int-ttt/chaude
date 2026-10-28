#!/usr/bin/env bash
# Install the Opus-lean setup into a project.
# usage: ./install.sh <project-dir> [--main opus|sonnet]
# Safe to re-run: existing CLAUDE.md / settings.json are backed up and merged, not overwritten.
set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
DST="${1:?usage: ./install.sh <project-dir> [--main opus|sonnet]}"
DST="$(cd "$DST" && pwd)"
MAIN_MODEL=""
if [ "${2:-}" = "--main" ]; then
  MAIN_MODEL="${3:?--main needs opus or sonnet}"
  case "$MAIN_MODEL" in opus|sonnet) ;; *) echo "--main 은 opus 또는 sonnet"; exit 1 ;; esac
fi
export MAIN_MODEL

command -v python3 >/dev/null || { echo "python3가 필요합니다. 터미널에서 xcode-select --install 실행 후 다시 시도하세요."; exit 1; }

TS="$(date +%Y%m%d-%H%M%S)"
BK="$DST/.claude-backup-$TS-$$"
for f in CLAUDE.md .claude/settings.json; do
  if [ -f "$DST/$f" ]; then mkdir -p "$BK/$(dirname "$f")"; cp "$DST/$f" "$BK/$f"; fi
done

mkdir -p "$DST/.claude/agents" "$DST/.claude/hooks" "$DST/.claude/skills" "$DST/scripts" "$DST/templates" \
         "$DST/notes/research" "$DST/notes/runs" "$DST/notes/pastes" "$DST/logs" "$DST/docs/claude-lean"
cp "$SRC"/.claude/agents/*.md "$DST/.claude/agents/"
cp "$SRC"/.claude/hooks/*.py "$DST/.claude/hooks/"
cp -R "$SRC"/.claude/skills/. "$DST/.claude/skills/"
cp "$SRC"/scripts/*.py "$DST/scripts/"
cp "$SRC"/templates/*.md "$DST/templates/"
cp "$SRC"/README.md "$SRC"/APPLY_GUIDE.md "$SRC"/FRESH_START.md "$SRC"/MIGRATE_NOW_PROMPT.md "$SRC"/CHANGELOG.md "$DST/docs/claude-lean/"
chmod +x "$DST"/.claude/hooks/*.py "$DST"/scripts/*.py

[ -f "$DST/notes/FACTS.md" ] || cp "$SRC/templates/FACTS.template.md" "$DST/notes/FACTS.md"
[ -f "$DST/notes/DECISIONS.md" ] || cp "$SRC/templates/DECISIONS.template.md" "$DST/notes/DECISIONS.md"

python3 "$SRC/tools/merge_settings.py" "$SRC" "$DST" python3 "$MAIN_MODEL" python3 "$BK"

echo
[ -d "$BK" ] && echo "백업: $BK"
echo "설치 완료: $DST"
echo "다음: cd \"$DST\" && claude   (새 세션에서 /agents, /hooks 확인 — docs/claude-lean/APPLY_GUIDE.md 3단계)"
