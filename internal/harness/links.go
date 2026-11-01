package harness

import (
	"regexp"
	"sort"
	"strings"
)

// Links in the output pane: URLs (underlined) and file paths get a distinct color. The styling is only
// SGR escapes inserted after wrapping, so visible cells, wrap width and selection coordinates are unchanged.
const (
	linkColorOn  = "\x1b[38;2;95;175;255m"
	linkColorOff = "\x1b[39m"
	linkURLOn    = "\x1b[4m" + linkColorOn
	linkURLOff   = "\x1b[24m" + linkColorOff
)

var (
	linkSGRRe = regexp.MustCompile(`\x1b\[[0-9;:?]*[ -/]*[@-~]`)
	linkURLRe = regexp.MustCompile(`https?://[^\s<>"'\x60]+`)
	linkWinRe = regexp.MustCompile(`[A-Za-z]:[\\/][^\s<>"'|?*\x60]+`)
	linkDotRe = regexp.MustCompile(`\.{1,2}/[\w.\-+@~%]+(?:/[\w.\-+@~%]+)*(?::\d+(?::\d+)?)?`)
	linkRelRe = regexp.MustCompile(`[\w.\-+@~%]+(?:/[\w.\-+@~%]+)+\.[A-Za-z0-9]+(?::\d+(?::\d+)?)?`)
	linkAbsRe = regexp.MustCompile(`/[\w.\-+@~%]+(?:/[\w.\-+@~%]+)+(?::\d+(?::\d+)?)?`)
)

// linkSpan is one link in visible text: byte range [start,end) plus the escapes that open and close it.
type linkSpan struct {
	start, end  int
	open, close string
	text        string
}

// linkTrimTail drops sentence punctuation that closed over a link ("see (https://x.io/a)." -> ".../a").
func linkTrimTail(s string) string {
	for len(s) > 0 && strings.ContainsRune(".,;:!?)]}'\"", rune(s[len(s)-1])) {
		s = s[:len(s)-1]
	}
	return s
}

// linkBoundary reports whether a match at s[start:] begins a token (not the middle of a word, URL or path).
func linkBoundary(s string, start int) bool {
	if start == 0 {
		return true
	}
	c := s[start-1]
	return !(c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' || strings.IndexByte("_./\\-~%+@", c) >= 0)
}

// linkSpans finds URL and path links in visible text. Earlier patterns win overlaps (a URL keeps its
// path-like tail; "a/b/c.go" is one path, not "a/b" plus "/b/c.go").
func linkSpans(s string) []linkSpan {
	var out []linkSpan
	overlaps := func(a, b int) bool {
		for _, sp := range out {
			if a < sp.end && sp.start < b {
				return true
			}
		}
		return false
	}
	for i, re := range []*regexp.Regexp{linkURLRe, linkWinRe, linkDotRe, linkRelRe, linkAbsRe} {
		for _, m := range re.FindAllStringIndex(s, -1) {
			a, b := m[0], m[0]+len(linkTrimTail(s[m[0]:m[1]]))
			if b <= a || !linkBoundary(s, a) || overlaps(a, b) {
				continue
			}
			sp := linkSpan{start: a, end: b, open: linkColorOn, close: linkColorOff, text: s[a:b]}
			if i == 0 {
				sp.open, sp.close = linkURLOn, linkURLOff
			}
			out = append(out, sp)
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].start < out[j].start })
	return out
}

// linkify styles the links of one wrapped line. Existing SGR escapes are copied as they are.
func linkify(line string) string {
	if !strings.ContainsAny(line, "/\\") { // cheap reject: every pattern needs a slash or backslash
		return line
	}
	esc := linkSGRRe.FindAllStringIndex(line, -1)
	var vis strings.Builder
	prev := 0
	for _, e := range esc {
		vis.WriteString(line[prev:e[0]])
		prev = e[1]
	}
	vis.WriteString(line[prev:])
	spans := linkSpans(vis.String())
	if len(spans) == 0 {
		return line
	}
	var sb strings.Builder
	v, si, k := 0, 0, 0 // visible byte offset, next span, next escape
	for i := 0; i < len(line); {
		if k < len(esc) && esc[k][0] == i {
			sb.WriteString(line[i:esc[k][1]])
			i = esc[k][1]
			k++
			continue
		}
		if si < len(spans) && spans[si].start == v {
			sb.WriteString(spans[si].open)
		}
		sb.WriteByte(line[i])
		i++
		v++
		if si < len(spans) && spans[si].end == v {
			sb.WriteString(spans[si].close)
			si++
		}
	}
	return sb.String()
}

// wrapLinked wraps text with wrap, then styles links on every wrapped line outside ``` fences.
// Fenced code is wrapped but left as written.
func wrapLinked(text string, wrap func(string) string) string {
	fence := false
	var runs []string
	var runCode []bool
	var cur []string
	curCode := false
	for _, ln := range strings.Split(text, "\n") {
		marker := strings.HasPrefix(strings.TrimSpace(ln), "```")
		code := fence || marker
		if marker {
			fence = !fence
		}
		if len(cur) > 0 && curCode != code {
			runs, runCode, cur = append(runs, strings.Join(cur, "\n")), append(runCode, curCode), nil
		}
		cur, curCode = append(cur, ln), code
	}
	runs, runCode = append(runs, strings.Join(cur, "\n")), append(runCode, curCode)
	for i, r := range runs {
		s := wrap(r)
		if !runCode[i] {
			ls := strings.Split(s, "\n")
			for j := range ls {
				ls[j] = linkify(ls[j])
			}
			s = strings.Join(ls, "\n")
		}
		runs[i] = s
	}
	return strings.Join(runs, "\n")
}
