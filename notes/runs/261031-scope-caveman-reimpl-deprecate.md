# scope: caveman 재구현 + 모델제어 deprecated 이동 + v0.9.0 재릴리즈

status: ESCALATE (구현·삭제·릴리즈 미실행)
read: plan/caveman-structure.md 1-248행 읽음, 249-255행 미확인 (U8 전달문, U9/U10 항목으로 추정)

## 충돌 (요구사항 간)
- C-A: 사용자 요청 "라우터 등 모델 제어 요소 전부 deprecated/ 이동" vs 계획서 1절 G4 "기존 에이전트 체계·모델 고정·메인 스레드 가드 동작은 그대로 둠" 및 9절 "바꾸지 않는 것: main-thread-guard.py, agent-model-pin.py 동작". 후보에 agent-model-pin.py가 있어 직접 충돌. haiku-context-guard.py, settings.json 모델 설정도 같은 범주.
- C-B: 계획서 7절 라우팅(orchws.go ws 파싱, haiku 세션 재개)과 classifier.go/haikupool.go 이동이 겹칠 수 있음. internal/harness 패키지 의존(repl.go 등) 미조사. 이동 시 빌드·테스트 깨짐 가능.
- C-C: "plan대로 구현"은 이미 v0.9.0(커밋 27c60e2 "release: v0.9.0 (caveman S0~S5)", 병합 0ee3501)에서 S0~S5 구현·병합됨. 새로 구현할 항목이 무엇인지(재구현인지, 누락 보완인지) 불명.
- C-D: 계획서 8절 "master 작업 트리 직접 적용 금지, 단계별 worktree 브랜치". 현재 master에 미커밋 변경(notes/LEDGER.md 등) 있음. 요청은 master 직접 작업.

## 열린 결정 (사용자만 답 가능)
- Q-A: 모델 고정(agent-model-pin.py, haiku-context-guard.py, main-thread-guard 포함 여부)을 deprecated로 옮기면 계획서 G4/9절을 개정하는지?
- Q-B: 이동 범위 확정 목록: classifier*, haikupool*, routerprompt_test, hooks 2종, settings.json 모델 설정 중 어디까지.
- Q-C: 구현 대상 = v0.9.0에서 달라지는 점이 무엇인지.
- Q-D: 새 버전 번호(0.9.1? 0.10.0?)와 worktree 사용 여부.

## 옵션
- O-1: 계획서를 그대로 유지(모델 고정 유지), 라우터(classifier/haikupool/routerprompt_test/classifier_model_test)만 이동. 위험 낮음.
- O-2: 요청대로 모델 제어 전부 이동 + 계획서 9절 개정. 위험: 서브에이전트 모델 고정 상실, 빌드 의존 깨짐.
- O-3: v0.9.0 유지, 재릴리즈 없이 diff 보고. 

## 증거 없음
- 어느 설계가 맞는지 증거 없음. 삭제·릴리즈는 비가역이라 보류.
