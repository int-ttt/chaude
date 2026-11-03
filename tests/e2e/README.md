# e2e: 하네스 end-to-end 테스트

하네스(루트 `CLAUDE.md` + `.claude/` 에이전트·훅)를 빈 임시 작업공간에 복사한 뒤,
`claude -p` 로 "조금 어려운" 과제 5개를 풀게 하고 결과를 채점한다.

- 과제 1개당 채점 두 가지
  1. **check**: 숨겨진 검사기(`tasks/<t>/check.py`)가 산출물을 실행·검증해 PASS/FAIL 판정
  2. **answer**: 최종 답변이 비어 있지 않고, 한국어이며, 만든 파일/용어와 실행 방법을 언급하는지 휴리스틱 확인
     (`E2E_JUDGE=1` 이면 haiku 판정 `judge_prompt.md` 를 추가로 돌린다)
- 과제마다 비용(`total_cost_usd`), 토큰(입력/출력/캐시), 턴 수, 소요 시간을 기록한다.

## 구성

```
tests/e2e/
  run.sh              러너 (작업공간 생성 -> claude -p -> 채점 -> 요약)
  score.py            채점기 (task / summary 서브커맨드, python3 -I 로 실행)
  judge_prompt.md     haiku 판정 프롬프트 ({{TASK}}, {{REPLY}} 치환)
  tasks/<t>/
    prompt.ko.txt     과제 원문 (사용자가 쓰는 말투의 한국어)
    check.py          숨겨진 검사기 (작업공간 밖에 있음, 에이전트에게 보이지 않음)
    meta.json         답변 휴리스틱용 산출물 파일명·용어
  results/<stamp>/    실행 결과 (gitignore 됨)
    <t>.json          claude -p 원본 출력
    <t>.err           claude stderr
    <t>.score.json    과제별 채점 결과
    summary.json      전체 합계
```

과제: `lru_ttl`(Python, TTL+LRU 캐시), `c_brackets`(C, 괄호 검사, 긴 입력),
`go_wordfreq`(Go, 단어 빈도 CLI, 고루틴), `py_intervals`(Python, 구간 병합),
`js_roman`(Node.js, 로마 숫자 변환·검증).

## 실행

필요한 것: `claude` CLI(로그인 또는 `ANTHROPIC_API_KEY`), `perl`, `python3`(3.9 이상),
과제별 툴체인(`gcc`, `go`, `node`).

```bash
bash tests/e2e/run.sh                          # 과제 5개 전부
TASKS="lru_ttl js_roman" bash tests/e2e/run.sh  # 일부만
E2E_JUDGE=1 bash tests/e2e/run.sh              # haiku 판정 추가
```

종료코드: 0 = 전부 PASS, 1 = FAIL 이 하나 이상 또는 과제 누락, 2 = 전제조건(claude/perl) 없음.
표와 함께 `tests/e2e/results/<stamp>/summary.json` 이 남는다.

## 환경변수

| 변수 | 기본값 | 뜻 |
|---|---|---|
| `TASKS` | `lru_ttl c_brackets go_wordfreq py_intervals js_roman` | 실행할 과제 목록 |
| `E2E_MAX_BUDGET` | `2` | 과제당 `--max-budget-usd` (달러) |
| `E2E_TIMEOUT` | `900` | 과제당 제한 시간 (초), perl alarm 으로 강제 종료 |
| `E2E_OUT` | `tests/e2e/results/<yymmdd-HHMMSS>` | 결과 폴더 |
| `E2E_JUDGE` | 미설정 | `1` 이면 haiku 판정 수행 |

## 비용

- 과제 하나는 하네스 특성상 서브에이전트를 여럿 부르므로 수십 턴이 될 수 있다.
  과제당 상한은 `E2E_MAX_BUDGET` 으로 막는다. 전체 상한은 대략 `과제 수 × E2E_MAX_BUDGET`.
- 실행 한 번이 실제 API 비용을 쓴다. CI 에서는 `workflow_dispatch` 로만 돈다.
- `summary.json` 의 `cost_usd` 와 `tokens` 로 실제 사용량을 확인한다.

## 주의

- 과제 실행은 `--permission-mode bypassPermissions` 로 돈다. 작업공간은 빈 임시 폴더이고
  하네스 파일(CLAUDE.md, .claude)만 들어 있지만, 에이전트가 그 폴더 안에서 명령을 임의로 실행할 수 있다.
  신뢰하지 않는 저장소에 대해 돌리지 말 것.
- 작업공간(`$TMPDIR/e2e-<task>.XXXXXX`)은 실행 후에도 지우지 않는다. 검사 실패 원인을 직접 볼 수 있다.
  경로는 실행 로그와 `<t>.score.json` 의 `workdir` 에 기록된다.
- `check.py` 는 숨겨진 검사기다. 과제 프롬프트나 작업공간에 들어가지 않는다. 검사기 내용을 에이전트에 노출하면 결과가 무의미해진다.
- 검사기는 `python3 -I` 로 돌려 스크립트 디렉터리와 현재 디렉터리의 모듈이 끼어들지 않게 한다.
  이 폴더의 `score.py` 나 `check.py` 를 수정하면 결과 해석이 달라진다.
- macOS 에는 `timeout` 이 없어서 `perl -e 'alarm shift; exec @ARGV'` 로 제한 시간을 구현했다.

## CI

`.github/workflows/e2e.yml` 은 `workflow_dispatch` 로만 돈다 (ubuntu-latest).
`ANTHROPIC_API_KEY` 시크릿이 없으면 notice 를 남기고 건너뛴다. 있으면 `bash tests/e2e/run.sh` 를 실행한다.
