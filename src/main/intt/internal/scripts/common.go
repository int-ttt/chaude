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

// helpText is the -h / usage text per script (shown on -h and on flag errors).
var helpText = map[string]string{
	"charcount": `usage: chaude charcount FILE [--section HEADING] [--limit N]
  텍스트/마크다운 파일의 글자 수를 센다 (공백 포함·제외, 바이트). FILE 대신 - 를 주면 표준입력.
  --section HEADING  해당 제목 아래 섹션만 센다
  --limit N          N자 제한과 비교해 초과 여부를 출력`,
	"claude-history": `usage: chaude claude-history [PROJECT_DIR]
  프로젝트(기본: CLAUDE_PROJECT_DIR 또는 현재 폴더)의 Claude Code 대화 기록 폴더 경로를 출력한다.
  기록 폴더는 ~/.claude/projects 아래에 있다. 찾지 못하면 오류 메시지를 stderr로 출력하고 종료 코드 1.`,
	"compress-archive": `usage: chaude compress-archive DIR [DIR...]
  DIR 아래(숨김 폴더 제외)의 .jsonl 대화 기록을 .jsonl.gz로 압축하고 원본을 지운다.
  완료 후 기록 개수와 용량 변화(MB)를 출력한다. 인자 없이 실행하면 사용법을 출력한다.`,
	"export-history": `usage: chaude export-history [--project DIR] [--file F] [--out DIR] [--width N] [--split-kb N] [--hook]
  프로젝트 대화 기록을 읽기 좋은 텍스트 파일로 내보낸다.
  --project DIR   대상 프로젝트 (기본: CLAUDE_PROJECT_DIR 또는 현재 폴더)
  --file F        특정 세션 .jsonl 파일만 내보냄
  --out DIR       출력 폴더
  --width N       줄 너비 (기본 120)
  --split-kb N    출력 파일 분할 크기 KB (기본 500)
  --hook          Claude Code 훅 JSON을 표준입력에서 읽는다`,
	"extract-transcript": `usage: chaude extract-transcript SOURCE [--out DIR] [--chunk N] [--reply-chars N]
  SOURCE(.jsonl 파일, 프로젝트 경로, 또는 ~/.claude/projects 하위 폴더)의 대화를 조각 파일로 나눈다.
  사용자/Claude 메시지만 추출하고, 긴 붙여넣기는 해시로 중복 제거한다.
  --out DIR           출력 폴더 (기본 notes/distill); DIR/chunks/NNN.md 와 DIR/INDEX.md 생성
  --chunk N           조각 크기 목표 글자 수 (기본 7000)
  --reply-chars N     Claude 답변 자르기 글자 수 (기본 800)`,
	"search-history": `usage: chaude search-history "<terms>" [--project DIR] [--since YYYY-MM-DD] [--who user|claude|all] [--limit N] [--width N]
  프로젝트 대화 기록에서 검색어가 포함된 메시지를 찾아 최신순으로 출력한다.
  --project DIR      대상 프로젝트 (기본: CLAUDE_PROJECT_DIR 또는 현재 폴더)
  --since DATE       이 날짜 이후만 (YYYY-MM-DD)
  --who user|claude|all  발화자 필터 (기본 all)
  --limit N          표시 건수 (기본 12)
  --width N          스니펫 너비 (기본 300)`,
	"usage-report": `usage: chaude usage-report [PROJECTS_DIR] [--top N]
  대화 기록(기본 ~/.claude/projects)의 모델별 요청·캐시·출력 토큰과 추정 비용을 집계한다.
  --top N  비용 상위 세션 표시 개수 (기본 15)`,
}

func newFS(name string) *flag.FlagSet {
	fs := flag.NewFlagSet(name, flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	fs.Usage = func() {
		if h, ok := helpText[name]; ok {
			fmt.Fprintln(os.Stderr, h)
		} else {
			fmt.Fprintf(os.Stderr, "usage: chaude %s [args]\n", name)
		}
	}
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
