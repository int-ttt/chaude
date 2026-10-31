# chaude v0.8.0 전체 구조 설명과 향후 개선 제안

작성 근거: notes/FACTS.md, notes/DECISIONS.md, 그리고 파일 묶음별로 Haiku 정찰 에이전트가 돌려준 요약입니다. 저는 소스를 직접 열어 보지 않았으므로, 아래 내용 중 "확인 필요"로 표시한 부분은 요약에 근거한 추정입니다.

## 1. 한눈에 보기

chaude는 Go로 만든 단일 실행 파일입니다. 사용자가 터미널에서 실행하면 Claude Code(`claude -p`)를 자식 프로세스로 띄우고, 그 위에서 화면(TUI), 모델 라우팅, 컨텍스트 회전(handoff), 비용 기록을 담당합니다. 이와 별개로 `.claude/` 아래에 Python 훅, 에이전트 정의, 스킬이 있어 Claude Code 자체의 동작을 규율합니다. 즉 구조가 두 층입니다.

- 바깥 층(Go, chaude): 자식 프로세스 관리, 분류, 회전, 예산, 지표, 화면
- 안쪽 층(Claude Code 설정): CLAUDE.md 정책, 훅 9개, 에이전트 11개, 스킬 7개, notes/ 상태 파일

## 2. 패키지별 역할

### cmd/chaude
진입점입니다. 버전은 0.8.0이고 빌드할 때 ldflags로 덮어쓸 수 있습니다. 인자가 없거나 `repl`, `--from-claude`, `--import-dry-run`, 또는 `-`로 시작하는 인자면 `internal/harness`로 넘기고, `metrics`는 지표 요약기로, `gen-settings`는 settings 패키지로, 그 외 이름은 scripts 레지스트리에 있으면 거기로 보냅니다. 알 수 없는 명령은 종료 코드 2입니다. 도움말에 `gen-settings`의 `--out`만 적혀 있고 실제로는 `--src`, `--python`도 받는 점은 사소한 불일치입니다.

### internal/osenv
OS별 차이를 가두는 유일한 곳입니다. `claude` 실행 파일 경로를 `LookPath`로 찾고(Windows는 `.cmd/.exe`), 상태 디렉터리를 정합니다. Windows는 `%LOCALAPPDATA%\claude-harness`, mac은 호환을 위해 `~/.claude-harness`, Linux는 `$XDG_STATE_HOME` 또는 `~/.local/state/claude-harness`입니다.

### internal/settings
`.claude/settings.json`은 posix 형태가 원본이고, `gen-settings windows`가 훅 명령을 `py -3` 같은 인터프리터 + 스크립트 인자 형태로 바꿔 줍니다. mac/linux는 원문을 그대로 통과시키고, 결과는 JSON 유효성을 검사한 뒤 출력합니다. `Bash(bin/chaude ...)` 허용 항목은 `PowerShell(bin/chaude.exe ...)`로 함께 복제합니다.

### internal/lean
플래그와 위치 인자가 섞여 있어도 파싱해 주는 작은 도우미(`ParseInterspersed`)입니다. scripts 쪽 명령들이 사용합니다.

### internal/scripts
예전 Python 보조 스크립트를 Go로 옮긴 서브커맨드 모음입니다. `register/Has/Run` 레지스트리로 7개 안팎의 명령을 디스패치하고, `common.go`에 Python 호환 문자열/JSON 도우미가 있습니다.
- claude-history: 현재 프로젝트의 Claude 대화 기록 폴더(`~/.claude/projects/...`)를 찾습니다. 폴더 이름 규칙으로 먼저 찾고, 실패하면 jsonl 앞부분의 `cwd` 값을 비교합니다.
- export-history: 세션 jsonl에서 사용자/어시스턴트 텍스트만 뽑아 `logs/history/`에 날짜-세션별 txt로 저장합니다(기본 500KB 단위 분할, INDEX 생성).
- search-history: 현재 기록과 `~/claude-archive/` 아래 jsonl, jsonl.gz에서 검색어가 모두 포함된 줄을 찾아 최신순으로 보여 줍니다(기본 12건).
- extract-transcript: 대화를 7,000자 단위 chunk로 잘라 `notes/distill/chunks/`에 씁니다. distill 스킬의 입력입니다.
- compress-archive: jsonl을 gzip하고 원본을 지웁니다.
- charcount: 글자 수 세기(`--section`, `--limit`). 글자 수 제한이 있는 문서 검토용입니다.
- usage-report: 어시스턴트 usage를 message id로 중복 제거해 모델군별 요청 수, 토큰, 비용을 합산합니다(서브에이전트 기록 포함).

### internal/harness (핵심)
파일이 30개가 넘어서 기능별로 묶어 설명합니다. 아래 3~8절에서 흐름 중심으로 풀어 씁니다.

| 묶음 | 파일 | 역할 |
|---|---|---|
| 세션/루프 | core, repl, dispatch | `claude -p` 래퍼(Session), 대화 루프와 회전 판정, 자식 재사용 판단 |
| 자식 프로세스 | children, livechild, switchchild, clients, haikupool | 모델별 대기 자식 보관, 전환, 재개, 등록부 |
| 회전 | handoff, resume, drain, importclaude | 인계 노트 검증, Claude 기록 가져오기, 이전 자식 정리 |
| 라우팅 | classifier, category, mode | 프롬프트 분류, 카테고리 전환 표식, 권한 모드 |
| 비용/지표 | budget, metrics, metricscmd | 작업별 예산, 턴별 JSONL, 요약 |
| 유휴 | idlesweep | 캐시가 식기 전에 대기 자식 정리/회전 |
| 화면 | tui, input, usagebar, ratestatus, agentview, agentstore, history, localcmd | bubbletea TUI, 한글 폭 계산 입력창, 사용량 바, 서브에이전트 보기, 입력 히스토리 |

## 3. 프로세스와 자식 모델

chaude 하나가 부모이고, `claude -p` 자식이 (작업 디렉터리, 모델) 쌍마다 최대 하나씩 존재할 수 있습니다. 지금 화면과 대화하는 자식이 "활성 슬롯"이고, 나머지는 "대기(parked)" 상태로 살아 있습니다. 사용자가 다른 모델 쪽으로 넘어가면 `switchChild`가 대기 중인 자식이 있으면 꺼내서 활성으로 붙이고, 없으면 새로 띄웁니다(필요하면 스냅샷이나 resume 포함). 떠나는 자식은 한 번도 API 응답을 받지 않았으면 바로 닫고, 아니면 대기 보관합니다. 같은 키의 이전 자식은 폐기하는데, 위임 에이전트가 아직 돌고 있으면 닫지 않고 drain으로 넘깁니다.

`clients.json`(`~/.chaude/clients.json`)은 이 자식들의 장부입니다. 행은 cwd+모델 키이고 상태는 live, parked, expired입니다. 다른 chaude 터미널이 살아 있는 PID로 같은 행을 잡고 있으면 가로채지 않습니다. 새 자식이 필요할 때 `pickChild`는 (1) 살아 있고 캐시가 따뜻한 자식 (2) 장부의 따뜻한 parked 행을 resume (3) 새로 시작 순서로 고릅니다. Haiku 작업자는 `DefaultMaxWorkers=3`으로 상한이 있고, 파일을 고친 Haiku 세션은 `haikupool`에 최대 5개까지 기억해 후속 수정 때 재사용 후보로 삼습니다.

## 4. 분류기와 라우팅

메인 세션은 Haiku가 기본입니다(비용 때문에 Opus는 설계 턴에만 쓰는 정책). 사용자가 프롬프트를 입력하면 `route()`가 먼저 별도의 일회성 `claude -p` 분류 호출을 합니다(도구 없음, 임시 디렉터리에서 실행). 응답은 5줄 형식으로, topic(same/new), tier(haiku/sonnet/opus), kind(lookup/edit/other), worker, task입니다. task가 비어 있거나 topic이 이상하면 분류 실패로 보고 원문을 그대로 보냅니다. tier가 없으면 옛 크기 태그로 대체하고 없으면 sonnet입니다.

그 다음 `pickModel`이 규칙을 적용합니다. 현재 모델이 비어 있거나 haiku면 haiku 유지, opus면 opus 유지(새 주제이면서 haiku급이면 위임 힌트), sonnet이면 유지합니다. 큰 작업이면 task 앞에 힌트를 붙입니다. tier가 opus면 "Haiku가 계획을 쓰고 opus-architect/sonnet-analyst가 검토한 뒤 Haiku가 실행", sonnet이면 sonnet 분석 힌트입니다. ㅇㅇ 같은 확인 응답과 `/`로 시작하는 명령은 분류를 건너뜁니다.

분류 모델은 환경변수 `CHAUDE_ROUTER_MODEL`로 바꿀 수 있고 기본값은 아직 sonnet입니다. Haiku용 프롬프트는 튜닝이 끝났지만 기본값 전환은 코드 주석상 "사용자 승인 대기"입니다.

별도로 카테고리 회전이 있습니다. 모델이 답변 끝에 `[cat: X]` 또는 `[cat-switch: X]`를 붙이고(규칙은 500토큰 이하의 시스템 프롬프트 접미사), chaude는 스트림에서 이 줄을 숨깁니다. `cat-switch`이면서 확인 응답이 아니고 라벨이 직전과 다를 때만 그 턴 이후 회전합니다. `--no-category-rotate`로 끌 수 있습니다.

권한 모드(`mode.go`)는 default, acceptEdits, plan, auto 네 가지를 순환합니다. auto는 모두 허용, acceptEdits는 편집 도구만 허용, plan은 읽기 도구만 허용, default는 매번 묻습니다.

## 5. 회전(handoff)과 재개

회전 트리거는 세 가지입니다. (1) 한 턴이 끝난 뒤 컨텍스트가 임계값 이상: Haiku 메인은 80k, Opus와 Sonnet 메인은 270k(결정 기록 기준). 코드에서는 `opusSonnetThreshold=270000`, 그 외는 `--threshold` 값입니다. (2) 카테고리 전환(쿨다운 적용). (3) 수동 `/rotate`, 유휴 `/idle-rotate`.

회전 절차는 이렇습니다. 낡은 자식에게 인계 노트를 쓰게 하는 턴을 최대 2번 시도하고, 노트가 200자 이상인지, 목표/결정/파일/진행 중/열린 질문/선호/정확한 값 섹션이 있는지, 이번에 바뀐 파일 이름이 모두 들어갔는지 검증합니다. 직전 답변의 "추천:" 줄은 별도로 보존해 새 세션 첫 메시지에 원문 그대로 앞붙임하고, 보존 여부를 byte 단위로 확인해 지표에 남깁니다. 검증에 실패하면 노트를 실패 안내문으로 바꾸고 "invalid"로 기록합니다. 노트도 턴도 없으면 스냅샷을 보류합니다.

위임 에이전트가 아직 실행 중이면 낡은 자식을 닫지 않고 drain 목록에 넣습니다(최대 개수 초과 시 가장 오래된 것 폐기). drain 중인 자식의 권한 요청은 자동 거절하고, 에이전트가 모두 끝나거나 타임아웃이 되면 닫으며, 완료 알림은 다음 사용자 입력 앞에 붙습니다. 종료할 때 전부 닫습니다.

`/resume`은 Claude Code가 저장한 세션 jsonl(`~/.claude/projects/...`)을 읽어 턴과 수정 파일을 뽑아 스냅샷으로 만들어 새 세션에 주입합니다. `--from-claude`, `--import-dry-run`이 같은 경로입니다. 지금은 Claude 출처만 받습니다.

## 6. 예산과 유휴 정리

예산(`budget.go`)은 작업 단위의 메모리 상 USD 장부입니다. 기본값은 꺼짐(`--task-budget-usd 0`)이고, 0보다 크게 주면 켜지며 같은 값을 `claude --max-budget-usd`에도 넘깁니다. `charge`는 직전 누적값과의 차이만 더하고(값이 줄면 새 세션으로 보고 전체를 더함), 분류기와 `/usage` 같은 일회성 호출은 `chargeOneShot`으로 더합니다. `allow`는 이미 한도를 넘었거나 (현재 지출 + 추정치)가 한도를 넘으면 거부하고, `over`는 진행 중인 턴의 중간 사용량까지 더해 한도를 넘었는지 보고 넘으면 Interrupt합니다. 추정치는 가격표에 출력 4,000토큰을 더한 값입니다. 장부는 작업이 바뀔 때 초기화되고 디스크에 저장되지 않습니다.

유휴 정리(`idlesweep.go`)는 프롬프트 캐시가 식기 전에 정리하려는 장치입니다. `--idle-rotate` 기본값은 3599초입니다. 캐시 TTL이 1시간으로 관측되면 이 값을, 5분이거나 아직 모르는 `auto`면 `min(290초, 값)`을 마감으로 씁니다. 값이 0이면 끕니다. TUI가 바쁘지 않고 권한 질문도 없을 때 `sweepIdle`을 호출하면, 대기 자식 중 마감을 넘기고 위임 에이전트가 없는 것은 닫고 expired로 표시하며, 활성 자식은 마지막 API 시각이 마감을 넘으면 값당 한 번 `/idle-rotate`를 보냅니다.

## 7. 화면과 입력

bubbletea/lipgloss TUI입니다. 환영 상자, 스크롤 영역, 테두리 입력창, 상태줄이 있고 파이프 입력이나 `HARNESS_PLAIN=1`이면 일반 REPL로 돌아갑니다. 입력창(`input.go`)은 go-runewidth로 글자 폭을 재서 한글을 정확히 처리하고 여러 줄 입력을 지원합니다. `/usage`는 생각 중에도 별도 `claude -p /usage`를 한 번 실행해 결과 팝업에 뜨고, 사용량 퍼센트는 claude 출력 그대로이며 chaude가 계산하는 것은 "Context (now)" 바뿐입니다(ctx를 270,000으로 나눔). `ratestatus`는 5시간 창 사용률(1 미만이면 100 곱함)과 리셋 시각을 `HH:MM/HH:MM · N%`로 보여 줄 뿐 토큰을 세지 않습니다. 로컬에서 처리하는 슬래시 명령은 `/help`, `/status` 정도이고 나머지는 자식에게 그대로 전달됩니다. 서브에이전트 목록은 `agentstore`가 저장/복원하며 재시작 때 실행 중이던 것은 interrupted로 표시합니다.

## 8. 지표(metrics)가 세는 것

`notes/metrics/<UTC 날짜>.jsonl`에 한 줄씩 기록합니다(경로 기본값은 cwd 기준 `notes/metrics`, 빈 문자열이면 끔). 모든 줄에 `ts`(UTC)와 실행 플래그가 붙습니다. stage별로 다음과 같습니다.

- turn: 시작/끝 시각, 모델, 컨텍스트 크기(`ctx`), 비용(`cost_usd`), 소요 시간, 회전 횟수, 위임 횟수, 그리고 입력/출력/캐시 읽기/캐시 생성 토큰. 포그라운드와 백그라운드 턴을 모두 기록합니다.
- rotate: 사유, 이전 세션, 인계 노트 유효 여부, 추천 보존 여부, 시도 횟수, 결과(drained 또는 closed), 회전 시점 토큰.
- classify: 분류 호출 비용과 모델. budget: 예산 초과 중단. category: 카테고리 전환. import: 가져오기. design_path: 설계 경로 선택.

`chaude metrics`는 이 중 turn, rotate, import만 요약합니다. 턴 수, 회전 수(유효/무효 구분), 가져오기 수, 위임 수(턴의 위임 합), 토큰 합(`result_*` 필드, 없으면 단순 키), 비용 합, 총 소요 시간(없으면 끝-시작)입니다. classify, budget, category, design_path는 요약에 나오지 않으므로 라우터 비용은 현재 요약 숫자에 포함되지 않습니다. `usage-report`는 이와 별개로 Claude 세션 기록(subagents 포함)을 message id로 중복 제거해 모델군별로 집계합니다.

## 9. 훅(.claude/hooks, Python)

stdlib만 쓰고 Python 3.8 이상입니다. settings.json이 이벤트에 다음과 같이 연결합니다.

- SessionStart: session-context.py가 notes/ 스냅샷과 이전 세션 요약을 주입하고 업데이트 알림을 냅니다.
- UserPromptSubmit(3종): style-adapter.py는 모드 전환, 상태 조회, 메모 저장을 모델 없이 처리하고 그 외에는 `[ack]`/`[frag]` 태그를 주입합니다. correction-guard.py는 정정 유형별 안내를 넣고 연속 오해가 2회 이상이면 체크리스트를 강제합니다. context-alert.py는 컨텍스트 경고용으로 보이나 요약에서 확인하지 못했습니다.
- PreToolUse: Agent/Task는 agent-model-pin.py가 model 인자를 제거하고 내장 general-purpose/Explore/Plan을 막습니다. 일반 도구는 main-thread-guard.py가 메인 세션의 작업 도구(Read, Write, Bash 등)를 막고 서브에이전트만 통과시킵니다. Read는 haiku-context-guard.py가 Haiku 에이전트의 `allow_large` 호출을 거부합니다.
- SubagentStop: ledger.py가 서브에이전트 마지막 3줄을 `notes/LEDGER.md`에 `[에이전트·모델]` 형식으로 붙이고 최근 300건만 유지합니다(핑 입력은 건너뜀).
- SessionEnd, PreCompact: history-export.py가 `bin/chaude export-history`를 호출해 대화를 `logs/history/`에 저장합니다.

`_lean.py`는 훅 공용 도우미(모드, 핑 판정)입니다.

## 10. 에이전트와 스킬

에이전트 11개: opus-architect(설계, effort high, 구현은 약 1파일/50줄까지만 직접, 나머지는 계획 후 haiku-coder에 위임), sonnet-analyst(분석, medium, 질문 답변은 answer 파일로, ESCALATE 규칙 보유), sonnet-reviewer(읽기 전용 검토, PASS/FIX), haiku-scout(검색/조회, high), haiku-researcher(웹 조사, high), haiku-coder(코드, medium), haiku-checker(검증, low), 그리고 haiku-drafter, haiku-extractor, haiku-batch-worker, haiku-runner(요약에서 세부 설정은 확인하지 못함). 모두 끝에 "결과 요약 몇 줄만 반환, 상세는 notes/runs/"를 지킵니다.

스킬 7개: research(웹 조사를 출처 표로 `notes/research/`에), correction(정정을 A~G로 분류해 델타만 고치고 F형만 DECISIONS에 한 줄), distill(옛 대화를 FACTS/DECISIONS/DELIVERABLES/OPEN/LESSONS로 정리), status(`logs/*.status` 진행률 보고), fact(FACTS 갱신 후 옛 사실이 쓰인 문서 전체 치환), check(변경 검증, 표는 `notes/runs/`), migrate-session(새 세션 이동용 FACTS/DECISIONS/SESSION_HANDOFF 작성).

## 11. notes/ 상태 흐름과 CLAUDE.md 정책

CLAUDE.md의 핵심은 "메인은 라우팅만 하고 작업은 서브에이전트가 한다"입니다. 검토는 sonnet-reviewer, 분석은 sonnet-analyst, 조회는 haiku, 설계만 opus입니다. 상태는 notes/ 아래에 쌓입니다.

- FACTS.md, DECISIONS.md: 사용자가 확인한 사실과 규칙(한 줄씩). 세션 시작 훅이 읽어 줍니다.
- LEDGER.md: 서브에이전트 완료 기록(ledger 훅).
- runs/: 에이전트 상세 결과와 계획 문서. 메인은 경로만 받습니다.
- metrics/: 턴별 JSONL.
- research/: 웹 조사 결과. distill/: 옛 대화 chunk.

흐름: 사용자 입력 -> (분류) -> 자식 세션 -> 서브에이전트 -> 상세는 runs/, 요약 3줄은 LEDGER, 턴 숫자는 metrics -> 회전 시 인계 노트가 새 세션에 주입 -> 세션 종료 시 history-export가 대화 보관.

## 12. 릴리스 파이프라인

`v*` 태그 push로 GitHub Actions(`release.yml`)가 ubuntu-latest 단일 잡으로 돕니다. `scripts/build-release.sh`가 대상별로 `CGO_ENABLED=0` 빌드 후 `dist/chaude_<ver>_<os>_<arch>.tar.gz`와 `checksums.txt`를 만듭니다. 직전 태그 이후 `git log --no-merges`로 `dist/notes.md`(이전 버전, 커밋 목록, 비교 링크)를 만들고 `gh release create`로 올립니다. 로컬 설치는 `install.sh/.ps1`가 프로젝트 안 `.tools/go`에 Go를 받아 빌드합니다.

## 13. 불확실한 점과 앞뒤가 안 맞는 부분

근거 수준은 모두 "요약 + 결정 기록"입니다. 소스 직접 확인은 아닙니다.

1. 메인 모델 정책이 기록마다 다릅니다. 결정 기록에는 "Opus 메인은 설계 턴에만"과 "Haiku 메인 고정"과 "Opus/Sonnet 메인은 270k, Haiku만 80k"가 함께 있습니다. 코드에서 Haiku 임계가 정말 80k인지는 확인하지 못했습니다(코드는 270k 상수와 `--threshold`만 확인).
2. 분류기 기본 모델이 sonnet입니다. Haiku 프롬프트가 튜닝되어 있는데도 기본값이 안 바뀌어 있어, 분류 호출 비용이 Sonnet 가격으로 나갑니다. 전환 승인이 대기 중입니다.
3. `route()`의 `router_prompt` 로그 필드에 프롬프트 대신 `routerModel()` 값이 들어갑니다(복사 실수로 보임). `pickTarget`의 `kind` 인자는 쓰이지 않습니다.
4. 사용량 바의 분모가 270,000 고정입니다. Haiku 메인(80k 임계)일 때는 의미가 어긋납니다.
5. 지표 요약기가 classify 비용을 합산하지 않아 총비용이 실제보다 작게 보입니다. 예산 장부는 저장되지 않아 재시작하면 사라집니다.
6. 이전 계획(훅을 `--settings`로 메인에만 주입, CLAUDE.md를 main.md로 개명)과 현재 상태(settings.json에 훅이 그대로 있고 CLAUDE.md 유지)가 달라 보입니다. 어느 쪽이 최종인지 결정 기록이 말해 주지 않습니다.
7. history-export 훅이 `bin/chaude`를 부르는데, 결정 기록의 옛 문구는 `bin/harness`입니다. 이름 변경이 모든 곳에 반영됐는지 확인이 필요합니다.
8. search-history는 검색어가 비면 panic이 나는 코드가 있고(요약 기준 한 줄), `.jsonl.gz`를 실제로 해제해 읽는지 요약만으로는 확인되지 않았습니다.
9. 릴리스 워크플로는 TARGETS를 지정하지 않으면 darwin/arm64만 빌드합니다(스크립트 기본값). 윈도/리눅스 산출물이 실제로 올라가는지는 실행 이력으로 확인하지 못했습니다.
10. 유휴 정리에서 `reconcile`의 호출 위치, 드레인 타임아웃과 최대 개수 기본값, 캐시 TTL이 정말 1시간으로 관측되는지는 확인하지 못했습니다. 결정 기록도 "미검증"이라고 적고 있습니다.
11. FACTS.md에는 가격표뿐이고 환경/제약 사실은 DECISIONS에 섞여 있습니다. 가격은 2026-10-29 기준이라 바뀌면 `budget.go`의 가격표가 어긋납니다.

## 14. 향후 개선 제안 (우선순위 순)

1. 지표와 비용 집계 정리 (classify, budget 포함, 총비용 한 줄). 이유: 지금은 라우터 비용이 요약에 안 잡혀서, "라우팅이 절약인가"를 숫자로 판단할 수 없습니다. 위험: 낮음. 기존 JSONL 필드가 이미 있어 요약기만 고치면 됩니다. 과거 파일과 필드 이름이 달라 일부 집계가 틀릴 수 있어 테스트가 필요합니다.
2. 정책 단일화: 메인 모델 정책, 임계값(80k/270k), 분류기 기본 모델을 한 곳(설정 구조체와 문서)에 모으고, 상태줄 분모와 사용량 바도 모델별 임계를 따르게 하기. 이유: 문서, 결정 기록, 코드가 서로 달라 사용자가 실제 동작을 예측하기 어렵습니다. 위험: 중간. 동작이 바뀌는 항목(분류기 Haiku 전환)은 사용자 승인 후에만 하고, 실제 30건 샘플 비교 결과를 근거로 삼아야 합니다.
3. 회전 품질 계측: 인계 노트 invalid 비율, 추천 보존율, 회전 후 첫 턴 캐시 생성 토큰을 요약에 추가. 이유: 회전은 비용을 아끼는 대신 맥락 손실 위험이 있는데 지금은 이를 한눈에 볼 수 없습니다. 위험: 낮음. 다만 지표만으로는 "품질"을 대신 못 하므로 사용자 체감 평가와 병행해야 합니다.
4. 예산 장부 영속화와 경고 단계 추가 (기본은 계속 꺼짐). 이유: 재시작하면 누적이 사라져 한도의 의미가 약합니다. 위험: 중간. 파일 동시 쓰기(여러 터미널)와 가격표 노후화로 잘못 차단할 수 있어, 처음에는 차단 말고 경고만 권합니다.
5. scripts의 방어 코드 보강: 빈 검색어 panic 제거, gz 검색 동작 테스트 추가, 빌드 대상(OS 3종) 릴리스 워크플로 명시와 산출물 검증 단계. 이유: 작고 확실한 결함이고 Windows 사용이 설계 전제입니다. 위험: 낮음. 교차 빌드 산출물은 실제 OS에서 실행해 보기 전에는 동작이 검증되지 않습니다.
6. 훅 구조 결정 확정: settings.json 훅 유지 vs `--settings` 메인 전용 주입. 이유: 워커(`claude -p` 자식)에게도 같은 훅이 걸리는지에 따라 규칙 위반 방지와 오작동 범위가 달라집니다. 위험: 높음. 인증 방식(subscription)에서 `--bare`가 안 되는 제약, 훅 실패가 비차단이라는 점, 서브에이전트가 CLAUDE.md를 잃을 수 있다는 점이 열려 있어, 실험 결과가 나온 뒤에 진행해야 합니다.
7. 유휴 정리와 캐시 TTL 실측 후 기본값 확정 (3599초 대 290초). 이유: 현재 `auto`는 관측 전까지 보수적으로 290초라 회전이 잦을 수 있습니다. 위험: 중간. 요금제나 상황에 따라 TTL이 바뀌면 캐시가 식은 채로 회전하여 비용이 오히려 늘 수 있습니다.

## 15. 요약

- 구조는 "Go 바깥 층(프로세스, 라우팅, 회전, 지표) + Claude Code 안쪽 층(정책, 훅, 에이전트, notes)"입니다.
- 가장 큰 리스크는 기능 부족이 아니라 문서, 결정 기록, 코드 사이의 불일치와 지표 사각지대입니다. 그래서 제안 1, 2를 먼저 권합니다.
