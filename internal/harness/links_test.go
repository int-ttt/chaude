package harness

import (
	"strings"
	"testing"

	"github.com/charmbracelet/x/ansi"
)

func linkTexts(s string) []string {
	var out []string
	for _, sp := range linkSpans(s) {
		out = append(out, sp.text)
	}
	return out
}

func TestLinkSpansDetects(t *testing.T) {
	cases := []struct {
		in   string
		want []string
	}{
		{"see https://example.com/a?b=1 now", []string{"https://example.com/a?b=1"}},
		{"(https://x.io/a).", []string{"https://x.io/a"}},
		{`C:\Users\me\proj\main.go`, []string{`C:\Users\me\proj\main.go`}},
		{`edit C:/src/app.go:42:7 today`, []string{`C:/src/app.go:42:7`}},
		{"open ./internal/harness/tui.go:120", []string{"./internal/harness/tui.go:120"}},
		{"see ../README.md", []string{"../README.md"}},
		{"file internal/harness/links.go:9 changed", []string{"internal/harness/links.go:9"}},
		{"abs /usr/local/bin/chaude ok", []string{"/usr/local/bin/chaude"}},
	}
	for _, c := range cases {
		got := linkTexts(c.in)
		if strings.Join(got, "|") != strings.Join(c.want, "|") {
			t.Errorf("linkSpans(%q) = %q, want %q", c.in, got, c.want)
		}
	}
}

func TestLinkSpansProseNegatives(t *testing.T) {
	for _, in := range []string{
		"and/or the 1/2 ratio",
		"ratio 3/4 and 10/20",
		"plain words only",
		"v1.2/beta mention", // no path-ish extension after slash-only segment
		"x:12 and 1:30",
	} {
		if got := linkTexts(in); len(got) != 0 {
			t.Errorf("linkSpans(%q) = %q, want none", in, got)
		}
	}
}

func TestLinkSpansOverlapAndBoundary(t *testing.T) {
	// URL keeps its path-like tail: no separate path span inside it.
	if got := linkTexts("https://h.io/a/b/c.go"); len(got) != 1 {
		t.Errorf("url split into %q", got)
	}
	// Middle of a word is not a token start.
	if got := linkTexts("foo.bar/baz/qux.go"); len(got) != 1 || got[0] != "foo.bar/baz/qux.go" {
		t.Errorf("word-internal path = %q", got)
	}
}

func TestLinkifyPreservesVisibleText(t *testing.T) {
	in := "\x1b[1mbold\x1b[0m open https://e.com/x and C:\\a\\b.txt:3 end"
	out := linkify(in)
	if out == in {
		t.Fatal("linkify did not style anything")
	}
	if ansi.Strip(out) != ansi.Strip(in) {
		t.Errorf("visible text changed:\n got %q\nwant %q", ansi.Strip(out), ansi.Strip(in))
	}
	if ansi.StringWidth(out) != ansi.StringWidth(in) {
		t.Errorf("width changed: %d -> %d", ansi.StringWidth(in), ansi.StringWidth(out))
	}
	if !strings.Contains(out, linkURLOn) || !strings.Contains(out, linkColorOn) {
		t.Errorf("missing link SGR in %q", out)
	}
}

func TestWrapLinkedSkipsFence(t *testing.T) {
	ident := func(s string) string { return s }
	text := "see ./a/b.go\n```\n./code/c.go\n```\nafter ./d/e.go"
	out := wrapLinked(text, ident)
	lines := strings.Split(out, "\n")
	if len(lines) != 5 {
		t.Fatalf("lines = %d, want 5: %q", len(lines), out)
	}
	if !strings.Contains(lines[0], linkColorOn) || !strings.Contains(lines[4], linkColorOn) {
		t.Errorf("prose links not styled: %q", out)
	}
	if strings.Contains(lines[2], "\x1b") {
		t.Errorf("fenced code was styled: %q", lines[2])
	}
	if ansi.Strip(out) != text {
		t.Errorf("visible text changed:\n got %q\nwant %q", ansi.Strip(out), text)
	}
}
