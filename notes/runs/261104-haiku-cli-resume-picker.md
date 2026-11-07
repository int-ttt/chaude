status: done
commit: 037386e9 (local, not pushed)
changed: crates/cli/src/tui/resume_pick.rs (new), crates/cli/src/tui/mod.rs, crates/cli/src/tui/sessview.rs (home_dir pub)
behavior: /resume args -> core record_turn::resume_target_in; (terms,false) -> resume_pick::groups -> group_list
  Err -> note status line; 1 group -> resume newest session via resume_claude; several -> GroupPicker (max 30, keys up/k down/j wrap, 1-9, enter, esc/q/ctrl+c)
  record answers -> existing records::pick / record_turn::resume path
  /resume claude no longer falls through to engine Submit
check:
  cargo clippy -p chaude-cli --no-deps -- -D warnings -> PASS (Finished)
  cargo fmt -p chaude-cli -- --check -> PASS
  cargo test -p chaude-cli -> 204 passed, 0 failed
divergence: Go pick sends sendImport (import group transcript); cli resumes via ResumeSession per brief. Core group merge of older files not applied on resume.
risk: /clear marks written only by existing code; groups split only by existing marks.
