# 261105 haiku parity loop3 I15-I19 result
status: done (I17 kept ignored by design, I19 log-only)
commits: df38ce8 (I15), 5b9e179 (I16); no commit I17, I18, I19
checks:
- I15: fmt PASS; clippy 0 warnings; test-isolated 670 passed / 0 failed / 50 ignored
- I16: fmt PASS; clippy 0 warnings; test-isolated 671 passed / 0 failed / 48 ignored
- I18: test-isolated x3, exit 0 each, 671 passed / 0 failed / 48 ignored
ignored total: 48 = 13 explicit #[ignore] + 35 parity_ignored! macro stubs
doubts: (a) cli lib target exists; (b) I13 flake not reproduced; (c) ae7a757 no edits in crates/scripts seen
not done: see notes/runs/261105-haiku-parity-loop3.md NOT DONE line
log: notes/runs/261105-haiku-parity-loop3.md (uncommitted)
