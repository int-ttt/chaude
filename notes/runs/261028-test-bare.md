# claude --bare OAuth test (2026-10-28)
cmd: env -u ANTHROPIC_API_KEY claude --bare -p "say ok" --output-format stream-json --verbose --model haiku [--append-system-prompt "You are a worker."]
claude_code_version: 2.1.312
scratch: /private/tmp/claude-501/-Users-leekwangseok-PycharmProjects-claude-harness/19fde733-87b5-4f1b-952b-c584071b9a22/scratchpad (run1.jsonl, run2.jsonl, *.err)

## run1 (plain)
exit: 1
result: is_error true, subtype success, terminal_reason api_error, api_error_status null
error: authentication_failed
text: "Not logged in · Please run /login"
init.apiKeySource: none
init.mcp_servers: []
init.tools: Bash, Edit, Read
hooks loaded: not observed in init (no hook events in stream)
CLAUDE.md loaded: not observed (--bare)
usage: input 0, output 0, cache_creation 0 (ephemeral_1h 0, ephemeral_5m 0), cache_read 0
start tokens: 0 (no API call made)
duration_ms: 156

## run2 (--append-system-prompt "You are a worker.")
exit: 1
result: identical to run1
error: authentication_failed
text: "Not logged in · Please run /login"
init.apiKeySource: none
usage: all 0 (cache_creation ephemeral_1h 0, ephemeral_5m 0)
start tokens: 0
duration_ms: 68

## conclusion
bare mode with OAuth: FAIL. apiKeySource none; --bare does not read the OAuth/keychain login.
keychain entry "Claude Code-credentials" exists, so the login is present but not used under --bare.
cache TTL (1h vs 5m) unverifiable: no API call reached.
