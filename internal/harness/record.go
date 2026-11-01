package harness

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode"
)

// recordBlock is one saved output block of a turn. Kind: user | asst | note | tool | result.
type recordBlock struct{ Kind, Text string }

// recordEntry is one block parsed back from a record file.
type recordEntry struct {
	Num   int
	Kind  string
	TS    string
	Topic string
	Text  string
}

const (
	recordMaxBytes   = 200 << 10 // file cap: a new NNN file starts once a turn would exceed it
	recordMaxAnswers = 100       // file cap: answers (asst blocks) per NNN file
	recordLockName   = ".record.lock"
	recordTailBytes  = 8 << 10 // bytes read from the end of a file to find its last rec number
)

var (
	recordHeaderRe   = regexp.MustCompile(`(?m)^<!-- rec #(\d+) (\w+) (\S+) cat=(.*) -->\n`)
	recordEscapeRe   = regexp.MustCompile(`(?m)^(\\*<!-- rec #)`)   // body line that looks like a header gets one more \
	recordUnescapeRe = regexp.MustCompile(`(?m)^\\(\\*<!-- rec #)`) // and loses one \ when read back
)

// recordFile is one *.md file found in a project folder.
type recordFile struct {
	name string
	last int       // rec number of the last turn in the file (0 if none)
	mod  time.Time // file mod time
}

// recordDir is the record folder: HARNESS_RECORD_DIR if set, else <main checkout root>/record
// (the same root for every stacked worktree), else cwd/record.
func recordDir(cwd string) string {
	if d := os.Getenv("HARNESS_RECORD_DIR"); d != "" {
		return d
	}
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	cmd := exec.Command("git", "rev-parse", "--git-common-dir")
	cmd.Dir = cwd
	if out, err := cmd.Output(); err == nil {
		if gc := strings.TrimSpace(string(out)); gc != "" {
			if !filepath.IsAbs(gc) {
				gc = filepath.Join(cwd, gc)
			}
			return filepath.Join(filepath.Dir(gc), "record")
		}
	}
	return filepath.Join(cwd, "record")
}

// recordAppendTurn appends one turn (all blocks share one rec number) to the project folder dir with a
// single O_APPEND write under the dir lock. The turn goes to the newest file of dir unless split is set
// (a /clear) or that file is at a cap; then a new file named by now is started. A turn larger than the
// cap stays in one file (accepted: no splitting inside a turn).
func recordAppendTurn(dir string, now time.Time, topic string, turn []recordBlock, split bool) error {
	if len(turn) == 0 {
		return nil
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	unlock, err := lockRecordDir(dir)
	if err != nil {
		return err
	}
	defer unlock()

	now = now.Local()
	files, err := listRecordFiles(dir)
	if err != nil {
		return err
	}
	num := 1
	var newest *recordFile // file holding the newest rec number
	for i := range files {
		f := &files[i]
		if f.last+1 > num {
			num = f.last + 1
		}
		if newest == nil || f.last > newest.last {
			newest = f
		}
	}
	payload := recordRender(num, now, topic, turn)
	answers := 0
	for _, bl := range turn {
		if bl.Kind == "asst" {
			answers++
		}
	}

	name := fmt.Sprintf("%s-%04d.md", now.Format("060102-150405"), num)
	if newest != nil && !split {
		data, err := os.ReadFile(filepath.Join(dir, newest.name))
		if err != nil {
			return err
		}
		have := 0
		for _, e := range parseRecordEntries(string(data)) {
			if e.Kind == "asst" {
				have++
			}
		}
		if len(data)+len(payload) <= recordMaxBytes && have+answers <= recordMaxAnswers {
			name = newest.name
		}
	}

	f, err := os.OpenFile(filepath.Join(dir, name), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return err
	}
	if _, err := f.Write([]byte(payload)); err != nil {
		f.Close()
		return err
	}
	return f.Close()
}

// nextRecordNumber returns the rec number for the next turn: max last rec number over all record
// files, plus one (1 when the dir has no record). File mtimes are not used, so touching a file cannot
// lower the number.
func nextRecordNumber(dir string) (int, error) {
	files, err := listRecordFiles(dir)
	if err != nil {
		return 0, err
	}
	n := 0
	for _, f := range files {
		if f.last > n {
			n = f.last
		}
	}
	return n + 1, nil
}

// listRecordFiles returns the record files (*.md) in dir, oldest rec number first. Names are not parsed:
// the rec number of the last turn in each file (lastRecordNum) gives the order.
func listRecordFiles(dir string) ([]recordFile, error) {
	ents, err := os.ReadDir(dir)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	var out []recordFile
	for _, e := range ents {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".md") {
			continue
		}
		path := filepath.Join(dir, e.Name())
		last, err := lastRecordNum(path)
		if err != nil {
			return nil, err
		}
		info, err := e.Info()
		if err != nil {
			return nil, err
		}
		out = append(out, recordFile{name: e.Name(), last: last, mod: info.ModTime()})
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].last < out[j].last })
	return out, nil
}

// lastRecordNum returns the rec number of the last turn in a record file, reading only the file tail
// unless the last turn is longer than the tail.
func lastRecordNum(path string) (int, error) {
	f, err := os.Open(path)
	if err != nil {
		return 0, err
	}
	defer f.Close()
	st, err := f.Stat()
	if err != nil {
		return 0, err
	}
	off := st.Size() - recordTailBytes
	if off < 0 {
		off = 0
	}
	buf := make([]byte, st.Size()-off)
	if _, err := f.ReadAt(buf, off); err != nil && err != io.EOF {
		return 0, err
	}
	data := string(buf)
	if off > 0 { // drop the partial first line
		i := strings.IndexByte(data, '\n')
		if i < 0 {
			data = ""
		} else {
			data = data[i+1:]
		}
	}
	if n := lastNumIn(data); n > 0 || off == 0 {
		return n, nil
	}
	all, err := os.ReadFile(path)
	if err != nil {
		return 0, err
	}
	return lastNumIn(string(all)), nil
}

func lastNumIn(data string) int {
	ms := recordHeaderRe.FindAllStringSubmatch(data, -1)
	if len(ms) == 0 {
		return 0
	}
	n, _ := strconv.Atoi(ms[len(ms)-1][1])
	return n
}

// recordCat is the one-line topic written into headers: whitespace collapsed, "-->" cannot end the comment.
func recordCat(topic string) string {
	return strings.ReplaceAll(strings.Join(strings.Fields(topic), " "), "-->", "->")
}

// recordRender formats one turn. Per block: header comment, heading, blank line, text, blank line.
// Text is written without trailing newlines so parseRecordEntries returns it unchanged.
func recordRender(num int, ts time.Time, topic string, turn []recordBlock) string {
	cat := recordCat(topic)
	hm, iso := ts.Format("15:04"), ts.Format("2006-01-02T15:04")
	var b strings.Builder
	for _, bl := range turn {
		fmt.Fprintf(&b, "<!-- rec #%04d %s %s cat=%s -->\n", num, bl.Kind, iso, cat)
		fmt.Fprintf(&b, "## #%04d · %s · %s\n\n", num, hm, cat)
		b.WriteString(recordEscapeRe.ReplaceAllString(strings.TrimRight(bl.Text, "\n"), `\$1`))
		b.WriteString("\n\n")
	}
	return b.String()
}

// parseRecordEntries reads blocks back from record file content, in file order.
func parseRecordEntries(data string) []recordEntry {
	locs := recordHeaderRe.FindAllStringSubmatchIndex(data, -1)
	out := make([]recordEntry, 0, len(locs))
	for i, loc := range locs {
		end := len(data)
		if i+1 < len(locs) {
			end = locs[i+1][0]
		}
		seg := data[loc[1]:end]
		if j := strings.IndexByte(seg, '\n'); j >= 0 { // drop heading line
			seg = seg[j+1:]
		}
		seg = strings.TrimSuffix(strings.TrimPrefix(seg, "\n"), "\n\n")
		seg = recordUnescapeRe.ReplaceAllString(seg, "$1")
		num, _ := strconv.Atoi(data[loc[2]:loc[3]])
		out = append(out, recordEntry{
			Num:   num,
			Kind:  data[loc[4]:loc[5]],
			TS:    data[loc[6]:loc[7]],
			Topic: data[loc[8]:loc[9]],
			Text:  seg,
		})
	}
	return out
}

// recordTopicOf picks the topic of a finished turn: last [cat-switch: X], else last [cat: X], else prev.
// It reuses catRe from category.go.
func recordTopicOf(reply, prev string) string {
	lastSwitch, lastCat := "", ""
	for _, m := range catRe.FindAllStringSubmatch(reply, -1) {
		label := strings.TrimSpace(m[2])
		if label == "" {
			continue
		}
		if m[1] == "cat-switch" {
			lastSwitch = label
		} else {
			lastCat = label
		}
	}
	switch {
	case lastSwitch != "":
		return lastSwitch
	case lastCat != "":
		return lastCat
	}
	return prev
}

// slugify makes a file-name part from a topic: Korean kept; whitespace, control characters and
// /:*?<>|"\ become '-'; at most 30 runes; empty becomes "misc".
func slugify(topic string) string {
	var b strings.Builder
	n := 0
	for _, r := range strings.TrimSpace(topic) {
		if n == 30 {
			break
		}
		if unicode.IsSpace(r) || unicode.IsControl(r) || strings.ContainsRune(`/:*?<>|"\`, r) {
			r = '-'
		}
		b.WriteRune(r)
		n++
	}
	if b.Len() == 0 {
		return "misc"
	}
	return b.String()
}
