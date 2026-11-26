# chaude vs Anthropic terms (third-party GUI/TUI wrapping local claude CLI)

fetched: 2026-11-07
scope: chaude = GUI/TUI that spawns user-installed `claude` as subprocess and shows output
auth_cases: (A) claude.ai Pro/Max OAuth login, (B) Anthropic API key

## Source status
- legal-compliance: https://code.claude.com/docs/en/legal-and-compliance | English | no date shown
- agent-sdk-overview: https://code.claude.com/docs/en/agent-sdk/overview | English | no date shown
- authentication: https://code.claude.com/docs/en/authentication | English | no date shown
- commercial-terms: https://www.anthropic.com/legal/commercial-terms | English | effective Jun 17, 2025
- consumer-terms: https://www.anthropic.com/legal/consumer-terms | fetch returned Korean only | effective Oct 8, 2025 (시행일 2025년 10월 8일)
- aup: https://www.anthropic.com/legal/aup | fetch returned Korean only | effective Nov 12, 2026 (future vs 2026-11-07)
- note: Korean quotes below are as returned by fetcher (model-rendered), not verified against original Korean text.
- note: English wording of Consumer Terms and AUP not retrieved; use the Korean quotes only as pointers.

## Clause table

| # | clause | source | verdict | quote (as fetched) |
|---|---|---|---|---|
| 1 | Running Claude Code inside your product | legal-compliance, "Can customers offer Claude Code in their products?" | restricted (conditional) | "preinstalling or running Claude Code in your products or services ... requires agreeing to our Commercial Terms of Service and complying with the conditions below" |
| 1a | binary must not be modified | same | restricted | "The Claude Code binary must not be modified. Claude Code must be installed and run as published by Anthropic" |
| 1b | no reselling/intermediating end-user usage | same | restricted | "Customers may not pay for, resell, or intermediate Claude usage on their end users' behalf. Each end user must authenticate with their own Anthropic API key, Claude subscription plan credentials, or 3P inference provider credential" |
| 1c | naming/logo | same | restricted (name) / allowed (plain text) | "You can accurately say, in plain text, that your product ... runs Claude Code. But you can't use the Claude Code or Anthropic names or logos as part of your own product, feature, or company name" |
| 2 | Subscription OAuth scope | legal-compliance, "Authentication and credential use" | restricted (for developers) | "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Ultra, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications." |
| 2a | developer offering claude.ai login / routing plan credentials | same | restricted | "Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users." |
| 2b | collecting/storing tokens | same | restricted | "developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow." |
| 2c | end user signs in to unmodified binary | same | allowed | "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code" |
| 2d | enforcement | same | - | "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice." |
| 3 | Agent SDK login/rate-limit rule | agent-sdk-overview, callout | restricted (SDK-built products) | "Unless previously approved, Anthropic does not allow third party developers to offer claude.ai login or rate limits for their products, including agents built on the Claude Agent SDK. Use the API key authentication methods" |
| 3a | Agent SDK branding | agent-sdk-overview, "Branding guidelines" | restricted (name) | Not permitted: "Claude Code" or "Claude Code Agent"; "Claude Code-branded ASCII art or visual elements that mimic Claude Code". Allowed: "Claude Agent", "{YourAgentName} Powered by Claude". "Your product should maintain its own branding and not appear to be Claude Code or any Anthropic product." |
| 3b | Agent SDK licensing | agent-sdk-overview, "License and terms" | restricted (scope) | "Use of the Claude Agent SDK is governed by Anthropic's Commercial Terms of Service, including when you use it to power products and services that you make available to your own customers and end users" |
| 4 | Subprocess use of CLI | agent-sdk-overview, intro | allowed (signal) | "To drive the same agent loop from a language other than Python or TypeScript, run the CLI as a subprocess with the -p flag and --output-format json." |
| 5 | Login methods / credential storage | authentication | info | "Claude Desktop and cloud sessions do not call apiKeyHelper or read these environment variables" ; subscription login is default for Pro/Max ("Subscription OAuth credentials from /login ... default for Claude Pro, Max, Ultra, Team, and Enterprise users") |
| 6 | Consumer Terms: automated/non-human access (Sec.3 item 7) | consumer-terms (Korean fetch) | restricted unless API key or explicit permission | "봇(bot), 스크립트(script) 또는 기타 방식을 통해 자동화된 또는 비인간적인 방식을 통해 당사 서비스에 접속" ; exception: "귀하가 Anthropic API 키를 통해 당사 서비스에 접속하거나 당사가 이를 명시적으로 허용하는 경우" |
| 7 | Consumer Terms: crawling/scraping (Sec.3 item 4) | consumer-terms (Korean fetch) | restricted | "본 약관에서 허용된 경우를 제외하고 당사 서비스에서 데이터 또는 정보를 크롤링(crawl), 스크래핑(scrape) 또는 다른 방식으로 수집" |
| 8 | Consumer Terms: competing products (Sec.3 item 2) | consumer-terms (Korean fetch) | restricted | "당사 서비스와 경쟁하는 상품 또는 서비스 개발" |
| 9 | Consumer Terms scope | consumer-terms preamble (Korean fetch) | info | Commercial Terms govern API key/Console; Consumer Terms cover Claude.ai/Pro. Legal-compliance page: Consumer Terms govern "Free, Pro, Max, and Ultra users" |
| 10 | Commercial Terms: no resale | commercial-terms D.4(a) (English) | restricted | "resell the Services except as expressly approved by Anthropic" |
| 11 | Commercial Terms: products for end users | commercial-terms A.1 (English) | allowed (API key customers) | "to power products and services Customer makes available to its own customers and end users" |
| 12 | Usage Policy: resale/proxy | aup (Korean fetch), effective Nov 12, 2026 | restricted (from 2026-11-12) | "허가되지 않은 수단을 통해 Claude를 재판매하거나, 대리하거나, 다른 방법으로 접근 권한을 제공하는 행위" (reselling, proxying, or otherwise providing access to Claude through unauthorized means) |
| 13 | Usage Policy: circumventing product limits | aup (Korean fetch) | restricted | "Anthropic의 사전 승인 없이 제품 내에 설정된 기능, 제한 사항, 가드레일을 의도적으로 우회하거나 방해하는 행위" |
| 14 | Usage Policy: model scraping/distillation | aup (Korean fetch) | restricted | "Anthropic의 사전 승인 없이 출력을 활용해 AI 모델을 학습하는 행위(예: '모델 스크레이핑' 또는 '모델 증류')" |
| 15 | Ordinary use assumption | legal-compliance, "Acceptable use" | info | "Advertised usage limits for Pro, Max, and Ultra plans assume ordinary, individual usage of Claude Code and the Agent SDK." |

## Secondary (not official; do not rely on)
- The Register, 2026-02-20, https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access : quotes Anthropic engineer Thariq Shihipar social post (Jan 2026, not spokesperson): "Third-party harnesses using Claude subscriptions ... are prohibited by our Terms of Service". Also quotes an older legal-page sentence "Using OAuth tokens obtained through Claude Free, Pro, or Max accounts in any other product, tool, or service" -- NOT found in current legal-compliance page (fetched 2026-11-07). Treat as earlier page version, unverified current.
- Digg summary, https://digg.com/ai/k14ct0tt : says AUP change targets pooled consumer Pro/Max subscriptions for API-like access. Secondhand.
- Blog claim that running Claude Code locally is exempt from automation ban: commentary only, unverified.

## Not found / not checked
- not_found: explicit rule on GUI wrappers around CLI (legal-compliance and agent-sdk pages do not name wrappers).
- not_found: English Consumer Terms text and English AUP text (fetcher returned Korean only).
- not_checked: docs.claude.com mirrors; Anthropic trademark guidelines (linked, not fetched); Claude Code headless docs.

## Verdict
- personal use (A: own subscription, own machine, unmodified binary, no token handling): low-moderate. Clause 2c supports end user signing in to unmodified binary; risk remains from Consumer Terms item 7 (automation) if chaude drives claude non-interactively at scale.
- personal use (B: own API key): low. Commercial Terms apply; no resale; API key is the explicitly sanctioned automation path.
- public distribution (A): high / unclear. Clause 1 (running Claude Code in your product requires Commercial Terms), clause 2a/2b (no login offer, no token handling), 3a (no "Claude Code" in name). Written confirmation from Anthropic sales advised (docs direct branding and licensing questions there).
- public distribution (B): medium. Allowed to build products for end users (A.1) but no resale (D.4); clause 1 conditions apply if bundling Claude Code.
- name check: "chaude" contains no "Claude Code"; keep it that way, no Claude logo or ASCII art.
