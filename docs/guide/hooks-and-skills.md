# 훅과 스킬

이 문서는 `.claude/hooks/`의 훅 스크립트와 `.claude/skills/`의 스킬을 설명합니다. 훅은 `.claude/settings.json`에 등록된 순서대로 동작합니다.

훅은 모두 Python 3 표준 라이브러리만 씁니다. Python 3.8 이상에서 돌아갑니다. macOS와 Linux에서는 `python3 "$CLAUDE_PROJECT_DIR/.claude/hooks/X.py"` 형식으로 등록됩니다. Windows에서는 `harness gen-settings windows`가 실행 형식으로 만듭니다.

## 1. 훅 목록

훅 스크립트는 10개입니다. 여기에 공용 라이브러리 `_lean.py`가 하나 있습니다. `_lean.py`는 직접 등록되지 않고, 다른 훅이 가져다 씁니다. 절약 모드가 켜져 있는지 확인하는 함수를 가집니다.

| 훅 | 이벤트 | 하는 일 |
|---|---|---|
| correction-guard.py | UserPromptSubmit | 사용자의 정정 유형을 나누고, 같은 실수가 두 번 이어지면 점검표를 강제합니다 |
| style-adapter.py | UserPromptSubmit | 일반모드와 절약모드를 바꾸고, 상태 질문에 로그를 읽어 답하며, 짧은 정보는 INBOX에 쌓습니다 |
| agent-model-pin.py | PreToolUse (Agent, Task) | 에이전트 호출을 검사합니다. 모델 값을 지우고, 메인 전용 에이전트를 막고, Opus 에이전트가 `haiku-*`만 부르게 합니다 |
| main-thread-guard.py | PreToolUse (Read, Write, Edit, MultiEdit, NotebookEdit, Bash, PowerShell, Grep, Glob, WebSearch, WebFetch) | Opus 메인이 파일, 웹, Bash를 직접 쓰면 막고, 맞는 `haiku-*`를 알려 줍니다 |
| haiku-context-guard.py | PreToolUse (Read, Write, Edit, MultiEdit, NotebookEdit, Grep, Glob, Bash, PowerShell) | 비 Haiku 에이전트의 파일과 셸 접근을 막습니다. 검사 중 오류가 나면 막는 쪽으로 실패합니다 |
| caveman-format.py | PostToolUse (Agent, Task) | 에이전트 반환이 caveman 형식의 필수 항목을 갖췄는지 기록합니다. 막지는 않습니다 |
| ledger.py | SubagentStop | 에이전트 결과를 `record/LEDGER.md`에 한 줄씩 적습니다. 이름과 실제 모델이 다르면 표시합니다 |
| session-context.py | SessionStart (startup, resume, compact, clear) | 시작할 때 `notes/OPEN.md`, `record/LEDGER.md` 끝부분, `record/INBOX.md` 끝부분을 요약해 보여 줍니다 |
| history-export.py | SessionEnd, PreCompact | `bin/chaude export-history`를 실행해 대화 기록을 저장합니다. 실패해도 세션을 막지 않습니다 |

`haiku-context-guard.py`가 막을 때의 안내는 이렇습니다. 파일은 `haiku-scout`(읽기), `haiku-batch-worker`(쓰기), `haiku-coder`(코드)에게 맡기라는 내용입니다.

`main-thread-guard.py`가 막을 때는 직접 작업을 원하시면 "일반모드"라고 입력하라는 안내가 나옵니다. `CLAUDE_ALLOW_MAIN_TOOLS=1`이나 `CHAUDE_ORCH=e`가 설정되어 있으면 이 훅은 통과시킵니다.

## 2. 훅 등록 요약

`.claude/settings.json`에 등록된 값은 다음과 같습니다.

- `UserPromptSubmit`: correction-guard, style-adapter (각 5초)
- `PreToolUse` (Agent|Task): agent-model-pin (5초)
- `PreToolUse` (도구 목록): main-thread-guard (5초)
- `PreToolUse` (파일과 셸 도구): haiku-context-guard (5초)
- `PostToolUse` (Agent|Task): caveman-format (5초)
- `SubagentStop`: ledger (5초)
- `SessionStart` (startup|resume|compact|clear): session-context (5초)
- `SessionEnd`: history-export (60초)
- `PreCompact`: history-export (60초)

## 3. 정정 유형

`correction-guard.py`는 사용자의 말을 여섯 가지로 나눕니다.

- **C (사실 오류)**: 사실이 틀렸다는 정정입니다.
- **D (빠진 변경)**: 바꿔야 할 것을 빠뜨렸다는 정정입니다.
- **F (방향)**: 방향이 틀렸다는 정정입니다.
- **A (잘못 읽음)**: 요청을 잘못 이해했다는 정정입니다.
- **E (반박)**: 내 답에 반대한다는 말입니다.
- **B (질문)**: 확인을 묻는 말입니다.

잘못 읽음이 두 번 이어지면 점검표를 강제합니다. 기록은 `logs/.corr-<세션 ID>`에 남습니다.

## 4. 스킬 목록

스킬은 7개입니다. 대부분은 `context: fork`로 돌고, 지정된 에이전트에 일을 넘깁니다.

| 스킬 | 호출 말 | 하는 일 | 에이전트 |
|---|---|---|---|
| check | 바꾼거 맞아?, 했어?, 안 바뀐 것 같은데 | 바꾼 내용이 맞는지 확인합니다. 인수는 "<무엇을 바꿨나> [대상 경로]" 형식입니다 | haiku-checker (fork) |
| correction | 아니, 근데, 아, 그게 아니라, 아니면, 그냥, -니깐, -라고 | 정정 유형 A부터 E까지를 나눠 맞는 스킬이나 에이전트로 보냅니다 | 없음 (본문에서 /fact, /check, haiku-scout, haiku-researcher 등으로 보냄) |
| distill | 긴 대화를 정리할 때 (명시 호출) | 대화를 조각으로 나누고, `haiku-extractor`로 뽑은 뒤 `haiku-batch-worker`로 합칩니다. 에이전트는 조각 4개당 1개씩, 최대 20개입니다 | 없음 (본문에서 haiku-extractor, haiku-batch-worker 호출) |
| fact | "A가 아니라 B야" | 사실을 바꾸고, 바꿀 파일 전체에 옮깁니다. 인수는 "<이전> → <새> [맥락]" 형식입니다 | haiku-batch-worker (fork) |
| migrate-session | 대화 컨텍스트가 500K 이상일 때 | 긴 세션을 인계 파일로 옮깁니다. 디스크 작업은 Haiku가 합니다 | 없음 |
| research | 웹 조사가 필요할 때 (`/research <질문>`) | 웹에서 자료를 찾고 출처를 붙입니다 | haiku-researcher (fork) |
| status | 진행상황, 끝났어?, 상태 | `logs/*.status`를 읽어 작업 진행 상황을 보여 줍니다 | haiku-runner (fork) |

`fact`는 `notes/FACTS.md`를 고칩니다. `migrate-session`은 `notes/DECISIONS.md`와 `notes/FACTS.md`를 읽고, 인계 파일을 `SESSION_HANDOFF.md` 형식으로 씁니다.

## 5. 상태 파일과 훅의 관계

훅은 `record/`와 `logs/`에 기록을 남기고, 스킬은 `notes/`를 고칩니다. 두 쪽 모두 세션이 바뀌어도 이어갈 수 있도록 파일에 남깁니다. 자세한 경로는 `models-and-agents.md`의 notes/ 절을 보십시오.
