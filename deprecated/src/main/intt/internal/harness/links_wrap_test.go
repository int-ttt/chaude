package harness

import (
	"strings"
	"testing"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
)

// linkColoredRuns replays the SGR stream of a rendered block like a terminal would (color on/off only)
// and returns the visible text of every link colored in it. A link split by a wrap is joined across the
// line break; trailing padding and surrounding spaces are not part of the link.
func linkColoredRuns(out string) []string {
	var pieces []string
	carry := false // previous line's visible content ended inside a colored run
	for _, ln := range strings.Split(out, "\n") {
		var vis strings.Builder
		var on []bool
		colored := false
		prev := 0
		for _, e := range linkSGRRe.FindAllStringIndex(ln, -1) {
			for i := prev; i < e[0]; i++ {
				vis.WriteByte(ln[i])
				on = append(on, colored)
			}
			colored = applyColorSGR(colored, ln[e[0]:e[1]])
			prev = e[1]
		}
		for i := prev; i < len(ln); i++ {
			vis.WriteByte(ln[i])
			on = append(on, colored)
		}
		v := vis.String()
		content := strings.TrimRight(v, " ")
		var runs []string
		for i := 0; i < len(content); {
			if !on[i] {
				i++
				continue
			}
			j := i
			for j < len(content) && on[j] {
				j++
			}
			runs = append(runs, v[i:j])
			i = j
		}
		if carry && len(runs) > 0 && len(on) > 0 && on[0] { // link continues from the previous line
			pieces[len(pieces)-1] += runs[0]
			runs = runs[1:]
		}
		pieces = append(pieces, runs...)
		carry = len(content) > 0 && len(on) >= len(content) && on[len(content)-1]
	}
	var out2 []string
	for _, p := range pieces {
		if p = strings.Trim(p, " "); p != "" {
			out2 = append(out2, p)
		}
	}
	return out2
}

// applyColorSGR updates the "link color on" state for one SGR escape.
func applyColorSGR(colored bool, esc string) bool {
	if !strings.HasSuffix(esc, "m") || !strings.HasPrefix(esc, "\x1b[") {
		return colored
	}
	params := strings.Split(esc[2:len(esc)-1], ";")
	if len(params) == 1 && params[0] == "" {
		return false
	}
	for i := 0; i < len(params); i++ {
		switch params[i] {
		case "0", "", "39":
			colored = false
		case "38":
			if i+4 < len(params) && params[i+1] == "2" {
				colored = params[i+2] == "95" && params[i+3] == "175" && params[i+4] == "255"
				i += 4
			}
		}
	}
	return colored
}

func TestWrapLinkedColorsWholeTokens(t *testing.T) {
	cases := []struct {
		name  string
		in    string
		width int
		want  []string
	}{
		{"wrapped long URL with query and fragment",
			"see https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbbbbbbbbbbbbbb?q=1&r=%20#frag end",
			16, []string{"https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbbbbbbbbbbbbbb?q=1&r=%20#frag"}},
		{"URL with query, fragment and tilde",
			"open https://x.io/p?a=1&b=(2)~#top, ok", 80,
			[]string{"https://x.io/p?a=1&b=(2)~#top"}},
		{"URL followed by Korean particle",
			"https://x.com/a에서 열기", 80,
			[]string{"https://x.com/a"}},
		{"URL followed by ideographic full stop",
			"링크는 https://x.com/a。 끝", 80,
			[]string{"https://x.com/a"}},
		{"wrapped URL followed by Korean particle",
			"https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbb에서", 16,
			[]string{"https://example.com/aaaaaaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbb"}},
		{"home-relative dir",
			"cd ~/dir/sub now", 80, []string{"~/dir/sub"}},
		{"absolute path",
			"run /usr/local/bin/chaude now", 80, []string{"/usr/local/bin/chaude"}},
		{"relative path with line number",
			"see src/main/intt/x.go:12 now", 80, []string{"src/main/intt/x.go:12"}},
		{"wrapped relative path with line number",
			"see src/main/intt/harness/links.go:12 ok", 20,
			[]string{"src/main/intt/harness/links.go:12"}},
		{"path inside backticks",
			"use `src/main/intt/x.go:12` now", 80, []string{"src/main/intt/x.go:12"}},
		{"path inside bold",
			"**src/main/intt/x.go** now", 80, []string{"src/main/intt/x.go"}},
		{"markdown link",
			"[docs](https://x.com/a/b.html) done", 80, []string{"https://x.com/a/b.html"}},
		{"trailing sentence punctuation excluded",
			"see (https://x.com/a). and ./a/b.go.", 80,
			[]string{"https://x.com/a", "./a/b.go"}},
		{"SGR reset inside a link does not end the color",
			"https://x.com/a\x1b[0mb/c end", 80, []string{"https://x.com/ab/c"}},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			wrap := func(s string) string { return lipgloss.NewStyle().Width(c.width).Render(s) }
			out := wrapLinked(mdBold(c.in), wrap)
			if strings.Contains(out, "\x1b[900") {
				t.Fatalf("internal marker leaked: %q", out)
			}
			if got := linkColoredRuns(out); strings.Join(got, "|") != strings.Join(c.want, "|") {
				t.Errorf("colored links = %q, want %q\nout:\n%s", got, c.want, ansi.Strip(out))
			}
			if a, b := strings.Join(strings.Fields(ansi.Strip(out)), ""), strings.Join(strings.Fields(ansi.Strip(mdBold(c.in))), ""); a != b {
				t.Errorf("visible text changed:\n got %q\nwant %q", a, b)
			}
		})
	}
}
