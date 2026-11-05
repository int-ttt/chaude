# 261105 haiku tui-parity tail: E27 a/b/c + E24 (run summary)
done:
  E27 b: 3b4cd68
  E27 a: defc5b9
  E27 c: 3ce1423
  E24 verify + regression test: 2911fae (no code fix needed)
  checks: test-isolated --no-fail-fast exit 0 (655 passed, 0 failed, 65 ignored); clippy 0 warnings; fmt exit 0; du 2.6G
remaining:
  live agent started_at = 0 in mod.rs agent_recs (not fixed)
  mcp.rs start_owned dir name collision flake (not fixed)
  E27 a: no #[ignore] for Go production divergence (timeout-only drain end)
detail: notes/runs/261105-haiku-tui-parity-tail-final.md
