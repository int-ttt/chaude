package harness

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
)

// MetricsMain implements `chaude metrics [--dir D] [--csv]`: summarize or dump per-turn JSONL.
func MetricsMain(args []string) int {
	fs := flag.NewFlagSet("metrics", flag.ContinueOnError)
	dir := fs.String("dir", filepath.Join("notes", "metrics"), "metrics directory")
	csv := fs.Bool("csv", false, "print turn rows as CSV instead of a summary")
	if err := fs.Parse(args); err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	files, _ := filepath.Glob(filepath.Join(*dir, "*.jsonl"))
	sort.Strings(files)
	var turns, rots, imps int
	var in, out, cr, cc int
	var cost, dur float64
	var deleg, valid, invalid int
	if *csv {
		fmt.Println("ts,stage,session,model,ctx,input,output,cache_read,cache_write,cost_usd,duration_ms,rotations,delegations,handoff_valid")
	}
	for _, p := range files {
		fh, err := os.Open(p)
		if err != nil {
			continue
		}
		br := bufio.NewReaderSize(fh, 1<<20)
		for {
			line, rerr := br.ReadString('\n')
			var d map[string]any
			if json.Unmarshal([]byte(line), &d) == nil {
				switch str(d["stage"]) {
				case "turn":
					turns++
					in += tok(d, "input_tokens")
					out += tok(d, "output_tokens")
					cr += tok(d, "cache_read_input_tokens")
					cc += tok(d, "cache_creation_input_tokens")
					if c, ok := d["cost_usd"].(float64); ok {
						cost += c
					}
					ms := num(d["duration_ms"])
					if ms == 0 {
						ms = num(d["end_ms"]) - num(d["start_ms"])
					}
					dur += float64(ms)
					deleg += num(d["delegations"])
					if *csv {
						fmt.Printf("%s,turn,%s,%s,%d,%d,%d,%d,%d,%s,%d,%d,%d,\n", str(d["ts"]), str(d["session"]), str(d["model"]),
							num(d["ctx"]), tok(d, "input_tokens"), tok(d, "output_tokens"),
							tok(d, "cache_read_input_tokens"), tok(d, "cache_creation_input_tokens"),
							costStr(d["cost_usd"]), ms, num(d["rotations"]), num(d["delegations"]))
					}
				case "rotate":
					rots++
					if v, _ := d["handoff_valid"].(bool); v {
						valid++
					} else {
						invalid++
					}
					if *csv {
						fmt.Printf("%s,rotate,%s,,%d,,,,,,,,,%v\n", str(d["ts"]), str(d["old_session"]), num(d["rotate_at_tokens"]), d["handoff_valid"])
					}
				case "import":
					imps++
				}
			}
			if rerr != nil {
				break
			}
		}
		fh.Close()
	}
	if *csv {
		return 0
	}
	fmt.Printf("metrics: %d file(s) in %s\nturns=%d rotations=%d (handoff valid=%d invalid=%d) imports=%d delegations=%d\n",
		len(files), *dir, turns, rots, valid, invalid, imps, deleg)
	fmt.Printf("tokens: input=%d output=%d cache_read=%d cache_write=%d\ncost_usd=%.4f total_duration_s=%.1f\n", in, out, cr, cc, cost, dur/1000)
	return 0
}

// tok reads a per-turn token field, preferring the turn total ("result_" prefix) over the last-call value of older logs.
func tok(d map[string]any, k string) int {
	if v, ok := d["result_"+k]; ok {
		return num(v)
	}
	return num(d[k])
}

func costStr(v any) string {
	if f, ok := v.(float64); ok {
		return fmt.Sprintf("%.6f", f)
	}
	return ""
}
