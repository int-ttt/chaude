package harness

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// Limits for the /import view. Long tool results are cut with an explicit "(+N ...)" marker, never silently.
const (
	viewResultLines = 20
	viewResultChars = 4000
	viewInputChars  = 300
)

// viewBuilder renders the whole transcript for the /import view: user and assistant text, thinking,
// tool calls and results, and subagent (sidechain) messages placed right after the Agent call that spawned them.
// The view is display-only; claudeImport.Turns (snapshot input) stays text-only.
type viewBuilder struct {
	turns      [][2]string
	user       string
	asst       []string
	subs       map[string]string // Agent tool_use id -> subagent transcript path (from subagents/*.meta.json)
	subDone    map[string]bool   // subagent transcript path -> already rendered
	inlineUUID map[string]bool   // uuids of sidechain events stored inside the main file (avoid rendering twice)
	nSub       int               // subagent transcripts rendered
}

// newViewBuilder maps Agent tool_use ids to <session>/subagents/agent-*.jsonl through their meta.json files.
func newViewBuilder(path string) *viewBuilder {
	vb := &viewBuilder{subs: map[string]string{}, subDone: map[string]bool{}, inlineUUID: map[string]bool{}}
	dir := filepath.Join(strings.TrimSuffix(path, ".jsonl"), "subagents")
	metas, _ := filepath.Glob(filepath.Join(dir, "*.meta.json"))
	for _, mp := range metas {
		b, err := os.ReadFile(mp)
		if err != nil {
			continue
		}
		var meta struct {
			ToolUseID string `json:"toolUseId"`
		}
		if json.Unmarshal(b, &meta) != nil || meta.ToolUseID == "" {
			continue
		}
		vb.subs[meta.ToolUseID] = strings.TrimSuffix(mp, ".meta.json") + ".jsonl"
	}
	return vb
}

// consume renders one main-file event. Meta (injected context) and non user/assistant events are skipped.
func (vb *viewBuilder) consume(ev event) {
	t := str(ev["type"])
	meta, _ := ev["isMeta"].(bool)
	if (t != "user" && t != "assistant") || meta {
		return
	}
	side, _ := ev["isSidechain"].(bool)
	prefix := ""
	if side {
		prefix = "┆ "
		vb.inlineUUID[str(ev["uuid"])] = true
	}
	vb.event(t, asMap(ev["message"]), side, prefix)
}

func (vb *viewBuilder) event(role string, msg map[string]any, side bool, prefix string) {
	switch c := msg["content"].(type) {
	case string:
		vb.text(role, c, side, prefix)
	case []any:
		for _, bi := range c {
			b := asMap(bi)
			switch str(b["type"]) {
			case "text":
				vb.text(role, str(b["text"]), side, prefix)
			case "thinking":
				if tx := strings.TrimSpace(str(b["thinking"])); tx != "" { // signature-only blocks have no text
					vb.lines(prefix+"[thinking] ", clip(tx), prefix+"  ")
				}
			case "tool_use":
				vb.add(prefix + "▸ " + str(b["name"]) + " " + trunc(jsonStr(b["input"]), viewInputChars))
				if p := vb.subs[str(b["id"])]; p != "" && !vb.subDone[p] {
					vb.subagent(p)
				}
			case "tool_result":
				vb.result(b, prefix)
			}
		}
	}
}

// text: a main-file user prompt starts a new turn; other user text is a note; assistant text is appended.
func (vb *viewBuilder) text(role, s string, side bool, prefix string) {
	tx := strings.TrimSpace(s)
	if tx == "" {
		return
	}
	switch {
	case role == "user" && !side && !strings.HasPrefix(tx, "<"):
		vb.flush()
		vb.user = tx
	case role == "user" && side:
		vb.lines(prefix+"task: ", tx, prefix+"  ")
	case role == "user":
		vb.lines(prefix+"· ", tx, prefix+"  ")
	default:
		vb.lines(prefix, tx, prefix)
	}
}

func (vb *viewBuilder) result(b map[string]any, prefix string) {
	mark := "← "
	if e, _ := b["is_error"].(bool); e {
		mark = "← error: "
	}
	body := strings.TrimSpace(resultText(b["content"]))
	if body == "" {
		vb.add(prefix + mark + "(empty)")
		return
	}
	vb.lines(prefix+mark, clipLines(body), prefix+"  ")
}

// subagent renders one subagent transcript with the sidechain prefix. A file whose first event is already
// stored inside the main transcript is skipped, so its messages do not appear twice.
func (vb *viewBuilder) subagent(path string) {
	vb.subDone[path] = true
	f, err := os.Open(path)
	if err != nil {
		return
	}
	defer f.Close()
	br := bufio.NewReaderSize(f, 1<<20)
	first := true
	for {
		line, rerr := br.ReadString('\n')
		if strings.TrimSpace(line) != "" {
			ev := parseEvent(line)
			if first && vb.inlineUUID[str(ev["uuid"])] {
				return
			}
			first = false
			vb.consume(ev)
		}
		if rerr != nil {
			break
		}
	}
	vb.nSub++
}

// finish renders subagents never linked to a tool call (background runs), flushes the open turn and returns the view.
func (vb *viewBuilder) finish() [][2]string {
	var rest []string
	for _, p := range vb.subs {
		if !vb.subDone[p] {
			rest = append(rest, p)
		}
	}
	sort.Strings(rest)
	for _, p := range rest {
		vb.subagent(p)
	}
	vb.flush()
	return vb.turns
}

func (vb *viewBuilder) flush() {
	if vb.user == "" && len(vb.asst) == 0 {
		return
	}
	u := vb.user
	if u == "" {
		u = "(session continued)"
	}
	vb.turns = append(vb.turns, [2]string{u, strings.Join(vb.asst, "\n")})
	vb.user, vb.asst = "", nil
}

func (vb *viewBuilder) add(s string) { vb.asst = append(vb.asst, strings.TrimRight(s, " ")) }

// lines appends text line by line: the first line gets first, the others rest.
func (vb *viewBuilder) lines(first, text, rest string) {
	for i, l := range strings.Split(text, "\n") {
		p := rest
		if i == 0 {
			p = first
		}
		vb.add(p + l)
	}
}

// clipLines keeps the first viewResultLines lines and then clip()s the rest.
func clipLines(body string) string {
	lines := strings.Split(body, "\n")
	if len(lines) > viewResultLines {
		extra := len(lines) - viewResultLines
		lines = append(lines[:viewResultLines], fmt.Sprintf("… (+%d lines)", extra))
	}
	return clip(strings.Join(lines, "\n"))
}

// clip cuts s to viewResultChars runes and says how many were dropped.
func clip(s string) string {
	r := []rune(s)
	if len(r) <= viewResultChars {
		return s
	}
	return string(r[:viewResultChars]) + fmt.Sprintf(" … (+%d chars)", len(r)-viewResultChars)
}
