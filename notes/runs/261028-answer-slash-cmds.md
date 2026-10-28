# 슬래시 명령 / 스크롤 결과 (Task 4)

패스스루 방식으로 구현했습니다. 각 명령을 자식 claude(stream-json)에 그대로 보내 본 결과입니다.

| 명령 | 처리 | 패스스루 테스트 결과 |
|---|---|---|
| /usage, /cost | 패스스루 | 구독 사용량 텍스트 그대로 반환 (비용 0, 턴 0) |
| /context | 패스스루 | 카테고리별 토큰 표 반환 |
| /compact | 패스스루 | 신규 세션에서 "Not enough messages to compact." |
| /config, /mcp, /agents | 패스스루 | 텍스트 반환 |
| /init, /review, /doctor | 패스스루 | 모델이 실제 턴을 실행(비용 발생). 프로브만 했고 pty에서 실행은 안 했습니다 |
| /status | 직접 구현 | 자식에서는 프로젝트 스킬로 해석되어 하네스 자체 상태를 표시 |
| /model, /clear, /rotate, /help, /exit, /quit | 직접 구현 | 하네스 고유 기능 |
| /version | 직접 구현 | 자식은 모델이 지어낸 답을 반환 -> `claude --version` 실행 |
| /mouse | 직접 구현(신규) | 마우스 휠 스크롤 on/off 토글 |
| /memory, /permissions, /resume | 미지원 | "isn't available in this environment" 거부. /help·자동완성에서 제외, 실행 시 안내 문구 |

스크롤: 마우스 휠, PgUp/PgDn, shift+↑/↓, Home/End(입력창 비었을 때). 스크롤 중 새 출력이 오면 위치를 유지하고 "↓ 새 메시지" 표시, 맨 아래면 자동 추적합니다. 마우스 캡처 때문에 텍스트 선택이 막히면 /mouse로 끄거나 shift+드래그를 쓰십시오.

캡처: notes/runs/261028-tui-capture4.txt (명령들), 261028-tui-capture5-scroll.txt (스크롤).
