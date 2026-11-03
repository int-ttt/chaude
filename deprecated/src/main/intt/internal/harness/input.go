package harness

import (
	"strings"
	"unicode"

	"github.com/charmbracelet/lipgloss"
	"github.com/mattn/go-runewidth"
)

// inputBox is a small multi-line editor that measures every rune by terminal cells
// (Hangul/CJK = 2, ASCII = 1, combining = 0) so wrapping, cursor and scrolling stay exact.
type inputBox struct {
	val   []rune
	pos   int // rune index of the cursor
	width int // max cells per visual row (cursor slack already excluded)
	top   int // first visible visual row
}

type vrow struct{ start, end int } // val[start:end], newline excluded

func rw(r rune) int { return runewidth.RuneWidth(r) }

func cells(rs []rune) int {
	n := 0
	for _, r := range rs {
		n += rw(r)
	}
	return n
}

func (b *inputBox) Value() string { return string(b.val) }

func (b *inputBox) SetValue(s string) {
	b.val = []rune(strings.ReplaceAll(strings.ReplaceAll(s, "\r\n", "\n"), "\r", "\n"))
	b.pos = len(b.val)
	b.top = 0
}

func (b *inputBox) Insert(rs []rune) {
	var in []rune
	for i := 0; i < len(rs); i++ {
		r := rs[i]
		if r == '\r' {
			if i+1 < len(rs) && rs[i+1] == '\n' {
				continue
			}
			r = '\n'
		}
		if r == '\n' || r == '\t' || r >= 32 && r != 127 {
			if r == '\t' {
				r = ' '
			}
			in = append(in, r)
		}
	}
	nv := make([]rune, 0, len(b.val)+len(in))
	nv = append(nv, b.val[:b.pos]...)
	nv = append(nv, in...)
	nv = append(nv, b.val[b.pos:]...)
	b.val, b.pos = nv, b.pos+len(in)
}

func (b *inputBox) Backspace() {
	if b.pos > 0 {
		b.val = append(b.val[:b.pos-1], b.val[b.pos:]...)
		b.pos--
	}
}

func (b *inputBox) Delete() {
	if b.pos < len(b.val) {
		b.val = append(b.val[:b.pos], b.val[b.pos+1:]...)
	}
}

// isWordRune: letters (Hangul included), digits and '_' form a word; other punctuation/space separates.
func isWordRune(r rune) bool { return unicode.IsLetter(r) || unicode.IsDigit(r) || r == '_' }

// DeleteWord (opt+delete) deletes the previous word; a newline before the cursor is deleted alone.
func (b *inputBox) DeleteWord() {
	if b.pos == 0 {
		return
	}
	if b.val[b.pos-1] == '\n' {
		b.Backspace()
		return
	}
	i := b.pos
	for i > 0 && b.val[i-1] != '\n' && !isWordRune(b.val[i-1]) {
		i--
	}
	for i > 0 && b.val[i-1] != '\n' && isWordRune(b.val[i-1]) {
		i--
	}
	b.val = append(b.val[:i], b.val[b.pos:]...)
	b.pos = i
}

// KillToStart (cmd+delete) deletes from the cursor to the start of the logical line.
func (b *inputBox) KillToStart() {
	if b.pos == 0 {
		return
	}
	if b.val[b.pos-1] == '\n' {
		b.Backspace()
		return
	}
	ls := b.pos
	for ls > 0 && b.val[ls-1] != '\n' {
		ls--
	}
	b.val = append(b.val[:ls], b.val[b.pos:]...)
	b.pos = ls
}

func (b *inputBox) Left() {
	if b.pos > 0 {
		b.pos--
	}
}
func (b *inputBox) Right() {
	if b.pos < len(b.val) {
		b.pos++
	}
}

// Home/End move within the logical line (between newlines).
func (b *inputBox) Home() {
	for b.pos > 0 && b.val[b.pos-1] != '\n' {
		b.pos--
	}
}
func (b *inputBox) End() {
	for b.pos < len(b.val) && b.val[b.pos] != '\n' {
		b.pos++
	}
}

// KillToEnd deletes from the cursor to the end of the logical line.
func (b *inputBox) KillToEnd() {
	e := b.pos
	for e < len(b.val) && b.val[e] != '\n' {
		e++
	}
	b.val = append(b.val[:b.pos], b.val[e:]...)
}

func (b *inputBox) rows() []vrow {
	w := b.width
	if w < 2 {
		w = 2
	}
	var rs []vrow
	start, cur := 0, 0
	for i, r := range b.val {
		if r == '\n' {
			rs = append(rs, vrow{start, i})
			start, cur = i+1, 0
			continue
		}
		c := rw(r)
		if cur+c > w && i > start {
			rs = append(rs, vrow{start, i})
			start, cur = i, 0
		}
		cur += c
	}
	return append(rs, vrow{start, len(b.val)})
}

// Cursor returns the visual row and the cell column of the cursor.
func (b *inputBox) Cursor() (row, col int) {
	rs := b.rows()
	for i, r := range rs {
		if r.start <= b.pos {
			row = i
		}
	}
	return row, cells(b.val[rs[row].start:b.pos])
}

// MoveRow moves the cursor one visual row up (-1) or down (+1), keeping the cell column.
// It reports false when there is no such row.
func (b *inputBox) MoveRow(d int) bool {
	rs := b.rows()
	row, col := b.Cursor()
	t := row + d
	if t < 0 || t >= len(rs) {
		return false
	}
	p, c := rs[t].start, 0
	for p < rs[t].end && c+rw(b.val[p]) <= col {
		c += rw(b.val[p])
		p++
	}
	b.pos = p
	return true
}

// Height is the number of visible rows (1..max).
func (b *inputBox) Height(max int) int {
	n := len(b.rows())
	if n > max {
		n = max
	}
	return n
}

// View renders up to max rows with a "> " prompt, a reverse-video cursor and a placeholder.
func (b *inputBox) View(max int, placeholder string, ph lipgloss.Style) string {
	if len(b.val) == 0 {
		cur := lipgloss.NewStyle().Reverse(true).Render(" ")
		return "> " + cur + ph.Render(placeholder)
	}
	rs := b.rows()
	crow, _ := b.Cursor()
	h := len(rs)
	if h > max {
		h = max
	}
	if crow < b.top {
		b.top = crow
	}
	if crow >= b.top+h {
		b.top = crow - h + 1
	}
	if b.top > len(rs)-h {
		b.top = len(rs) - h
	}
	rev := lipgloss.NewStyle().Reverse(true)
	var out []string
	for i := b.top; i < b.top+h; i++ {
		r := rs[i]
		pre := "  "
		if i == 0 {
			pre = "> "
		}
		line := string(b.val[r.start:r.end])
		if i == crow {
			at := b.pos - r.start
			rest, cur := "", rev.Render(" ")
			if b.pos < r.end {
				cur, rest = rev.Render(string(b.val[b.pos])), string(b.val[b.pos+1:r.end])
			}
			line = string(b.val[r.start:r.start+at]) + cur + rest
		}
		out = append(out, pre+line)
	}
	return strings.Join(out, "\n")
}
