package harness

import (
	"regexp"
	"sort"
	"strings"
)

// Links in the output pane: URLs (underlined) and file paths get a distinct color. Links are found on the
// whole logical line before wrapping, marked with zero-width private SGR codes, and the marks are turned
// into real styling per wrapped line. A link split by a wrap therefore stays colored on every piece, and
// visible cells, wrap width and selection coordinates are unchanged.
const (
	linkColorOn  = "\x1b[38;2;95;175;255m"
	linkColorOff = "\x1b[39m"
	linkURLOn    = "\x1b[4m" + linkColorOn
	linkURLOff   = "\x1b[24m" + linkColorOff
	// Marks are CSI SGR codes with unused parameters: wrap treats them as zero-width, and restyle removes them.
	markURLOn  = "\x1b[9001m"
	markPathOn = "\x1b[9002m"
	markOff    = "\x1b[9003m"
)

var (
	linkSGRRe = regexp.MustCompile(`\x1b\[[0-9;:?]*[ -/]*[@-~]`)
	linkURLRe = regexp.MustCompile(`https?://[^\s<>"'\x60\x{80}-\x{10FFFF}]+`)
	linkWinRe = regexp.MustCompile(`[A-Za-z]:[\\/][^\s<>"'|?*\x60\x{80}-\x{10FFFF}]+`)
	linkDotRe = regexp.MustCompile(`(?:\.{1,2}|~)/[\w.\-+@~%]+(?:/[\w.\-+@~%]+)*(?::\d+(?::\d+)?)?`)
	linkRelRe = regexp.MustCompile(`[\w.\-+@~%]+(?:/[\w.\-+@~%]+)+\.[A-Za-z0-9]+(?::\d+(?::\d+)?|#L\d+(?:C\d+)?)?`)
	linkAbsRe = regexp.MustCompile(`/[\w.\-+@~%]+(?:/[\w.\-+@~%]+)+(?::\d+(?::\d+)?)?`)
)

// linkSpan is one link in visible text: byte range [start,end) and whether it is a URL.
type linkSpan struct {
	start, end int
	url        bool
	text       string
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
			out = append(out, linkSpan{start: a, end: b, url: i == 0, text: s[a:b]})
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].start < out[j].start })
	return out
}

// markLinks inserts zero-width marks around each link of text (which may hold SGR escapes and newlines).
// Links are found on the visible text, so escapes never split a match and no link crosses a newline.
func markLinks(text string) string {
	if !strings.ContainsAny(text, "/\\") { // cheap reject: every pattern needs a slash or backslash
		return text
	}
	esc := linkSGRRe.FindAllStringIndex(text, -1)
	var vis strings.Builder
	var raw []int // raw[v] is the offset in text of visible byte v
	add := func(from, to int) {
		for i := from; i < to; i++ {
			vis.WriteByte(text[i])
			raw = append(raw, i)
		}
	}
	prev := 0
	for _, e := range esc {
		add(prev, e[0])
		prev = e[1]
	}
	add(prev, len(text))
	spans := linkSpans(vis.String())
	if len(spans) == 0 {
		return text
	}
	var sb strings.Builder
	last := 0
	for _, sp := range spans {
		open := markPathOn
		if sp.url {
			open = markURLOn
		}
		s, e := raw[sp.start], raw[sp.end-1]+1
		sb.WriteString(text[last:s])
		sb.WriteString(open)
		sb.WriteString(text[s:e])
		sb.WriteString(markOff)
		last = e
	}
	sb.WriteString(text[last:])
	return sb.String()
}

// restyle turns the marks in text into link styling, one line at a time. A link that continues onto the
// next line is closed at the end of the line and reopened at the start of it. Any other SGR escape inside
// a link (for example an inline-code reset) is followed by the link style again, so the color stays on.
func restyle(text string) string {
	lines := strings.Split(text, "\n")
	kind := 0 // 0 none, 1 path, 2 URL
	openOf := func(k int) string {
		if k == 2 {
			return linkURLOn
		}
		return linkColorOn
	}
	closeOf := func(k int) string {
		if k == 2 {
			return linkURLOff
		}
		return linkColorOff
	}
	for i, ln := range lines {
		var sb strings.Builder
		if kind != 0 {
			sb.WriteString(openOf(kind))
		}
		for p := 0; p < len(ln); {
			rest := ln[p:]
			esc := ""
			if rest[0] == 0x1b {
				if m := linkSGRRe.FindStringIndex(rest); m != nil && m[0] == 0 {
					esc = rest[:m[1]]
				}
			}
			switch {
			case strings.HasPrefix(rest, markURLOn):
				kind = 2
				sb.WriteString(linkURLOn)
				p += len(markURLOn)
			case strings.HasPrefix(rest, markPathOn):
				kind = 1
				sb.WriteString(linkColorOn)
				p += len(markPathOn)
			case strings.HasPrefix(rest, markOff):
				if kind != 0 {
					sb.WriteString(closeOf(kind))
					kind = 0
				}
				p += len(markOff)
			case esc != "":
				sb.WriteString(esc)
				if kind != 0 {
					sb.WriteString(openOf(kind))
				}
				p += len(esc)
			default:
				sb.WriteByte(rest[0])
				p++
			}
		}
		if kind != 0 {
			sb.WriteString(closeOf(kind))
		}
		lines[i] = sb.String()
	}
	return strings.Join(lines, "\n")
}

// linkify styles the links of one line (no wrapping). Existing SGR escapes are copied as they are.
func linkify(line string) string {
	return restyle(markLinks(line))
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
		if runCode[i] {
			runs[i] = wrap(r)
			continue
		}
		runs[i] = restyle(wrap(markLinks(r)))
	}
	return strings.Join(runs, "\n")
}
