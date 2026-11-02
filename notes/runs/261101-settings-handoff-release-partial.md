status: partial
done: settings file (chaude_settings.go), /settings picker (settings_picker.go, tui.go), coder_slots -> residentSlots(model, n), main_model -> a.Model when -model unset, exit handoff write (repl.go loop defer, tui.go runTUI defer) + prevOwned merge in ownSessionList, retention prune at Main start, tests (chaude_settings_test.go)
test: go build ./... OK; go test ./internal/... ok chaude/internal/harness
remaining: release 0.9.7 (version bump, README/CHANGELOG, build-release.sh, deploy target not read, commit, tag, push, gh release) NOT done
doubt: coder_slots default 4 => 4 eager haiku coders at start (was 1); coder_slots semantic interpreted as coder count
doubt: main_model full ids not checked against budget.go alias map (keys opus/sonnet/haiku)
doubt: exit recordSave not added; completed turns already saved by recordSave
doubt: resident coder routing picks first idle coder; all busy -> error
