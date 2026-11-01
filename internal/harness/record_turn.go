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

// recordResume answers /resume <arg> over the record root (one folder per project). An empty arg lists the
// project folders. A number picks that folder of the list; otherwise arg filters the folders by name or
// content: one match loads all its files as one history (in time order), several match as a list.
func recordResume(root, arg string) (lines []string, blocks []recordBlock, err error) {
	projects, err := listRecordProjects(root)
	if err != nil {
		return nil, nil, err
	}
	if len(projects) == 0 {
		return nil, nil, fmt.Errorf("no record in %s", root)
	}
	if arg == "" {
		return recordProjectLines("record projects (/resume <번호|이름|검색어> opens one):", projects, true), nil, nil
	}
	picked := recordProject{}
	if n, convErr := strconv.Atoi(arg); convErr == nil {
		if n < 1 || n > len(projects) {
			return nil, nil, fmt.Errorf("no project %d (1-%d)", n, len(projects))
		}
		picked = projects[n-1]
	} else {
		matches, err := filterRecordProjects(projects, arg)
		if err != nil {
			return nil, nil, err
		}
		switch len(matches) {
		case 0:
			return nil, nil, fmt.Errorf("no project matches %q", arg)
		case 1:
			picked = matches[0]
		default:
			return recordProjectLines("record projects matching "+arg+" (/resume <이름> opens one):", matches, false), nil, nil
		}
	}
	entries, err := loadRecordProject(picked.Dir)
	if err != nil {
		return nil, nil, err
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
	root := recordDir(r.a.Cwd)
	if err := migrateRecordRoot(root); err != nil {
		r.p("[record] "+err.Error(), "\n")
		return
	}
	if err := recordAppendTurn(recordProjectDir(root, r.a.Cwd), time.Now(), topic, []recordBlock{{"user", prompt}, {"asst", text}}, r.recSplit); err != nil {
		r.p("[record] "+err.Error(), "\n")
		return
	}
	r.recSplit = false
}
