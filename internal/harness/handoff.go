package harness

import (
	"path/filepath"
	"strings"
	"unicode"
	"unicode/utf8"
)

const recoPrefix = "추천:"

// handoffSections lists the required handoff sections. A section counts as
// present when any of its alternatives appears (case-insensitive substring).
var handoffSections = []struct {
	name string
	alts []string
}{
	{"goal", []string{"goal"}},
	{"decision", []string{"decision"}},
	{"files", []string{"files"}},
	{"in-progress", []string{"in-progress", "in progress"}},
	{"open question", []string{"open question"}},
	{"preference", []string{"preference", "instruction"}},
	{"exact", []string{"exact"}},
}

// extractReco returns the last "추천:" line of reply, from the start of that
// line to the end of reply with trailing whitespace trimmed. Leading '*', '#',
// '>' and spaces before the marker are allowed. Returns "" when there is none.
func extractReco(reply string) string {
	start := -1
	offset := 0
	for _, line := range strings.SplitAfter(reply, "\n") {
		body := strings.TrimLeft(line, "*#> \t")
		if strings.HasPrefix(body, recoPrefix) {
			start = offset
		}
		offset += len(line)
	}
	if start < 0 {
		return ""
	}
	return strings.TrimRightFunc(reply[start:], unicode.IsSpace)
}

// validateHandoff returns the list of problems with note; empty means valid.
func validateHandoff(note, reco string, changed []string) []string {
	var problems []string
	if utf8.RuneCountInString(strings.TrimSpace(note)) < 200 {
		problems = append(problems, "too short")
	}
	lower := strings.ToLower(note)
	for _, s := range handoffSections {
		found := false
		for _, alt := range s.alts {
			if strings.Contains(lower, alt) {
				found = true
				break
			}
		}
		if !found {
			problems = append(problems, "missing section: "+s.name)
		}
	}
	for _, path := range changed {
		base := filepath.Base(path)
		if !strings.Contains(note, base) {
			problems = append(problems, "file not mentioned: "+base)
		}
	}
	return problems
}

// withReco prepends the previous recommendation verbatim to snapshot.
func withReco(reco, snapshot string) string {
	if reco == "" {
		return snapshot
	}
	return "## Previous recommendation (verbatim)\n" + reco + "\n\n" + snapshot
}

// recoPreserved reports whether reco appears byte-for-byte in firstMsg.
func recoPreserved(reco, firstMsg string) bool {
	if reco == "" {
		return true
	}
	return strings.Contains(firstMsg, reco)
}
