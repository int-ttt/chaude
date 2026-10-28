# e2e 재구축 테스트 (빈 폴더에서 하네스로 프롬프트 6개)
- 위치: tests/e2e-rebuild/ (.gitignore 처리, .gitkeep만 추적 대상). 모델 haiku, threshold 35000, max-auto-rotations 4, 상한 선언: 프롬프트 6개·전환 4회(비용 약 $0.3 이하).
- 결과: 6턴 모두 완료(235초), 전환 4회, handoff_valid 4/4, reco_preserved 4/4(메트릭 기준), 턴 ctx 32.5k-37.3k. 산출물: go.mod, core.go(+test), cmd/rebuild/{main,repl}.go(+test), metrics 기록. 자식 `go build`/`go test` 통과(제가 재실행 확인).
- 비용 표시([…$])는 세션별 누적이라 합산 불가; 대략 $0.28 [추정].
- 한계: 산출물은 축소판(메트릭/전환 골격). 모델이 위임(백그라운드 3개)을 했으나 하네스 트래커의 위임 수는 메트릭에 안 남김(미구현). 5번째 턴 이후 서브에이전트는 완료 전에 종료됐을 수 있음(마지막 응답이 "알림 대기").
- 원본: tests/e2e-rebuild/t.jsonl, notes/metrics/2026-10-28.jsonl.
