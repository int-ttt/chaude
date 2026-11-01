package harness

import (
	"fmt"
	"os"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/atotto/clipboard"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/x/ansi"
)

// textPos is a point in the transcript in content coordinates: line is an index into the rendered
// transcript lines (not a screen row) and col is a cell column. New output is appended below, so a stored
// pos keeps pointing at the same text while the answer streams and while the view scrolls.
type textPos struct{ line, col int }

// textSel is a drag selection between two content positions. anchor is where the press started, head is
// the latest drag/release point; the two may be in either order.
type textSel struct {
	set          bool
	anchor, head textPos
}

// ordered returns the selection start and end (end exclusive in col).
func (s textSel) ordered() (textPos, textPos) {
	a, b := s.anchor, s.head
	if b.line < a.line || (b.line == a.line && b.col < a.col) {
		a, b = b, a
	}
	return a, b
}

// empty reports whether nothing is selected (no selection, or press and release on the same cell).
func (s textSel) empty() bool {
	a, b := s.ordered()
	return !s.set || a == b
}

// span returns the selected cell range [from,to) on content line i, given the line's cell width w.
// ok is false when line i is outside the selection.
func (s textSel) span(i, w int) (from, to int, ok bool) {
	if s.empty() {
		return 0, 0, false
	}
	a, b := s.ordered()
	if i < a.line || i > b.line {
		return 0, 0, false
	}
	from, to = 0, w
	if i == a.line {
		from = min(a.col, w)
	}
	if i == b.line {
		to = min(b.col, w)
	}
	return from, to, from < to
}

// text returns the plain text under the selection; lines are the transcript lines it indexes.
func (s textSel) text(lines []string) string {
	if s.empty() {
		return ""
	}
	a, b := s.ordered()
	var out []string
	for i := a.line; i <= b.line && i < len(lines); i++ {
		from, to, ok := s.span(i, ansi.StringWidth(lines[i]))
		if !ok {
			out = append(out, "")
			continue
		}
		out = append(out, ansi.Strip(ansi.Cut(lines[i], from, to)))
	}
	return strings.Join(out, "\n")
}

// highlightLine draws cells [from,to) of line in reverse video; ANSI styling outside the range is kept.
func highlightLine(line string, from, to int) string {
	w := ansi.StringWidth(line)
	if from >= to || from >= w {
		return line
	}
	to = min(to, w)
	return ansi.Cut(line, 0, from) + "\x1b[7m" + ansi.Strip(ansi.Cut(line, from, to)) + "\x1b[27m" + ansi.Cut(line, to, w)
}

// highlightView applies the selection to the viewport frame. screen row r shows content line yOffset+r,
// so the highlight is recomputed from content coordinates on every frame.
func highlightView(frame string, lines []string, yOffset int, s textSel) string {
	if s.empty() {
		return frame
	}
	rows := strings.Split(frame, "\n")
	for r := range rows {
		i := yOffset + r
		if i < 0 || i >= len(lines) {
			continue
		}
		if from, to, ok := s.span(i, ansi.StringWidth(lines[i])); ok {
			rows[r] = highlightLine(rows[r], from, to)
		}
	}
	return strings.Join(rows, "\n")
}

// copyNoteMsg ends the "복사됨" status note; seq ignores a timer for a note that was replaced since.
type copyNoteMsg struct{ seq int }

// copyToClipboard puts text on the system clipboard (win32 API / pbcopy / wl-copy, xclip, xsel). It also
// writes OSC 52 so a remote session (SSH) reaches the local terminal's clipboard, even when the native copy
// fails. The returned error is the native copy's error.
func copyToClipboard(text string) error {
	err := clipboard.WriteAll(text)
	fmt.Fprint(os.Stdout, ansi.SetClipboard('c', text))
	return err
}

// selectMouse handles left-button press/drag/release inside the transcript viewport. It returns false for
// events it does not use (wheel, overlays, clicks outside the viewport) so they reach the normal handlers.
// Release copies the selection to the clipboard; the highlight stays until the next press.
func (m *tuiModel) selectMouse(v tea.MouseMsg) (bool, tea.Cmd) {
	if v.Button != tea.MouseButtonLeft || m.av != nil || m.sv != nil || m.perm != nil || m.usage != "" {
		return false, nil
	}
	if v.Action == tea.MouseActionPress {
		if v.Y < 0 || v.Y >= m.vp.Height {
			return false, nil
		}
		p := textPos{m.vp.YOffset + v.Y, v.X}
		m.selc = textSel{set: true, anchor: p, head: p}
		m.dragging = true
		return true, nil
	}
	if !m.dragging {
		return false, nil
	}
	m.selc.head = textPos{m.vp.YOffset + max(0, min(v.Y, m.vp.Height-1)), max(0, v.X)}
	if v.Action == tea.MouseActionRelease {
		m.dragging = false
		return true, m.copySelected()
	}
	return true, nil
}

// copySelected copies the current selection and shows "복사됨 (N자)" (or the error) for 2s. A plain click
// selects nothing and copies nothing.
func (m *tuiModel) copySelected() tea.Cmd {
	text := m.selc.text(m.lines)
	if text == "" {
		return nil
	}
	clip := m.clip
	if clip == nil {
		clip = copyToClipboard
	}
	m.copySeq++
	if err := clip(text); err != nil {
		m.copyNote = "복사 실패: " + err.Error()
	} else {
		m.copyNote = fmt.Sprintf("복사됨 (%d자)", utf8.RuneCountInString(text))
	}
	seq := m.copySeq
	return tea.Tick(2*time.Second, func(time.Time) tea.Msg { return copyNoteMsg{seq} })
}
