# Claude Code — Opus는 듣기만, 작업은 하위 에이전트, Opus 토큰 최소화

**버전 2026.10.28-8** · 버전별 변경 사항은 `CHANGELOG.md`

메인 세션(Opus)은 **입력을 듣고 → 판단하고 → 에이전트에게 맡기고 → 결과를 이해하기 쉽게 설명**합니다. 파일·웹·명령 도구는 메인 세션에서 훅으로 차단됩니다.

## Opus 토큰을 줄이는 장치
| 줄이는 것 | 장치 | 위치 |
|---|---|---|
| 매 턴 다시 읽는 고정 분량 | CLAUDE.md를 3.6KB → 3.1KB로 축소 (규칙은 짧은 영어 한 줄씩). 배분 규칙은 각 에이전트 설명에 둠 | `CLAUDE.md`, `.claude/agents/` |
| 대화 누적 크기 | 메인은 300k 창에서 자동 압축 (압축 횟수 최소화) + 압축 시 남길 것 지정. Haiku는 80k, Sonnet·Opus 에이전트는 100k | `settings.json` `autoCompactWindow`, `CLAUDE.md` "Compact instructions" |
| Opus의 생각 토큰 | 메인 생각 강도 `low`. 판단이 필요한 턴만 프롬프트에 `ultrathink` | `settings.json` `effortLevel` |
| 에이전트 결과가 쌓이는 양 | 모든 에이전트 반환 5줄 이내, 자세한 내용은 `notes/runs/`에 파일로 | 에이전트 정의 |
| 왕복 횟수 | 사용자 메시지 1개당 위임 1회(병렬). 에이전트끼리 Opus를 거쳐 연결하지 않음, haiku-drafter가 스스로 검증 | `CLAUDE.md`, `haiku-drafter.md` |
| 지시서 크기 | 지시서 5줄, 경로만 전달 | `CLAUDE.md` |
| 정정 처리 | 훅이 한 줄 태그만 주입 (`[corr A] …`) | `correction-guard.py` |
| 예전 대화 찾기 | Haiku `haiku-scout`가 `scripts/search_history.py`로 현재·보관 대화 기록에서 사용자 메시지와 답변 본문만 검색 (압축으로 요약된 원문 포함) | `haiku-scout.md`, `search_history.py` |
| 압축 없이 리셋 | 150k 안내 → `/exit` 후 같은 터미널에서 `claude`. 새 세션에 OPEN·LEDGER·직전 대화 15개 자동 주입 (모델 호출 없음) | `ledger.py`, `session-context.py` |
| Opus가 직접 하는 작업 | 메인 세션 도구 차단 | `main-thread-guard.py` |
| 대화 기록 | 세션 종료·압축 직전에 `logs/history/*.txt`로 자동 저장 (120자 줄바꿈, 500KB 분할) | `history-export.py`, `scripts/export_history.py` |

## Haiku 80k 교대
Haiku 에이전트가 모델에 보내는 입력(대화 기록 + 도구 결과)이 **80k를 넘기 전에 새 에이전트로 교대**(Haiku 단가가 오르는 100k 아래)합니다. 대화를 요약(압축)해서 이어 쓰지 않으므로, 요약 과정에서 내용이 빠지거나 요약 비용이 드는 일이 없습니다. 한 요청당 다시 읽는 양도 절반으로 줄어듭니다.

| 단계 | 장치 | 동작 |
|---|---|---|
| 1. 도구 출력 상한 | `settings.json` env | 파일 읽기 1회 최대 8k 토큰, 셸 출력 12,000자, MCP 출력 10k 토큰 |
| 2. 사전 차단 훅 | `.claude/hooks/haiku-context-guard.py` | 실제 사용 토큰 + 이번 도구 결과 예상치(동시 호출 합산)가 80k를 넘을 호출은 차단. 사진은 파일 크기가 아니라 축소된 크기(장당 약 1.7k)로 계산. 75k 이상이면 인계 파일 저장만 허용. `allow_large` 항상 금지 |
| 3. 인계 | 에이전트 정의 | `notes/runs/`에 한 일·남은 일·수정한 파일·다음 단계를 저장하고 `CONTEXT-LIMIT: <경로>` 반환 |
| 4. 교대 | `CLAUDE.md` | Opus는 이전 에이전트를 이어 쓰지 않고, 같은 종류의 새 에이전트에게 인계 파일 경로만 넘김 |
| 예비 | 에이전트 `autoCompactWindow: 100000` | 훅이 동작하지 않는 환경에서만 쓰이는 마지막 안전장치 |

상한을 바꾸려면 `settings.json`의 `HAIKU_CONTEXT_CAP`을 고치세요. Sonnet·Opus 에이전트(`sonnet-*`, `opus-architect`)는 제외됩니다.
하위 에이전트는 시스템 지시와 도구 정의만으로도 기본 분량이 있어서, 80k 중 실제로 작업에 쓸 수 있는 양은 그보다 적습니다. 100000 이상으로는 올리지 마세요(단가 상승).

## 누가 답하는가 (파일·웹 작업이 없는 질문)
| 경우 | 담당 |
|---|---|
| 설명·결과 보고·개념·의견 | Opus 직접. 일반 Opus처럼 자연스러운 문장, 비유 없이 대략 어떻게 돌아가는지 쉽게, 내부 코드명·파일 이름은 생략, 길이 제한 없음 |
| 이 대화의 미묘한 맥락이 필요 (모호한 의도, 우선순위·전략 판단, 반론 판정) | Opus 직접 |
| 보관할 문서 (전체 계획, 비교표, 조사가 필요한 분석) | Sonnet `sonnet-analyst`가 `notes/runs/`에 작성 → Opus는 경로와 핵심 내용을 설명 |

## 짧고 빠른 입력 스타일 최적화 (`style-adapter.py`)
입력 습관은 바꾸지 않고, 짧은 프롬프트 한 번에 드는 비용을 최소화합니다.

| 입력 예 (실제 기록) | 처리 | 모델 호출 |
|---|---|---|
| "진행상황", "끝났어?", "로그 확인해봐", "지금은" | 훅이 `logs/*.status`와 로그 끝부분을 바로 보여줌 | **없음** |
| "그건 테스트 데이터였어", "그때는 서버가 꺼진 상태였어" 같은 정보 문장 | `notes/INBOX.md`에 메모 → 다음 지시와 함께 한 번에 전달 | **없음** |
| "ㅇㅇ", "A로 가자", "바꿔", "진행해" | `[ack]`: 직전 응답의 `▶ 기본` 실행, 다시 분석하지 않음 | 짧게 |
| "다시 진행해", "다시 시도해" | `[retry]`: 같은 에이전트로 실패한 항목만 다시 | 짧게 |
| "새 데이터 기준으로", "로컬도 넘겨" | `[frag]`: 진행 중인 에이전트에 그대로 전달 | 짧게 |
| 긴 붙여넣기 (같은 글 반복 포함) | `notes/pastes/`에 저장, 에이전트에는 경로만 전달 | 보통 |

- Opus는 제안하거나 물을 때마다 응답 끝에 `▶ 기본: <행동> · 대상: <파일/버전>`을 붙입니다. 그래서 "ㅇㅇ"이나 "바꿔"만 보내도 무엇을 어디에 적용할지 분명합니다.
- **메모 기준:** 지시어("해/바꿔/확인/분석…"), 질문("?/왜/어느/아니면…"), 정정("아니/근데…")이 없는 서술문만 메모합니다. Opus의 직전 응답이 질문이나 제안으로 끝났다면 답으로 보고 바로 전달합니다.
- **바로 답을 받고 싶으면** 끝에 `?`나 `!`를 붙이세요. 메모 기능을 끄려면 `settings.json`의 env에 `"STYLE_INBOX": "0"`을 넣으세요.

## 모델 배분
| 모델 | 에이전트 | 생각 강도 |
|---|---|---|
| Haiku | `haiku-scout` 탐색 · `haiku-checker` 검증 · `haiku-batch-worker` 일괄 작업 · `haiku-runner` 장시간 작업 | low |
| Haiku | `haiku-researcher` 웹 조사 · `haiku-drafter` 글 작성·수정 · `haiku-coder` 코드 | medium |
| Sonnet | `sonnet-analyst` 판단이 필요한 작업 전반(범위 먼저 보고 어려우면 바로 넘김), Opus 계획서 구현, Haiku 2회 실패 시 · `sonnet-reviewer` 제출 전 검토 | medium |
| Opus (에이전트) | `opus-architect` Sonnet이 넘긴 일, `opus로`, 같은 정정 2회, 실패한 일: 설계·구조·알고리즘·원인 불명 진단·전략 판단. 작은 구현은 직접, 아니면 계획서 → Sonnet 구현 | high |
| Opus | 메인: 듣기·짧은 답·맥락 판단·배분 | low (+ `ultrathink`) |

**누가 맡나:** 단순·기계적 작업은 Haiku, 판단이 필요한 작업(종합, 설계, 여러 파일 코드, 긴 답)은 `sonnet-analyst`가 맡습니다. Sonnet은 구현 전에 파일 3개 이내로 범위를 먼저 보고, 그럴듯한 설계가 2개 이상인데 근거가 없거나, 요구사항이 충돌하거나, 얽힌 파일이 5개 이상이거나, 자기 검증이 1번 실패하면 구현하지 않고 `ESCALATE`로 정리본을 넘깁니다. Opus는 그 정리본을 그대로 `opus-architect`(Opus, 깊게 생각)에 넘깁니다. `opus로`라고 쓰거나, 같은 정정이 2번 나왔거나, 에이전트가 이미 실패한 일은 처음부터 `opus-architect`로 갑니다. 응답 첫머리에 `[sonnet-analyst]`, `[ESCALATE → opus-architect]`처럼 경로를 보여 줍니다. Opus 설계가 계획서로 나오면 경로와 요지를 보여 주고 `▶ 기본: 구현`을 기다립니다. Sonnet·Opus 에이전트는 단순 작업을 Haiku 에이전트에게 직접 넘깁니다(실제 Claude Code 2.1.312에서 확인, `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=2`). Haiku 외의 에이전트 호출은 훅이 막습니다. 중첩이 꺼진 환경이면 결과 파일의 `[haiku] steps`를 메인이 Haiku들에게 병렬로 넘깁니다.

**모델 표시:** 에이전트 이름 앞부분이 실행 모델입니다 (`haiku-*`, `sonnet-*`). Claude Code 화면과 `/list-agents`에는 모델 칸이 없어서 이름으로 보이게 했습니다. `agent-model-pin.py` 훅이 Opus가 호출할 때 모델을 덮어쓰지 못하게 하고, 기본 제공 에이전트(general-purpose, Explore, Plan)는 Haiku로 돌립니다. 실제로 돈 모델은 `notes/LEDGER.md`에 `[haiku-drafter · haiku 5.5]`처럼 기록되고, 이름과 다르면 `⚠`가 붙습니다.

바로가기(Haiku에서 바로 실행): `/status`, `/check <무엇> [대상]`, `/fact <이전> → <정정>`, `/research <질문>`

## 사용 습관 (Opus 호출 자체를 줄이기)
1. **Opus를 거치지 않는 확인:** 진행상황은 `!cat logs/*.status`처럼 `!`로 셸을 직접 실행하면 모델 호출이 없습니다.
2. **Haiku 보조 터미널:** 단순 작업(상태 확인, 변환, 사실 정정, 조사)은 다른 터미널에서 `claude --model haiku`로 처리하세요. Opus 사용량은 0입니다.
3. **짧은 답은 모아서:** "ㅇㅇ", "바꿔", "A" 같은 한 단어 답도 Opus 한 턴입니다. 다음 지시와 합쳐 보내세요. 예: "A로 가고, 세션 묶음도 같이 바꿔".
4. **판단이 필요할 때만 `ultrathink`:** 설계 선택, 전략, 반론 판정 같은 턴에만 붙이세요.
5. **`/clear`·압축 없이 리셋:** "맥락 150k" 안내가 나오면 `/exit` → 같은 터미널에서 `claude`. 대화는 화면 스크롤과 `logs/history/*.txt`(자동 저장, 가벼운 텍스트)에 남고, 새 세션에는 직전 대화 15개와 notes 요약이 자동으로 들어갑니다.
6. **정정:** 통째로 틀렸으면 `Esc` 두 번(되감기). 일부만 틀렸으면 "아니, 대상: ○○ / 원하는 것: ○○". 질문만 할 때는 "(질문)"을 붙이세요.
7. **한 번만 하는 점검:**
   - `/config`에서 프롬프트 제안(prompt suggestions)을 끄세요. 응답마다 Opus 요청이 하나씩 더 갑니다.
   - `/mcp`에서 안 쓰는 MCP 서버(Google Drive 등)를 끄고, `/context`로 고정 분량을 확인하세요.
   - `/usage`에서 어디에 사용량이 쓰이는지 보세요.

## 설치
어떤 프로젝트에도 적용됩니다. 프로젝트 고유 내용은 각 프로젝트의 `notes/`에만 들어가고, 에이전트·훅·규칙은 공통입니다.
`./install.sh <프로젝트 폴더> [--main sonnet]` 한 줄이면 됩니다. 기존 대화 정리까지 여러 프로젝트를 한 번에 하려면 `./fresh_start.sh <폴더> <폴더> ...`. 기존 `CLAUDE.md`·`settings.json`은 백업 후 병합됩니다. 단계별 안내와 확인 방법, 되돌리기는 `APPLY_GUIDE.md`, 이 패키지를 계속 다듬는 세션의 인계는 `DESIGN_HANDOFF.md`를 보세요.

## 기타 파일
- `.claude/skills/correction`: "아니/근데/아" 정정 유형(A~G)별 처리
- `FRESH_START.md`, `fresh_start.sh`, `/distill`, `haiku-extractor`, `scripts/extract_transcript.py`: 완전 초기화. 기존 대화 기록에서 중요 내용만 `notes/`로 추출 (권장)
- `.claude/skills/migrate-session`, `MIGRATE_NOW_PROMPT.md`: 진행 중인 세션을 그대로 이어가는 경우
- `scripts/charcount.py`: 정확한 글자 수
- `templates/`: 인계 문서·FACTS·DECISIONS 양식
- `ANALYSIS.md`: 이 패키지를 설계할 때 쓴 사용 기록 분석 (패키지 폴더에만 있고 프로젝트에는 복사되지 않음)

## 알아두실 점
- **전체 토큰은 약간 늘고, Opus 토큰은 크게 줄어듭니다.** 하위 에이전트가 매번 필요한 파일을 새로 읽기 때문입니다.
- **메인 생각 강도 `low`에서 의도를 잘못 알아듣는 일이 늘면** `"effortLevel": "medium"`으로 올리세요. Opus 5.5의 기본값이 medium입니다.
- **결과 내용은 Opus가 아니라 `notes/runs/` 파일로 확인하게 됩니다.**
- **일반 모드:** 평소처럼 Opus가 직접 작업하게 하려면 `일반모드`라고 입력하세요 (모델 호출 없음). 이 프로젝트에서 절약 규칙(도구 차단, 짧은 답, 태그, 리셋 안내)이 꺼지고, `절약모드`를 입력하면 다시 켜집니다. 그 세션만 일반 모드로 시작하려면 `CLAUDE_LEAN=0 claude`. 대화 기록 저장과 LEDGER는 일반 모드에서도 계속됩니다.
- **긴 세션 도중 모델을 바꾸지 마세요.** 바뀐 모델이 대화 전체를 캐시 없이 다시 읽습니다.
