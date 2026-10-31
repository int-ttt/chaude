# Claude Code 과금 공식 출처 검증 (조회일 2026-10-29)

| # | 주장 | 판정 | 출처 URL | 날짜 |
|---|---|---|---|---|
| 1 | Pro/Max 구독에 Claude Code가 플랜 한도 안에서 포함되며 토큰당 과금이 없다 | 확인됨 | https://support.claude.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan | 날짜 미표기 (조회일 2026-10-29) |
| 1 | (보충) 한도 초과분은 사용 크레딧(extra usage)으로 과금될 수 있다 | 확인됨 | https://code.claude.com/docs/en/agent-sdk/cost-tracking | 날짜 미표기 (조회일 2026-10-29) |
| 2 | ANTHROPIC_API_KEY 설정 또는 API 과금이면 구독과 별도의 토큰당 과금이 발생한다 | 확인됨 | https://support.claude.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan ("resulting in API usage charges rather than using your subscription's included usage"; "Usage will be billed at standard API rates (distinct from Pro/Max Plan pricing)") | 날짜 미표기 (조회일 2026-10-29) |
| 2 | (보충) API 키가 설정돼 있으면 Claude Code가 이를 사용한다 | 확인됨 | https://code.claude.com/docs/en/authentication | 날짜 미표기 (조회일 2026-10-29) |
| 3 | 인증 우선순위: API 키가 구독 OAuth보다 우선 적용된다 | 확인됨 | https://code.claude.com/docs/en/authentication (항목 3 ANTHROPIC_API_KEY, 항목 7 /login 구독 OAuth; "If you have an active Claude subscription but also have ANTHROPIC_API_KEY set ... Claude Code uses the API key once you approve it.") | 날짜 미표기 (조회일 2026-10-29) |
| 3 | `claude -p`(비대화형)에서도 구독이 적용되는가 | 부분 | 확인된 문장: "In non-interactive mode (-p), the key is always used when present." (https://code.claude.com/docs/en/authentication). `-p`에서 구독 OAuth가 적용된다는 명시 문장은 확인하지 못함 | 날짜 미표기 (조회일 2026-10-29) |
| 4 | `total_cost_usd`는 구독자에게 실제 청구액이 아니라 추정치다 | 부분 | https://code.claude.com/docs/en/agent-sdk/cost-tracking ("client-side estimates, not authoritative billing data"); https://code.claude.com/docs/en/costs ("The figure is an estimate, so for authoritative billing see the Usage page in the Claude Console."); https://code.claude.com/docs/en/headless ("client-side estimates"). 구독자 전용 문장은 없음 | 날짜 미표기 (조회일 2026-10-29) |
| 5 | 5시간 한도는 여러 동시 세션에 걸쳐 공유된다 | 확인 안 됨 | https://support.claude.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan 은 5시간 세션 한도와 주간 한도를 명시하나 동시 세션 공유 여부는 언급하지 않음. 제3자 가이드(claudelog, ai-toolbox 등)는 공유 풀이라고 하나 공식 출처가 아님 | 날짜 미표기 (조회일 2026-10-29) |

## 참고
- 공식 페이지 날짜는 확인하지 못함. 날짜 열은 조회일 기준.
- https://support.claude.com/en/articles/12429409-extra-usage-for-paid-claude-plans 은 cost-tracking 페이지에서 링크로만 확인했고 직접 fetch하지 않음.
- 제3자 출처(검색 스니펫)의 "Max 구독 API 크레딧 사용 불가" 주장은 검증하지 못해 판정에 쓰지 않음.
- 확인 안 됨 항목: 5번 (동시 세션 공유), 3번의 `-p` 구독 적용 명시.
