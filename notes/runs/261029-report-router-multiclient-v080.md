# 라우터 교체 · 멀티클라이언트 · v0.8.0 점검 보고

근거: haiku-scout 3묶음 요약 + haiku-runner 실행 결과 (제가 원문을 직접 대량으로 읽지는 않았습니다. 요약 기반이라 일부는 미확인입니다.)

## 1. 라우터 (교체 / Haiku 최적화 / Sonnet deprecated)
- 환경변수 CHAUDE_ROUTER_MODEL(sonnet|haiku) 추가됨. 기본값은 여전히 sonnet. 잘못된 값이면 경고 후 sonnet. (코드 확인)
- 모델별 프롬프트 상수 분리됨. Haiku 프롬프트는 "초안1 동결" 주석만 있고 실제 튜닝 결과는 미확인.
- 150건 평가셋(모델별 50개) 파일은 있으나 실행 결과 없음. 통과 기준 판정 미실시.
- Sonnet 프롬프트에 Deprecated 주석만 추가됨. 기본값 유지, 실제 교체(기본값 haiku 전환) 안 됨.
- CLAUDE.md:32 (분류기=Sonnet 고정)는 낡은 상태. DECISIONS.md에 승인 원문 없음(미확인).
- 결론: 요청된 "교체 / 최적화 / deprecated" 중 deprecated 주석과 스위치만 구현, 교체와 최적화는 미완.
- 근거 표본(이전 대화): Haiku 90회 중 3회 오분류, haiku로 과소 분류 0건. 표본 작음(sonnet 4개, opus 11개).

## 2. 멀티프로세스(멀티클로드) U-step
LEDGER에는 U1a만 기록(LEDGER:202). 나머지는 integration note와 코드 grep 기반.
- 완료(코드): U1a/b/c clients.go, U2a withCacheTTL, U3a parseCacheTTL, U3b 플래그(--cache-ttl, --idle-rotate, --clients-file), U4a/b children.go (Repl에 미연결), U6a pickChild.
- 부분: U2b observedTTL (함수만, 호출처 없음), U8b 고아 정리 (reconcile 있음, orphan-alive 사유와 /clear CloseAll 없음).
- 미착수: U5(a,b,c,d), U6b, U7, U8, U9(budget.go, 버전, CHANGELOG).
- 미확인: U0a(미커밋 분리, 승인 필요), U0b(TTL 게이트 호출), U10a/b(DECISIONS 갱신, 승인 필요).
- 남은 순서: U5a -> U5b -> U5d -> U5c -> U6b -> U7 -> U8 -> U8b -> U9 -> U10a/b.
- 차단: U0a 승인, U0b 게이트 미실행, 0.01 USD 스모크 미실행.
- 즉 멀티클라이언트 핵심(Repl 연동, 유휴 정리, 종료 경로)은 구현되지 않았습니다.

## 3. v0.8.0 릴리즈 준비
- go build ./... PASS, go vet ./... PASS, go test ./... PASS (chaude/internal/harness 0.930s).
- 버전: cmd/chaude/main.go:16 = "0.7.2" (유일한 코드 원천). 0.8.0 아님.
- CHANGELOG.md 최상단 0.6.6, 0.6.7~0.7.2 및 0.8.0 항목 없음. README는 0.6.6 표기(낡음). dist는 0.6.1.
- 태그: v0.7.2까지, v0.8.0 없음. 미커밋 변경 52개(수정 + 신규 다수).
- 버전 번호 충돌: plan-v0.8-impl.md는 0.8.0(멀티프로세스는 0.9.0이라 CHANGELOG에 "multi-client" 표기 금지), plan-multiproc-rebind.md는 같은 변경을 0.7.3으로 지칭.
- 판단: 사용자 요청의 "멀티클로드 구현 후 v0.8.0"은 전제(구현 완료)가 충족되지 않아 릴리즈 불가.

## 사용자 승인 필요 항목
1. 미커밋 변경 분리/커밋 (U0a)
2. 라우터 기본값을 haiku로 전환 + CLAUDE.md / DECISIONS.md 수정 (승인 원문 필요)
3. 버전 번호 확정 (0.7.3 vs 0.8.0 vs 0.9.0)과 CHANGELOG 작성
4. 비용 발생: 150건 평가 실행, TTL 게이트 호출, 0.01 USD 스모크
5. 커밋, 태그, push, 릴리즈

## 불확실성
- scout 요약 기반. router 쪽 scout는 상세 요약 저장이 제한되어 한 단락만 회신됨. 필요 시 재조회.
- U1b~U6a 완료 판정은 LEDGER 기록 없이 코드/노트만 근거.
