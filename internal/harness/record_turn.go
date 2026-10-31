package harness

import (
	"fmt"
	"sort"
	"strconv"
	"strings"
	"time"
)

// recordSkip reports prompts whose answer is never saved: slash commands and empty prompts.
func recordSkip(prompt string) bool {
	f := strings.Fields(prompt)
	return len(f) == 0 || strings.HasPrefix(f[0], "/")
}

// measureKeys are the fixed measurement keys, in output order. result_* keys follow, sorted.
var measureKeys = []string{"duration_ms", "cost_turn_usd", "cost_usd", "ctx", "rotations", "delegations"}

// measureLine is the one-line per-turn measurement ("[측정] k=v ..."), numeric values only.
// It returns "" when no key is present.
func measureLine(fields map[string]any) string {
	var extra []string
	for k := range fields {
		if strings.HasPrefix(k, "result_") {
			extra = append(extra, k)
		}
	}
	sort.Strings(extra)
	var parts []string
	for _, k := range append(append([]string{}, measureKeys...), extra...) {
		var s string
		switch x := fields[k].(type) {
		case int:
			s = fmt.Sprintf("%d", x)
		case int64:
			s = fmt.Sprintf("%d", x)
		case float64:
			s = fmt.Sprintf("%.4g", x)
		default:
			continue
		}
		parts = append(parts, k+"="+s)
	}
	if len(parts) == 0 {
		return ""
	}
	return "[측정] " + strings.Join(parts, " ")
}

// recordTopicFor takes the reply of the turn that just ended (r.lastReply, cleared here). It returns the
// topic the answer is saved under and the reply without [cat:] markers. topic is "" when the turn is not saved.
func (r *Repl) recordTopicFor(prompt string) (topic, text string) {
	reply := r.lastReply
	r.lastReply = ""
	if recordSkip(prompt) || reply == "" {
		return "", ""
	}
	r.recTopic = recordTopicOf(reply, r.recTopic)
	return r.recTopic, strings.TrimSpace(catRe.ReplaceAllString(reply, ""))
}

// recordResume answers /resume <arg>. An empty arg returns the topic list lines. Otherwise arg is a list
// number or a slug, and the blocks of that topic are returned (the last recordShowMax, with a leading note when cut).
func recordResume(dir, arg string) (lines []string, blocks []recordBlock, err error) {
	topics, err := listRecordTopics(dir)
	if err != nil {
		return nil, nil, err
	}
	if len(topics) == 0 {
		return nil, nil, fmt.Errorf("no record in %s", dir)
	}
	if arg == "" {
		lines = []string{"record topics (/resume <번호|slug> shows one):"}
		for i, t := range topics {
			lines = append(lines, fmt.Sprintf("%2d. %s · %s~%s · 파일 %d개 · 답변 %d개", i+1, t.Slug, t.First, t.Last, len(t.Files), t.Answers))
		}
		return lines, nil, nil
	}
	slug := arg
	if n, convErr := strconv.Atoi(arg); convErr == nil {
		if n < 1 || n > len(topics) {
			return nil, nil, fmt.Errorf("no topic %d (1-%d)", n, len(topics))
		}
		slug = topics[n-1].Slug
	}
	entries, err := loadRecordTopic(dir, slug)
	if err != nil {
		return nil, nil, err
	}
	if len(entries) > recordShowMax {
		blocks = append(blocks, recordBlock{"note", fmt.Sprintf("앞 %d개 생략 (최근 %d개만 표시)", len(entries)-recordShowMax, recordShowMax)})
		entries = entries[len(entries)-recordShowMax:]
	}
	for _, e := range entries {
		blocks = append(blocks, recordBlock{e.Kind, e.Text})
	}
	return nil, blocks, nil
}

// recordResumePlain prints /resume output in the plain REPL.
func (r *Repl) recordResumePlain(arg string) {
	lines, blocks, err := recordResume(recordDir(r.a.Cwd), arg)
	if err != nil {
		r.p(err.Error(), "\n")
		return
	}
	for _, l := range lines {
		r.p(l, "\n")
	}
	for _, b := range blocks {
		switch b.Kind {
		case "user":
			r.p("> "+b.Text, "\n")
		case "note":
			r.p(dim+b.Text+rst, "\n")
		default:
			r.p(b.Text, "\n")
		}
	}
}

// recordSave saves the turn that just ended (prompt and r.lastReply) under its topic, unless skipped.
func (r *Repl) recordSave(prompt string) {
	defer func() { r.lastMeasure = "" }()
	topic, text := r.recordTopicFor(prompt)
	if topic == "" {
		return
	}
	if r.lastMeasure != "" {
		text += "\n\n" + r.lastMeasure
	}
	if err := recordAppendTurn(recordDir(r.a.Cwd), time.Now(), topic, []recordBlock{{"user", prompt}, {"asst", text}}); err != nil {
		r.p("[record] "+err.Error(), "\n")
	}
}
