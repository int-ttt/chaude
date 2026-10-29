package harness

import (
	"bytes"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

func TestUsageTextAndDeltas(t *testing.T) {
	r := &Repl{lastCtx: 135_000}
	if got := r.usageText(); !strings.Contains(got, "아직 사용량 이벤트가 없습니다") || !strings.Contains(got, "컨텍스트  [█████░░░░░] 50%") {
		t.Fatalf("empty rate:\n%s", got)
	}
	reset := float64(time.Now().Add(90 * time.Minute).Unix())
	r.rate = map[string]any{"unifiedWindows": map[string]any{
		"five_hour": map[string]any{"utilization": 0.55, "resetsAt": reset, "status": "allowed"},
		"seven_day": map[string]any{"utilization": 0.41, "resetsAt": reset, "status": "allowed"},
	}}
	got := r.usageText()
	for _, w := range []string{"세션(5시간)  [██████░░░░] 55%", "주간(7일)  [████░░░░░░] 41%", "(1시간 29분 후)", "(1시간 30분 후)"} {
		if !strings.Contains(got, w) && !(strings.HasPrefix(w, "(1시간") && (strings.Contains(got, "(1시간 29분 후)") || strings.Contains(got, "(1시간 30분 후)"))) {
			t.Errorf("missing %q in:\n%s", w, got)
		}
	}
	if strings.Contains(got, "Fable") || strings.Contains(got, "[allowed]") {
		t.Errorf("unexpected text:\n%s", got)
	}
	// deltas and the optional Fable window
	r0 := map[string]any{"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": 0.50}}}
	r1 := map[string]any{"unifiedWindows": map[string]any{"five_hour": map[string]any{"utilization": 0.55}, "seven_day_fable": map[string]any{"utilization": 0.10}}}
	if d := pctDelta(r0, r1, "five_hour"); d != "+5.0%" {
		t.Errorf("delta %q", d)
	}
	if d := pctDelta(r1, r0, "five_hour"); d != "reset" {
		t.Errorf("reset delta %q", d)
	}
	if d := pctDelta(r0, r1, "seven_day"); d != "?" {
		t.Errorf("missing window delta %q", d)
	}
	if p := pctStr(r1, "fable"); p != "10%" {
		t.Errorf("fable window %q", p)
	}
	if n := windowName("seven_day_opus"); n != "주간 opus" {
		t.Errorf("windowName %q", n)
	}
	if s := resetText(time.Unix(0, 0), time.Unix(10, 0)); !strings.HasSuffix(s, "(지남)") {
		t.Errorf("past reset %q", s)
	}
}

// The real terminal cursor is the IME anchor: a frame shows it only while it is parked in the input box.
func TestCursorWriterShowsCursorOnlyWhenParked(t *testing.T) {
	var col, up atomic.Int32
	var out bytes.Buffer
	cw := &cursorWriter{w: &out, col: &col, up: &up}
	frame := "\x1b[H\n> ab\x1b[K\r\n\x1b[3;H"
	col.Store(5)
	up.Store(1)
	if _, err := cw.Write([]byte(frame)); err != nil {
		t.Fatal(err)
	}
	if s := out.String(); !strings.HasPrefix(s, "\x1b[?2026h") || !strings.HasSuffix(s, "\x1b[?25h\x1b[?2026l") || !strings.Contains(s, "\x1b[2;5H") {
		t.Fatalf("parked frame: %q", s)
	}
	out.Reset()
	col.Store(0)
	_, _ = cw.Write([]byte(frame))
	if s := out.String(); !strings.HasSuffix(s, "\x1b[?25l\x1b[?2026l") {
		t.Fatalf("unparked frame must hide the cursor: %q", s)
	}
	out.Reset()
	_, _ = cw.Write([]byte("\x1b[?1000h")) // not a frame: passed through, cursor untouched
	if s := out.String(); s != "\x1b[?2026h\x1b[?1000h\x1b[?2026l" {
		t.Fatalf("non-frame: %q", s)
	}
}
