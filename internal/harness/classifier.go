package harness

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"regexp"
	"strings"
	"time"

	"chaude/internal/osenv"
)

const (
	conductorModel = "sonnet" // classifier model
	classifyPrompt = "You are a router, not an answerer. Never answer or explain. Output exactly two lines:\n" +
		"model: haiku|sonnet|opus\ntask: <the user's request, terse, machine-style: no honorifics, greetings or filler; keep paths, names, numbers; end with 'reply: Korean 존댓말, one final message only after the work is done, no interim status message, no 추천 block; if you delegate to an agent, run it in the foreground and wait for its result before replying'>\n" +
		"haiku = mechanical (exact edits, conversion, search, count, lookup, run/monitor). sonnet = judgment work (multi-file edits needing decisions, diagnosis, synthesis, explanation, long documents). " +
		"opus = ANY of: choosing between options or weighing trade-offs, a judgment call, pushback or disagreement with a previous answer ('아니 그게 아니라', '그건 틀린데'), a correction of earlier work, strategy/priority/roadmap questions, design/architecture, 'opus로', repeated failure. " +
		"If the prompt contains any such element, choose opus even when it also mentions files or code. " +
		"fable = ONLY when the prompt itself asks for it ('fable로', '페이블로', 'fable') or says opus already failed on this item; it is the most expensive model, never choose it otherwise."
)

// routeModels are the session models the classifier may pick.
var routeModels = map[string]bool{"haiku": true, "sonnet": true, "opus": true, "fable": true}

// parseRoute extracts the model and task from the classifier output; ok=false when unusable.
func parseRoute(out string) (model, task string, ok bool) {
	for _, l := range strings.Split(out, "\n") {
		l = strings.TrimSpace(l)
		switch {
		case strings.HasPrefix(strings.ToLower(l), "model:"):
			model = strings.ToLower(strings.TrimSpace(l[6:]))
		case strings.HasPrefix(strings.ToLower(l), "task:"):
			task = strings.TrimSpace(l[5:])
		case task != "" && l != "":
			task += "\n" + l
		}
	}
	if !routeModels[model] || task == "" {
		return "", "", false
	}
	return model, task, true
}

// classify makes one short Sonnet call; returns model, terse task, cost and token usage.
func classify(line, cur, lastUser string) (model, task string, cost float64, usage map[string]any, err error) {
	in := "current session model: " + cur + "\nprevious user prompt: " + trunc(lastUser, 200) + "\nuser prompt:\n" + line
	cmd := exec.Command(osenv.ResolveBin("claude"), "-p", in, "--model", conductorModel, "--output-format", "json",
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
	m, t, ok := parseRoute(str(j["result"]))
	if !ok {
		return "", "", cost, usage, os.ErrInvalid
	}
	return m, t, cost, usage, nil
}

// forcedRe matches an explicit model request in the prompt: `haiku로`, `sonnet으로`, `opus로`, `fable로`
// (Korean names too), as a word followed by space, punctuation or the end.
var forcedRe = regexp.MustCompile(`(?i)(haiku|하이쿠|sonnet|소넷|opus|오퍼스|fable|페이블)(?:으로|로)(?:[\s,.!?~:;)]|$)`)

var koreanModels = map[string]string{"하이쿠": "haiku", "소넷": "sonnet", "오퍼스": "opus", "페이블": "fable"}

// forcedModel returns the model the user named in the prompt, or "" (the last mention wins).
func forcedModel(prompt string) string {
	ms := forcedRe.FindAllStringSubmatch(prompt, -1)
	if len(ms) == 0 {
		return ""
	}
	m := strings.ToLower(ms[len(ms)-1][1])
	if k, ok := koreanModels[m]; ok {
		return k
	}
	return m
}

// route classifies a prompt and switches the session model if needed; returns the text to send
// (the terse task, or the original line for acks / forced models / classifier failure).
// A prompt that names its model (`opus로`, `fable로`) is routed directly: no classifier call, no latency.
func (r *Repl) route(line string) string {
	if isAck(line) || strings.HasPrefix(line, "/") {
		return line
	}
	t0 := time.Now()
	last := ""
	if n := len(r.tr.turns); n > 0 {
		last = r.tr.turns[n-1][0]
	}
	var (
		model, task string
		cost        float64
		usage       map[string]any
		err         error
	)
	forced := forcedModel(line)
	if forced != "" {
		model, task = forced, line
	} else {
		model, task, cost, usage, err = classify(line, r.a.Model, last)
	}
	f := map[string]any{"stage": "classify", "cost_usd": cost, "duration_ms": time.Since(t0).Milliseconds(), "from_model": r.a.Model, "conductor": true, "forced": forced != ""}
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
