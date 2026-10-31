# OAuth 경계 검토 (정책 문구 기준 해석, 법률 판단 아님)

출처: notes/research/claude-oauth-policy-and-cli.md, notes/runs/261031-haiku-researcher-claude-oauth-policy.md
주의: 인용문은 WebFetch 모델 추출본이라 법적 의존 전 원문 재확인 필요. 자격증명 값은 다루지 않음.

## 사용자 주장
"Go 클라이언트는 Claude Code REPL만 연결하고 OAuth 토큰은 건드리지 않으면 됨. 외부 클라이언트 차단은 Claude Code 경유 방식은 감지 불가, 직접 서버 전송만 해당."

## 판정: 부분적으로 맞음

### (1) Go가 claude CLI를 자식 프로세스로 띄우는 구조
근거(공식 1차, legal-and-compliance 페이지):
- "OAuth authentication is intended exclusively for purchasers of ... subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications."
- "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code ..."
- 금지: "route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users", "may not collect, store, or intermediate Claude.ai credentials or session tokens".
해석(추론):
- 수정 안 된 claude 바이너리에 사용자 본인이 로그인하고 Go가 토큰을 읽거나 저장하지 않으면, 문구상 허용되는 쪽에 가까움.
- 단, "Claude Code를 경유하면 무조건 안전"이 기준은 아님. 기준은 (a) 바이너리 무수정, (b) 사용자 본인 구독, (c) 토큰 비수집/비중계, (d) 타인 대신 라우팅 아님.
- 개인 사용을 넘어 다수 사용자 대상 서비스로 제공하면 "products or services" 항목(API 키 사용 요구)에 걸릴 수 있음: 미확인.
- claude -p / Agent SDK는 구독 한도를 쓴다는 지원 문서 있음(사용량 한도 문서이며 토큰 정책 아님). 2026-06-15 변경 보류 공지 있음.
- Consumer Terms의 OAuth 조항: 미확인.

### (2) 서버가 "Claude Code 경유 여부"를 감지 가능한가
증거:
- 2026-01-09 서버측 보호 조치 보도(2차, paddo.dev): 오류문 "This credential is only authorized for use with Claude Code", 직원 발언 "tightened our safeguards against spoofing the Claude Code harness". 즉 서버가 Claude Code가 아닌 요청을 구분하려 한 정황이 있음.
- 계정 정지 사례는 OAuth 프록시 대상(사용자 보고, 비공식).
추론:
- 서버는 클라이언트 요청 형태(헤더, 프롬프트 구조 등)로 판별할 가능성이 높고, 진짜 claude 바이너리가 보낸 요청은 구분이 어려움. 따라서 "CLI 경유는 감지 불가"는 그럴듯하나 증명된 사실 아님.
한계:
- 공식 발표된 차단 기준/방법: 미확인(Jan-Oct 2026 공식 공지 못 찾음).
- 감지 불가여도 약관 위반 여부와는 별개. 감지 가능성은 준수 근거가 되지 못함.
- 2026-04-04 제3자 도구 구독 인증 차단 보도: 2차, 미확인.

### (3) 직접 API 호출만 위반인가
- 아님(부분). 직접 호출(토큰을 꺼내 서버로 보내기, 프록시)은 명백히 금지 쪽. 그러나 위반이 그것만은 아님: 토큰 수집/저장/중개, Claude.ai 로그인 제공, 타인 대신 구독 크리덴셜 라우팅, 수정된 바이너리/하니스 위장도 문구상 해당 가능.
- Anthropic은 "enforce ... without prior notice" 권리를 명시.
- 2026-02 "Agent SDK 포함 금지" 문장은 현재 페이지 본문에 없고 2차 보도뿐: 미확인.

## 권고(추론)
- Go는 claude를 무수정 자식 프로세스로 실행, ~/.claude 자격증명 파일/키체인 접근 금지, 토큰 로깅/복사 금지, 개인 본인 구독 사용 한정.
- 다중 사용자 서비스화 시 API 키 인증으로 전환 검토.

## 미확인 목록
- Consumer Terms OAuth 조항
- 서버 차단 방식의 공식 설명
- 2026-01-09, 2026-04-04 조치의 1차 출처
- 자식 프로세스 방식에 대한 명시적 공식 허용 문구(간접 근거만 있음)
- 현재 페이지 날짜(undated)
