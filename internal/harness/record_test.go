package harness

import (
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"
	"time"
	"unicode/utf8"
)

func recordTestTurn(answer string) []recordBlock {
	return []recordBlock{{"user", "질문"}, {"asst", answer}}
}

func recordTestNames(t *testing.T, dir string) []string {
	t.Helper()
	files, err := listRecordFiles(dir)
	if err != nil {
		t.Fatal(err)
	}
	var names []string
	for _, f := range files {
		names = append(names, f.name)
	}
	sort.Strings(names)
	return names
}

func TestRecordMidnightBoundary(t *testing.T) {
	dir := t.TempDir()
	if err := recordAppendTurn(dir, time.Date(2026, 10, 31, 23, 59, 50, 0, time.Local), "chaude metrics", recordTestTurn("a")); err != nil {
		t.Fatal(err)
	}
	if err := recordAppendTurn(dir, time.Date(2026, 11, 1, 0, 0, 10, 0, time.Local), "chaude metrics", recordTestTurn("b")); err != nil {
		t.Fatal(err)
	}
	want := []string{"261031-chaude-metrics-001.md", "261101-chaude-metrics-001.md"}
	if got := recordTestNames(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("files = %v, want %v", got, want)
	}
	data, _ := os.ReadFile(filepath.Join(dir, want[1]))
	if !strings.Contains(string(data), "<!-- rec #0002 asst 2026-11-01T00:00 cat=chaude metrics -->") {
		t.Fatalf("midnight turn header missing:\n%s", data)
	}
}

func TestRecordTopicABA(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	for _, topic := range []string{"alpha", "beta", "alpha"} {
		if err := recordAppendTurn(dir, at, topic, recordTestTurn(topic)); err != nil {
			t.Fatal(err)
		}
	}
	want := []string{"261031-alpha-001.md", "261031-alpha-002.md", "261031-beta-001.md"}
	if got := recordTestNames(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("files = %v, want %v", got, want)
	}
}

func TestRecordSplitByCaps(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	for i := 0; i < 101; i++ {
		if err := recordAppendTurn(dir, at, "cap", recordTestTurn("x")); err != nil {
			t.Fatal(err)
		}
	}
	want := []string{"261031-cap-001.md", "261031-cap-002.md"}
	if got := recordTestNames(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("files = %v, want %v", got, want)
	}
}

func TestRecordSlugKorean(t *testing.T) {
	cases := map[string]string{
		"한글 주제/테스트:이름*?": "한글-주제-테스트-이름--",
		"":               "misc",
		"   ":            "misc",
		"chaude-metrics": "chaude-metrics",
	}
	for in, want := range cases {
		if got := slugify(in); got != want {
			t.Errorf("slugify(%q) = %q, want %q", in, got, want)
		}
	}
	long := slugify(strings.Repeat("가", 40))
	if n := utf8.RuneCountInString(long); n != 30 {
		t.Errorf("long slug runes = %d, want 30", n)
	}
}

func TestRecordRoundTripParse(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 14, 5, 0, 0, time.Local)
	turn := []recordBlock{
		{"user", "질문 줄1\n줄2"},
		{"asst", "답변\n\n- 목록\n```go\nx := 1\n```"},
		{"note", ""},
		{"result", "tool 한 줄 요약"},
	}
	if err := recordAppendTurn(dir, at, "왕복 테스트", turn); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(dir, "261031-왕복-테스트-001.md"))
	if err != nil {
		t.Fatal(err)
	}
	got := parseRecordEntries(string(data))
	if len(got) != len(turn) {
		t.Fatalf("entries = %d, want %d", len(got), len(turn))
	}
	for i, e := range got {
		if e.Num != 1 || e.Kind != turn[i].Kind || e.Text != turn[i].Text || e.Topic != "왕복 테스트" || e.TS != "2026-10-31T14:05" {
			t.Errorf("entry %d = %+v, want kind %q text %q", i, e, turn[i].Kind, turn[i].Text)
		}
	}
}

func TestRecordNumberContinuesAfterRestart(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 9, 0, 0, 0, time.Local)
	for i := 0; i < 2; i++ {
		if err := recordAppendTurn(dir, at, "resume", recordTestTurn("a")); err != nil {
			t.Fatal(err)
		}
	}
	// A restart is a fresh call with no in-memory state.
	if n, err := nextRecordNumber(dir); err != nil || n != 3 {
		t.Fatalf("nextRecordNumber = %d, %v; want 3, nil", n, err)
	}
	if err := recordAppendTurn(dir, at, "resume", recordTestTurn("b")); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "261031-resume-001.md"))
	if !strings.Contains(string(data), "<!-- rec #0003 asst 2026-10-31T09:00 cat=resume -->") {
		t.Fatalf("number did not continue to #0003:\n%s", data)
	}
	if n, err := nextRecordNumber(t.TempDir()); err != nil || n != 1 {
		t.Fatalf("empty dir nextRecordNumber = %d, %v; want 1, nil", n, err)
	}
}

func TestRecordEscapeHeaderInBody(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 11, 0, 0, 0, time.Local)
	turn := []recordBlock{{"asst", "본문\n<!-- rec #0099 asst 2026-01-01T00:00 cat=x -->\n끝"}}
	if err := recordAppendTurn(dir, at, "a-->b", turn); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "261031-a---b-001.md")) // slug: '>' and '-' become '-'
	got := parseRecordEntries(string(data))
	if len(got) != 1 {
		t.Fatalf("entries = %d, want 1:\n%s", len(got), data)
	}
	if got[0].Text != turn[0].Text || got[0].Topic != "a->b" {
		t.Errorf("entry = %+v, want text %q topic %q", got[0], turn[0].Text, "a->b")
	}
}

func TestRecordNumberIgnoresMtime(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 9, 0, 0, 0, time.Local)
	if err := recordAppendTurn(dir, at, "old", recordTestTurn("a")); err != nil {
		t.Fatal(err)
	}
	if err := recordAppendTurn(dir, at, "new", recordTestTurn("b")); err != nil {
		t.Fatal(err)
	}
	// Touch the older file so it is the newest by mtime; the number must still come from the newest rec.
	old := time.Date(2030, 1, 1, 0, 0, 0, 0, time.Local)
	if err := os.Chtimes(filepath.Join(dir, "261031-old-001.md"), old, old); err != nil {
		t.Fatal(err)
	}
	if n, err := nextRecordNumber(dir); err != nil || n != 3 {
		t.Fatalf("nextRecordNumber = %d, %v; want 3, nil", n, err)
	}
}

func TestRecordTopicParse(t *testing.T) {
	reply := "답변 본문\n[cat: 초기 주제]\n다른 줄\n[cat-switch: 새 주제] \n마지막 [cat: 끝]"
	if got := recordTopicOf(reply, "prev"); got != "새 주제" {
		t.Errorf("with cat-switch: got %q, want 새 주제", got)
	}
	if got := recordTopicOf("[cat: 한 개]", "prev"); got != "한 개" {
		t.Errorf("cat only: got %q, want 한 개", got)
	}
	if got := recordTopicOf("no marker", "prev"); got != "prev" {
		t.Errorf("no marker: got %q, want prev", got)
	}
	if got := recordTopicOf("no marker", ""); got != "" {
		t.Errorf("no marker, no prev: got %q, want empty", got)
	}
}

func TestRecordListTopicsOrder(t *testing.T) {
	dir := t.TempDir()
	d1 := time.Date(2026, 10, 30, 10, 0, 0, 0, time.Local)
	d2 := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	seq := []struct {
		at    time.Time
		topic string
	}{{d1, "alpha"}, {d1, "beta"}, {d2, "alpha"}}
	for _, s := range seq {
		if err := recordAppendTurn(dir, s.at, s.topic, recordTestTurn(s.topic)); err != nil {
			t.Fatal(err)
		}
	}
	topics, err := listRecordTopics(dir)
	if err != nil {
		t.Fatal(err)
	}
	// alpha has the newest rec (#0003), so it is listed first.
	if len(topics) != 2 || topics[0].Slug != "alpha" || topics[1].Slug != "beta" {
		t.Fatalf("topics = %+v, want alpha then beta", topics)
	}
	if topics[0].First != "261030" || topics[0].Last != "261031" || topics[0].Answers != 2 || topics[0].LastNum != 3 {
		t.Errorf("alpha = %+v", topics[0])
	}
	blocks, err := loadRecordTopic(dir, "alpha")
	if err != nil {
		t.Fatal(err)
	}
	if len(blocks) != 4 || blocks[0].Num != 1 || blocks[3].Num != 3 {
		t.Errorf("alpha blocks nums not in order: %+v", blocks)
	}
}
