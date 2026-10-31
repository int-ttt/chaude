# 앞으로 구현할 항목 정리 + 결과 출력 방식 변경안
date: 2026-10-29 / mode: read-only (코드·문서 수정 없음). 근거: 파일 직접 확인(grep/부분 읽기), 추정은 "추정"으로 표시.

## Part 1. 미구현 항목 (우선순위 순)

사전 사실
- notes/OPEN.md 는 존재하지 않음 (ls notes 와 cat 실패 확인). CLAUDE.md:26 과 session-context.py 는 이 파일을 읽도록 되어 있어 항상 비어 있음 -> 아래 목록을 OPEN.md 로 만드는 것이 첫 후속 작업 후보.
- notes/LESSONS.md 는 2건뿐(한글 IME 한칸 밀림 수정 실패, /resume 수정 미적용). 둘 다 "다시 할 일"로 읽힘.
- 아래는 요약 문서와 결정 기록 기준이며 소스를 전부 재검증한 것은 아님.

| 순위 | 항목 | 상태/근거 | 출처 |
|---|---|---|---|
| 1 | /resume(=/import) 수정 미적용: draining 세션 때문에 거절된 수정이 적용 안 됨. import 는 TUI 블록에도 보여야 하고, rotate 가 대기 중인 import 스냅샷을 덮어쓰면 안 됨 | LESSONS 에 "never applied". tui.go:418-427 importedMsg 는 이미 있음 -> 부분 구현 추정, 재확인 필요 | notes/LESSONS.md:3 |
| 2 | 한글 IME 한칸 밀림: 0.6.11 의 bgH 수정은 효과 없음. JediTerm 은 preedit 를 cursorX+1 에 그림 -> 열(column) 문제 | 미해결. 터미널 소스 확인 후 조정 | notes/LESSONS.md:2 |
| 3 | 지표/비용 집계 정리: classify 비용이 요약에 안 잡힘, 예산 장부가 재시작하면 사라짐, router_prompt 로그 필드에 모델명이 들어감(복사 실수 추정), pickTarget 의 kind 인자 미사용 | 제안 1·4. 위험 낮음 | notes/runs/261029-analysis-structure-and-roadmap.md §13(3,5), §14(1,4) |
| 4 | 정책 단일화: 메인 모델 정책/임계(80k vs 270k)/분류기 기본 모델(sonnet) 불일치, 상태줄 분모 270k 고정. 분류기 Haiku 전환은 사용자 승인 대기 + 30건 샘플 비교 필요 | 제안 2 | 같은 문서 §13(1,2,4), §14(2) |
| 5 | 0.8.0 후속 미검증: 유휴 정리 reconcile 호출 위치, 드레인 타임아웃/최대 개수 기본값, 캐시 TTL 1시간 실측(3599초 대 290초), 가격표(docs.claude.com 미조회 -> 누락 모델 est=0), resume 비용 0 가정, `--idle-rotate (TUI only)` CHANGELOG 문구 | plan §6 doubts, 로드맵 §14(7) | notes/runs/261029-plan-multiproc-v080-final.md:154-160 |
| 6 | 멀티프로세스 U5/U6b/U7/U8 "not started" 기록: 문서 기준이고 v0.8.0 커밋(d9165a5)이 이미 있어 낡은 기록일 가능성 큼(추정). 상태 정정 필요 | status: partial 인 유일한 마커 | notes/runs/261029-haiku-coder-multiproc-integration.md |
| 7 | 릴리스 v0.6.1 PARTIAL "stopped before tag fix" | 낡았을 수 있음(추정). v0.8.0 푸시는 별도 기록 있음 | notes/runs/261029-release-v061.md |
| 8 | 회전 품질 계측(인계 노트 invalid 비율, 추천 보존율, 회전 후 첫 턴 캐시 생성) | 제안 3 | 로드맵 §14(3) |
| 9 | scripts 방어 코드: search-history 빈 검색어 panic, .jsonl.gz 해제 여부, 릴리스 TARGETS(기본 darwin/arm64만) | 제안 5 | 로드맵 §13(8,9), §14(5) |
| 10 | 훅 구조 확정(settings.json 훅 유지 vs --settings 메인 전용, CLAUDE.md -> main.md). --bare 구독 로그인 실패 제약. 위험 높음, 실험 후 | 제안 6, 결정 미확정 | DECISIONS.md:26-28, 로드맵 §13(6) |
| 11 | 검토만 된 항목: stateless conductor Sonnet(3번째 실험군), 커맨더 입력 = 최근 5쌍 + 추천 + 진행 요약(Haiku만), 회전 중 백그라운드 에이전트 대기 resume 미검증 | DECISIONS 에 "unverified/review only" | notes/DECISIONS.md:17,20,22 |
| 12 | 이름 변경 정합: history-export 훅은 bin/chaude, 결정 기록 옛 문구는 bin/harness | 확인 필요 | 로드맵 §13(7) |

불확실: 위 5~7번은 낡은 기록일 수 있음. 1번은 코드 현재 상태를 아직 안 봄.

## Part 2. 결과를 파일 대신 출력창(TUI)에 표시

### 현재 동작 (확인됨)
- TUI 는 메인 모델의 텍스트를 textMsg -> appendAsst(tui.go:386, 361-367) 로 "asst" 블록에 그대로 그림. 즉 메인이 쓴 답은 이미 전부 출력창에 나옴.
- 서브에이전트가 파일에 쓴 결과는 도구 결과(toolResultMsg, repl.go:533-534)로 오며 tui.go:400-408 에서 첫 줄 160자 + "… (+N lines)" 로 잘림. 그래서 사용자는 경로만 보게 됨.
- 파일로 가게 만드는 원인은 코드가 아니라 지시문: CLAUDE.md 와 classifier.go planHint(경로로 plan 을 쓰게 함).
- repl.go:897-905 (passthrough)에는 이미 "reply 가 비면 result 필드를 출력" 패턴이 있음. 일반 턴(repl.go:839 handle)에는 없음.

### CLAUDE.md 에서 충돌하는 줄
- L5: "Main-thread reply ≤ a few sentences; details live in `notes/runs/`, give the path." -> 충돌
- L15: "It writes the answer to `notes/runs/`; reply with the path + 1-line gist. Never copy its text into your reply" -> 직접 충돌
- L20: "Never read details yourself: agents put long output in `notes/runs/` and return the path; give the user the path." -> 직접 충돌 (사용자가 인용한 규칙)
- L17(브리프 "return ≤5 lines"), L12(plan: 경로 + 3줄), L32(chaude 워크플로가 plan/review 를 notes/runs 파일로 작성)는 에이전트 간 전달용이라 유지 가능. 단 L12 의 "reply with the path" 는 최종 사용자 응답에 한해 충돌.
- L9/L11 의 `ESCALATE: <path>` 와 `## [haiku] steps` 경로 전달은 에이전트 간 통신이므로 유지.
- (참고) .claude/agents/*.md 와 이 서브에이전트 지시문에도 "notes/runs/ 에 쓰고 경로만 반환"이 있을 가능성 큼. 이번에 읽지 않음 -> 변경 시 grep 필요.

### 제안 변경 (구현 안 함)
A. CLAUDE.md (사용자 노출 응답만 바꾸고 에이전트 간 전달은 유지)
- L5 끝 `details live in notes/runs/, give the path.` -> `the user's final answer is printed in full in the reply (TUI output pane), never only as a file path; notes/runs/ is for agent-to-agent hand-off.`
- L15 `It writes the answer to notes/runs/; reply with the path + 1-line gist. Never copy its text...` -> `It returns the answer text; relay it in the reply (user wants output in the TUI, not a file). Write notes/runs/ only if the user asks to keep a document.`
- L20 -> `Agents may put long output in notes/runs/ for hand-off, but the final reply to the user must contain the content itself, not just the path.`
- 의미: 메인(Haiku/Opus)이 서브에이전트 결과를 자신의 출력으로 다시 쓰므로 그 출력 토큰 비용이 늘어남(L15 가 막으려던 "Opus 출력 재청구"가 되살아남). 이를 줄이려면 아래 B 로 TUI 가 직접 보여주는 쪽이 비용상 낫다.

B. TUI 코드 (파일 결과를 메인이 다시 쓰지 않고 화면에 직접 표시)
- internal/harness/tui.go:400-408 (`case toolResultMsg`): Agent/Task 도구의 결과는 첫 줄 잘림 대신 전체(또는 상한 N줄, 예 60줄)를 "asst" 가 아닌 새 kind 로 추가. 현재 toolResultMsg 는 도구 이름을 갖고 있지 않으므로 repl.go:533-534 에서 tool_use_id -> 이름 매핑이 필요(이미 tracker.onToolResultText 가 id 를 쓰고 있음, repl.go:306-307). 렌더는 tui.go:307-308 "result" 분기에 줄 수 제한 인자만 추가.
- internal/harness/repl.go:839-841 (`reply, result, usage := r.runTurn(text, true)` 직후): `if strings.TrimSpace(reply)=="" && result != nil { r.ui.send(textMsg(str(result["result"]))) }` -> repl.go:897-905 의 기존 패턴과 동일. 모든 턴 종료 후 최종 결과가 반드시 출력창에 남도록 하는 안전망. 같은 변경을 idleFinish(repl.go:354-381)에도.
- internal/harness/tui.go:435 (`case doneMsg`): 마지막 user 블록 이후 asst 블록이 하나도 없으면 "(응답 없음)" note 를 추가하고 m.vp.GotoBottom() 호출 -> "모든 작업 완료 후 TUI 에 출력" 보장. 스크롤 규칙(CLAUDE.md:6)과 일치.
- internal/harness/classifier.go:81-82 planHint: plan 을 파일에 쓰게 하는 지시. 사용자가 보는 응답에는 영향 없으나, 설계 결과를 보려면 plan 파일 경로 대신 내용 요약을 최종 답에 포함하도록 한 문장 추가 필요.

C. 선택지 (결정 필요)
1. 지시문만 바꿈(A): 구현 최소, 메인이 결과를 재출력해 비용 증가, 품질은 메인 요약에 의존.
2. A + B(repl.go 안전망 + tui.go 도구 결과 전체 표시): 추천. 결과 원문이 화면에 나오고 메인은 짧게 답해도 됨.
3. 파일 생성 자체 금지: 에이전트 지시문 전체 수정 필요(5+ 파일 연동) -> 권하지 않음. 에이전트 간 전달과 compaction 복구가 notes/runs 에 의존함.

추천: 2번. 적용 범위는 CLAUDE.md L5/L15/L20 + repl.go:839 + tui.go:400,435 (파일 3개). 승인되면 CLAUDE.md 는 haiku-coder, Go 두 곳은 sonnet-analyst + 테스트 후 sonnet-reviewer.

## 불확실/미확인
- 도구 이름이 toolResultMsg 에 없다는 점은 정의(tui.go:29, string)로 확인, 매핑 구현 난이도는 미검증.
- 이 서브에이전트 호출 환경의 지시문("notes/runs 에 쓰라")도 같은 충돌 대상이며 CLAUDE.md 변경만으로는 안 바뀜.
