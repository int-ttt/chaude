package scripts

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

func init() { register("usage-report", usageReport) }

type price [4]float64

var priceFams = []string{"opus", "sonnet", "haiku"}
var prices = map[string]price{
	"opus":   {4.0, 8.0, 0.20, 20.0},
	"sonnet": {2.0, 4.0, 0.20, 10.0},
	"haiku":  {0.10, 0.20, 0.01, 0.50},
}

type agg struct{ req, in, cw, cr, out, peak int64 }

// ordAgg keeps first-seen order of model families (python dict order).
type ordAgg struct {
	order []string
	m     map[string]*agg
}

func newOrd() *ordAgg { return &ordAgg{m: map[string]*agg{}} }
func (o *ordAgg) get(f string) *agg {
	if a, ok := o.m[f]; ok {
		return a
	}
	a := &agg{}
	o.m[f] = a
	o.order = append(o.order, f)
	return a
}

func family(model string) string {
	m := strings.ToLower(model)
	for _, f := range priceFams {
		if strings.Contains(m, f) {
			return f
		}
	}
	return "opus"
}

func num(v any) int64 {
	if f, ok := v.(float64); ok {
		return int64(f)
	}
	return 0
}

func scanUsage(path string, ag *ordAgg, seen map[string]bool) {
	_ = eachFileLine(path, func(line string) {
		e := parseLine(line)
		if e == nil {
			return
		}
		msg := asMap(e["message"])
		u := asMap(msg["usage"])
		if strOf(e["type"]) != "assistant" || !truthy(u) {
			return
		}
		mid := strOf(msg["id"])
		if mid == "" {
			mid = strOf(e["uuid"])
		}
		if seen[mid] {
			return
		}
		seen[mid] = true
		a := ag.get(family(strOf(msg["model"])))
		inp, cw, cr, out := num(u["input_tokens"]), num(u["cache_creation_input_tokens"]), num(u["cache_read_input_tokens"]), num(u["output_tokens"])
		a.req++
		a.in += inp
		a.cw += cw
		a.cr += cr
		a.out += out
		if s := inp + cw + cr; s > a.peak {
			a.peak = s
		}
	})
}

func cost(f string, a *agg) float64 {
	p := prices[f]
	s := float64(float64(a.in)*p[0]) + float64(float64(a.cw)*p[1])
	s = s + float64(float64(a.cr)*p[2])
	s = s + float64(float64(a.out)*p[3])
	return s / 1e6
}

type row struct {
	total     float64
	proj, sid string
	ag        *ordAgg
}

func padR(s string, n int) string {
	if k := runeLen(s); k < n {
		return s + strings.Repeat(" ", n-k)
	}
	return s
}

func usageReport(args []string) int {
	fs := newFS("usage-report")
	top := fs.Int("top", 15, "")
	pos, rc := parseArgs(fs, args)
	if rc >= 0 {
		return rc
	}
	root := expandUser("~/.claude/projects")
	if len(pos) > 1 {
		fmt.Fprintln(os.Stderr, "usage: usage-report [PROJECTS_DIR] [--top N]")
		return 2
	}
	if len(pos) == 1 {
		root = pos[0]
	}
	var rows []row
	dirs, _ := os.ReadDir(root)
	for _, d := range dirs {
		if strings.HasPrefix(d.Name(), ".") {
			continue
		}
		pdir := filepath.Join(root, d.Name())
		for _, mainF := range globDir(pdir, ".jsonl") {
			sid := strings.TrimSuffix(filepath.Base(mainF), filepath.Ext(mainF))
			ag := newOrd()
			seen := map[string]bool{}
			scanUsage(mainF, ag, seen)
			for _, sub := range globDir(filepath.Join(pdir, sid, "subagents"), ".jsonl") {
				scanUsage(sub, ag, seen)
			}
			if len(ag.order) == 0 {
				continue
			}
			t := 0.0
			for _, f := range ag.order {
				t += cost(f, ag.m[f])
			}
			rows = append(rows, row{t, d.Name(), sid, ag})
		}
	}
	sort.SliceStable(rows, func(i, j int) bool {
		a, b := rows[i], rows[j]
		if a.total != b.total {
			return a.total > b.total
		}
		if a.proj != b.proj {
			return a.proj > b.proj
		}
		return a.sid > b.sid
	})
	grand := newOrd()
	for _, r := range rows {
		for _, f := range r.ag.order {
			a, g := r.ag.m[f], grand.get(f)
			g.req += a.req
			g.in += a.in
			g.cw += a.cw
			g.cr += a.cr
			g.out += a.out
			if a.peak > g.peak {
				g.peak = a.peak
			}
		}
	}
	M := func(n int64) string { return fmt.Sprintf("%.1fM", float64(n)/1e6) }
	fmt.Println("== 전체 (모델별) ==")
	for _, f := range grand.order {
		a := grand.m[f]
		c := cost(f, a)
		p := prices[f]
		share := ""
		if c != 0 {
			vals := []struct {
				k string
				v float64
			}{
				{"읽기(캐시)", float64(a.cr) * p[2]}, {"저장(캐시)", float64(a.cw) * p[1]},
				{"입력", float64(a.in) * p[0]}, {"출력·생각", float64(a.out) * p[3]},
			}
			var ps []string
			for _, x := range vals {
				ps = append(ps, fmt.Sprintf("%s %.0f%%", x.k, x.v/1e6/c*100))
			}
			share = strings.Join(ps, ", ")
		}
		fmt.Printf("%s 요청 %6d  캐시읽기 %s  캐시저장 %s  출력 %s  최대맥락 %dk  ≈ $%s  (%s)\n",
			padR(f, 7), a.req, M(a.cr), M(a.cw), M(a.out), a.peak/1000, commaStr(fmt.Sprintf("%.2f", c)), share)
	}
	fmt.Printf("\n== 비용 상위 세션 %d개 ==\n", *top)
	n := *top
	if n < 0 {
		n = len(rows) + n
		if n < 0 {
			n = 0
		}
	}
	if n > len(rows) {
		n = len(rows)
	}
	for _, r := range rows[:n] {
		var req, cr, peak int64
		for _, f := range r.ag.order {
			a := r.ag.m[f]
			req += a.req
			cr += a.cr
			if a.peak > peak {
				peak = a.peak
			}
		}
		den := req
		if den < 1 {
			den = 1
		}
		fmt.Printf("$%8.2f  요청 %5d  최대맥락 %4dk  요청당 평균읽기 %5.0fk  %s / %s\n",
			r.total, req, peak/1000, float64(cr)/float64(den)/1000, lastRunes(r.proj, 40), runeSlice(r.sid, 0, 8))
	}
	return 0
}

func lastRunes(s string, n int) string {
	r := []rune(s)
	if len(r) > n {
		r = r[len(r)-n:]
	}
	return string(r)
}
