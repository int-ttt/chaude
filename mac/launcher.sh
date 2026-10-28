#!/bin/bash
# Claude Lean.app launcher: native macOS dialogs (osascript) → install.sh / fresh_start.sh from the bundled package.
# Needs only what macOS ships plus python3 (the hooks need it anyway).
PKG="$(cd "$(dirname "$0")/../Resources/pkg" && pwd)"
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"
VER="$(cat "$PKG/VERSION")"
TITLE="Claude Lean $VER"

osa() { osascript -e "$1" 2>/dev/null; }
alert() { osa "display dialog \"$1\" with title \"$TITLE\" buttons {\"확인\"} default button 1 with icon ${2:-note}" >/dev/null; }
# choose "<prompt>" "<current label>" "<label1>" "<label2>" ... → prints the chosen label (empty if cancelled)
choose() {
  local prompt="$1" cur="$2"; shift 2
  local items="" l; for l in "$@"; do items="$items\"$l\","; done
  osa "set r to choose from list {${items%,}} with title \"$TITLE\" with prompt \"$prompt\" default items {\"$cur\"}
       if r is false then return \"\"
       return item 1 of r"
}

if ! command -v python3 >/dev/null; then
  alert "python3가 필요합니다.\n터미널에서 xcode-select --install 을 실행해 설치한 뒤 다시 열어 주세요." stop; exit 1
fi

# 1) project folders (several with ⌘-click)
FOLDERS="$(osa 'set fs to choose folder with prompt "설치할 프로젝트 폴더를 고르세요 (여러 개는 ⌘-클릭)" with multiple selections allowed
  set out to ""
  repeat with f in fs
    set out to out & POSIX path of f & linefeed
  end repeat
  return out')"
[ -n "$FOLDERS" ] || exit 0

# 2) task
A1="설치·업그레이드만"; A2="기존 대화 정리 후 새로 시작 (Claude 창을 닫고 실행)"
ACT="$(choose "어떤 작업을 할까요?" "$A1" "$A1" "$A2")"; [ -n "$ACT" ] || exit 0

# 3) options: defaults, then what the first project saved last time
FIRST="$(printf '%s\n' "$FOLDERS" | head -n 1)"
eval "$(python3 - "$FIRST" <<'PY'
import json, os, sys
o = {"main": "opus", "style": "explain", "inbox": True, "reset_alert": 150000, "history_export": True, "nesting": True, "start_mode": "lean"}
try:
    o.update(json.load(open(os.path.join(sys.argv[1], ".claude", "claude-lean.options.json"), encoding="utf-8")))
except Exception:
    pass
o.pop("haiku_cap", None)
for k, v in o.items():
    print(f"O_{k}='{json.dumps(v)}'")
PY
)"
pick() {  # pick <var> <prompt> <json1> <label1> <json2> <label2> ...
  local var="$1" prompt="$2"; shift 2
  local cur="${!var}" curl="" labels=() vals=()
  while [ $# -gt 0 ]; do vals+=("$1"); labels+=("$2"); [ "$1" = "$cur" ] && curl="$2"; shift 2; done
  local r; r="$(choose "$prompt" "${curl:-${labels[0]}}" "${labels[@]}")"; [ -n "$r" ] || exit 0
  local i; for i in "${!labels[@]}"; do [ "${labels[$i]}" = "$r" ] && printf -v "$var" '%s' "${vals[$i]}"; done
}
HOW="$(choose "선택사항" "지금 값 그대로 (처음이면 기본값)" "지금 값 그대로 (처음이면 기본값)" "하나씩 고르기")"; [ -n "$HOW" ] || exit 0
if [ "$HOW" = "하나씩 고르기" ]; then
  pick O_main "메인 모델" '"opus"' "Opus (판단이 많은 작업)" '"sonnet"' "Sonnet (지시가 명확한 작업, 더 저렴)"
  pick O_style "답변 방식" '"explain"' "설명형 (일반 Opus처럼 풀어서)" '"concise"' "간결형 (5줄 이내)"
  pick O_inbox "정보 문장 메모 (모델 호출 없이 보관했다가 다음 지시와 함께 전달)" true "켜기" false "끄기"
  pick O_reset_alert "맥락이 커지면 /exit 후 다시 시작하라는 안내" 150000 "150k에서 안내" 250000 "250k에서 안내" 0 "끄기"
  pick O_history_export "대화 기록을 logs/history에 txt로 자동 저장" true "켜기" false "끄기"
  pick O_nesting "Sonnet·Opus 에이전트가 단순 작업을 Haiku에게 직접 넘기기" true "켜기" false "끄기"
  pick O_start_mode "설치 직후 모드" '"lean"' "절약 모드" '"normal"' "일반 모드 (절약 규칙 꺼짐)"
fi

# 4) confirm
N="$(printf '%s\n' "$FOLDERS" | grep -c .)"
osa "display dialog \"폴더 ${N}개 · ${ACT}\n\n진행할까요?\" with title \"$TITLE\" buttons {\"취소\", \"설치\"} default button 2" | grep -q "설치" || exit 0

# 5) run
OPT="$(mktemp -t claude-lean)"
printf '{"main": %s, "style": %s, "inbox": %s, "reset_alert": %s, "history_export": %s, "nesting": %s, "start_mode": %s}' \
  "$O_main" "$O_style" "$O_inbox" "$O_reset_alert" "$O_history_export" "$O_nesting" "$O_start_mode" > "$OPT"
export CLAUDE_LEAN_OPTIONS="$OPT"
LOG="$HOME/Library/Logs/claude-lean-install.log"
MAIN="$(echo "$O_main" | tr -d '"')"
IFS=$'\n'; DIRS=($FOLDERS); unset IFS
{
  if [ "$ACT" = "$A2" ]; then bash "$PKG/fresh_start.sh" "${DIRS[@]}" --main "$MAIN"
  else for d in "${DIRS[@]}"; do bash "$PKG/install.sh" "$d" --main "$MAIN"; echo; done; fi
} > "$LOG" 2>&1
RC=$?
rm -f "$OPT"
SUMMARY="$(grep -E "설치 완료|버전|✗|기록 보관|오류|Error" "$LOG" | tail -n 12 | sed 's/"/\\"/g')"
if [ $RC -eq 0 ] && ! grep -q "✗" "$LOG"; then
  alert "완료했습니다. 각 프로젝트 폴더에서 claude를 새로 시작하세요.\n\n$SUMMARY\n\n전체 기록: $LOG"
else
  alert "일부 작업이 실패했습니다.\n\n$SUMMARY\n\n전체 기록: $LOG" caution
fi
