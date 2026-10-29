# scroll-bottom
change: tui.go refresh() follows tail (GotoBottom) unless m.held (user scrolled away via wheel/pgup/pgdn/ctrl+b/f/shift+up/down, set = !vp.AtBottom()) and vp was not at bottom
change: held && not at bottom -> offset kept on new output, unseen=true when content sig changed ("↓ 새 메시지" indicator restored)
change: resume follow: scroll back to bottom (held=false in refresh), typing key, sendPrompt; agent view close / usage popup close still GotoBottom
unchanged: agent view av.off scroll, cursor/IME calc
test: TestScrollBottomOnInputAndUpdate (updated: GotoBottom before append), TestWheelScrollHoldsPosition (new) in agentview_test.go
verify: go build, go vet, go test -count=1 -race ./internal/harness pass
