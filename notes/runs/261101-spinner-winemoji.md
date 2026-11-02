task: Windows spinner glyph green (color emoji) before 생각중
cause: spinner frames in tui.go:253 (✻✽✶✳✢·); ✳ U+2733 is Emoji=Yes w/ text default, Windows Terminal may draw color emoji
fix: U+FE0E appended to all 6 frames (tui.go:253-254), text presentation forced
changed: internal/harness/tui.go (spinner line only; other uncommitted edits untouched)
check: go vet ./... -> 0; go test ./internal/harness/ -> 0; GOOS=windows GOARCH=amd64 go build ./... -> 0
doubt: not visually verified on Windows Terminal; glyph identity by Unicode data, brief's candidate list (●⏺❇✴⬤) not in frames
