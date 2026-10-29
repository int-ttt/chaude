# jetbrains-hangul (2026-10-29)
status: cause NOT confirmed; no code change; go test ./... ok (harness 0.3s)
prior: notes/runs/input-repro.md, input-fix.md (NFC model math correct; NFD off by 2/syllable but not IME path)

## draw path (read: input.go View/Cursor, tui.go syncCursor/cursorWriter)
- committed text: bubbletea renders "> " + text + reverse-video space (fake cursor cell) via View()
- real cursor: cursorWriter appends ESC[upA ESC[colG after EVERY frame; col = 1(border)+2(prompt)+cells+1 = the fake cursor cell
- before next frame: ESC[upB moves back down (pre)
- IME preedit (ㅎ, 하) is drawn by the terminal/IDE at the real cursor = ON the fake cursor cell; no KeyRunes until commit
- spinner tick (8 FPS) forces frames -> cursor moved back and re-placed repeatedly during composition

## hypothesis (unverified)
JediTerm (JetBrains) draws preedit at real cursor and, on re-placement via ESC[G during composition, resets/offsets the composing overlay; a syllable w/o batchim is "complete-looking" 2-cell and gets committed/re-measured differently from one with batchim (still composing). Not provable without the terminal.
Alt: fake reverse cell + preedit overlap -> visual +1 cell when preedit width 2 replaces 1-cell cursor.

## checks (one at a time, in JetBrains terminal)
1. disable real-cursor move: syncCursor store 0 (curCol=0) -> shift gone? => cursorWriter is cause
2. else remove fake cursor cell (View: cur="" ) -> shift gone? => overlap cause
3. stop spinner ticks while typing (frames during composition)
4. same binary in macOS Terminal/iTerm2: shift absent => JediTerm-specific
log m.in.Cursor() and Value() per key to confirm model col is right.
