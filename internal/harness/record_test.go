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
	if err := recordAppendTurn(dir, time.Date(2026, 10, 31, 23, 59, 50, 0, time.Local), "chaude metrics", recordTestTurn("a"), false); err != nil {
		t.Fatal(err)
	}
	// Same folder, next day, no split: still the newest file (a file is only started by /clear or a cap).
	if err := recordAppendTurn(dir, time.Date(2026, 11, 1, 0, 0, 10, 0, time.Local), "chaude metrics", recordTestTurn("b"), false); err != nil {
		t.Fatal(err)
	}
	want := []string{"261031-235950-0001.md"}
	if got := recordTestNames(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("files = %v, want %v", got, want)
	}
	data, _ := os.ReadFile(filepath.Join(dir, want[0]))
	if !strings.Contains(string(data), "<!-- rec #0002 asst 2026-11-01T00:00 cat=chaude metrics -->") {
		t.Fatalf("midnight turn header missing:\n%s", data)
	}
}

func TestRecordSplitOnClearStartsNewFile(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	if err := recordAppendTurn(dir, at, "alpha", recordTestTurn("a"), false); err != nil {
		t.Fatal(err)
	}
	// /clear sets split: the turn starts a new file in the same folder.
	if err := recordAppendTurn(dir, at, "beta", recordTestTurn("b"), true); err != nil {
		t.Fatal(err)
	}
	// After the split, writes continue in the newest file.
	if err := recordAppendTurn(dir, at, "beta", recordTestTurn("c"), false); err != nil {
		t.Fatal(err)
	}
	want := []string{"261031-100000-0001.md", "261031-100000-0002.md"}
	if got := recordTestNames(t, dir); !reflect.DeepEqual(got, want) {
		t.Fatalf("files = %v, want %v", got, want)
	}
	data, _ := os.ReadFile(filepath.Join(dir, want[1]))
	if !strings.Contains(string(data), "<!-- rec #0002 asst 2026-10-31T10:00 cat=beta -->") ||
		!strings.Contains(string(data), "<!-- rec #0003 asst 2026-10-31T10:00 cat=beta -->") {
		t.Fatalf("newest file lacks turns #2 and #3:\n%s", data)
	}
}

func TestRecordSplitByCaps(t *testing.T) {
	dir := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	for i := 0; i < 101; i++ {
		if err := recordAppendTurn(dir, at, "cap", recordTestTurn("x"), false); err != nil {
			t.Fatal(err)
		}
	}
	// The 101st turn (rec #0101) starts a new file named by its own rec number.
	want := []string{"261031-100000-0001.md", "261031-100000-0101.md"}
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

func TestRecordProjectName(t *testing.T) {
	cases := map[string]string{
		"/Users/me/PycharmProjects/claude-harness": "claude-harness",
		"/work/my project":                         "my-project",
		"/work/.hidden":                            "hidden",
	}
	for in, want := range cases {
		if got := recordProjectName(in); got != want {
			t.Errorf("recordProjectName(%q) = %q, want %q", in, got, want)
		}
	}
	root := t.TempDir()
	if got := recordProjectDir(root, "/x/claude-harness"); got != filepath.Join(root, "claude-harness") {
		t.Errorf("recordProjectDir = %q", got)
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
	if err := recordAppendTurn(dir, at, "왕복 테스트", turn, false); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(dir, "261031-140500-0001.md"))
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
		if err := recordAppendTurn(dir, at, "resume", recordTestTurn("a"), false); err != nil {
			t.Fatal(err)
		}
	}
	// A restart is a fresh call with no in-memory state.
	if n, err := nextRecordNumber(dir); err != nil || n != 3 {
		t.Fatalf("nextRecordNumber = %d, %v; want 3, nil", n, err)
	}
	if err := recordAppendTurn(dir, at, "resume", recordTestTurn("b"), false); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "261031-090000-0001.md"))
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
	if err := recordAppendTurn(dir, at, "a-->b", turn, false); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "261031-110000-0001.md"))
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
	if err := recordAppendTurn(dir, at, "old", recordTestTurn("a"), false); err != nil {
		t.Fatal(err)
	}
	if err := recordAppendTurn(dir, at, "new", recordTestTurn("b"), true); err != nil {
		t.Fatal(err)
	}
	// Touch the older file so it is the newest by mtime; the number must still come from the newest rec.
	old := time.Date(2030, 1, 1, 0, 0, 0, 0, time.Local)
	if err := os.Chtimes(filepath.Join(dir, "261031-090000-0001.md"), old, old); err != nil {
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

// writeProjectTurn appends one turn to the project folder root/name (split starts a new file).
func writeProjectTurn(t *testing.T, root, name, answer string, at time.Time, split bool) {
	t.Helper()
	if err := recordAppendTurn(filepath.Join(root, name), at, name, recordTestTurn(answer), split); err != nil {
		t.Fatal(err)
	}
}

func TestRecordFolderListing(t *testing.T) {
	root := t.TempDir()
	d1 := time.Date(2026, 10, 30, 10, 0, 0, 0, time.Local)
	d2 := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	writeProjectTurn(t, root, "beta", "b1", d1, false)
	writeProjectTurn(t, root, "alpha", "a1", d1, false)
	writeProjectTurn(t, root, "alpha", "a2", d2, true)
	if err := os.Chtimes(filepath.Join(root, "alpha", "261031-100000-0002.md"), d2, d2); err != nil {
		t.Fatal(err)
	}
	if err := os.Chtimes(filepath.Join(root, "alpha", "261030-100000-0001.md"), d1, d1); err != nil {
		t.Fatal(err)
	}
	if err := os.Chtimes(filepath.Join(root, "beta", "261030-100000-0001.md"), d1, d1); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, "empty"), 0o755); err != nil {
		t.Fatal(err)
	}
	projects, err := listRecordProjects(root)
	if err != nil {
		t.Fatal(err)
	}
	if len(projects) != 2 || projects[0].Name != "alpha" || projects[1].Name != "beta" {
		t.Fatalf("projects = %+v, want alpha then beta (empty folder skipped)", projects)
	}
	if projects[0].Files != 2 || projects[1].Files != 1 {
		t.Errorf("file counts = %d, %d; want 2, 1", projects[0].Files, projects[1].Files)
	}
	if got := projects[0].Last.Format("2006-01-02"); got != "2026-10-31" {
		t.Errorf("alpha last = %s, want 2026-10-31", got)
	}
	lines, blocks, err := recordResume(root, "")
	if err != nil || blocks != nil || len(lines) != 3 || !strings.Contains(lines[1], "alpha") || !strings.Contains(lines[1], "파일 2개") {
		t.Fatalf("bare /resume lines = %q blocks = %v err = %v", lines, blocks, err)
	}
}

func TestRecordFolderQueryFilter(t *testing.T) {
	root := t.TempDir()
	at := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	writeProjectTurn(t, root, "alpha", "배포 방법을 알려줘", at, false)
	writeProjectTurn(t, root, "beta", "정규식 설명", at, false)
	writeProjectTurn(t, root, "gamma", "배포 체크리스트", at, false)

	// Content match with one hit loads that folder's blocks.
	_, blocks, err := recordResume(root, "정규식")
	if err != nil || len(blocks) != 2 || blocks[1].Text != "정규식 설명" {
		t.Fatalf("content query blocks = %v err = %v", blocks, err)
	}
	// Name match.
	_, blocks, err = recordResume(root, "beta")
	if err != nil || len(blocks) != 2 || blocks[1].Text != "정규식 설명" {
		t.Fatalf("name query blocks = %v err = %v", blocks, err)
	}
	// Several hits list the folders instead of loading.
	lines, blocks, err := recordResume(root, "배포")
	if err != nil || blocks != nil || len(lines) != 3 {
		t.Fatalf("multi-hit lines = %q blocks = %v err = %v", lines, blocks, err)
	}
	// No hit is an error (the caller then treats the query as a Claude Code session query).
	if _, _, err := recordResume(root, "없는검색어"); err == nil {
		t.Fatal("no-match query returned no error")
	}
}

func TestRecordFolderLoadAllInTimeOrder(t *testing.T) {
	root := t.TempDir()
	d1 := time.Date(2026, 10, 30, 10, 0, 0, 0, time.Local)
	d2 := time.Date(2026, 10, 31, 10, 0, 0, 0, time.Local)
	writeProjectTurn(t, root, "alpha", "first", d1, false)
	writeProjectTurn(t, root, "alpha", "second", d1, true) // /clear: new file, same folder
	writeProjectTurn(t, root, "alpha", "third", d2, true)

	dir := filepath.Join(root, "alpha")
	if n := len(recordTestNames(t, dir)); n != 3 {
		t.Fatalf("files = %d, want 3", n)
	}
	entries, err := loadRecordProject(dir)
	if err != nil {
		t.Fatal(err)
	}
	var nums []int
	var texts []string
	for _, e := range entries {
		nums = append(nums, e.Num)
		if e.Kind == "asst" {
			texts = append(texts, e.Text)
		}
	}
	if want := []int{1, 1, 2, 2, 3, 3}; !reflect.DeepEqual(nums, want) {
		t.Errorf("rec numbers = %v, want %v", nums, want)
	}
	if want := []string{"first", "second", "third"}; !reflect.DeepEqual(texts, want) {
		t.Errorf("answers = %v, want %v", texts, want)
	}
}

func TestRecordMigrateLegacyRoot(t *testing.T) {
	root := filepath.Join(t.TempDir(), "myproj", "record") // parent basename is the folder name
	if err := os.MkdirAll(root, 0o755); err != nil {
		t.Fatal(err)
	}
	legacy := []string{"261030-alpha-001.md", "261031-beta-001.md"}
	for _, n := range legacy {
		if err := os.WriteFile(filepath.Join(root, n), []byte("<!-- rec #0001 user 2026-10-30T10:00 cat=x -->\nq\n"), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	if err := migrateRecordRoot(root); err != nil {
		t.Fatal(err)
	}
	for _, n := range legacy {
		if _, err := os.Stat(filepath.Join(root, "myproj", n)); err != nil {
			t.Errorf("%s not moved into myproj: %v", n, err)
		}
		if _, err := os.Stat(filepath.Join(root, n)); !os.IsNotExist(err) {
			t.Errorf("%s still in root", n)
		}
	}
	projects, err := listRecordProjects(root)
	if err != nil {
		t.Fatal(err)
	}
	if len(projects) != 1 || projects[0].Name != "myproj" || projects[0].Files != 2 {
		t.Fatalf("projects after migrate = %+v", projects)
	}
}
