package harness

import "testing"

func TestParkFrameSkipsWalkAndParksOnce(t *testing.T) {
	// 4 rows: row1,row2 unchanged, row3 (input) changed, row4 (status) unchanged; last row = 4, input row = 3 (up 1).
	frame := "\x1b[H\n\n> ab\x1b[K\r\n\x1b[4;H"
	want := "\x1b[3;1H> ab\x1b[K\x1b[3;7H"
	if got := parkFrame(frame, 7, 1); got != want {
		t.Fatalf("got %q want %q", got, want)
	}
	legacy := "\x1b[H\x1b[Jx\x1b[4;H"
	if got := parkFrame(legacy, 7, 1); got != legacy+"\x1b[1A\x1b[7G" {
		t.Fatalf("legacy fallback: %q", got)
	}
}
