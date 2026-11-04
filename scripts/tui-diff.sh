#!/usr/bin/env bash
# TUI parity harness: drive Go chaude and Rust chaude in tmux (real tty, 100x30),
# both pointed at the same fake `claude` (same stdin/stdout contract as
# crates/core/tests/engine.rs), capture panes with `capture-pane -p -e`,
# and print a unified diff + match % per scenario.
#
# Output: logs/tui-diff/<scenario>.{go,rs}.txt, <scenario>.diff, summary.txt
# Status: logs/tui-diff.status (one "step: ... / pct: N" line per step)
#
# Usage: scripts/tui-diff.sh [scenario ...]   (no args = all scenarios)
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/logs/tui-diff"
STATUS="$ROOT/logs/tui-diff.status"
GO_SRC="$ROOT/deprecated/src/main/intt"
GO_BIN="$OUT/build/chaude-go"
RS_BIN="$ROOT/target/debug/chaude"
FIX="$ROOT/crates/core/tests/fixtures"
FAKE_DIR="$OUT/fakebin"
WORK="$OUT/work"
COLS=100
ROWS=30
ALL_SCENARIOS=(startup-idle type-text multiline submit-stream tool-use slash-menu scroll-up esc-interrupt resize-80x24 exit usage settings resume view)

mkdir -p "$OUT" "$FAKE_DIR" "$WORK" "$OUT/build"
: >"$STATUS"

step() { # step <text> <pct>
  echo "step: $1 / pct: $2" >>"$STATUS"
  echo "[tui-diff] $1 ($2%)"
}

command -v tmux >/dev/null || { echo "tmux not found (brew install tmux)" >&2; exit 2; }
command -v go >/dev/null || { echo "go not found" >&2; exit 2; }

# ---- fake claude: same stdin/stdout contract as crates/core/tests/engine.rs ----
# Each user frame sleeps FAKE_SLEEP seconds, then replays FAKE_FIXTURE.
cat >"$FAKE_DIR/claude" <<'EOF'
#!/bin/sh
case "$*" in
  *--version*) echo "2.0.0 (Claude Code)"; exit 0 ;;
  # one-shot /usage only (`-p /usage --output-format json`); the session argv starts with `-p` too, so match the json format
  *"--output-format json"*) echo '{}'; exit 0 ;;
esac
while IFS= read -r line; do
  case "$line" in
    *'"type":"user"'*)
      sleep "${FAKE_SLEEP:-0.5}"
      cat "$FAKE_FIXTURE"
      ;;
  esac
done
EOF
chmod +x "$FAKE_DIR/claude"

# ---- build ----
step "build go chaude" 0
(cd "$GO_SRC" && CGO_ENABLED=0 go build -trimpath -o "$GO_BIN" ./cmd/chaude) \
  || { step "go build failed" 0; echo "go build failed" >&2; exit 1; }
step "build rust chaude" 10
(cd "$ROOT" && cargo build -p chaude-cli) >"$OUT/build/cargo.log" 2>&1 \
  || { step "cargo build failed" 10; echo "cargo build failed, see $OUT/build/cargo.log" >&2; exit 1; }

# ---- helpers ----
kill_sess() { tmux kill-session -t "$1" 2>/dev/null; true; }

launch() { # launch <session> <bin> <fixture> <fake_sleep>
  kill_sess "$1"
  tmux new-session -d -s "$1" -x "$COLS" -y "$ROWS" -c "$WORK" \
    "env PATH='$FAKE_DIR:$PATH' FAKE_FIXTURE='$FIX/$3' FAKE_SLEEP=$4 '$2'"
  # keep the pane after the process exits so the exit scenario can still be captured
  tmux set-option -t "$1" remain-on-exit on 2>/dev/null
}
text() { tmux send-keys -t "$1" -l "$2"; }
key() { tmux send-keys -t "$1" "$2"; }
pause() { sleep "$1"; }

drive() { # drive <scenario> <session>
  local s="$2"
  case "$1" in
    startup-idle)
      pause 3 ;;
    type-text)
      pause 2; text "$s" "hello world"; pause 1 ;;
    multiline)
      pause 2; text "$s" "line one"; key "$s" C-j; text "$s" "line two"; pause 1 ;;
    submit-stream)
      pause 2; text "$s" "hi there"; key "$s" Enter; pause 3 ;;
    tool-use)
      pause 2; text "$s" "scan the repo"; key "$s" Enter; pause 3 ;;
    slash-menu)
      pause 2; text "$s" "/"; pause 1 ;;
    scroll-up)
      pause 2
      for i in 1 2 3 4; do text "$s" "msg $i"; key "$s" Enter; pause 1.5; done
      key "$s" PPage; pause 1; text "$s" "x"; pause 1 ;;
    esc-interrupt)
      pause 2; text "$s" "long task"; key "$s" Enter; pause 1.5; key "$s" Escape; pause 1.5 ;;
    resize-80x24)
      pause 2; text "$s" "hi"; key "$s" Enter; pause 2
      tmux resize-window -t "$s" -x 80 -y 24; pause 2 ;;
    exit)
      pause 2; text "$s" "/exit"; key "$s" Enter; pause 2 ;;
    usage)
      # /usage opens the full-screen window (one-shot claude -p; the fake returns '{}', so both sides show the same failure text).
      pause 2; text "$s" "/usage"; key "$s" Enter; pause 3 ;;
    settings)
      # /settings opens the settings picker below the input box (no claude call).
      pause 2; text "$s" "/settings"; key "$s" Enter; pause 2 ;;
    resume)
      # /resume with no argument lists the record topics (empty work dir: same message on both sides).
      pause 2; text "$s" "/resume"; key "$s" Enter; pause 2 ;;
    view)
      # /view with no owned Claude Code session shows the same note on both sides.
      pause 2; text "$s" "/view"; key "$s" Enter; pause 2 ;;
  esac
}

run_side() { # run_side <scenario> <go|rs> <bin>
  local scn="$1" side="$2" bin="$3" fix="text_turn.jsonl" fsleep="0.5" sess
  [ "$scn" = tool-use ] && fix="agent_turn.jsonl"
  [ "$scn" = esc-interrupt ] && fsleep="5"
  sess="tuidiff-$scn-$side"
  launch "$sess" "$bin" "$fix" "$fsleep"
  drive "$scn" "$sess"
  if tmux has-session -t "$sess" 2>/dev/null; then
    tmux capture-pane -p -e -t "$sess" >"$OUT/$scn.$side.txt"
  else
    echo "(session ended before capture)" >"$OUT/$scn.$side.txt"
  fi
  kill_sess "$sess"
}

# ---- scenarios ----
SCENARIOS=("$@")
[ "${#SCENARIOS[@]}" -eq 0 ] && SCENARIOS=("${ALL_SCENARIOS[@]}")
total=${#SCENARIOS[@]}
n=0
for scn in "${SCENARIOS[@]}"; do
  n=$((n + 1))
  pct=$((10 + n * 85 / total))
  step "run scenario $scn" "$pct"
  run_side "$scn" go "$GO_BIN"
  run_side "$scn" rs "$RS_BIN"
done

# ---- compare ----
: >"$OUT/summary.txt"
printf '%-16s %8s %8s %8s\n' scenario match% go_rows rs_rows | tee -a "$OUT/summary.txt"
for scn in "${SCENARIOS[@]}"; do
  g="$OUT/$scn.go.txt"; r="$OUT/$scn.rs.txt"
  diff -u --label "go/$scn" --label "rs/$scn" "$g" "$r" >"$OUT/$scn.diff"
  # unchanged-line-format emits one '=' per equal line; other formats emit nothing
  # row-wise equality (BSD diff has no --unchanged-line-format); tot = max row count
  read -r eq gn rn tot < <(awk 'NR==FNR{g[FNR]=$0; gn=FNR; next}
    {rn=FNR; if (g[FNR]==$0 && FNR<=gn) e++}
    END{t=(gn>rn?gn:rn); if(t==0)t=1; print e+0, gn+0, rn+0, t}' "$g" "$r")
  pct=$(( eq * 100 / tot ))
  printf '%-16s %7s%% %8s %8s\n' "$scn" "$pct" "$gn" "$rn" | tee -a "$OUT/summary.txt"
done

echo "" | tee -a "$OUT/summary.txt"
echo "worst diffs (changed lines per scenario):" | tee -a "$OUT/summary.txt"
for scn in "${SCENARIOS[@]}"; do
  c=$(grep -c '^[-+][^-+]' "$OUT/$scn.diff")
  echo "  $scn: $c changed lines, see $OUT/$scn.diff" | tee -a "$OUT/summary.txt"
done
step "done" 100
