# 현재 세션 토큰·컨텍스트 사용 요인 분석 (2026-10-29)

source: ~/.claude/projects/-Users-leekwangseok-PycharmProjects-claude-harness/ transcripts (message.id+requestId 중복 제거)
FACTS.md: 헤더만 있고 항목 없음 (사실 근거로 쓴 것 없음)
total = input+output+cache read+cache write (4종 합, 기존 cumulative-usage.md와 같은 기준)

## 0. 어떤 세션을 "현재"로 봤는가 (중요)
- 가장 최신 파일 26e84f4e (10-29 10:48 수정): haiku 메인 14턴 + 서브에이전트 8개. 총 2.02M 토큰. 작아서 "주요 요인"을 보기엔 표본이 작음.
- 직전의 장기 세션 19fde733 (5.8MB, opus 메인 263턴 + 서브에이전트 약 50개, 총 41.85M 토큰)이 누적 66.4M의 약 63%. 실질적 토큰 소비는 이쪽.
- 그래서 비교 대상으로 둘 다 분석함. 사용자가 말한 "현재 세션"이 어느 쪽인지 확인 필요 (open question).

## 1. 세션별 합계
### 19fde733 (opus 메인 장기 세션), 총 41.85M
| 구분 | 응답수 | cache read | cache write | output | 총합 비중 |
|---|---|---|---|---|---|
| opus main | 263 | 20.95M | 0.36M | 0.175M | 51.3% |
| sonnet sub | 490 | 14.16M | 1.36M | 0.43M | 38.1% |
| opus sub (architect 1개, 64턴) | 64 | 2.62M | 0.18M | 0.055M | 6.8% |
| haiku sub | 105 | 1.21M | 0.27M | 0.069M | 3.7% |
cache read 합 약 38.97M = 93%. input(비캐시)은 0.002M으로 무시 가능, output 0.73M = 1.7%.

### 26e84f4e (최신, haiku 메인), 총 2.02M
| 구분 | 응답수 | 총합 비중 |
|---|---|---|
| haiku main | 14 | 31.3% |
| sonnet sub (6개) | 55 | 63.5% |
| haiku sub (2개) | 9 | 5.2% |

## 2. 요인별 정량 (주로 19fde733 기준)
### D1. 서브에이전트(특히 sonnet)의 다턴 실행: 총 38.1% + opus sub 6.8%
- sonnet 서브 490턴, 평균 컨텍스트 약 29K/턴. 큰 것: abf5622e 80턴 2.9M cr, a6f387e4 42턴 1.49M, aa63848f 37턴 1.50M, a80c3379 27턴 0.85M, a001afcc 58턴 1.9M.
- 서브에이전트 시작 컨텍스트는 약 5~9K로 작음. 문제는 시작 크기가 아니라 턴 수(툴 호출 1회마다 컨텍스트 전체를 cache read).
- opus-architect 1개(a7adafd0)가 64턴 2.62M = 세션의 6.3%. 같은 일을 sonnet이 했다면 가중 비용이 훨씬 낮음(가중치는 요금제 의존, 추정).
- 최신 세션에서도 sonnet 서브가 63.5%로 동일 패턴.

### D2. opus 메인의 고정 컨텍스트(매 턴 재읽기): 약 28% (추정)
- 메인 첫 턴 컨텍스트 44,652 토큰 (실측). 이게 매 턴 cache read의 바닥.
- 44.6K x 263턴 = 11.7M = opus 메인 cache read(20.95M)의 56%, 세션 전체의 28%.
- 최신 세션(haiku 메인)의 첫 턴은 35,991 토큰 (실측).
- 바닥 구성 (attachments/prompt_snapshot 문자수, 토큰은 문자수/3.5~4 단순 추정이라 오차 큼):
  - tools 정의 42,153자 (prompt_snapshot.tools)  ~10K tok
  - skill_listing 19,497자  ~5K tok
  - system prompt 11,297자  ~3K tok
  - CLAUDE.md(instructions attachment) 7,767자 (파일 자체 7,431B)  ~2K tok
  - SessionStart hook snapshot 6,040 + 5,759자  ~3K tok
  - deferred_tools_delta 8,053자 (Claude Docs/Google Docs MCP, Cron, Worktree 등 이름 목록)  ~2K tok
  - agent_listing_delta 7,391자 (agent 정의 요약, .claude/agents 합 26,396B 중 description만 노출)  ~2K tok
  - mcp_instructions_delta 2,284자, session_context 2,514자, UserPromptSubmit hook 약 500자 x 매 프롬프트
- 합이 약 130K자 = 33K~37K 토큰 -> 실측 36K(haiku)와 대체로 일치.
- 19fde733 전체에서 attachment 합계: skill_listing 106,830자, deferred_tools_delta 50,502자, hook_success 50,452자, agent_listing_delta 31,320자(= 여러 번 재주입), 즉 델타/재주입이 반복됨.

### D3. opus 메인의 "텍스트만" 턴(서브에이전트 hand-back 후 상태 보고): 33.4%
- 메인 263턴 중 text-only 174턴 = cache read 14.0M (opus 메인 cr의 67%, 세션의 33%).
- Agent 호출 33턴 2.47M, SendMessage 53턴 4.32M (세션의 10%), ToolSearch 3턴.
- 서브에이전트 완료마다 queued_command(hand-back)가 들어와 opus가 한 턴 깨어남. 25개 서브에이전트 hand-back당 컨텍스트 약 80K 재읽기.
- output은 turn당 평균 약 660 토큰(최대 1.58K)로 작음 -> output이 아니라 "깨어나는 횟수"가 비용.

### D4. 서브에이전트 출력이 메인에 돌아오는 양: 작음 (약 0.5% 이하)
- 19fde733: tool_result 합계 53,920자 (Agent 36회 38,551자, SendMessage 56회 15,201자) = 약 14K 토큰. 개당 상한 약 1.1K자.
- hand-back queued_command attachment 합 24,592자 (약 6K tok).
- 최신 세션: Agent 결과 7건 2,707자, queued hand-back 8건 합 약 31K자(개당 3~6.5K자) -> 메인 컨텍스트 36.0K->51.7K 증가분(15.7K)의 대부분 (약 8~10K tok 추정).
- 즉 "반환 내용 크기"는 요인이 아님. 반환 횟수(D3)가 요인.

### D5. 메인 컨텍스트 성장
- 19fde733 opus 메인: 44.6K 시작, 최대 116.8K, 4번 급락(15K 이상, 압축/정리). 샘플 평균 약 80K.
- 최신 세션: 35,991 -> 51,652 (14턴, 턴당 약 +1.1K). 새로 쓰인(cache write) 큰 값은 첫 턴 35,989, 이후 턴당 0.5~2K.
- cache write 큰 턴 5개(50.6K, 44.7K, 23.7K, 21.4K, 21.2K) = 총 cw 0.36M의 약 45%. 5분 초과 공백 14회 후 cw는 11K에 불과 -> 캐시 만료는 요인 아님.

### D6. 반복 대용량 파일 읽기
- 메인에는 Read/Bash/Grep 툴콜이 0건 (툴콜은 Agent/SendMessage/ToolSearch뿐) -> 메인의 반복 파일 읽기 없음. 지휘 전용 구조가 작동 중.
- 서브에이전트 내부 파일 읽기 중복은 이번에 전수 조사하지 않음 (미확인).
- notes/DECISIONS.md는 17,060B로 SessionStart 스냅샷·읽기 후보로 커지는 중. 서브에이전트가 매번 읽으면 비용(추정, 미측정).

### D7. Opus 메인 호출은 줄일 수 있는가
- 근거: 263턴 중 툴콜 없이 텍스트만 174턴, 나머지는 위임(Agent/SendMessage) 호출. 메인이 직접 한 코드/분석 작업은 0건 -> 순수 지휘·보고.
- 최신 세션은 이미 haiku 메인 (14턴). 같은 역할을 하므로 opus 메인은 대체로 불필요했을 가능성이 큼 (추정; 사용자 의도 파악 품질 차이는 미측정).
- opus 총 기여: 메인 51.3% + opus sub 6.8% = 58.1%. 토큰 수는 모델 바꿔도 비슷하지만 opus 가중치가 높으므로 비용은 큰 폭 감소 가능(단가는 기록 없어 미계산).

## 3. 절감안
| # | 제안 | 예상 효과 | 위험 |
|---|---|---|---|
| S1 | 지휘 메인을 opus -> haiku(또는 sonnet)로 고정, opus는 설계 요청에만 | 토큰은 동일하나 opus 메인 51%분의 가중 비용 대폭 감소(최신 세션이 이미 이 방식). 단가 미확인이라 %는 미산정 | 지시 해석·라우팅 품질 저하 가능, 사용자가 "opus로" 한 경우 예외 필요 |
| S2 | 메인 고정 바닥 축소: skill 목록 19K자, deferred tools(미사용 Claude Docs/Google Docs MCP) 8K자, SessionStart 스냅샷 12K자, agent 설명 단축 | 바닥 36~45K -> 약 25~30K 목표. 메인 cr 약 20~30% 감소 = 세션의 5~9% (추정) | 스킬/MCP 제거 시 해당 기능 상실. 훅 스냅샷 축소 시 상태 파악 약화 |
| S3 | hand-back 병합: 서브에이전트 완료 알림을 즉시 한 턴씩 깨우지 말고 모아서 처리, 또는 상태 보고 턴 생략 (text-only 174턴 중 정형 보고분) | text-only 턴의 30~50% 감소 가정 시 세션의 10~16% | 사용자에게 진행 상황 지연, 하니스 수정 필요 |
| S4 | 서브에이전트 턴 상한/배칭: 한 턴에 Read/Grep 병렬 호출, 40턴 초과 시 중간 보고, abf5622e(80턴)·a001afcc(58턴) 같은 장기 작업은 단계 분할 | sonnet sub 38%의 20~30% 절감 가정 시 세션의 8~11% | 병렬 호출 시 정확도 하락 가능, 분할 시 컨텍스트 재구성 비용 |
| S5 | opus-architect 사용을 설계 결정에 한정, 구현은 sonnet/haiku에 위임(이미 규칙) 64턴짜리 opus sub 점검 | opus sub 6.8% 중 절반만 줄여도 가중 비용 절감 큼 | 설계 품질 |
| S6 | 메인 컨텍스트 정리 주기 단축 (100K 도달 전 압축/새 세션 인계) | 평균 컨텍스트 80K -> 60K면 메인 cr 약 25% 감소 | 정보 손실, 인계 비용(cw 재발생) |
효과 합산은 중복(S2/S6/S1)이 있어 단순 합산 금지.

## 4. 불확실성
- 토큰 환산은 문자수/3.5~4 단순 추정 (한글 비중 높아 실제는 더 클 수 있음). 첫 턴 총합(실측)과 대략 일치하는지로만 검증.
- 가중 비용은 단가·요금제 가중치 정보 없어 계산 안 함.
- 서브에이전트 내부 중복 파일 읽기, 서브에이전트가 읽는 notes 크기는 미조사.
- 19fde733과 26e84f4e의 모델/역할이 다름(opus 메인 vs haiku 메인). 사용자가 말한 "현재"가 어느 쪽인지 미확정.
- 트랜스크립트 메타 필드 일부는 하니스가 추가한 것(prompt_snapshot 등)이라 문자수가 실제 API 전송 크기와 다를 수 있음.
