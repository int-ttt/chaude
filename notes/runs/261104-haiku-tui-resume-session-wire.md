# 261104 haiku tui /resume session wiring

status: done
scope: crates/cli/src/tui/mod.rs only (core untouched)
signature used: EngineHandle::resume_session(&self, session_id: &str) (core 6d035d1)
event used: UiEvent::SessionResumed { old_session, session_id }

changes:
  - Action::ResumeSession(String) -> engine.resume_session in the tui() loop
  - /resume <hex prefix 8+> matches one id in <record>/.sessions.jsonl (handoff::read_sessions)
  - picked_resumes: previous run's main session follows handoff::resume_mode; Fresh -> note, no engine call
  - other ids: taken as named (core rule: resume_mode does not apply to explicit ids)
  - apply(SessionResumed): notice "session resumed: <new> (was <old>)" + reopens /view list (open_sessions)
  - open_view split: mode note stays in open_view, list in open_sessions
  - App.resume_rule (model, rotate threshold) set in tui()

tests:
  resume_argument_names_one_logged_session_id
  picked_previous_main_session_follows_resume_mode
  session_resumed_notes_the_switch_and_opens_the_session_list

checks:
  cargo build -p chaude-cli -> PASS
  cargo test -p chaude-cli -> PASS (172 passed, 0 failed)
  cargo clippy -p chaude-cli --no-deps -- -D warnings -> PASS
  cargo fmt -p chaude-cli -- --check -> PASS

gaps:
  - /resume with no id still lists records only; no session picker UI
  - /view list does not include ids from this run (existing TODO(view-owned))
