# 261031-plan-verify-via-go-review
verdict: 실행 가능, 아래 보완 반영
improvements:
- risk: 트랜스크립트가 부가 호출(제목 생성·compaction)을 기록하지 않을 수 있음 -> 13번은 "기록 없음"을 "호출 없음"으로 결론 내지 말 것. api-request 줄의 분류 필드(querySource 등) 존재 여부를 먼저 확인
- risk: 2번 첫 턴 프롬프트 토큰 비교는 CLAUDE.md·--append-system-prompt·모델이 다르면 무의미 -> 모델별·entrypoint별로 나누어 보고, 차이는 "원인 미확인"으로 표시
- missing_step: 6번은 트랜스크립트만으로 부족. -p 자식은 stream stdout에 이벤트가 없으면 요청 없음으로 볼 수 있으나 인터랙티브는 옵션 B 전까지 부분 확인
- ordering: E3(/clear)는 사용량을 소모하므로 E1·E2 뒤에 1회만
- agent_choice: Go 스캐너는 저장소 모듈 밖(/tmp/ccscan)에 두어 go vet ./... 영향 없게 유지 (계획대로)
- follow_up: 상시 운영용이면 스캐너를 internal/harness/transcript.go 로 이식하는 별도 계획 필요 (이번 범위 아님)
