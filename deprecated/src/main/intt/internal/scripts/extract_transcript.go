package scripts

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

func init() { register("extract-transcript", extractTranscript) }

func clip(s string, n int) string {
	r := []rune(s)
	if len(r) <= n {
		return s
	}
	head := n * 5 / 8
	tailN := n * 3 / 8
	tail := r
	if tailN > 0 {
		tail = r[len(r)-tailN:]
	}
	if head > len(r) {
		head = len(r)
	}
	return string(r[:head]) + fmt.Sprintf("\n[…%d자 생략…]\n", len(r)-n) + string(tail)
}

type entry struct{ ts, sid, label, text string }

func extractTranscript(args []string) int {
	fs := newFS("extract-transcript")
	out := fs.String("out", "notes/distill", "")
	chunkN := fs.Int("chunk", 7000, "")
	replyChars := fs.Int("reply-chars", 800, "")
	pos, rc := parseArgs(fs, args)
	if rc >= 0 {
		return rc
	}
	if len(pos) != 1 {
		fmt.Fprintln(os.Stderr, "usage: extract-transcript SOURCE [--out DIR] [--chunk N] [--reply-chars N]")
		return 2
	}
	src := absPath(pos[0])
	var files []string
	if strings.HasSuffix(src, ".jsonl") {
		files = []string{src}
	} else {
		d := ""
		if normCase(filepath.Dir(src)) == normCase(ProjectsRoot()) {
			d = src
		} else {
			d = HistoryDir(src)
		}
		if d != "" {
			files = globDir(d, ".jsonl")
		}
	}
	if len(files) == 0 {
		return fail("대화 기록을 찾지 못했습니다: %s\n~/.claude/projects 아래 폴더를 직접 지정해 보세요.", filepath.ToSlash(src))
	}
	var entries []entry
	seenPaste := map[string]bool{}
	for _, path := range files {
		base := filepath.Base(path)
		sid := runeSlice(strings.TrimSuffix(base, filepath.Ext(base)), 0, 8)
		if err := eachFileLine(path, func(line string) {
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
			label := ""
			if kind == "user" && truthy(e["isCompactSummary"]) {
				label, t = "압축요약", clip(t, 4000)
			} else if kind == "user" {
				label = "사용자"
				if runeLen(t) > 1500 {
					h := sha8(t)
					if seenPaste[h] {
						t = "[이전과 같은 붙여넣기 #" + h + " 반복]"
					} else {
						seenPaste[h] = true
						t = "[붙여넣기 #" + h + "]\n" + clip(t, 3000)
					}
				}
			} else {
				label, t = "Claude", clip(t, *replyChars)
			}
			entries = append(entries, entry{ts, sid, label, t})
		}); err != nil {
			return fail("%v", err)
		}
	}
	sort.SliceStable(entries, func(i, j int) bool { return entries[i].ts < entries[j].ts })
	if err := os.MkdirAll(filepath.Join(*out, "chunks"), 0o755); err != nil {
		return fail("%v", err)
	}
	var chunks []string
	cur, curLen := "", 0
	for _, en := range entries {
		block := fmt.Sprintf("### %s [%s] %s\n%s\n\n", en.ts, en.sid, en.label, en.text)
		bl := runeLen(block)
		if curLen > 0 && curLen+bl > *chunkN {
			chunks = append(chunks, cur)
			cur, curLen = "", 0
		}
		cur += block
		curLen += bl
	}
	if curLen > 0 {
		chunks = append(chunks, cur)
	}
	total := 0
	for i, c := range chunks {
		_ = os.WriteFile(filepath.Join(*out, "chunks", fmt.Sprintf("%03d.md", i+1)), []byte(c), 0o644)
		total += runeLen(c)
	}
	first, last := "-", "-"
	if len(entries) > 0 {
		first, last = entries[0].ts, entries[len(entries)-1].ts
	}
	par := (len(chunks) + 3) / 4
	if par < 1 {
		par = 1
	}
	idx := fmt.Sprintf("# 추출 인덱스\n- 원본: %d개 세션, 메시지 %d개\n- 조각: %d개, 총 %s자\n", len(files), len(entries), len(chunks), commaInt(total))
	idx += fmt.Sprintf("- 기간: %s ~ %s\n", first, last)
	idx += fmt.Sprintf("- 권장 배분: haiku-extractor 1개당 조각 4개 → %d개 병렬\n", par)
	_ = os.WriteFile(filepath.Join(*out, "INDEX.md"), []byte(idx), 0o644)
	fmt.Printf("세션 %d개 · 메시지 %d개 → 조각 %d개 (%s자) · %s\n", len(files), len(entries), len(chunks), commaInt(total), filepath.ToSlash(filepath.Join(*out, "INDEX.md")))
	return 0
}
