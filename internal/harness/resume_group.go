package harness

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode"
)

// sessionGroup is one conversation made of consecutive Claude Code session files (newest first).
// Picking it resumes the newest file; the older ones are merged in as history.
type sessionGroup struct {
	Files []sessionInfo
}

const (
	groupMaxGap     = 48 * time.Hour // consecutive files further apart start a new group
	groupMinOverlap = 0.2            // Jaccard word overlap of first prompts needed to join
	clearMarkFile   = ".clear-marks" // in the record dir: one "<unixnano>\t<cwd>" line per harness /clear
)

// key is the id list /import loads: all files of the group, oldest first (loadImportArg merges them).
func (g sessionGroup) key() string {
	ids := make([]string, 0, len(g.Files))
	for i := len(g.Files) - 1; i >= 0; i-- {
		ids = append(ids, g.Files[i].ID)
	}
	return strings.Join(ids, ",")
}

// latest is the newest file: its id is the session the group resumes.
func (g sessionGroup) latest() sessionInfo { return g.Files[0] }

func (g sessionGroup) turns() int {
	n := 0
	for _, s := range g.Files {
		n += s.Turns
	}
	return n
}

func (g sessionGroup) label() string {
	oldest, newest := g.Files[len(g.Files)-1], g.Files[0]
	s := fmt.Sprintf("%s  %3d turns  %s … %s", newest.Mod.Format("2006-01-02 15:04"), g.turns(),
		trunc(oldest.First, 40), trunc(newest.Last, 40))
	if n := len(g.Files); n > 1 {
		s += fmt.Sprintf("  (%d files)", n)
	}
	return s
}

// matchesTerm reports whether term (lower case) is a substring of a member's id, name, first/last prompt
// or cwd (or of the project cwd), or, for terms of 4+ letters, one edit away from a word there (typos).
func (g sessionGroup) matchesTerm(term, cwd string) bool {
	for _, s := range g.Files {
		for _, f := range []string{s.ID, s.Name, s.First, s.Last, s.Cwd, cwd} {
			if strings.Contains(strings.ToLower(f), term) {
				return true
			}
			if len([]rune(term)) >= 4 {
				for w := range promptWords(f) {
					if levenshtein(term, w) <= 1 {
						return true
					}
				}
			}
		}
	}
	return false
}

// matchGroups keeps the groups matching every term (case-insensitive; see matchesTerm). No terms keeps all.
func matchGroups(gs []sessionGroup, cwd string, terms []string) []sessionGroup {
	var ts []string
	for _, t := range terms {
		ts = append(ts, strings.Fields(strings.ToLower(t))...)
	}
	if len(ts) == 0 {
		return gs
	}
	var out []sessionGroup
	for _, g := range gs {
		ok := true
		for _, t := range ts {
			if !g.matchesTerm(t, strings.ToLower(cwd)) {
				ok = false
				break
			}
		}
		if ok {
			out = append(out, g)
		}
	}
	return out
}

// levenshtein is the edit distance between a and b (runes).
func levenshtein(a, b string) int {
	ra, rb := []rune(a), []rune(b)
	prev := make([]int, len(rb)+1)
	for j := range prev {
		prev[j] = j
	}
	for i := 1; i <= len(ra); i++ {
		cur := make([]int, len(rb)+1)
		cur[0] = i
		for j := 1; j <= len(rb); j++ {
			cost := 1
			if ra[i-1] == rb[j-1] {
				cost = 0
			}
			cur[j] = min(prev[j]+1, cur[j-1]+1, prev[j-1]+cost)
		}
		prev = cur
	}
	return prev[len(rb)]
}

// sessionStart is the first event time, or the modification time when the file has none.
func sessionStart(s sessionInfo) time.Time {
	if s.Start.IsZero() {
		return s.Mod
	}
	return s.Start
}

// promptWords is the lower-case word set of a prompt (letters and digits, any script).
func promptWords(s string) map[string]bool {
	set := map[string]bool{}
	for _, w := range strings.FieldsFunc(strings.ToLower(s), func(r rune) bool {
		return !unicode.IsLetter(r) && !unicode.IsDigit(r)
	}) {
		set[w] = true
	}
	return set
}

// wordJaccard is |A∩B| / |A∪B| over the word sets of a and b (0 when both are empty).
func wordJaccard(a, b string) float64 {
	wa, wb := promptWords(a), promptWords(b)
	inter := 0
	for w := range wa {
		if wb[w] {
			inter++
		}
	}
	union := len(wa) + len(wb) - inter
	if union == 0 {
		return 0
	}
	return float64(inter) / float64(union)
}

// markBetween reports whether a /clear mark lies in [from, to].
func markBetween(marks []time.Time, from, to time.Time) bool {
	for _, t := range marks {
		if !t.Before(from) && !t.After(to) {
			return true
		}
	}
	return false
}

// continues reports whether cur belongs to the group that prev (its predecessor) ends.
// A /clear always ends a group: prev.Cleared or cur.ClearHead (the /clear is in the transcript), or a
// harness /clear mark between the two files.
func continues(prev, cur sessionInfo, marks []time.Time) bool {
	if prev.Cleared || cur.ClearHead {
		return false
	}
	if markBetween(marks, prev.Mod, sessionStart(cur)) {
		return false
	}
	if prev.Cwd != cur.Cwd {
		return false
	}
	if sessionStart(cur).Sub(prev.Mod) > groupMaxGap {
		return false
	}
	return wordJaccard(prev.First, cur.First) >= groupMinOverlap
}

// groupSessions joins consecutive session files into groups, newest group first. marks are the harness
// /clear times of this project (see loadClearMarks).
func groupSessions(l []sessionInfo, marks []time.Time) []sessionGroup {
	asc := append([]sessionInfo(nil), l...)
	sort.SliceStable(asc, func(i, j int) bool { return sessionStart(asc[i]).Before(sessionStart(asc[j])) })
	var groups [][]sessionInfo
	for _, s := range asc {
		n := len(groups)
		if n > 0 {
			last := groups[n-1]
			if continues(last[len(last)-1], s, marks) {
				groups[n-1] = append(last, s)
				continue
			}
		}
		groups = append(groups, []sessionInfo{s})
	}
	out := make([]sessionGroup, 0, len(groups))
	for _, g := range groups {
		sort.SliceStable(g, func(i, j int) bool { return g[i].Mod.After(g[j].Mod) })
		out = append(out, sessionGroup{Files: g})
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].latest().Mod.After(out[j].latest().Mod) })
	return out
}

// clearMarkPath is the harness /clear mark file of the record dir for cwd.
func clearMarkPath(cwd string) string { return filepath.Join(recordDir(cwd), clearMarkFile) }

// recordClearMark appends one "<unixnano>\t<cwd>" line to path at time at.
func recordClearMark(path, cwd string, at time.Time) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	f, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return err
	}
	defer f.Close()
	_, err = fmt.Fprintf(f, "%d\t%s\n", at.UnixNano(), cwd)
	return err
}

// loadClearMarks returns the marks of cwd in path (nil when the file is missing or has none).
func loadClearMarks(path, cwd string) []time.Time {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	var out []time.Time
	for _, line := range strings.Split(string(data), "\n") {
		ts, c, ok := strings.Cut(line, "\t")
		if !ok || c != cwd {
			continue
		}
		if n, err := strconv.ParseInt(ts, 10, 64); err == nil {
			out = append(out, time.Unix(0, n))
		}
	}
	return out
}

// clearMark records a harness /clear in this project: /resume groups split there.
func (r *Repl) clearMark() {
	if err := recordClearMark(clearMarkPath(r.a.Cwd), r.a.Cwd, time.Now()); err != nil {
		r.p(dim+"clear mark failed: "+err.Error()+rst, "\n")
	}
}

// groupList is the /resume session answer: the groups of this project matching terms.
func (r *Repl) groupList(terms ...string) ([]sessionGroup, string) {
	l, msg := r.sessionList()
	if msg != "" {
		return nil, msg
	}
	marks := loadClearMarks(clearMarkPath(r.a.Cwd), r.a.Cwd)
	gs := matchGroups(groupSessions(l, marks), r.a.Cwd, terms)
	if len(gs) == 0 {
		return nil, "no session matches " + strings.Join(terms, " ")
	}
	return gs, ""
}
