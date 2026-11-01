package harness

import (
	"context"
	"encoding/json"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"time"

	"chaude/internal/osenv"
)

// usagePollEvery throttles the live /usage poll (one claude process per poll).
const usagePollEvery = 5 * time.Minute

// usageRawTimeout bounds one /usage claude process.
const usageRawTimeout = 60 * time.Second

var (
	usageRe     = regexp.MustCompile(`Current session:\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`)
	usageWeekRe = regexp.MustCompile(`Current week \(all models\):\s*([0-9.]+)%\s*used(?:\s*·\s*resets\s*([^\n]+))?`)
)

// parseUsage extracts percentages from the /usage text; ok requires the session line.
func parseUsage(text string) (map[string]any, bool) {
	m := usageRe.FindStringSubmatch(text)
	if m == nil {
		return nil, false
	}
	s, err := strconv.ParseFloat(m[1], 64)
	if err != nil {
		return nil, false
	}
	rec := map[string]any{"session_pct": s}
	if r := strings.TrimSpace(m[2]); r != "" {
		rec["session_resets"] = r
	}
	if w := usageWeekRe.FindStringSubmatch(text); w != nil {
		if wp, err := strconv.ParseFloat(w[1], 64); err == nil {
			rec["week_pct"] = wp
			if r := strings.TrimSpace(w[2]); r != "" {
				rec["week_resets"] = r
			}
		}
	}
	return rec, true
}

// usageRaw runs /usage in a one-shot claude process (same exec as usageFetch; usageFetch is unchanged).
var usageRaw = func() (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), usageRawTimeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, osenv.ResolveBin("claude"), "-p", "/usage", "--output-format", "json", "--no-session-persistence")
	cmd.Env = cleanEnv(0)
	cmd.Dir = os.TempDir()
	out, err := cmd.Output()
	if err != nil {
		return "", err
	}
	var j map[string]any
	if err := json.Unmarshal(out, &j); err != nil {
		return "", err
	}
	return str(j["result"]), nil
}

// pollUsage starts at most one background /usage poll per usagePollEvery; never blocks the turn loop.
func (r *Repl) pollUsage() {
	if os.Getenv("HARNESS_USAGE_POLL") == "0" || r.metrics == nil {
		return
	}
	r.usageMu.Lock()
	now := time.Now()
	if r.usagePolling || now.Sub(r.usagePollAt) < usagePollEvery {
		r.usageMu.Unlock()
		return
	}
	r.usagePollAt, r.usagePolling = now, true
	r.usageMu.Unlock()
	go func() {
		defer func() {
			r.usageMu.Lock()
			r.usagePolling = false
			r.usageMu.Unlock()
		}()
		r.pollUsageSync()
	}()
}

// pollUsageSync performs one poll and logs a stage=usage row (no throttle, no env check).
func (r *Repl) pollUsageSync() {
	text, err := usageRaw()
	if err != nil {
		_ = r.metrics.Log(map[string]any{"stage": "usage", "error": err.Error()})
		return
	}
	rec, ok := parseUsage(text)
	if !ok {
		head := []rune(text)
		if len(head) > 120 {
			head = head[:120]
		}
		_ = r.metrics.Log(map[string]any{"stage": "usage", "error": "parse", "head": string(head)})
		return
	}
	rec["stage"], rec["source"] = "usage", "cli"
	_ = r.metrics.Log(rec)
}

// turnCostUSD: this turn's cost from the cumulative total_cost_usd of its session.
// A new session, or a cost that went down, starts again from 0. ok=false when cost is not a number.
func (r *Repl) turnCostUSD(sess string, cost any) (float64, bool) {
	c, ok := cost.(float64)
	if !ok {
		return 0, false
	}
	r.costMu.Lock()
	defer r.costMu.Unlock()
	prev := r.lastCost
	if sess != r.lastCostSess || c < prev {
		prev = 0
	}
	r.lastCostSess, r.lastCost = sess, c
	return c - prev, true
}
