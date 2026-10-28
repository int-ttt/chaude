package harness

import (
	"regexp"
	"strings"
	"unicode/utf8"
)

// categoryRule is appended to the main session's system prompt (<=500 tokens).
const categoryRule = "Topic tracking: end EVERY reply with one last line, exactly `[cat: X]`, where X is a 1-3 word label of the " +
	"task category the user is working in (e.g. chaude-metrics, resume-writing). Keep the label stable while the user continues work " +
	"within the same category. If the user's latest prompt starts work in a clearly different category, end with `[cat-switch: X]` instead. " +
	"Follow-ups, fixes, questions or small asides about the current work are NOT a switch. Short acknowledgements or replies " +
	"(ㅇㅇ, ok, 응, 좋아, 계속, 네, thanks) NEVER switch. Decide quickly and briefly; do no extra analysis for this."

var catRe = regexp.MustCompile("(?m)^[\\s*>`]*\\[(cat|cat-switch):\\s*([^\\]\\n]*?)\\s*\\][\\s*`]*$")

var ackWords = map[string]bool{"ㅇㅇ": true, "ㅇㅋ": true, "응": true, "네": true, "넵": true, "예": true, "좋아": true, "계속": true,
	"ok": true, "okay": true, "yes": true, "y": true, "go": true, "thanks": true, "ㄱㄱ": true, "고": true}

// isAck reports whether prompt is a short acknowledgement that must never switch category.
func isAck(prompt string) bool {
	p := strings.ToLower(strings.Trim(strings.TrimSpace(prompt), ".!?~ "))
	return p == "" || ackWords[p] || utf8.RuneCountInString(p) <= 2
}

// parseCategory returns the last category marker of reply: label and whether it announces a switch.
func parseCategory(reply string) (cat string, sw bool) {
	ms := catRe.FindAllStringSubmatch(reply, -1)
	if len(ms) == 0 {
		return "", false
	}
	m := ms[len(ms)-1]
	return m[2], m[1] == "cat-switch"
}

// categoryRotation decides whether to rotate after a turn. It returns the new category label and
// whether to rotate. Disabled mode, short acks, a missing label, or a switch to the same label never rotate.
func categoryRotation(prompt, reply, cur string, disabled bool) (cat string, rotate bool) {
	cat, sw := parseCategory(reply)
	if cat == "" {
		return cur, false
	}
	if disabled || !sw || isAck(prompt) || strings.EqualFold(cat, cur) {
		return cat, false
	}
	return cat, true
}

var catLineRe = regexp.MustCompile("^[\\s*>`]*\\[(cat|cat-switch):[^\\]\\n]*\\][\\s*`]*$")

// markerFilter strips [cat: X] / [cat-switch: X] lines from streamed text. A trailing partial line that
// starts with '[' is held back until it is complete.
type markerFilter struct{ pend string }

// Feed returns the text that is safe to display after adding chunk.
func (f *markerFilter) Feed(chunk string) string {
	f.pend += chunk
	i := strings.LastIndex(f.pend, "\n")
	var out strings.Builder
	complete := ""
	if i >= 0 {
		complete, f.pend = f.pend[:i+1], f.pend[i+1:]
	}
	for _, ln := range strings.SplitAfter(complete, "\n") {
		if ln != "" && !catLineRe.MatchString(strings.TrimRight(ln, "\r\n")) {
			out.WriteString(ln)
		}
	}
	if !strings.HasPrefix(strings.TrimLeft(f.pend, " \t*>`"), "[") && strings.TrimSpace(f.pend) != "" {
		out.WriteString(f.pend)
		f.pend = ""
	}
	return out.String()
}

// Flush returns the held remainder (dropped when it is a marker line).
func (f *markerFilter) Flush() string {
	p := f.pend
	f.pend = ""
	if catLineRe.MatchString(strings.TrimSpace(p)) {
		return ""
	}
	return p
}
