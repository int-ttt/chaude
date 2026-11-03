#!/usr/bin/env bash
# e2e: 하네스(CLAUDE.md + .claude)를 빈 임시 작업공간에 복사해 과제 5개를 풀게 하고 채점한다.
# 사용: bash tests/e2e/run.sh            (TASKS="lru_ttl js_roman" 처럼 일부만 실행 가능)
# 주의: claude -p 를 실제로 호출하므로 API 비용이 든다. README.md 참고.
set -uo pipefail

E2E_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$E2E_DIR/.." && pwd)"
STAMP="$(date +%y%m%d-%H%M%S)"
OUT="${E2E_OUT:-$E2E_DIR/results/$STAMP}"
TASKS="${TASKS:-lru_ttl c_brackets go_wordfreq py_intervals js_roman}"
MAX_BUDGET="${E2E_MAX_BUDGET:-2}"
TIMEOUT="${E2E_TIMEOUT:-900}"

if ! command -v claude >/dev/null 2>&1; then
  echo "error: claude CLI 가 없음 (npm install -g @anthropic-ai/claude-code)" >&2
  exit 2
fi
if ! command -v perl >/dev/null 2>&1; then
  echo "error: perl 이 필요함 (타임아웃 래퍼)" >&2
  exit 2
fi

mkdir -p "$OUT" || exit 2
echo "out: $OUT"
echo "budget/task: \$$MAX_BUDGET  timeout/task: ${TIMEOUT}s  tasks: $TASKS"

rc_all=0
for t in $TASKS; do
  tdir="$E2E_DIR/tasks/$t"
  if [ ! -f "$tdir/prompt.ko.txt" ] || [ ! -f "$tdir/check.py" ]; then
    echo "skip $t: tasks/$t 없음" >&2
    rc_all=1
    continue
  fi
  wd="$(mktemp -d "${TMPDIR:-/tmp}/e2e-$t.XXXXXX")" || exit 2
  cp "$REPO/CLAUDE.md" "$wd/" && cp -R "$REPO/.claude" "$wd/" || { echo "copy 실패: $wd" >&2; exit 2; }
  find "$wd" \( -name __pycache__ -o -name .DS_Store \) -prune -exec rm -rf {} +
  prompt="$(cat "$tdir/prompt.ko.txt")"

  echo "== $t  workdir=$wd"
  (cd "$wd" && CLAUDE_PROJECT_DIR="$wd" perl -e 'alarm shift; exec @ARGV' "$TIMEOUT" \
     claude -p "$prompt" --output-format json --permission-mode bypassPermissions \
     --max-budget-usd "$MAX_BUDGET") >"$OUT/$t.json" 2>"$OUT/$t.err"
  cli_rc=$?
  echo "   claude rc=$cli_rc"

  python3 -I "$E2E_DIR/score.py" task --task "$t" --task-dir "$tdir" --workdir "$wd" \
    --json "$OUT/$t.json" --cli-rc "$cli_rc" --out "$OUT" || rc_all=1
done

echo
python3 -I "$E2E_DIR/score.py" summary --out "$OUT" || rc_all=1
echo "result: $OUT/summary.json  (workdirs kept for inspection)"
exit $rc_all
