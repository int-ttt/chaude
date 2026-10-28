package harness

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestParseSamples(t *testing.T) {
	f, err := os.Open(filepath.Join("..", "..", "samples", "probe1.jsonl"))
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
