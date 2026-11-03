task: plan/tui-parity.md group C (slash commands), Go ref deprecated/src/main/intt/internal/harness/{tui.go,repl.go,mode.go,busyslash_test.go,usagebusy_test.go}
file: crates/cli/src/tui/commands.rs (new), crates/cli/src/tui/mod.rs (+ `pub mod commands;` only)
gates:
  cargo fmt --check: PASS (repo-wide clean at time of run)
  cargo clippy -p chaude-cli --all-targets -- -D warnings: PASS
  cargo test -p chaude-cli: PASS 43 passed, 0 failed (commands: 19)
design: dispatch(line, &Ctx) -> Option<Dispatch>; Dispatch enum = effects, not executed. busy gate inside dispatch (tui.go:796).
done (parse/dispatch/output implemented + unit-tested; engine effect = Dispatch variant):
  C01 C02 C03 C04 C05 C06 C07 C17 C18 C19 C20 C21 C22 C23 C24 C25* C26* C27 C28
  * C25: spec says "runs a turn"; Go code routes /init /review /doctor via passthroughCmds (repl.go:1167, tui.go:1401) -> Dispatch::Passthrough. Code followed.
  * C26: Go busyOK (tui.go:841) includes /settings; spec list omits it. Code followed.
ported Go tests: TestBusySlashAllowedAndRefused, TestVersionNoteShowsChaudeVersion, TestUsageSlashAcceptedWhileBusy, TestUsageAllowedWhileBusy (busy-message literal)
partial (dispatch variant only, behavior blocked):
  C08 settings picker: OpenSettings variant; picker keys/retention/model choices not ported (render.rs + settings file) -> blocked
  C10 /resume: Resume{args} variant; record store / Claude session lookup not in Rust -> blocked (engine)
  C12 /view: OpenView{arg} variant; openSessView ambiguous/no-match messages need session list -> blocked (engine)
  C29 agent view: no command; overlay + sub-agent events -> blocked (render/engine)
blocked:
  C09 settings save message: needs settings file write (settings_picker.go:47) -> engine
  C11 record picker keys/wheel: picker render (render.rs), record store -> blocked
  C13 C14 C15 C16 session browser: render + session store -> blocked
  C30 C31 agent view click/read-only: render + agent tracker -> blocked
  C32 usage popup open/close: render.rs overlay -> blocked
  C33 overlay key ordering: mod.rs key handling + render -> blocked (outside commands.rs)
  Go tests not ported (need render/engine): TestRecordPickCandidates, TestRecordPickerKeysWheelEnter, TestViewArgOpensSessionDirectly, TestViewListsOnlyOwnSessions, TestViewSessionSelectionAndPromptNavigation, TestViewEscBacksOutToNormalScreen, TestImportShowsTurnsInTUI, TestRotateKeepsPendingSnapshot, TestLiveRotationCarriesContext, TestAgentViewClickReadOnlyClose, TestUsagePopupOpenClose, TestUsageViewEscCloses, TestUsageMsgWhileBusyOpensPopup
spec mismatches to fix in plan/tui-parity.md: C25 wording, C26 missing /settings
