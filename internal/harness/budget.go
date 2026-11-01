package harness

import (
	"strings"
	"sync"
)

// budget is the per-task USD ledger (U9). cap<=0 = off. In memory only.
// total_cost_usd from the claude CLI is cumulative per process, so charge books the delta vs last[s].
type budget struct {
	mu                  sync.Mutex
	cap, spent, running float64
	last                map[*Session]float64
}

func newBudget(cap float64) *budget { return &budget{cap: cap, last: map[*Session]float64{}} }

// reset starts a new task: spent and running to 0; per-process baselines are kept.
func (b *budget) reset() {
	if b == nil {
		return
	}
	b.mu.Lock()
	b.spent, b.running = 0, 0
	b.mu.Unlock()
}

// charge books the cost growth of process s given its cumulative total. A drop (new process or CLI
// counter reset) books the whole total.
func (b *budget) charge(s *Session, total float64) {
	if b == nil || b.cap <= 0 {
		return
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	d := total - b.last[s]
	if d < 0 {
		d = total
	}
	b.spent += d
	b.last[s] = total
	b.running = 0
}

// chargeOneShot books a cost of a separate one-shot process (/usage).
func (b *budget) chargeOneShot(cost float64) {
	if b == nil || b.cap <= 0 || cost <= 0 {
		return
	}
	b.mu.Lock()
	b.spent += cost
	b.mu.Unlock()
}

// allow reports whether a call with estimated cost est may start.
func (b *budget) allow(est float64) bool {
	if b == nil || b.cap <= 0 {
		return true
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	return !(b.spent >= b.cap || b.spent+est > b.cap)
}

// over reports whether spent plus the in-flight estimate reached the cap.
func (b *budget) over(running float64) bool {
	if b == nil || b.cap <= 0 {
		return false
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	b.running = running
	return b.spent+running >= b.cap
}

// Spent returns the booked USD of the current task.
func (b *budget) Spent() float64 {
	if b == nil {
		return 0
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.spent
}

// resumeBaseline is the cost baseline of a process started with --resume. Always 0 (user decision):
// no seeding from the prior session id.
func resumeBaseline() float64 { return 0 }

// modelPrice is USD per million tokens. Source: https://docs.claude.com/en/docs/about-claude/pricing
// (redirects to platform.claude.com), fetched 2026-10-29. Assumed alias map: opus=Opus 5.5,
// sonnet=Sonnet 5.5, haiku=Haiku 5.5 (up to 100k tokens).
type modelPrice struct{ in, write5m, write1h, read, out float64 }

var priceTable = map[string]modelPrice{
	"opus":   {4, 5, 8, 0.20, 20},
	"sonnet": {2, 2.50, 4, 0.10, 10},
	"haiku":  {0.10, 0.125, 0.20, 0.01, 0.50},
}

// priceKey maps a full model id (claude-opus-5-5, claude-sonnet-5-5, ...) to its priceTable alias. Other
// names are returned as-is.
func priceKey(model string) string {
	m := strings.ToLower(model)
	for _, alias := range []string{"opus", "sonnet", "haiku"} {
		if strings.Contains(m, alias) {
			return alias
		}
	}
	return model
}

// estimate is the planned cost of the next call: ctx tokens at the cache-read price (warm) or the
// cache-write price for ttl ("1h" 2x input, else 1.25x), plus 4k output tokens. Unknown model = 0.
func estimate(model string, ctx int, warm bool, ttl string) float64 {
	p, ok := priceTable[priceKey(model)]
	if !ok {
		return 0
	}
	per := p.read
	if !warm {
		per = p.write5m
		if ttl == "1h" {
			per = p.write1h
		}
	}
	return (float64(ctx)*per + 4000*p.out) / 1e6
}

// midTurnEstimate is the in-flight cost guess from one main assistant usage event.
func midTurnEstimate(model string, usage map[string]any) float64 {
	p, ok := priceTable[priceKey(model)]
	if !ok {
		return 0
	}
	num := func(k string) float64 { f, _ := usage[k].(float64); return f }
	return (num("cache_read_input_tokens")*p.read + num("cache_creation_input_tokens")*p.write5m +
		num("input_tokens")*p.in + num("output_tokens")*p.out) / 1e6
}
