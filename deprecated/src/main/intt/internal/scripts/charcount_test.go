package scripts

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// runCharcount runs charcount on a file and returns its stdout.
func runCharcount(t *testing.T, content []byte) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "doc.md")
	if err := os.WriteFile(path, content, 0o644); err != nil {
		t.Fatal(err)
	}
	outPath := filepath.Join(t.TempDir(), "out.txt")
	out, err := os.Create(outPath)
	if err != nil {
		t.Fatal(err)
	}
	saved := os.Stdout
	os.Stdout = out
	rc := charcount([]string{path})
	os.Stdout = saved
	out.Close()
	if rc != 0 {
		t.Fatalf("rc=%d", rc)
	}
	b, _ := os.ReadFile(outPath)
	return string(b)
}

func TestCharcountBOMAndCRLFMatchPlain(t *testing.T) {
	plain := []byte("# 제목\n\n본문 한글 test\n")
	bomCRLF := []byte("\xef\xbb\xbf# 제목\r\n\r\n본문 한글 test\r\n")
	want := runCharcount(t, plain)
	if got := runCharcount(t, bomCRLF); got != want {
		t.Fatalf("BOM+CRLF output differs from plain LF:\n got: %q\nwant: %q", got, want)
	}
	// 공백 포함(줄바꿈 포함) = runes after strip, BOM must not add a char
	if !strings.Contains(want, "공백 포함(줄바꿈 포함): 16") {
		t.Fatalf("unexpected count: %q", want)
	}
}

func TestCharcountBOMOnlyStripped(t *testing.T) {
	got := runCharcount(t, []byte("\xef\xbb\xbfabc"))
	if !strings.Contains(got, "공백 포함(줄바꿈 포함): 3") || !strings.Contains(got, "바이트(UTF-8): 3 ") {
		t.Fatalf("BOM not stripped: %q", got)
	}
}
