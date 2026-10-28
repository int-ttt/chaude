package harness

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/mattn/go-runewidth"
)

// ctxBarMax is the context size the /usage ctx bar is measured against.
const ctxBarMax = 270_000

const barCells = 10

// renderBar draws "[████░░░░░░] 45%"; fill is clamped to the bar, the label keeps the real percent.
func renderBar(pct float64) string {
	if pct < 0 {
		pct = 0
	}
	n := int(pct/100*barCells + 0.5)
	if n > barCells {
		n = barCells
	}
	bar := strings.Repeat("█", n) + strings.Repeat("░", barCells-n)
	s := "[" + bar + "] " + strconv.Itoa(int(pct+0.5)) + "%"
	if pct > 100 {
		s += " !"
	}
	return s
}

// barWidth is the display width of a rendered bar (go-runewidth, so ambiguous-width glyphs are counted consistently).
func barWidth(pct float64) int { return runewidth.StringWidth(renderBar(pct)) }

var pctRe = regexp.MustCompile(`(\d+(?:\.\d+)?)%`)

// decorateUsage keeps the original /usage text and appends a bar after each line's percentages,
// then adds a current-context bar against ctxBarMax.
func decorateUsage(text string, ctx int) string {
	lines := strings.Split(text, "\n")
	for i, l := range lines {
		m := pctRe.FindStringSubmatch(l)
		if m == nil {
			continue
		}
		v, _ := strconv.ParseFloat(m[1], 64)
		lines[i] = l + "  " + renderBar(v)
	}
	out := strings.Join(lines, "\n")
	pct := float64(ctx) / ctxBarMax * 100
	return out + fmt.Sprintf("\n\nContext (now): %s  (%s / %s)", renderBar(pct), commas(ctx), commas(ctxBarMax))
}
