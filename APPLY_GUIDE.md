# 적용 가이드 — Opus는 듣기만, 작업은 하위 에이전트

어떤 프로젝트(문서, 코드, 데이터 분석, 장시간 작업)에도 같은 방식으로 적용됩니다. 프로젝트 고유의 내용(사실, 규칙, 산출물)은 각 프로젝트의 `notes/`에만 두고, 에이전트·훅·규칙은 공통입니다. 처음에는 한 프로젝트에만 적용해 며칠 써 본 뒤 넓히는 것을 권합니다.

---

## 0. 준비 (1회)
```bash
claude --version        # 2.1.296 이상 권장 (에이전트별 autoCompactWindow 지원). 낮으면: claude update
python3 --version       # 없으면: xcode-select --install
unzip ~/Downloads/claude-opus-lean.zip -d ~/claude-opus-lean
```

**적용 전 기준값 저장** (나중에 효과 비교용):
```bash
python3 ~/claude-opus-lean/scripts/usage_report.py > ~/usage-before.txt
```

## 1. 설치
```bash
cd ~/claude-opus-lean
chmod +x install.sh fresh_start.sh
./install.sh <프로젝트 폴더>                 # 메인 Opus
./install.sh <프로젝트 폴더> --main sonnet   # 판단이 적은 프로젝트는 메인 Sonnet
```
여러 프로젝트는 한 줄씩 실행하거나, 완전 초기화까지 함께 할 때는 `./fresh_start.sh <폴더> <폴더> ...`를 씁니다 (`FRESH_START.md`).
설치 스크립트가 하는 일:

| 항목 | 내용 |
|---|---|
| 백업 | 기존 `CLAUDE.md`, `.claude/settings.json` → `.claude-backup-<시각>/` |
| CLAUDE.md | 정책을 맨 위에 추가, 기존 내용은 아래에 유지 (재실행해도 중복 없음) |
| settings.json | 모델·생각 강도·압축 설정과 env, 훅 10개를 **병합** (기존 훅 유지) |
| 에이전트 11개 · 스킬 7개 · 훅 파일 10개 | `.claude/` 아래 복사 |
| 폴더 | `notes/`(FACTS, DECISIONS, research, runs, pastes), `logs/` |
| 문서 | `docs/claude-lean/` (이 가이드, FRESH_START, README, 이전 프롬프트) |

git 저장소라면 `.gitignore`에 `logs/`와 `notes/pastes/`를 추가하세요.

## 2. 기존 대화 정리 (둘 중 하나)
- **완전 초기화 (권장):** 오래된 대화 기록에서 중요한 내용만 `notes/`로 뽑고 새로 시작합니다. → `FRESH_START.md` (`fresh_start.sh`가 설치까지 함께 하므로 1단계를 다시 해도 문제없습니다).
- **세션 이어가기:** 진행 중인 작업을 그대로 넘깁니다. → `MIGRATE_NOW_PROMPT.md`
- 정리할 기존 대화가 없으면 건너뜁니다.

## 3. 동작 확인 (새 세션에서, 5분)
훅은 **새로 시작한 세션**에만 적용됩니다.
```bash
cd <프로젝트 폴더> && claude
```
| 확인 | 기대 결과 |
|---|---|
| `/agents` | haiku-scout, haiku-researcher, haiku-drafter, haiku-checker, haiku-batch-worker, haiku-runner, haiku-coder, haiku-extractor (Haiku), sonnet-analyst, sonnet-reviewer (Sonnet), opus-architect (Opus) |
| `/hooks` | UserPromptSubmit 3개, PreToolUse 3개, SubagentStop 1개, SessionStart 1개, SessionEnd 1개, PreCompact 1개 |
| `/context` | 고정 분량 확인 (안 쓰는 MCP 서버가 크면 `/mcp`에서 끄기) |
| "README.md 첫 줄 알려줘" 입력 | Opus가 직접 읽지 않고 `haiku-scout`에게 맡김. 직접 읽으려 하면 훅이 차단 |
| "ㅇㅇ" 입력 | 응답이 짧게 끝남 (`[ack]` 처리) |
| 에이전트 실행 중 `/list-agents` | `[running] · haiku-scout`처럼 이름 앞에 모델이 보임 |
| 에이전트가 끝난 뒤 `notes/LEDGER.md` | `[haiku-scout · haiku 5.5]`처럼 실제로 돈 모델이 기록됨 |
| "진행상황" 입력 | `logs/*.status`가 없으면 모델에 전달, 있으면 모델 호출 없이 바로 표시 |

훅 단독 점검 (터미널):
```bash
echo '{"prompt":"ㅇㅇ"}' | python3 .claude/hooks/style-adapter.py      # [ack] … 출력
echo '{"tool_name":"Read"}' | python3 .claude/hooks/main-thread-guard.py  # deny JSON 출력
```

새 세션의 첫 입력 예 (2단계를 했다면):
```
notes/OPEN.md 기준으로 오늘 할 일부터 시작하자
```

## 4. 일상 사용법
| 하고 싶은 것 | 입력 |
|---|---|
| 평소처럼 지시 | 그대로 (짧아도 됨). 응답 끝 `다음 할 일:` 줄에 "ㅇㅇ"이면 그대로 실행 |
| 진행상황 | "진행상황" / "끝났어?" → 모델 호출 없이 표시. 분석까지 원하면 끝에 `?` |
| 정보만 알려주기 | "그 데이터는 9월 것이었어" 같은 서술문 → 메모로 보관, 다음 지시 때 함께 전달. 바로 답을 원하면 끝에 `?`/`!` |
| 사실 정정 | `/fact <이전> → <정정>` (예: `/fact 마감 11/3 → 11/5`) |
| 반영 여부 확인 | `/check <바꾼 내용> [대상 파일]` |
| 조사 | `/research <질문>` |
| 깊게 판단해야 할 때 | 프롬프트에 `ultrathink` 포함 |
| 결과 자세히 보기 | 응답에 나온 `notes/runs/…` 파일 열기 |
| 예전 대화 내용 찾기 | "예전에 마감 언제라고 했지?"처럼 그냥 물어보기 → Haiku `haiku-scout`가 대화 기록 검색 (보관한 기록 포함). 터미널에서 직접: `python3 scripts/search_history.py "<단어>"` |
| 다른 작업으로 넘어갈 때 | 그냥 이어서 입력 (`/clear` 불필요. 메인 대화가 150k 창 가까이 차면 자동 압축) |
| 맥락이 커졌을 때 ("맥락 100k" 안내가 나오면) | `/exit` → 같은 터미널에서 `claude`. 압축 없이 가벼워지고, 대화는 화면에 남으며 직전 대화가 자동으로 이어짐 |
| 오래 쉬었다 돌아올 때 | `claude`(새 세션)로 시작. `claude -c`보다 훨씬 쌈 (아래 참고) |
| 맡길 모델을 직접 정하기 | 프롬프트에 `haiku로` / `sonnet으로` / `opus로`. 없으면 Opus가 정하고 응답 첫머리에 `[sonnet-analyst]`처럼 표시 |
| 어려운 설계를 Opus에게 | Sonnet이 어렵다고 보면 `ESCALATE`로 자동 전환, 처음부터는 `opus로`. 계획서가 나오면 "ㅇㅇ"으로 Sonnet 구현 진행, 바로 끝까지 하려면 "opus로 설계하고 바로 구현까지" |
| 일반 모드로 쓰기 (Opus가 직접 작업) | `일반모드` 입력 (모델 호출 없음). 되돌리기는 `절약모드`. 이 프로젝트에서 되돌릴 때까지 유지 |
| 에이전트가 어떤 모델로 돌았는지 | 이름 앞부분(`haiku-*`, `sonnet-*`)이 모델. 실제 기록은 `notes/LEDGER.md`, 모델별 사용량은 `/usage` |
| 예전 대화 읽기 (사람) | `logs/history/*.txt` 를 아무 편집기로 열기 (가볍게 저장되어 바로 열림) |
| 대화 중간에 모델·MCP 바꾸기 | 피하세요. 바꾸면 캐시가 식어 그 뒤 첫 요청이 비싸집니다. 필요하면 새 세션에서 바꾸세요 |
| 쉬었다 이어 쓰는 서브에이전트 | 5분 넘게 쉬면 캐시가 식습니다. 이어 쓸 거면 5분 안에 보내세요 |

### 맥락 리셋 (압축 없이, 대화 기록 유지)
`/clear`와 압축은 쓰지 않습니다. 압축은 대화 전체를 한 번 더 보내는 요청이라 사용량이 크기 때문입니다.
- **리셋 방법:** `/exit` → **같은 터미널 창**에서 `claude`. 이전 대화는 터미널 스크롤에 그대로 남습니다 (기본 화면 모드 기준).
- **흐름 잇기 (모델 호출 없음):** 새 세션을 시작할 때 `notes/OPEN.md`, 최근 에이전트 결과(LEDGER 12줄), **직전 세션의 마지막 대화 15개**가 자동으로 들어갑니다.
- **언제:** 리셋 안내는 기본으로 꺼져 있습니다. 켜면 메인 맥락이 선택한 값(100k 또는 120k, `LEAN_RESET_AT`)을 넘을 때 답 끝에 안내가 한 줄 붙습니다. 메인 맥락이 150k(`autoCompactWindow`)에 이르면 Claude Code가 자동으로 압축하므로, 안내는 그 전에 리셋하려는 선택사항입니다.
- **대화 기록 파일:** 세션이 끝날 때와 압축 직전에 `logs/history/<날짜>-<세션>-NN.txt`로 자동 저장됩니다. 사용자 메시지와 답변만 남기고, 120자 줄바꿈, 약 500KB 단위로 나눠서 PyCharm·Obsidian·VS Code에서 바로 열립니다. 직접 만들려면 `python3 scripts/export_history.py`.
- **터미널 스크롤 길이:** 화면에 남는 양은 터미널 설정으로 정해집니다. 크게 늘려 두세요.

| 터미널 | 설정 위치 |
|---|---|
| PyCharm·WebStorm 내장 터미널 | Settings → Advanced Settings → Terminal → 스크롤 버퍼 크기 |
| macOS 터미널 | 설정 → 프로파일 → 윈도우 → 스크롤백 "제한 없음" |
| iTerm2 | Profiles → Terminal → Unlimited scrollback |
| Windows Terminal | 설정 → 프로필 → 고급 → 기록 크기 |

- **쉬었다 돌아올 때의 비용:** 1시간 넘게 쉬면 캐시가 만료됩니다. `claude -c`로 이어가면 대화 크기만큼 다시 저장(100k면 약 $0.8)하고, `claude`로 새로 시작하면 약 $0.15입니다.

## 5. 조절 손잡이 (`.claude/settings.json`)
| 설정 | 기본값 | 바꾸는 경우 |
|---|---|---|
| `effortLevel` | `low` | 의도를 자주 잘못 알아들으면 `medium` |
| `env.LEAN_RESET_AT` | `0` | 리셋 안내를 띄우는 메인 맥락 크기. `0`은 끔. 켜려면 `100000` 또는 `120000` |
| `autoCompactWindow` | `150000` | 비용을 더 줄이려면 `100000` (압축이 더 잦아짐) |
| `env.CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH` | `2` | 메인 → Sonnet/Opus → Haiku 연계 깊이. `1`이면 연계 끔 (메인이 `[haiku] steps`를 대신 배분) |
| `env.STYLE_INBOX` | (없음 = 켜짐) | 메모 기능이 불편하면 `"0"` |
| `env.CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS` | `8000` | 큰 파일을 자주 다루면 `12000` |
| 입력 `일반모드` / `절약모드` | 절약 | 절약 규칙을 끄고 켬. `.claude/lean-off` 파일로 상태 저장, 모델 호출 없음 |
| 실행 시 `CLAUDE_LEAN=0 claude` | — | 그 세션만 일반 모드로 시작 (프로젝트 상태는 그대로) |

프로젝트 성격별 권장:
| 성격 | 메인 | 비고 |
|---|---|---|
| 문서·글쓰기 (지시가 모호하고 정정이 잦음) | Opus | 기본값 |
| 코드 | Opus (기능 설계가 많으면) / Sonnet (수정 위주) | 선택 플러그인 RTK |
| 장시간 작업·데이터 처리 (지시가 명확) | Sonnet | `--main sonnet`, 선택 플러그인 RTK |

## 5-1. 업그레이드 (이미 설치한 프로젝트)
예전 패키지 폴더를 지우고 새 zip을 같은 위치에 푼 뒤 설치 명령을 그대로 다시 실행하면 됩니다 (예전 파일이 섞이지 않게). 기존 설정은 백업한 뒤 아래를 자동으로 처리합니다.
- CLAUDE.md의 정책 부분을 최신으로 교체 (프로젝트 고유 내용은 유지)
- 패키지에서 뺀 설정 제거 (예: 압축을 앞당기던 `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` 70)
- `notes/`의 빈 양식 줄 삭제 (예: `- [대회]`)
- 예전 버전에만 있던 파일은 백업 폴더(`.claude-backup-*/obsolete/`)로 이동
- 이름이 바뀐 에이전트(2026.10.28부터 `scout` → `haiku-scout` 등)의 예전 파일도 백업으로 이동. 이 패키지가 설치한 파일일 때만 옮기고, 같은 이름의 사용자 에이전트는 그대로 둠
- 설치 버전을 `.claude/claude-lean.version`에 기록

업그레이드 후에는 `/exit` → `claude -c`(또는 `claude --resume`)로 같은 대화를 이어가세요. 새 규칙(CLAUDE.md)·에이전트·설정·훅은 Claude Code가 실행을 시작할 때 읽기 때문에, 열려 있던 세션에는 반영되지 않습니다. 세션이 열린 채로 업그레이드했다면 다음 입력 때 Opus가 이 안내를 한 줄 덧붙이고(세션당 한 번), 이어간 세션 첫머리에 "X로 업데이트됨"이 화면에 표시됩니다. 세션 안에서 `/resume`으로 전환하는 것은 같은 프로세스에서 대화만 바꾸는 것이라 CLAUDE.md·에이전트가 다시 읽히지 않습니다(테스트로 확인). 이때도 같은 안내가 표시됩니다.

## 6. 효과 확인 (적용 3~7일 후)
```bash
python3 ~/claude-opus-lean/scripts/usage_report.py > ~/usage-after.txt
diff ~/usage-before.txt ~/usage-after.txt
```
- **요청당 평균 읽기**가 크게 줄었는지 (목표: 수십만 → 5만 이하)
- 모델별 비중: Opus가 줄고 Haiku가 늘었는지
- 턴 단위로 보려면 Turn Usage 플러그인 (선택)

## 7. 문제 해결
| 증상 | 조치 |
|---|---|
| Opus가 "도구가 차단됐다"며 멈춤 | 정상 동작. 계속 그러면 CLAUDE.md가 적용됐는지 확인 (`/memory`) |
| 에이전트 작업이 결과 없이 끝남 | 0.4.10부터 에이전트는 맥락이 차면 자동 압축 후 계속합니다. 예전 버전이면 업그레이드 |
| 메모로 빠지면 안 되는 말이 메모됨 | 끝에 `!` 붙여 재전송, 반복되면 `STYLE_INBOX=0` |
| 답이 너무 짧아 이해가 어려움 | "자세히" 요청 → `sonnet-analyst`가 `notes/runs/`에 작성 |
| `notes/LEDGER.md`에 `⚠ 이름과 다름` | `/hooks`에서 `agent-model-pin.py` 등록 확인. 등록돼 있으면 사용량 한도 등으로 Claude Code가 모델을 바꾼 경우 |
| 훅 오류 메시지 | `python3 .claude/hooks/<훅>.py` 직접 실행해 오류 확인, `/hooks`에서 등록 확인 |

## 8. 되돌리기
```bash
cd <프로젝트 폴더>
cp .claude-backup-<시각>/CLAUDE.md . ; cp .claude-backup-<시각>/.claude/settings.json .claude/
rm -f .claude/hooks/{correction-guard,style-adapter,main-thread-guard,haiku-context-guard,agent-model-pin,ledger,session-context,context-alert,history-export,_lean}.py
```
(백업이 없던 프로젝트는 `CLAUDE.md`와 `.claude/settings.json`을 삭제)

## 9. 플러그인 판단 요약
- **권장:** 측정용 Turn Usage 또는 token-usage (둘 중 하나)
- **선택:** RTK — 코드·로그 위주 프로젝트에만
- **비권장:** caveman (이미 5줄 제한과 겹치고, 존댓말·문서 문체와 충돌), AgentOne Token Compression (외부 전송·훅 충돌), token-saver (인계 구조와 중복)
