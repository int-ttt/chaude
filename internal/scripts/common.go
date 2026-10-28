package scripts

import (
	"bufio"
	"chaude/internal/lean"
	"compress/gzip"
	"crypto/sha1"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"unicode"
)

// ---- python-compatible string helpers ----

func pyIsSpace(r rune) bool {
	return unicode.IsSpace(r) || (r >= 0x1c && r <= 0x1f)
}

func pyStrip(s string) string { return strings.TrimFunc(s, pyIsSpace) }

func pyFields(s string) []string { return strings.FieldsFunc(s, pyIsSpace) }

func runeLen(s string) int { return len([]rune(s)) }

// pySplitLines mimics str.splitlines() (no keepends).
func pySplitLines(s string) []string {
	var out []string
	var cur []rune
	rs := []rune(s)
	for i := 0; i < len(rs); i++ {
		r := rs[i]
		switch r {
		case '\n', '\v', '\f', 0x1c, 0x1d, 0x1e, 0x85, 0x2028, 0x2029:
			out = append(out, string(cur))
			cur = cur[:0]
		case '\r':
			out = append(out, string(cur))
			cur = cur[:0]
			if i+1 < len(rs) && rs[i+1] == '\n' {
				i++
			}
		default:
			cur = append(cur, r)
		}
	}
	if len(cur) > 0 {
		out = append(out, string(cur))
	}
	return out
}

func lowerRunes(s string) string { return strings.Map(unicode.ToLower, s) }

func sha8(s string) string {
	h := sha1.Sum([]byte(s))
	return hex.EncodeToString(h[:])[:8]
}

func commaInt(n int) string { return commaStr(strconv.Itoa(n)) }

func commaStr(s string) string {
	neg := strings.HasPrefix(s, "-")
	if neg {
		s = s[1:]
	}
	intPart, frac := s, ""
	if i := strings.IndexByte(s, '.'); i >= 0 {
		intPart, frac = s[:i], s[i:]
	}
	var b strings.Builder
	for i, c := range intPart {
		if i > 0 && (len(intPart)-i)%3 == 0 {
			b.WriteByte(',')
		}
		b.WriteRune(c)
	}
	res := b.String() + frac
	if neg {
		res = "-" + res
	}
	return res
}

// ---- textwrap.wrap(replace_whitespace=False, drop_whitespace=False, break_long_words=True, break_on_hyphens=False) ----

func expandTabs(s string, tab int) string {
	var b []rune
	col := 0
	for _, r := range s {
		switch r {
		case '\t':
			if tab > 0 {
				n := tab - col%tab
				for i := 0; i < n; i++ {
					b = append(b, ' ')
				}
				col += n
			}
		case '\n', '\r':
			b = append(b, r)
			col = 0
		default:
			b = append(b, r)
			col++
		}
	}
	return string(b)
}

func isWrapWS(r rune) bool {
	return r == '\t' || r == '\n' || r == '\v' || r == '\f' || r == '\r' || r == ' '
}

func wrapText(text string, width int) []string {
	text = expandTabs(text, 8)
	var chunks [][]rune
	var cur []rune
	curWS := false
	for _, r := range text {
		w := isWrapWS(r)
		if len(cur) > 0 && w != curWS {
			chunks = append(chunks, cur)
			cur = nil
		}
		curWS = w
		cur = append(cur, r)
	}
	if len(cur) > 0 {
		chunks = append(chunks, cur)
	}
	// reverse
	for i, j := 0, len(chunks)-1; i < j; i, j = i+1, j-1 {
		chunks[i], chunks[j] = chunks[j], chunks[i]
	}
	var lines []string
	for len(chunks) > 0 {
		var line []rune
		curLen := 0
		for len(chunks) > 0 {
			l := len(chunks[len(chunks)-1])
			if curLen+l <= width {
				line = append(line, chunks[len(chunks)-1]...)
				chunks = chunks[:len(chunks)-1]
				curLen += l
			} else {
				break
			}
		}
		if len(chunks) > 0 && len(chunks[len(chunks)-1]) > width {
			spaceLeft := 1
			if width >= 1 {
				spaceLeft = width - curLen
			}
			c := chunks[len(chunks)-1]
			end := spaceLeft
			if end > len(c) {
				end = len(c)
			}
			line = append(line, c[:end]...)
			chunks[len(chunks)-1] = c[end:]
		}
		if len(line) > 0 {
			lines = append(lines, string(line))
		}
	}
	return lines
}

// ---- transcript helpers ----

var noiseTags = []string{"command-name", "command-message", "command-args", "local-command-stdout", "system-reminder"}

// stripNoise == re.sub(r"<(tag...)>.*?</\1>", "", s, flags=re.S)
func stripNoise(s string) string {
	var out strings.Builder
	pos := 0
	for {
		bestStart, bestEnd := -1, -1
		for _, tag := range noiseTags {
			open, closeT := "<"+tag+">", "</"+tag+">"
			i := strings.Index(s[pos:], open)
			if i < 0 {
				continue
			}
			i += pos
			j := strings.Index(s[i+len(open):], closeT)
			if j < 0 {
				continue
			}
			end := i + len(open) + j + len(closeT)
			if bestStart < 0 || i < bestStart {
				bestStart, bestEnd = i, end
			}
		}
		if bestStart < 0 {
			break
		}
		out.WriteString(s[pos:bestStart])
		pos = bestEnd
	}
	out.WriteString(s[pos:])
	return out.String()
}

func asMap(v any) map[string]any {
	m, _ := v.(map[string]any)
	return m
}

func truthy(v any) bool {
	switch x := v.(type) {
	case nil:
		return false
	case bool:
		return x
	case float64:
		return x != 0
	case string:
		return x != ""
	case []any:
		return len(x) > 0
	case map[string]any:
		return len(x) > 0
	}
	return true
}

func strOf(v any) string {
	s, _ := v.(string)
	return s
}

func textOf(content any) string {
	switch c := content.(type) {
	case string:
		return c
	case []any:
		for _, it := range c {
			if m := asMap(it); m != nil && m["type"] == "tool_result" {
				return ""
			}
		}
		var parts []string
		for _, it := range c {
			if m := asMap(it); m != nil && m["type"] == "text" {
				parts = append(parts, strOf(m["text"]))
			}
		}
		return strings.Join(parts, "\n")
	}
	return ""
}

// tsOf: (e.get("timestamp") or "")[:16].replace("T"," ")
func tsOf(e map[string]any) string {
	r := []rune(strOf(e["timestamp"]))
	if len(r) > 16 {
		r = r[:16]
	}
	return strings.ReplaceAll(string(r), "T", " ")
}

func runeSlice(s string, a, b int) string {
	r := []rune(s)
	if a > len(r) {
		a = len(r)
	}
	if b > len(r) {
		b = len(r)
	}
	if a >= b {
		return ""
	}
	return string(r[a:b])
}

// eachLine streams text lines (invalid UTF-8 dropped, like errors="ignore").
func eachLine(r io.Reader, fn func(line string)) {
	br := bufio.NewReaderSize(r, 1<<20)
	for {
		s, err := br.ReadString('\n')
		if len(s) > 0 {
			fn(strings.ToValidUTF8(s, ""))
		}
		if err != nil {
			return
		}
	}
}

// eachFileLine opens plain or .gz files.
func eachFileLine(path string, fn func(line string)) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	defer f.Close()
	var r io.Reader = f
	if strings.HasSuffix(path, ".gz") {
		gz, err := gzip.NewReader(f)
		if err != nil {
			return err
		}
		defer gz.Close()
		r = gz
	}
	eachLine(r, fn)
	return nil
}

func parseLine(line string) map[string]any {
	var v any
	if err := json.Unmarshal([]byte(line), &v); err != nil {
		return nil
	}
	return asMap(v)
}

// globDir == sorted glob.glob(dir/*suffix) (hidden names skipped like python's glob)
func globDir(dir, suffix string) []string {
	ents, err := os.ReadDir(dir)
	if err != nil {
		return nil
	}
	var out []string
	for _, e := range ents {
		n := e.Name()
		if strings.HasPrefix(n, ".") || !strings.HasSuffix(n, suffix) {
			continue
		}
		out = append(out, filepath.Join(dir, n))
	}
	sort.Strings(out)
	return out
}

func expandUser(p string) string {
	if p == "~" || strings.HasPrefix(p, "~/") || strings.HasPrefix(p, "~\\") {
		if h, err := os.UserHomeDir(); err == nil {
			return h + p[1:]
		}
	}
	return p
}

func absPath(p string) string {
	a, err := filepath.Abs(expandUser(p))
	if err != nil {
		return p
	}
	return a
}

func newFS(name string) *flag.FlagSet {
	fs := flag.NewFlagSet(name, flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	return fs
}

// parseArgs returns positionals and an exit code (-1 = continue).
func parseArgs(fs *flag.FlagSet, args []string) ([]string, int) {
	pos, err := lean.ParseInterspersed(fs, args)
	if err != nil {
		if err == flag.ErrHelp {
			return nil, 0
		}
		return nil, 2
	}
	return pos, -1
}

func fail(format string, a ...any) int {
	fmt.Fprintf(os.Stderr, format+"\n", a...)
	return 1
}
