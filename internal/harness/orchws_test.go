package harness

import (
	"strings"
	"testing"
	"time"
	"unicode/utf8"
)

func TestParseWSCases(t *testing.T) {
	id, rest, ok := parseWS("[ws:abc-1_X] hello there")
	if !ok || id != "abc-1_X" || rest != "hello there" {
		t.Fatalf("valid tag: got id=%q rest=%q ok=%v", id, rest, ok)
	}
	id, rest, ok = parseWS("plain line")
	if ok || id != "" || rest != "plain line" {
		t.Fatalf("no tag: got id=%q rest=%q ok=%v", id, rest, ok)
	}
	if _, _, ok = parseWS("[ws:]x"); ok {
		t.Fatal("empty id accepted")
	}
	if _, _, ok = parseWS("[ws:" + strings.Repeat("a", 33) + "] x"); ok {
		t.Fatal("33-char id accepted")
	}
	if _, _, ok = parseWS("[ws:bad.id] x"); ok {
		t.Fatal("id with dot accepted")
	}
	if _, _, ok = parseWS("x [ws:abc] y"); ok {
		t.Fatal("tag not at start accepted")
	}
}

func TestWSNeedsRotateCases(t *testing.T) {
	now := time.Now()
	if wsNeedsRotate(nil, now) {
		t.Fatal("nil state rotates")
	}
	if wsNeedsRotate(&wsState{Turns: 99, LastUsed: now}, now) {
		t.Fatal("state without session id rotates")
	}
	base := func() *wsState { return &wsState{SessionID: "s", LastUsed: now} }
	st := base()
	st.Turns = 19
	if wsNeedsRotate(st, now) {
		t.Fatal("turns 19 rotates")
	}
	st.Turns = 20
	if !wsNeedsRotate(st, now) {
		t.Fatal("turns 20 does not rotate")
	}
	st = base()
	st.Ctx = 39999
	if wsNeedsRotate(st, now) {
		t.Fatal("ctx 39999 rotates")
	}
	st.Ctx = 40000
	if !wsNeedsRotate(st, now) {
		t.Fatal("ctx 40000 does not rotate")
	}
	if wsNeedsRotate(&wsState{SessionID: "s", LastUsed: now.Add(-270 * time.Second)}, now) {
		t.Fatal("idle 270s rotates")
	}
	if !wsNeedsRotate(&wsState{SessionID: "s", LastUsed: now.Add(-271 * time.Second)}, now) {
		t.Fatal("idle 271s does not rotate")
	}
}

func TestWSHandoffRuneCap(t *testing.T) {
	long := strings.Repeat("한글", 2500) // 5000 runes
	h := wsHandoff("w1", &wsState{SessionID: "s", LastText: long})
	if n := utf8.RuneCountInString(h); n != wsHandoffMax {
		t.Fatalf("handoff runes = %d, want %d", n, wsHandoffMax)
	}
	if !strings.HasPrefix(h, "[ws:w1 handoff] ") {
		t.Fatalf("handoff prefix missing: %q", h[:20])
	}
	short := wsHandoff("w1", &wsState{LastText: "짧음"})
	if short != "[ws:w1 handoff] 짧음" {
		t.Fatalf("short handoff = %q", short)
	}
}

func TestRouteOrchBReturnsLine(t *testing.T) {
	r := &Repl{a: Args{Orch: "b", Model: "opus"}}
	line := "[ws:x] 파일 읽어줘"
	if got := r.routeOrch(line); got != line {
		t.Fatalf("routeOrch(b) = %q, want %q", got, line)
	}
	if r.orchMain != "" || r.curWS != "" {
		t.Fatalf("mode b changed state: orchMain=%q curWS=%q", r.orchMain, r.curWS)
	}
}
