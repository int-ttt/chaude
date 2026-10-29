package harness

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"strings"
	"time"
)

const (
	conductorModel = "sonnet" // classifier model
	classifyPrompt = "You are a router, not an answerer. Never answer or explain. Output exactly three lines:\n" +
		"topic: same|new\nsize: small|large\ntask: <the user's request, terse, machine-style: no honorifics, greetings or filler; keep paths, names, numbers; end with 'reply: Korean 존댓말, one final message only after the work is done, no interim status message, no 추천 block; if you delegate to an agent, run it in the foreground and wait for its result before replying'>\n" +
		"topic: new when the prompt is unrelated to the current work (a different workstream, or a side task such as a simple test/lookup even if the subject is not new); follow-ups, corrections, pushback, questions about the same files or subject are same. When unsure, same. " +
		"size: small = single-file or mechanical edits, lookup, search, count, run/monitor, short explanation, small fix. " +
		"large = multi-file or multi-step changes, design/architecture, hard diagnosis, weighing trade-offs, judgment calls, strategy/priority/roadmap, 'opus로', repeated failure. " +
		"Always output both topic and size."
)

// delegateHint is prepended to the task when an Opus main session gets an unrelated or simple prompt:
// Opus keeps its context (no rotation/handoff) and a cheaper agent does the work.
const delegateHint = "[routing: unrelated or simple side task. Do not do it in the main context; delegate to a Haiku agent (mechanical/lookup/test run) or sonnet-analyst (needs judgment) in the foreground, and reply with its result.]\n"

// pickModel applies the switch policy. Main stays Opus by default. On Opus, a prompt that is
// unrelated to the current work (topic new) and small (simple/test/lookup) is delegated as a
// subagent from the same Opus session (delegate=true) instead of rotating to a Sonnet main session,
// which would cost a handoff and lose the cache. A Sonnet main (forced by the user) stays unless
// a new large task needs Opus. The main session is never haiku.
func pickModel(cur, topic, size string) (model string, delegate bool) {
	if cur == "" || cur == "opus" {
		return "opus", topic == "new" && size == "small"
	}
	if topic == "new" && size == "large" {
		return "opus", false
	}
	return cur, false
}

// parseRoute extracts the target model and task from the classifier output; ok=false when unusable.
func parseRoute(out, cur string) (model, task string, ok bool) {
	var topic, size string
	for _, l := range strings.Split(out, "\n") {
		l = strings.TrimSpace(l)
		lo := strings.ToLower(l)
		switch {
		case strings.HasPrefix(lo, "topic:"):
			topic = strings.TrimSpace(lo[6:])
		case strings.HasPrefix(lo, "size:"):
			size = strings.TrimSpace(lo[5:])
		case strings.HasPrefix(lo, "task:"):
			task = strings.TrimSpace(l[5:])
		case task != "" && l != "":
			task += "\n" + l
		}
	}
	if task == "" || topic != "same" && topic != "new" || size != "small" && size != "large" {
		return "", "", false
	}
	model, del := pickModel(cur, topic, size)
	if del {
		task = delegateHint + task
	}
	return model, task, true
}

// classify makes one short Sonnet call; returns model, terse task, cost and token usage.
func classify(line, cur, lastUser string) (model, task string, cost float64, usage map[string]any, err error) {
	in := "current session model: " + cur + " (keep it unless topic is new)\nprevious user prompt: " + trunc(lastUser, 200) + "\nuser prompt:\n" + line
	cmd := exec.Command("claude", "-p", in, "--model", conductorModel, "--output-format", "json",
		"--system-prompt", classifyPrompt, "--tools", "", "--no-session-persistence")
	cmd.Dir = os.TempDir() // no project CLAUDE.md in the classifier context
	var out bytes.Buffer
	cmd.Stdout = &out
	if err = cmd.Run(); err != nil {
		return
	}
	var j map[string]any
	if err = json.Unmarshal(out.Bytes(), &j); err != nil {
		return
	}
	cost, _ = j["total_cost_usd"].(float64)
	usage = asMap(j["usage"])
	m, t, ok := parseRoute(str(j["result"]), cur)
	if !ok {
		return "", "", cost, usage, os.ErrInvalid
	}
	return m, t, cost, usage, nil
}

// route classifies a prompt and switches the session model if needed; returns the text to send
// (the terse task, or the original line for acks / classifier failure).
func (r *Repl) route(line string) string {
	if isAck(line) || strings.HasPrefix(line, "/") {
		return line
	}
	t0 := time.Now()
	last := ""
	if n := len(r.tr.turns); n > 0 {
		last = r.tr.turns[n-1][0]
	}
	model, task, cost, usage, err := classify(line, r.a.Model, last)
	f := map[string]any{"stage": "classify", "cost_usd": cost, "duration_ms": time.Since(t0).Milliseconds(), "from_model": r.a.Model, "conductor": true}
	for k, v := range usageFields(usage) {
		f["result_"+k] = v
	}
	if err != nil {
		f["error"] = err.Error()
		_ = r.metrics.Log(f)
		return line
	}
	f["routed_model"] = model
	_ = r.metrics.Log(f)
	r.p(dim+"[route → "+model+"]"+rst, "\n")
	if model != r.a.Model {
		r.a.Model = model
		if len(r.tr.turns) == 0 && r.pendingSnapshot == "" {
			r.sess.Close()
			if err := r.start(); err != nil {
				return task
			}
		} else {
			r.rotate("route " + model)
		}
	}
	return task
}
