# plan: router 3-tier tag (haiku|sonnet|opus)
status: draft v2 (Opus 리뷰 반영), 코드/CLAUDE.md/DECISIONS.md 미수정
review: notes/runs/261029-plan-router-3tier-review.md

## 목표
- 분류기 출력 `size: small|large` -> `tier: haiku|sonnet|opus`
- 이유: large 한 덩어리에 분석/리뷰(Sonnet 적합)와 설계/전략(Opus 적합)이 섞여 항상 planHint(Opus 리뷰 경로)로 감
- 비목표: 메인 세션 Haiku 고정 유지(pickModel 불변), kind/worker/topic 변경 없음, opus tier 워크플로 변경 없음

## 현재 동작 (근거)
- classifier.go:15-23 classifyPrompt: `size: small|large`(:16), 정의(:18-19), 모호 시 large(:20), "small인 lookup/edit은 Haiku로"(:22)
- pickModel :39-47: 강제 opus 메인 + topic==new + size==small 일 때만 delegate=true(:44)
- planHint 부착 :101-103: `model=="haiku" && size=="large"`; planHint 상수 :32-34
- parse :77,91-94: small/large 아니면 large + SizeDefaulted; routeResult :55-59; 메트릭 :167-170
- Size 사용처: classifier.go + conductor_test.go 만 (A1 확인됨). metricscmd.go 는 size 미사용 (A2 확인됨)

## 새 태그 정의 (결정 3 반영)
- haiku: 단순 조회/검색/카운트/실행·모니터/한 단계 편집(한 파일, 범위 명확); 버전 bump(정확한 문구, 동일 문구 교체)
- sonnet: 분석, 리뷰, 비교 정리, 여러 파일 코드 변경, 범위가 분명한 다단계 작업; release/deploy/merge; 신규 기능·리팩터링(범위가 주어진 경우)
- opus: 설계, 아키텍처, 트레이드오프/추천, 전략/우선순위/로드맵, 원인 불명 진단, 반복 실패, `opus로`; 신규 기능·리팩터링이라도 설계 미정이면 opus
- 모호 시 sonnet (haiku/sonnet, sonnet/opus 경계 모두). opus 트리거 목록은 분류 프롬프트에 명시 유지(unsure 범위 좁게)
- classifyPrompt :22 문구 -> "A lookup or edit that is tier haiku is sent straight to a Haiku session"
- 길이: 현재 대비 +300자 이내
- 메트릭: `tier` 추가, `size` 파생 유지(haiku->small, sonnet|opus->large), `size_defaulted` 키 이름 유지

## 동작 (결정 1, 2 반영)
- haiku: hint 없음, Haiku 에이전트가 직접 수행
- sonnet: planHint 없음, Haiku plan 단계 없음. sonnetHint 로 한 번의 foreground 라운드에 sonnet-analyst 직접 위임. `ESCALATE: <path>` 는 Haiku plan 단계 없이 opus-architect 에 그 경로 그대로 전달. sonnet-reviewer 는 release/merge/제출 전에만
- opus: 기존 plan->review->merge 유지(planHint). 리뷰어는 opus-architect 단독(planHint 단계 2에서 "and/or sonnet-analyst" 삭제). planHint 접두 "[routing: large or judgment task." -> "[routing: opus tier (design/judgment)."
- sonnetHint 문구(새 const): "[routing: sonnet tier. Do not write a plan. In ONE foreground round delegate to sonnet-analyst: user's words quoted + target paths + 'reply: Korean 존댓말'. If it returns ESCALATE: <path>, give that path to opus-architect as is (no Haiku plan step). sonnet-reviewer only before release/merge/submission.]\n"
- hint 는 Haiku 메인에서만 부착. 강제 opus/sonnet 메인은 hint 없음. 강제 opus 메인 + new + tier haiku -> delegate(구 small과 동일)
- 기본값 sonnet 근거: 분류기=Sonnet 30/30 정답, 파싱 실패 0; Haiku 오분류는 전부 상향. 안전망은 sonnet-analyst 의 ESCALATE. (sonnet 직접 위임을 바꾸면 기본값은 opus 로 재검토)

## 파싱 호환
- `tier:` 와 구 `size:` 동시 존재 -> tier 우선
- size 만: small->haiku, large->sonnet, SizeDefaulted=true
- 누락/무효 -> sonnet, SizeDefaulted=true. 과거 jsonl 은 그대로 유효

## 구현 단위 (순서대로, 단위 = 파일 1개 + 함수 1개)
코드 (internal/harness/classifier.go)
1. classifyPrompt: 출력 줄/정의/모호/:22 문구를 tier 로 교체
2. routeResult(타입): Tier 필드 추가, Size 는 haiku->small 파생
3. parseRouteFull: tier 파싱 + 호환 규칙 + 기본 sonnet
4. pickModel: 인자 size->tier, delegate 조건 `tier=="haiku"`
5. sonnetHint/planHint(상수, 함수 아님): sonnetHint 신규, planHint 접두/단계 2 수정, :30-31 주석 "large task"->"opus-tier task"
6. route(): hint 분기(opus->planHint, sonnet->sonnetHint, haiku->없음) + 메트릭 `tier`/파생 `size`/`size_defaulted`
테스트 (internal/harness/conductor_test.go, 구현 전 lines 22-141 필독, "인자만 변경" 판단은 틀림)
7. TestParseRouteSize (:22): 케이스 :27-30 (missing->sonnet, "Small."->haiku, medium->sonnet), :47-51 단언 반전(planHint 없음, sonnetHint 있음); size-only 호환 케이스 1개 유지
8. TestMainRouteSwitchPolicy (:83): :103 "size: small|large"->"tier: haiku|sonnet|opus"; 입력 :88-93 `size:`->`tier:`; 강제 opus 메인 sonnet tier 무 hint 케이스 추가
9. TestHaikuPlanWorkflow (:119,:121): `size: large`(plan=true)가 sonnet 으로 매핑돼 실패 -> `tier: opus`(plan=true)로 교체 + `tier: sonnet`(plan=false, sonnetHint=true) 추가; :136 목록에서 "sonnet-analyst" 제거("opus-architect","notes/runs/","Haiku agents" 확인)
10. TestClassifyPromptSizeRule (:54): 두 문자열을 "tier: haiku|sonnet|opus" 와 새 unsure(sonnet) 문구로 교체
11. TestSonnetTierHint (신규): sonnetHint 가 "sonnet-analyst","ESCALATE","opus-architect" 포함, "plan-<topic>","write the full design" 미포함; sonnet=sonnetHint 접두+planHint 없음, haiku=둘 다 없음
비정책 문구 (코드와 같은 커밋, 승인 불필요)
12. repl.go flag help (:898 해당 함수): "tags size and adds the plan->review->execute hint to large tasks" -> "tags tier (haiku|sonnet|opus); sonnet tier -> sonnet-analyst hint, opus tier -> plan->review->execute hint"
13. repl.go (:60 해당 타입/함수) 주석: "large work:" -> "opus-tier work:"
- 보존: .claude/agents/opus-architect.md:22, sonnet-analyst.md:23 Plan review 문단

## 정책 변경 (사용자 승인 후, 구현 단위 아님)
- notes/DECISIONS.md: 새 줄 append (:38 수정 금지; "Newest at bottom" 관례 확인됨). 제안: "- [routing] 261029 user: <사용자 원문> -> classifier tags tier haiku|sonnet|opus (size kept as derived metric: haiku=small, sonnet|opus=large). haiku: Haiku agents direct; sonnet: Haiku delegates directly to sonnet-analyst (no plan; ESCALATE -> opus-architect); opus: planHint workflow (Haiku plan -> opus-architect review -> merge -> Haiku agents). Missing/invalid tier -> sonnet. Supersedes the size part of the [routing] 261029 Haiku-fixed entry. Tests: TestParseRouteSize, TestHaikuPlanWorkflow, TestSonnetTierHint."
- CLAUDE.md:32 변경 5개:
  a. "only tags size" -> "only tags tier (haiku|sonnet|opus)"
  b. "Large/judgment/design task (arrives with a `[routing: large ...]` hint) → workflow:" -> "Opus-tier task (design/architecture/trade-off/strategy/unknown-cause diagnosis/repeated failure; arrives with a `[routing: opus tier ...]` hint) → workflow:"
  c. 단계 (2) 에서 "and/or `sonnet-analyst`" 삭제
  d. "Small tasks" 앞에 삽입: "Sonnet-tier task (analysis, review, comparison, multi-file change, scoped multi-step, release/merge; arrives with a `[routing: sonnet tier ...]` hint) → Haiku delegates it directly to `sonnet-analyst` in one foreground round, no plan; `ESCALATE: <path>` → `opus-architect` with that path."
  e. "Small tasks: Haiku does them via Haiku agents directly." -> "Haiku-tier tasks: ..."
  - 불변: "Opus is for design only, never main.", 강제 `opus로`/`sonnet으로`, 마지막 문장, CLAUDE.md:9, :12

## 검증 순서
1. 30샘플 라벨 파일을 호출 전에 고정(notes/runs/261029-test-router-3tier-labels.md): 기존 P1-P10/N1-N10/L1-L10 재사용, L*/large P* 를 sonnet/opus 로 분할, P3->haiku, opus·sonnet 각 >=10 되도록 추가
2. 단위 1-13 (haiku-coder 1회 brief, 스펙=본 plan+리뷰)
3. `go test ./internal/harness -run 'ParseRoute|ClassifyPrompt|MainRouteSwitch|HaikuPlan|SonnetTier|CategoryRuleSize'` 후 `go test ./...`
4. 30샘플 라이브(haiku-runner): 분류기 Sonnet(`conductorModel`), classify() 경유, 건당 ~$0.007 -> ~$0.21, 상한 $2.50 중단
5. 합격선: 정확도 >=80%, opus->sonnet <=2/10, opus->haiku 0, sonnet->haiku <=1, tier 누락 0. 혼동행렬 기록
6. 승인 후 CLAUDE.md/DECISIONS 수정(haiku-batch-worker, 위 정확한 문구)

## 반영 기록
- 결정1 적용: sonnet tier 직접 위임(옵션 a), (b) 기각, ESCALATE->opus-architect
- 결정2 적용: opus tier 유지, opus-architect 단독 리뷰, 접두 rename, ESCALATE 는 Haiku plan 생략
- 결정3 적용: 기본 sonnet, size_defaulted 기록, 신규 정의 3종(버전 bump=haiku, release/merge=sonnet, feature/refactor=범위별), tier 우선 파싱
- 결정4 적용(계획에 문구만): DECISIONS 새 줄, :38 불변, CLAUDE.md 5개 문구 + repl.go :898/:60 + classifier.go :30-31 주석
- 결정5 적용: A1/A2 해소, conductor_test.go 단위 7-11 구체화, "인자만 변경" 정정, 구현 전 22-141 필독
- 보류: (1) CLAUDE.md/DECISIONS 실제 수정=사용자 승인 대기, (2) TestClassifierLive tier 필드 추가=선택, (3) routeFields 헬퍼 분리 테스트=선택, (4) size 파생 유지 기간=미정
- 열린 질문 해소: Q1=(a), Q2=수용(호환 규칙), Q4=30샘플로 판단. Q3 보류
