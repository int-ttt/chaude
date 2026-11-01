package scripts

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

func init() { register("search-history", searchHistory) }

type hit struct{ ts, sid, who, snip string }

func searchHistory(args []string) int {
	fs := newFS("search-history")
	defProj := os.Getenv("CLAUDE_PROJECT_DIR")
	if defProj == "" {
		defProj, _ = os.Getwd()
	}
	project := fs.String("project", defProj, "")
	since := fs.String("since", "", "")
	who := fs.String("who", "all", "")
	limit := fs.Int("limit", 12, "")
	width := fs.Int("width", 300, "")
	pos, rc := parseArgs(fs, args)
	if rc >= 0 {
		return rc
	}
	if len(pos) != 1 {
		fmt.Fprintln(os.Stderr, `usage: search-history "<terms>" [--project DIR] [--since YYYY-MM-DD] [--who user|claude|all] [--limit N]`)
		return 2
	}
	if *who != "user" && *who != "claude" && *who != "all" {
		fmt.Fprintf(os.Stderr, "search-history: error: argument --who: invalid choice: '%s' (choose from 'user', 'claude', 'all')\n", *who)
		return 2
	}
	rawTerms := pos[0]
	proj := absPath(*project)
	home, _ := os.UserHomeDir()
	var dirs []string
	if d := HistoryDir(proj); d != "" {
		dirs = append(dirs, d)
	}
	if ms, _ := filepath.Glob(filepath.Join(home, "claude-archive", filepath.Base(proj)+"-*")); len(ms) > 0 {
		sort.Strings(ms)
		dirs = append(dirs, ms...)
	}
	var files []string
	for _, d := range dirs {
		files = append(files, globDir(d, ".jsonl")...)
		files = append(files, globDir(d, ".jsonl.gz")...)
	}
	if len(files) == 0 {
		return fail("대화 기록 없음: %s", proj)
	}
	var terms []string
	for _, t := range pyFields(rawTerms) {
		terms = append(terms, lowerRunes(t))
	}
	allIn := func(s string) bool {
		for _, t := range terms {
			if !strings.Contains(s, t) {
				return false
			}
		}
		return true
	}
	var hits []hit
	for _, path := range files {
		sid := runeSlice(filepath.Base(path), 0, 8)
		_ = eachFileLine(path, func(line string) {
			if !allIn(lowerRunes(line)) {
				return
			}
			e := parseLine(line)
			if e == nil {
				return
			}
			kind := strOf(e["type"])
			if (kind != "user" && kind != "assistant") || truthy(e["isMeta"]) {
				return
			}
			if (*who == "user" && kind != "user") || (*who == "claude" && kind != "assistant") {
				return
			}
			ts := tsOf(e)
			if *since != "" && runeSlice(ts, 0, 10) < *since {
				return
			}
			t := strings.Join(pyFields(stripNoise(textOf(asMap(e["message"])["content"]))), " ")
			low := lowerRunes(t)
			if t == "" || !allIn(low) {
				return
			}
			if len(terms) == 0 {
				panic("IndexError: list index out of range")
			}
			i := len([]rune(low[:strings.Index(low, terms[0])]))
			start := i - *width/3
			if start < 0 {
				start = 0
			}
			n := runeLen(t)
			snip := ""
			if start > 0 {
				snip = "…"
			}
			snip += runeSlice(t, start, start+*width)
			if start+*width < n {
				snip += "…"
			}
			w := "Claude"
			if truthy(e["isCompactSummary"]) {
				w = "압축요약"
			} else if kind == "user" {
				w = "사용자"
			}
			hits = append(hits, hit{ts, sid, w, snip})
		})
	}
	sort.SliceStable(hits, func(i, j int) bool { return hits[i].ts > hits[j].ts })
	shown := *limit
	if shown > len(hits) {
		shown = len(hits)
	}
	fmt.Printf("'%s' 일치 %d건 (최신순 %d건 표시, 세션 파일 %d개)\n", rawTerms, len(hits), shown, len(files))
	end := *limit
	if end < 0 {
		end = len(hits) + end
		if end < 0 {
			end = 0
		}
	}
	if end > len(hits) {
		end = len(hits)
	}
	for _, h := range hits[:end] {
		fmt.Printf("- %s [%s] %s: %s\n", h.ts, h.sid, h.who, h.snip)
	}
	return 0
}
