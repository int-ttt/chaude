# /resume claude 필터 + 라우터 자체 판단 실행 가능성

## Task 1 요약 (구현 완료)
- 현황: /resume 목록은 이미 Claude Code transcript(~/.claude/projects/<cwd>/*.jsonl)만 읽음. 코드에 다른 소스는 없음 (grep 확인).
- 변경: `/resume` 와 `/resume claude` 는 동일하게 Claude 대화만 나열, 그 외 인자(`/resume codex`, `/resume claude x`)는 사용법 안내 후 중단.
- 변경 파일:
  - internal/harness/resume.go: resumeSourceMsg 추가, sessionList/resumeLine 이 인자를 받도록 변경
  - internal/harness/repl.go: plain REPL 에서 resumeLine(f[1:]...) 전달
  - internal/harness/tui.go: TUI picker 에서 sessionList(f[1:]...) 전달, 자동완성 설명에 "(/resume claude)" 표기
  - internal/harness/resume_test.go: TestResumeSourceArg 추가
- 테스트: `go test ./...` 통과 (chaude/internal/harness ok, gofmt 깨끗).
- 주의: 다른 소스가 아직 없어서 "Claude만" 필터는 현재 동작과 같고, 앞으로 소스가 추가될 때 인자로 구분하는 틀이 생긴 것입니다.

## Task 2 답변 (한국어)
질문: 현재 구조에서 디스패처/라우터가 Opus로 명령을 넘기지 않고 Opus가 아닌 모델이 스스로 판단해서 프롬프트를 실행할 수 있는지.

결론부터 말씀드리면, 라우터 자체가 판단해서 직접 실행하는 것은 지금 구조에서 불가능하고, "Opus가 아닌 모델이 메인으로 실행"되는 경로는 제한적으로만 있습니다.

### 근거 (코드)
- classifier.go: 분류기는 `claude -p --model sonnet` 단발 호출이며 도구와 세션이 없습니다. 출력은 topic(same|new), size(small|large), task 세 줄뿐이고 "Never answer or explain"으로 답변을 금지합니다. 즉 분류기는 실행 주체가 아니고 task 문장을 만들 뿐입니다.
- pickModel: 모델은 분류기가 고르는 것이 아니라 이 함수가 topic/size로 결정합니다. 현재 모델이 opus(또는 비어 있음)이면 항상 "opus"를 반환하고, topic=new 이면서 size=small 이면 delegate=true만 켭니다. 메인이 haiku가 되는 경우는 없습니다("The main session is never haiku").
- delegate=true 일 때: task 앞에 delegateHint를 붙여 Opus 메인 세션에 그대로 보냅니다. 힌트는 "haiku-scout 등 서브에이전트에 위임하라"는 문구일 뿐이라, 프롬프트는 여전히 Opus가 받고(컨텍스트와 토큰 비용 발생) 위임 여부도 Opus가 결정합니다. 위임 이행은 보장되지 않습니다.
- Sonnet이 메인으로 실행되는 경우: 사용자가 /model 로 Sonnet을 강제한 뒤에만 가능하고, 그 상태에서는 topic=new + size=large 일 때만 Opus로 올라가고 나머지는 Sonnet 유지입니다. 라우터가 Opus→Sonnet 으로 내리는 경로는 없습니다.
- 훅(agent-model-pin.py): 서브에이전트 호출 시 모델을 에이전트 파일의 모델로 고정하고, 서브에이전트 안에서는 haiku-* 와 sonnet-reviewer 만 호출 가능하게 막습니다. 서브에이전트를 부르는 주체는 메인(Opus)이어야 하며, 훅은 라우팅을 결정하지 않고 제약만 합니다.
- 라우팅 규칙은 CLAUDE.md 문구와 delegateHint에 있어 모델이 따르는 "권고"이지 코드 강제가 아닙니다.

### 지금 가능한 것
- 분류기(Sonnet)가 topic/size 판단 → 사용자가 Sonnet 메인을 쓰는 중이면 그 세션 유지 또는 Opus로 승격.
- Opus 메인이 힌트를 받아 haiku/sonnet 서브에이전트에 위임(Opus가 판단하고 실행 지시).

### 지금 불가능한 것
- 라우터가 Opus를 거치지 않고 Haiku/Sonnet 세션에 프롬프트를 직접 보내 실행시키는 것(Opus 기본값에서의 하향 라우팅).
- 라우터 스스로 도구를 써서 실행하는 것(분류기에 도구가 없음).
- 위임을 강제하는 것(힌트는 권고).

### 필요한 변경
1. pickModel 하향 규칙 추가: Opus 기본에서 topic=new + size=small 이면 delegate 대신 model="sonnet"(또는 haiku) 반환. 기존 코드의 rotate/start 경로(route())가 이미 모델 전환을 처리하므로 재사용 가능하나, 전환마다 handoff·캐시 손실 비용이 든다는 주석의 이유(현재 delegate 선택 근거)를 다시 따져야 합니다.
2. 또는 분류기 출력에 `model:` 줄을 추가하고 parseRoute가 검증 후 사용(현재는 파생값). 안전장치로 허용 목록과 size=large 는 opus 고정 규칙 필요.
3. "메인은 haiku 불가" 규칙을 풀려면 haiku 메인 세션의 도구/훅 세트를 별도 정의해야 합니다(DECISIONS의 worker 훅 세트 구상과 맞닿음).
4. 가장 가벼운 대안: Opus를 거치지 않는 별도 워커 세션(`claude -p` 하위 세션)으로 task를 직접 실행하고 결과만 메인에 알림. DECISIONS [arch] 멀티 세션 워커안과 같은 방향이며 실험 전이라 검증되지 않았습니다.

불확실한 점: 위 변경 후 실제 비용·품질(핸드오프 손실, 캐시 미스)은 측정하지 않았고, 코드 읽기 기반 추론입니다.
