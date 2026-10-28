#!/usr/bin/env bash
# Fresh start for one or more projects: install, extract old transcripts into notes/distill, archive the old transcripts.
# usage: ./fresh_start.sh <project-dir> [<project-dir> ...] [--main opus|sonnet] [--force]
# Close every Claude Code window for these projects first. Then, per project: cd <dir> && claude, and type /distill.
set -uo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)"
MAIN=()
PROJECTS=()
FORCE=0
while [ $# -gt 0 ]; do
  case "$1" in
    --main) MAIN=(--main "$2"); shift 2 ;;
    --force) FORCE=1; shift ;;
    *) PROJECTS+=("$1"); shift ;;
  esac
done
[ ${#PROJECTS[@]} -gt 0 ] || { echo "usage: ./fresh_start.sh <project-dir> [...] [--main opus|sonnet] [--force]"; exit 1; }

if [ "$FORCE" -eq 0 ] && pgrep -x claude >/dev/null 2>&1; then
  echo "실행 중인 Claude Code가 있습니다. 이 프로젝트들의 Claude 창을 닫은 뒤 다시 실행하세요."
  echo "(다른 프로젝트의 Claude만 열려 있다면 끝에 --force 를 붙이세요)"
  exit 1
fi

mkdir -p ~/claude-archive
[ -f ~/usage-before.txt ] || python3 "$SRC/scripts/usage_report.py" > ~/usage-before.txt 2>/dev/null || true

DONE=()
for ARG in "${PROJECTS[@]}"; do
  P="$(cd "$ARG" 2>/dev/null && pwd)" || { echo "✗ 폴더 없음: $ARG"; continue; }
  NAME="$(basename "$P")"
  HIST="$(python3 "$SRC/scripts/claude_history.py" "$P")"
  echo "== $NAME ($P)"

  INST="$("$SRC/install.sh" "$P" ${MAIN[@]+"${MAIN[@]}"})" || { echo "✗ 설치 실패: $NAME"; continue; }
  printf '%s\n' "$INST" | grep -E "CLAUDE.md|settings.json|백업" | sed 's/^/   /'

  if [ -z "$HIST" ] || [ ! -d "$HIST" ]; then
    echo "   대화 기록 없음 → 추출·보관 건너뜀. 바로 'cd \"$P\" && claude'로 시작하세요."
    DONE+=("$P|없음"); continue
  fi

  OUT="$(python3 "$P/scripts/extract_transcript.py" "$HIST" --out "$P/notes/distill" 2>&1)"
  echo "   $OUT"
  N="$(printf '%s' "$OUT" | sed -n 's/.*메시지 \([0-9][0-9]*\)개.*/\1/p')"
  if [ -z "$N" ] || [ "$N" -eq 0 ]; then
    echo "✗ 추출된 메시지가 없어 기록을 보관하지 않았습니다. 위 출력을 확인하세요."
    continue
  fi

  DEST="$HOME/claude-archive/$NAME-$(date +%Y%m%d)"
  i=2; while [ -e "$DEST" ]; do DEST="$HOME/claude-archive/$NAME-$(date +%Y%m%d)-$i"; i=$((i+1)); done
  mv "$HIST" "$DEST" && echo "   기록 보관: $DEST" && python3 "$P/scripts/compress_archive.py" "$DEST"
  DONE+=("$P|distill")
done

echo
echo "다음 단계:"
for d in ${DONE[@]+"${DONE[@]}"}; do
  P="${d%%|*}"; K="${d##*|}"
  if [ "$K" = "distill" ]; then echo "  cd \"$P\" && claude   → /distill 입력 → notes/distill/REVIEW.md 질문에 답하기"
  else echo "  cd \"$P\" && claude"; fi
done
