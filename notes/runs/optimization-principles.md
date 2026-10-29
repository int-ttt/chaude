# chaude 최적화 원리 요약

범위: CLAUDE.md, .claude/settings.json, .claude/hooks/*.py (일부 앞부분), .claude/agents/haiku-coder.md, internal/harness/{classifier,repl(rotate/snapshot),drain,handoff,resume,mode}.go 를 읽고 정리했습니다. 읽지 않은 파일(tui.go, history.go, metrics*.go, 나머지 훅·에이전트 전문)은 아래 "한계"에 표시했습니다. 수치는 코드·설정에 적힌 값이며 실제 절감률은 측정하지 않았습니다.

## 1. 구조 요약

- 두 층으로 되어 있습니다.
  - 정책 층(`CLAUDE.md`, `.claude/`): 메인 스레드(Opus)는 듣고 라우팅만 하고, 파일/웹 작업은 전부 서브에이전트가 합니다. 훅(Python)이 이를 강제합니다.
  - 실행 층(`internal/harness/*.go`, 바이너리 `chaude`): `claude` CLI를 자식 프로세스로 띄우는 REPL/TUI입니다. 프롬프트마다 모델을 고르고, 컨텍스트가 커지면 세션을 새로 열고 요약을 넘깁니다.
- 상태는 대화 컨텍스트가 아니라 `notes/`(FACTS, DECISIONS, LEDGER, LESSONS, runs/)에 둡니다. 새 세션이나 압축 후에는 훅이 작은 스냅샷만 주입합니다.

## 2. 핵심 원리별 동작

### 2-1. 비싼 모델의 출력·도구 사용을 막음 (메인 스레드 최소화)
- 동작: Opus는 대기하다가 프롬프트가 오면 위임 한 번, 최종 답 한 번만 합니다. 중간 상태 문구·서술 금지, 답은 몇 문장, 상세는 `notes/runs/`에 쓰고 경로만 줍니다 (CLAUDE.md 1~5, 14, 19행).
- 강제: `main-thread-guard.py`가 메인 스레드의 Read/Write/Edit/Bash/Grep/Glob/Web 도구 호출을 거부합니다(`agent_id`가 있는 서브에이전트 호출은 통과). 결과적으로 파일 내용이 Opus 컨텍스트에 들어오지 않습니다.
- 관련: `.claude/hooks/main-thread-guard.py`, CLAUDE.md.

### 2-2. 작업 난이도별 모델 라우팅 (두 단계)
- 에이전트 단계: 이름이 모델을 결정합니다(`haiku-*`, `sonnet-*`, `opus-*`, 각 `.claude/agents/*.md`의 `model:`). `agent-model-pin.py`가 메인이 넘긴 `model` 인자를 제거해 에이전트 파일의 모델이 항상 이기게 하고, 내장 general-purpose/Explore/Plan은 거부합니다(기본 프롬프트가 크기 때문). 서브에이전트 안에서는 haiku-* 만 호출 가능하여 Sonnet/Opus가 Opus를 다시 부르는 일을 막습니다. 환경변수 `CLAUDE_CODE_SUBAGENT_MODEL=haiku`, 최대 서브에이전트 깊이 2.
- 규칙: 기계적·명세 확정 작업은 Haiku, 판단·다파일은 Sonnet, Sonnet이 `ESCALATE: <경로>`를 반환했거나 사용자가 `opus로`라고 한 경우만 Opus (CLAUDE.md 9~11행).
- 세션 모델 단계(conductor, 기본 켜짐): `classifier.go`가 프롬프트마다 `claude -p --model sonnet`을 한 번 호출합니다. 도구 없음(`--tools ""`), 세션 저장 없음, 작업 디렉터리를 임시 폴더로 해서 프로젝트 CLAUDE.md가 안 실리게 합니다. 출력은 `topic: same|new`, `size: small|large`, `task:` 3줄입니다.
  - `pickModel`: 주제가 same이면 현재 모델 유지, new + small이면 sonnet, new + large면 opus. 메인 세션은 haiku가 되지 않습니다.
  - 사용자 원문 대신 분류기가 만든 간결한 `task:`(존댓말·인사 제거, 경로·숫자 유지, "최종 답 한 번만" 지시 포함)가 메인 세션에 전달됩니다. 입력·출력 토큰을 줄이는 효과를 노린 설계입니다.
  - 확인/슬래시 명령(`isAck`, `/`로 시작)은 분류하지 않고 그대로 보냅니다. 분류기 실패 시에도 원문을 보내고 모델을 바꾸지 않습니다.
  - 모델이 바뀌면 `rotate`(아래 2-3)로 세션을 교체합니다. 기록이 없으면 단순 재시작입니다.
- 관련: `classifier.go`, `.claude/hooks/agent-model-pin.py`, `.claude/agents/*.md`, `notes/runs/model-switch-policy.md`, `main-model-routing.md`.

### 2-3. 세션 회전과 핸드오프 (컨텍스트 상한 관리)
- 트리거: 답변 후 마지막 컨텍스트 토큰이 `--threshold`(기본 270,000) 이상이면 회전합니다. 자식 `claude`에는 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=400000`을 자식에게만 주어, 자동 압축이 회전보다 먼저 일어나지 않게 합니다(`core.go cleanEnv`). 그 밖에 카테고리 전환 마커(`category.go`), 모델 전환 시에도 회전합니다. 실패 후 쿨다운, 자동 회전 횟수 상한 옵션이 있습니다.
- 방식(`repl.go rotate`): 현재 세션에 도구 없이 핸드오프 노트를 쓰게 합니다(Goal, Decisions, Files, In-progress, Open questions, Preferences, Exact values). `handoff.go validateHandoff`가 200자 이상, 7개 섹션 존재, 바뀐 파일명 언급 여부를 검사하고 최대 2회 시도합니다. 실패하면 "검증 실패" 표시와 최근 턴만 담습니다.
- 스냅샷(`buildSnapshot`): 안내 문구 + 핸드오프 노트 + 변경 파일 목록 + 마지막 N턴(각 1,500자로 절단), 전체를 `SnapshotChars`로 절단합니다. 직전 `추천:` 줄은 원문 그대로 앞에 붙여 보존합니다. 새 세션의 첫 사용자 메시지 앞에 붙여 한 번 보냅니다.
- 효과의 원리: 긴 세션은 매 턴 전체 컨텍스트를 다시 읽는 비용이 쌓이므로, 요약만 가진 새 세션으로 바꾸면 이후 턴의 입력 크기가 줄어듭니다. 단, 회전 자체에 핸드오프 생성 호출이 한 번 듭니다.
- 에이전트 진행 중 회전: `drain.go`가 위임된 에이전트가 안 끝났으면 옛 세션을 살려 두고(stdout 소비, 권한 요청 자동 거부) 끝나면 `[agent done] ...` 알림을 다음 메시지에 붙입니다. 타임아웃·동시 개수 상한이 있습니다.
- 관련: `repl.go`, `handoff.go`, `drain.go`, `core.go`.

### 2-4. 노트 기반 상태와 훅이 쓰는 0-토큰 기록
- `ledger.py`(SubagentStop): 에이전트 결과를 `notes/LEDGER.md`에 모델 호출 없이 추가하고, 실제 실행 모델이 이름과 다르면 표시합니다.
- `session-context.py`(SessionStart: startup/resume/compact/clear): notes/의 작은 스냅샷을 주입해, `/clear`·재독 없이 압축 후나 새 세션에서 이어 갑니다. CLAUDE.md는 `/clear`·`/compact`를 권하지 않고 `/exit` 후 `claude`로 재시작하라고 합니다.
- `style-adapter.py`(UserPromptSubmit): "진행상황/끝났어?"는 `logs/*.status`로 모델 호출 없이 답하고, 단순 정보성 발언은 `notes/INBOX.md`에 쌓았다가 다음 지시와 함께 전달합니다(주석 기준).
- `/resume`(`resume.go`): Claude Code 대화 기록(jsonl)을 읽어 턴 요약 스냅샷으로 가져옵니다.
- 장기 지식은 FACTS/DECISIONS(사용자가 확인한 것만)에 두고, 에이전트의 긴 결과는 `notes/runs/`에 파일로 둡니다. 메인 컨텍스트에는 경로만 전달됩니다.

### 2-5. 서브에이전트 컨텍스트 억제 (Haiku 슬라이스 읽기 등)
- 읽기 상한: `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS=8000`, `BASH_MAX_OUTPUT_LENGTH=12000`, `MAX_MCP_OUTPUT_TOKENS=10000`.
- `haiku-context-guard.py`: Haiku 서브에이전트의 `Read(allow_large)` 사용을 거부해 offset/limit 슬라이스나 grep로 읽게 합니다. Sonnet/Opus는 통과합니다.
- 에이전트 정의: Haiku 에이전트는 `omitClaudeMd: true`(프로젝트 CLAUDE.md를 싣지 않음), `autoCompactWindow: 100000`(자동 압축), 반환은 ≤5줄, 긴 작업은 `notes/runs/`에 done/remaining 목록을 남기도록 지시합니다. 2회 실패 시 중단하고 Sonnet→Opus로 상향합니다(반복 시도로 인한 토큰 낭비 방지).
- 큰 작업은 처음부터 병렬·순차 에이전트로 쪼갭니다(CLAUDE.md 18행).
- 메인 설정: `model: opus`, `effortLevel: low`, `autoCompactWindow: 150000`.

### 2-6. 비용·토큰 계측
- `metrics.go`/`metricscmd.go`: 단계별(분류, 회전, 턴) 비용·시간·토큰을 `notes/metrics/*.jsonl`에 기록합니다. 상태줄에 컨텍스트 토큰, 비용, 5시간 사용량이 표시됩니다(`repl.go` 500행 부근, `usagebar.go`).

## 3. 한계 / 불확실한 점

- 절감 효과의 정량 근거 없음: 이 문서는 코드·설정을 읽은 결과이며, 절감률을 측정하거나 metrics jsonl을 분석하지 않았습니다.
- 분류기의 same/new, small/large 판단 정확도는 라이브로 검증되지 않았다고 `notes/runs/model-switch-policy.md`에 적혀 있습니다(`TestClassifierLive` 미실행).
- 분류기 호출 자체가 프롬프트마다 Sonnet 호출 비용·지연을 추가하고, 모델이 바뀔 때마다 회전(핸드오프 생성)이 한 번 듭니다. 순이익은 사용 패턴에 따라 달라지며 측정 전입니다.
- 모델 전환/회전 시 캐시된 프롬프트가 사라져 첫 턴 입력 비용이 커질 수 있습니다. 코드로 확인한 것은 아니고 추정입니다.
- 분류기가 사용자 원문을 요약문으로 바꿔 보내므로 뉘앙스가 손실될 수 있습니다. 이 위험은 코드상 완화책(원문 유지)이 ack/슬래시/실패 경우뿐입니다.
- 읽지 않은 부분: `tui.go`, `history.go`, `importclaude.go`, `metrics*.go`, 나머지 훅(`correction-guard`, `_lean`)과 스킬 전문, 에이전트 파일 중 haiku-coder 외. 위 설명 중 해당 파일에 의존하는 내용은 주석·CLAUDE.md 기준입니다.
- `LEAN_RESET_AT`(`context-alert.py`)은 현재 설정이 0이라 꺼져 있습니다.
- 현재 작업 트리에 미커밋 변경이 있어, 읽은 코드가 마지막 커밋(0.6.11)과 다를 수 있습니다.
