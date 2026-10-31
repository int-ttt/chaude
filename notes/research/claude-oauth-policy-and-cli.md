# claude-oauth-policy-and-cli
date_fetched: 2026-10-31
method: WebFetch of official pages (re-verified 2026-10-31 for legal-and-compliance and authentication); WebSearch for press/community items
legend: 공식 1차 = Anthropic docs/support page; 2차 = press/community; 미확인 = not found in a primary source
note: quotes are model-extracted from WebFetch output; check live pages before legal reliance

## Q1 Policy: subscription OAuth tokens in other products / Agent SDK / third-party clients

| # | item | value (verbatim quote) | source URL | date | status |
|---|---|---|---|---|---|
| Q1-a | OAuth scope | "OAuth authentication is intended exclusively for purchasers of Claude Free, Pro, Max, Ultra, Team, and Enterprise subscription plans and is designed to support ordinary use of Claude Code and other native Anthropic applications." | https://code.claude.com/docs/en/legal-and-compliance | page undated (fetched 2026-10-31) | 공식 1차 |
| Q1-b | developers / third parties | "**Developers** building products or services that interact with Claude's capabilities, including those using the Agent SDK, should use API key authentication through Claude Console or a supported cloud provider. Anthropic does not permit third-party developers to offer Claude.ai login into their own applications, or to route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users. Moreover, developers may not collect, store, or intermediate Claude.ai credentials or session tokens — sign-in to a Claude account must complete through Anthropic's own flow." | https://code.claude.com/docs/en/legal-and-compliance | undated | 공식 1차 |
| Q1-c | end user | "Nor does it prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription, including where a platform hosts Claude Code as described under *Can customers offer Claude Code in their products?*" | https://code.claude.com/docs/en/legal-and-compliance | undated | 공식 1차 |
| Q1-d | enforcement | "Anthropic reserves the right to take measures to enforce these restrictions and may do so without prior notice." | https://code.claude.com/docs/en/legal-and-compliance | undated | 공식 1차 |
| Q1-e | Agent SDK / claude -p usage (support article) | "when you're signed in with your Claude plan, the Claude Agent SDK and `claude -p` still draw from your subscription limits." (October 7 update, 연도 미확인) ; "Update June 15, 2026: We've paused the previously-announced changes to Claude Agent SDK usage." | https://support.claude.com/en/articles/15036540-use-the-agent-sdk-with-your-claude-plan | Oct 7 (year 미확인); Jun 15 2026 | 공식 1차 (usage limits, not third-party token policy) |
| Q1-f | Feb-2026 wording "including the Agent SDK ... not permitted" | "Using OAuth tokens obtained through Claude Free, Pro, or Max accounts in any other product, tool, or service — including the Agent SDK — is not permitted and constitutes a violation of the Consumer Terms of Service." | https://winbuzzer.com/2026/02/19/anthropic-bans-claude-subscription-oauth-in-third-party-apps-xcxwbn/ ; https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access/ | 2026-02-19 (WinBuzzer), 2026-02-20 (The Register) | 2차. Sentence NOT in current legal page text (fetched 2026-10-31) |
| Q1-g | "docs clean up" | Anthropic reportedly called the change a "docs clean up" | https://paddo.dev/blog/anthropic-walled-garden-crackdown/ | undated | 2차, 미확인 (primary not found) |
| Q1-h | Consumer Terms | WebFetch of https://www.anthropic.com/legal/consumer-terms did not return an OAuth/third-party section | https://www.anthropic.com/legal/consumer-terms | 미확인 | 미확인 |

Answer Q1 (summary): 현 공식 문서상 구독 OAuth 토큰을 다른 제품·도구·클라이언트·Agent SDK에서 사용하는 것은 허용되지 않음 (개발자는 API key 또는 지원 클라우드 제공자 사용; 구독 자격증명 경유 라우팅 금지). 단, 최종 사용자가 수정되지 않은 Claude Code 바이너리에 본인 구독으로 로그인하는 것은 허용 (Q1-c). Feb-2026 "including the Agent SDK ... not permitted" 문구는 2차 출처에만 있음.

## Q2 Credential storage

| # | item | value (verbatim quote) | source URL | status |
|---|---|---|---|---|
| Q2-a | macOS | "On macOS, credentials are stored in the encrypted macOS Keychain. When the Keychain rejects the write, such as when it's locked in an SSH session, Claude Code stores your login in `~/.claude/.credentials.json` with file mode `0600` instead, the same storage it uses on Linux." | https://code.claude.com/docs/en/authentication | 공식 1차 |
| Q2-b | Linux | "On Linux, credentials are stored in `~/.claude/.credentials.json` with file mode `0600`." | https://code.claude.com/docs/en/authentication | 공식 1차 |
| Q2-c | Windows | "On Windows, credentials are stored in `%USERPROFILE%\.claude\.credentials.json` and inherit the access controls of your user profile directory" | https://code.claude.com/docs/en/authentication | 공식 1차 |
| Q2-d | CLAUDE_CONFIG_DIR | "If you've set the `CLAUDE_CONFIG_DIR` environment variable, Claude Code keeps the `.credentials.json` file under that directory instead, including the file the macOS fallback writes, and keys the macOS Keychain entry to that directory too, so a session with a different `CLAUDE_CONFIG_DIR` reads a different entry." | https://code.claude.com/docs/en/authentication | 공식 1차 |
| Q2-e | macOS Keychain item name | 공식 문서에 Keychain item/service 이름 없음 | https://code.claude.com/docs/en/authentication | 미확인 (official) |
| Q2-f | community-observed service name | "Claude Code-credentials" (community report only) | https://github.com/anthropics/claude-code/issues/9403 | 2차, 미확인 |

Credential values were NOT fetched or printed.

## Q3 CLI flags (print mode)

Source: https://code.claude.com/docs/en/cli-reference (verified verbatim in earlier session, 2026-10-31).

| flag | verbatim description | print-mode note |
|---|---|---|
| `--tools` | "Restrict which built-in tools Claude can use. Use `""` to disable all, `"default"` for the default set, or tool names like `"Bash,Edit,Read"`. … The flag doesn't affect MCP tools; to deny those too, use `--disallowedTools "mcp__*"`." | 인터랙티브/print 구분 문구 없음 |
| `--disallowedTools`, `--disallowed-tools` | "Deny rules. A bare tool name removes the matching tools from Claude's context…" | 인터랙티브/print 구분 문구 없음 |
| `--system-prompt` | "Replace the entire system prompt with custom text" ; section note: "All five work in both interactive and non-interactive modes." | 두 모드 모두 (섹션 문구 기준) |
| `--strict-mcp-config` | "Only use MCP servers from `--mcp-config`, ignoring all other MCP configurations." | 인터랙티브/print 구분 문구 없음 |
| `--setting-sources` | "Comma-separated list of setting sources to load (`user`, `project`, `local`)." | 인터랙티브/print 구분 문구 없음 |
| `--print`, `-p` | "Print response without interactive mode" | - |
| `--allowedTools` | 미확인 (row not in fetched slice) | 미확인 |

Note: the "all five work in both modes" section covers the system-prompt flags as a group; the exact five-flag membership was not re-verified against the page in this pass.

## Q4 Reports of server-side blocking of third-party OAuth use (Jan–Oct 2026)

All rows below are 2차 or user reports. No Anthropic primary announcement of a server-side block was found.

| date (as reported) | event | source | status |
|---|---|---|---|
| 2026-01-09 (reported) | Server-side safeguard; error "This credential is only authorized for use with Claude Code"; Thariq Shihipar: "Yesterday we tightened our safeguards against spoofing the Claude Code harness" | https://paddo.dev/blog/anthropic-walled-garden-crackdown/ | 2차 |
| Jan 2026 | Thariq Shihipar social post, quoted: "Third-party harnesses using Claude subscriptions create problems for users and are prohibited by our Terms of Service" | https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access/ | 2차 (post itself not fetched) |
| 2026-02-19 to 02-20 | Legal-page clarification (see Q1-f); OpenCode withdrew Claude OAuth support, citing "anthropic legal requests" | https://winbuzzer.com/2026/02/19/anthropic-bans-claude-subscription-oauth-in-third-party-apps-xcxwbn/ ; https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access/ | 2차 |
| 2026-04-04 12:00 PM PT (reported) | Cutoff of third-party tools from subscription auth | decodethefuture, Medium, zylos reports (URLs not re-verified in this pass) | 2차, 미확인 (primary not found) |
| 2026-06 | Agent SDK monthly credit announced, then paused (support article) | https://support.claude.com/en/articles/15036540-use-the-agent-sdk-with-your-claude-plan | 공식 1차, not a block event |
| undated | User reports of account bans / suspensions on OAuth proxies (e.g., CLIProxyAPI issues #2211, #2599; anthropics/claude-code #40046) | issue numbers only (URLs not re-verified) | user reports, not Anthropic statements |
| Jan–Oct 2026 | Official Anthropic announcement of server-side block | not found | 미확인 |

## Unverified / 미확인 summary
- Exact current-page presence of Feb-2026 "including the Agent SDK ... not permitted" sentence: absent from fetched current text; 2차 only.
- Consumer Terms OAuth section: 미확인.
- macOS Keychain item name: 미확인 officially; "Claude Code-credentials" is community-only.
- Primary Anthropic statement of Jan 9 / Apr 4 enforcement: 미확인.
- Thariq and other staff quotes: 2차 only.
- `--allowedTools` row: 미확인.
- Support-article "October 7" year: 미확인.
