package scripts

import (
	"fmt"
	"io"
	"os"
	"regexp"
	"runtime"
	"strings"
)

func init() { register("charcount", charcount) }

const wsClass = `[\s\p{Z}\x{85}\x{1c}-\x{1f}]`

func charcount(args []string) int {
	fs := newFS("charcount")
	section := fs.String("section", "", "")
	limit := fs.Int("limit", 0, "")
	pos, rc := parseArgs(fs, args)
	if rc >= 0 {
		return rc
	}
	if len(pos) != 1 {
		fmt.Fprintln(os.Stderr, "usage: charcount FILE [--section HEADING] [--limit N]")
		return 2
	}
	var raw []byte
	var err error
	if pos[0] == "-" {
		raw, err = io.ReadAll(os.Stdin)
	} else {
		raw, err = os.ReadFile(pos[0])
	}
	if err != nil {
		return fail("%v", err)
	}
	text := string(raw)
	if pos[0] != "-" || runtime.GOOS == "windows" { // python: files and (windows) stdin use universal newlines
		text = strings.ReplaceAll(text, "\r\n", "\n")
		text = strings.ReplaceAll(text, "\r", "\n")
	}
	if *section != "" {
		re := regexp.MustCompile(`(?m)^(#+)` + wsClass + `*` + regexp.QuoteMeta(*section) + wsClass + `*$`)
		m := re.FindStringSubmatchIndex(text)
		if m == nil {
			return fail("section not found: %s", *section)
		}
		level := m[3] - m[2]
		rest := text[m[1]:]
		end := regexp.MustCompile(fmt.Sprintf(`(?m)^#{1,%d}%s`, level, wsClass)).FindStringIndex(rest)
		if end != nil {
			text = rest[:end[0]]
		} else {
			text = rest
		}
	}
	text = pyStrip(text)

	withNL := runeLen(text)
	noNL := runeLen(strings.NewReplacer("\r", "", "\n", "").Replace(text))
	noWS := 0
	hangul2 := 0
	for _, r := range text {
		if !pyIsSpace(r) {
			noWS++
		}
		if r == '\r' {
			continue
		}
		if r > 127 {
			hangul2 += 2
		} else {
			hangul2++
		}
	}
	fmt.Printf("공백 포함(줄바꿈 포함): %d\n", withNL)
	fmt.Printf("공백 포함(줄바꿈 제외): %d\n", noNL)
	fmt.Printf("공백 제외: %d\n", noWS)
	fmt.Printf("바이트(UTF-8): %d / 바이트(한글 2byte): %d\n", len(text), hangul2)
	if *limit != 0 {
		diff := *limit - noNL
		st := "OK"
		if diff < 0 {
			st = "OVER"
		}
		fmt.Printf("제한 %d: %s (%+d)\n", *limit, st, diff)
	}
	return 0
}
