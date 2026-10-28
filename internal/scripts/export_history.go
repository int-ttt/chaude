package scripts

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

func init() { register("export-history", exportHistory) }

// Stdin is where --hook reads the hook JSON from (hooks that already consumed
// os.Stdin may replace it, e.g. with strings.NewReader(raw)).
var Stdin io.Reader = os.Stdin

func wrapAll(t string, width int) string {
	var out []string
	lines := pySplitLines(t)
	if len(lines) == 0 {
		lines = []string{""}
	}
	for _, l := range lines {
		w := wrapText(l, width)
		if len(w) == 0 {
			w = []string{""}
		}
		out = append(out, w...)
	}
	return strings.Join(out, "\n")
}

func exportOne(path, outDir string, width, splitBytes int) []string {
	base := filepath.Base(path)
	sid := runeSlice(strings.TrimSuffix(base, filepath.Ext(base)), 0, 8)
	var blocks []string
	seen := map[string]bool{}
	firstTS := ""
	_ = eachFileLine(path, func(line string) {
		e := parseLine(line)
		if e == nil {
			return
		}
		kind := strOf(e["type"])
		if (kind != "user" && kind != "assistant") || truthy(e["isMeta"]) {
			return
		}
		t := pyStrip(stripNoise(textOf(asMap(e["message"])["content"])))
		if t == "" {
			return
		}
		ts := tsOf(e)
		if firstTS == "" {
			firstTS = ts
		}
		if kind == "user" && runeLen(t) > 1500 {
			h := sha8(t)
			if seen[h] {
				t = "[이전과 같은 붙여넣기 #" + h + "]"
			}
			seen[h] = true
		}
		who := "Claude"
		if truthy(e["isCompactSummary"]) {
			who = "압축 요약"
		} else if kind == "user" {
			who = "사용자"
		}
		mark := "----"
		if who == "사용자" {
			mark = "===="
		}
		blocks = append(blocks, fmt.Sprintf("%s %s  %s %s\n%s\n\n", mark, ts, who, mark, wrapAll(t, width)))
	})
	if len(blocks) == 0 {
		return nil
	}
	var digits []byte
	for i := 0; i < len(firstTS); i++ {
		if firstTS[i] >= '0' && firstTS[i] <= '9' {
			digits = append(digits, firstTS[i])
		}
	}
	if len(digits) > 12 {
		digits = digits[:12]
	}
	stamp := string(digits)
	if len(digits) >= 12 {
		stamp = stamp[:8] + "-" + stamp[8:12]
	}
	olds, _ := filepath.Glob(filepath.Join(outDir, "*-"+sid+"-*.txt"))
	for _, o := range olds {
		if !strings.HasPrefix(filepath.Base(o), ".") {
			os.Remove(o)
		}
	}
	var files [][]string
	var cur []string
	n := 0
	for _, b := range blocks {
		size := len(b)
		if len(cur) > 0 && n+size > splitBytes {
			files = append(files, cur)
			cur, n = nil, 0
		}
		cur = append(cur, b)
		n += size
	}
	files = append(files, cur)
	var names []string
	for i, part := range files {
		name := fmt.Sprintf("%s-%s-%02d.txt", stamp, sid, i+1)
		_ = os.WriteFile(filepath.Join(outDir, name), []byte(strings.Join(part, "")), 0o644)
		names = append(names, name)
	}
	return names
}

func writeIndex(outDir string) {
	ms, _ := filepath.Glob(filepath.Join(outDir, "*-*-*.txt"))
	var rows []string
	for _, m := range ms {
		if b := filepath.Base(m); !strings.HasPrefix(b, ".") {
			rows = append(rows, b)
		}
	}
	sort.Strings(rows)
	s := "대화 기록 (사용자 메시지와 Claude 답변만, 최신이 아래)\n\n" + strings.Join(rows, "\n") + "\n"
	_ = os.WriteFile(filepath.Join(outDir, "INDEX.txt"), []byte(s), 0o644)
}

func exportHistory(args []string) int {
	fs := newFS("export-history")
	defProj := os.Getenv("CLAUDE_PROJECT_DIR")
	if defProj == "" {
		defProj, _ = os.Getwd()
	}
	project := fs.String("project", defProj, "")
	file := fs.String("file", "", "")
	out := fs.String("out", "", "")
	width := fs.Int("width", 120, "")
	splitKB := fs.Int("split-kb", 500, "")
	hook := fs.Bool("hook", false, "")
	pos, rc := parseArgs(fs, args)
	if rc >= 0 {
		return rc
	}
	if len(pos) > 0 {
		fmt.Fprintln(os.Stderr, "export-history: unrecognized arguments")
		return 2
	}
	var files []string
	if *hook {
		var data map[string]any
		if b, err := io.ReadAll(Stdin); err == nil {
			var v any
			if json.Unmarshal(b, &v) == nil {
				data = asMap(v)
			}
		}
		if p := strOf(data["transcript_path"]); p != "" {
			files = []string{p}
		}
	} else if *file != "" {
		files = []string{*file}
	} else if d := HistoryDir(*project); d != "" {
		files = globDir(d, ".jsonl")
	}
	outDir := *out
	if outDir == "" {
		outDir = filepath.Join(absPath(*project), "logs", "history")
	}
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		return fail("%v", err)
	}
	total := 0
	for _, p := range files {
		if _, err := os.Stat(p); err == nil {
			total += len(exportOne(p, outDir, *width, *splitKB*1024))
		}
	}
	writeIndex(outDir)
	if !*hook {
		fmt.Printf("세션 %d개 → 파일 %d개 · %s\n", len(files), total, outDir)
	}
	return 0
}
