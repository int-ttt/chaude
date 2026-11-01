package harness

import (
	"bufio"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
)

// inputHistory is the up/down prompt recall list (shell style). Pure state, no I/O.
type inputHistory struct {
	items []string
	idx   int // == len(items) while not browsing
	draft string
}

const maxHistory = 500

// historyFileFor is <project cwd>/.chaude/history.jsonl, one JSON string per line (oldest first).
// An empty cwd means the process working directory, as settingsFileFor does.
func historyFileFor(cwd string) string {
	if cwd == "" {
		wd, err := os.Getwd()
		if err != nil {
			return ""
		}
		cwd = wd
	}
	return filepath.Join(cwd, ".chaude", "history.jsonl")
}

// loadInputHistory reads the saved prompts at path. Missing or undecodable lines are skipped; the
// Add rules (dedupe consecutive, blanks, cap) apply. A missing file yields an empty list.
func loadInputHistory(path string) []string {
	if path == "" {
		return nil
	}
	f, err := os.Open(path)
	if err != nil {
		return nil
	}
	defer f.Close()
	var h inputHistory
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 0, 64*1024), 16*1024*1024)
	for sc.Scan() {
		var s string
		if json.Unmarshal([]byte(strings.TrimSpace(sc.Text())), &s) == nil {
			h.Add(s)
		}
	}
	return h.items
}

// saveInputHistory rewrites path with items through a tmp file and rename.
func saveInputHistory(path string, items []string) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	var sb strings.Builder
	for _, s := range items {
		b, err := json.Marshal(s)
		if err != nil {
			return err
		}
		sb.Write(b)
		sb.WriteByte('\n')
	}
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, []byte(sb.String()), 0o600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}

// Add records a submitted prompt (consecutive duplicates and blanks are skipped) and ends browsing.
func (h *inputHistory) Add(s string) {
	if s != "" && (len(h.items) == 0 || h.items[len(h.items)-1] != s) {
		h.items = append(h.items, s)
		if len(h.items) > maxHistory {
			h.items = h.items[len(h.items)-maxHistory:]
		}
	}
	h.idx, h.draft = len(h.items), ""
}

// Prev steps back; cur is the current input (saved as draft when browsing starts).
func (h *inputHistory) Prev(cur string) (string, bool) {
	if len(h.items) == 0 || h.idx == 0 {
		return "", false
	}
	if h.idx >= len(h.items) {
		h.idx, h.draft = len(h.items), cur
	}
	h.idx--
	return h.items[h.idx], true
}

// Next steps forward; past the newest entry it returns the saved draft.
func (h *inputHistory) Next() (string, bool) {
	if h.idx >= len(h.items) {
		return "", false
	}
	h.idx++
	if h.idx == len(h.items) {
		return h.draft, true
	}
	return h.items[h.idx], true
}

// Browsing reports whether a history entry is currently shown.
func (h *inputHistory) Browsing() bool { return h.idx < len(h.items) }

// rewindState is the turn picker for "go back to an earlier prompt".
// Unit of rewind = one user turn (a "user" block and everything after it in the transcript view).
type rewindState struct {
	open  bool
	turns []int // indices into blocks of "user" blocks that are real prompts
	sel   int
}

// Open lists prompt turns (slash commands excluded). It reports false when there is nothing to rewind to.
func (s *rewindState) Open(blocks []block) bool {
	s.turns, s.sel = nil, 0
	for i, b := range blocks {
		if b.kind == "user" && len(b.text) > 0 && b.text[0] != '/' {
			s.turns = append(s.turns, i)
		}
	}
	s.open = len(s.turns) > 0
	s.sel = len(s.turns) - 1
	return s.open
}

func (s *rewindState) Move(d int) {
	if n := len(s.turns); n > 0 {
		s.sel = (s.sel + d + n) % n
	}
}

func (s *rewindState) Cancel() { s.open = false }

// Confirm closes the picker and returns the cut index (blocks[:cut] are kept) and the prompt text to re-edit.
func (s *rewindState) Confirm(blocks []block) (cut int, text string, ok bool) {
	if !s.open || s.sel < 0 || s.sel >= len(s.turns) {
		return 0, "", false
	}
	s.open = false
	cut = s.turns[s.sel]
	return cut, blocks[cut].text, true
}
