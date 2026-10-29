# input-fix (2026-10-29)
## conductor default
done: internal/harness/repl.go:814 BoolVar default false -> true; --conductor=false still works (Go flag bool)
verify: code read (a.Conductor set only via this flag + /mode toggles); go build ./... ok; go test ./... ok
note: no test asserts the default; not added

## 한글 shift
status: cause NOT confirmed, no fix applied
read: input.go (Insert/Backspace/rows/Cursor/View), tui.go syncCursor (curCol = 1 border + 2 prompt + col + 1 = col+4, matches box w/o padding at tui.go:762), cursorWriter (real cursor ESC[nA ESC[colG for IME)
excluded: key loop (tea.KeyRunes -> Insert whole rune slice, pos += len(in) rune-based, ok); cursor column formula (correct); wrap/rows (cell-based, ok); NFC syllable width (한=2, ㅎ/ㅏ=2)
found, real but not matching symptom: NFD conjoining jamo (U+1112 U+1161 U+11AB) counted 4 cells by cells(), terminal draws 2 (runewidth counts medial/final jamo 2). Off by 2 per syllable, only for NFD text (paste/macOS filenames), not IME-typed NFC. Candidate fix: rw()=0 for U+1160-11FF, U+D7B0-D7FF, or NFC-normalize in Insert. Not applied.
untested: terminal-side: cursorWriter pre/post cursor moves vs bubbletea renderer partial-frame writes; IME preedit drawn at real cursor, overlapping fake reverse-video cursor cell
next: reproduce in the real terminal (Terminal/iTerm2/Ghostty?) -> note terminal + whether shift appears at end of line only, or mid-line/after wrap; try disabling cursorWriter (curCol=0) to see if shift vanishes; log m.in.Cursor() and Value() runes on each key
