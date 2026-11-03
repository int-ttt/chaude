package harness

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestParseSamples(t *testing.T) {
	f, err := os.Open(filepath.Join("..", "..", "..", "..", "..", "..", "samples", "probe1.jsonl"))
	if err != nil {
		t.Skip(err)
	}
	defer f.Close()
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	var deltas, results int
	for sc.Scan() {
		ev := parseEvent(sc.Text())
		switch ev["type"] {
		case "_unparsed":
			t.Fatalf("unparsed: %.80s", sc.Text())
		case "stream_event":
			e := asMap(ev["event"])
			if e["type"] == "content_block_delta" && asMap(e["delta"])["type"] == "text_delta" {
				deltas++
			}
		case "result":
			results++
			if ctxTokens(asMap(ev["usage"])) <= 0 {
				t.Error("ctx tokens zero")
			}
		}
	}
	if deltas == 0 || results == 0 {
		t.Fatalf("deltas=%d results=%d", deltas, results)
	}
	if ev := parseEvent("not json"); ev["type"] != "_unparsed" || ev["raw"] != "not json" {
		t.Error("unparsed fallback")
	}
}

func TestCtxTokens(t *testing.T) {
	u := map[string]any{"input_tokens": 2.0, "cache_read_input_tokens": 13008.0, "cache_creation_input_tokens": 7581.0}
	if got := ctxTokens(u); got != 20591 {
		t.Fatalf("got %d", got)
	}
	if commas(20591) != "20,591" || commas(5) != "5" || commas(1234567) != "1,234,567" {
		t.Error("commas")
	}
}

func TestObservedTTL(t *testing.T) {
	u1h := map[string]any{"cache_creation": map[string]any{"ephemeral_1h_input_tokens": 7581.0, "ephemeral_5m_input_tokens": 0.0}}
	u5m := map[string]any{"cache_creation": map[string]any{"ephemeral_1h_input_tokens": 0.0, "ephemeral_5m_input_tokens": 120.0}}
	if got := observedTTL(u1h, "5m"); got != "1h" {
		t.Errorf("1h usage: got %q", got)
	}
	if got := observedTTL(u5m, "1h"); got != "5m" {
		t.Errorf("5m usage: got %q", got)
	}
	if got := observedTTL(map[string]any{"input_tokens": 2.0}, "auto"); got != "auto" {
		t.Errorf("no cache_creation: got %q", got)
	}
}

func TestSnapshot(t *testing.T) {
	r := &Repl{a: Args{TailTurns: 2, SnapshotChars: 24000}, tr: &Transcript{}}
	r.files = []string{"/a/b.go"}
	for _, u := range []string{"one", "two", "three"} {
		r.tr.turns = append(r.tr.turns, [2]string{u, "re:" + u})
	}
	s := r.buildSnapshot("  NOTE ZEBRA-77 ")
	for _, want := range []string{preamble, "NOTE ZEBRA-77", "- /a/b.go", "User: two\nAssistant: re:two", "User: three"} {
		if !strings.Contains(s, want) {
			t.Errorf("missing %q", want)
		}
	}
	if strings.Contains(s, "User: one") {
		t.Error("tail not applied")
	}
	r.a.SnapshotChars = 10
	if len([]rune(r.buildSnapshot("x"))) != 10 {
		t.Error("cap")
	}
	r.a.SnapshotChars = 24000
	if !strings.Contains(r.buildSnapshot(" "), "(empty)") {
		t.Error("empty")
	}
}

func TestCleanEnv(t *testing.T) {
	t.Setenv("ANTHROPIC_API_KEY", "x")
	for _, kv := range cleanEnv(300000) {
		if strings.HasPrefix(kv, "ANTHROPIC_API_KEY=") {
			t.Fatal("key leaked")
		}
	}
	found := false
	for _, kv := range cleanEnv(300000) {
		found = found || kv == "CLAUDE_CODE_AUTO_COMPACT_WINDOW=300000"
	}
	if !found {
		t.Fatal("window")
	}
}

func TestWithCacheTTL(t *testing.T) {
	base := []string{"A=1", "CLAUDE_CODE_PROMPT_CACHE_TTL=5m"}
	cases := map[string]string{"1h": "CLAUDE_CODE_PROMPT_CACHE_TTL=1h", "5m": "CLAUDE_CODE_PROMPT_CACHE_TTL=5m"}
	for ttl, want := range cases {
		got := withCacheTTL(base, ttl)
		n := 0
		for _, kv := range got {
			if strings.HasPrefix(kv, "CLAUDE_CODE_PROMPT_CACHE_TTL=") {
				n++
			}
		}
		if n != 1 || got[len(got)-1] != want {
			t.Fatalf("withCacheTTL(%q) = %v; want single %q at end", ttl, got, want)
		}
	}
	for _, ttl := range []string{"", "auto"} {
		got := withCacheTTL(base, ttl)
		if strings.Join(got, "|") != strings.Join(base, "|") {
			t.Fatalf("withCacheTTL(%q) changed env: %v", ttl, got)
		}
	}
}

func TestFmtK(t *testing.T) {
	cases := map[int]string{0: "0", 999: "999", 1000: "1.00k", 1234: "1.23k", 1999: "1.99k", 12345: "12.3k",
		123456: "123k", 999999: "999k", 1234567: "1.23M", -5: "0", 200000: "200k", 80000: "80.0k"}
	for n, want := range cases {
		if got := fmtK(n); got != want {
			t.Errorf("fmtK(%d)=%q want %q", n, got, want)
		}
	}
	if got := fmtLimit(200000); got != "200k" {
		t.Errorf("fmtLimit(200000)=%q", got)
	}
	if got := fmtLimit(80000); got != "80k" {
		t.Errorf("fmtLimit(80000)=%q", got)
	}
}

func TestCtxBar(t *testing.T) {
	if got := ctxBar(0, 200000, nil, 0); got != "M 0/200k P 0/80k T 0/80k C 0/80k" {
		t.Errorf("empty: %q", got)
	}
	if got := ctxBar(12345, 200000, map[string]int{"P": 1000, "C": 999}, 2); got != "M 12.3k/200k P 1.00k/80k T 0/80k C 999/80k" {
		t.Errorf("coder: %q", got)
	}
}
