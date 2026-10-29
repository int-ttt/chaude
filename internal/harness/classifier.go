package harness

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

const (
	conductorModel = "sonnet" // classifier model
	classifyPrompt = "You are a router, not an answerer. Never answer or explain. Output exactly five lines:\n" +
		"topic: same|new\nsize: small|large\nkind: lookup|edit|other\nworker: <id from the candidate list>|new\ntask: <the user's request, terse, machine-style: no honorifics, greetings or filler; keep paths, names, numbers; end with 'reply: Korean 존댓말, one final message only after the work is done, no interim status message, no 추천 block; if you delegate to an agent, run it in the foreground and wait for its result before replying'>\n" +
		"topic: new when the prompt is unrelated to the current work (a different workstream, or a side task such as a simple test/lookup even if the subject is not new); follow-ups, corrections, pushback, questions about the same files or subject are same. When unsure, same. " +
		"size: small = single-file or mechanical edits, lookup, search, count, run/monitor, short explanation, small fix. " +
		"large = multi-file or multi-step changes, design/architecture, hard diagnosis, weighing trade-offs, judgment calls, strategy/priority/roadmap, 'opus로', repeated failure. " +
		"kind: lookup = simple lookup/search/'find what this was'/count/read-only question; edit = simple edit of file content (replace text, rename a string, fix a typo, small mechanical change); other = anything else, and always other for judgment, strategy, trade-offs, corrections, pushback, design. " +
		"A lookup or edit that is small is sent straight to a Haiku session. worker: only for kind edit; pick the candidate whose task or edited files fit this request's context, otherwise new; use new for lookup and other. " +
		"Always output topic, size, kind and worker."
)

// delegateHint is prepended to the task when a forced Opus main session gets an unrelated or simple prompt:
// Opus keeps its context (no rotation/handoff) and a cheaper agent does the work.
const delegateHint = "[routing: unrelated or simple side task. Do not do it in the main context; delegate in the foreground: search/lookup/'find what this was' questions -> haiku-scout (effort high); mechanical/test run -> another Haiku agent; review/check -> sonnet-reviewer; analysis/judgment -> sonnet-analyst, and reply with its result.]\n"

// planHint is prepended to a large task on the Haiku-fixed main session: Haiku orchestrates, Opus/Sonnet only
// review the plan (design), then Haiku executes the improved plan.
const planHint = "[routing: large or judgment task. Haiku main runs this workflow: (1) write the full design/orchestration plan to notes/runs/<YYMMDD>-plan-<topic>.md yourself; " +
	"(2) in ONE foreground round call opus-architect (design review, effort high) and/or sonnet-analyst with that plan path and ask for improvements only, written to notes/runs/<YYMMDD>-plan-<topic>-review.md; " +
	"(3) merge their improvements into the plan; (4) execute the improved plan through Haiku agents (haiku-coder/batch-worker/scout/runner), sonnet-reviewer before finishing. Opus/Sonnet do not execute.]\n"

// pickModel applies the switch policy. The main session is Haiku-fixed: Opus/Sonnet are used only as
// design/review subagents, never as main. A main forced by the user (--model sonnet|opus) stays as is.
// delegate=true only for a forced Opus main with an unrelated small task (same-session subagent delegation).
func pickModel(cur, topic, size string) (model string, delegate bool) {
	switch cur {
	case "", "haiku":
		return "haiku", false
	case "opus":
		return "opus", topic == "new" && size == "small"
	}
	return cur, false
}

// pickTarget: Haiku always, whatever the kind (a forced Sonnet/Opus main is kept).
func pickTarget(cur, topic, size, kind string) (model string, delegate bool) {
	return pickModel(cur, topic, size)
}

// route is a parsed classifier answer.
type routeResult struct {
	Model, Task, Kind, Worker string // Worker: a candidate session id when Model is haiku and kind is edit, else ""
}

// parseRoute extracts the target model and task from the classifier output; ok=false when unusable.
func parseRoute(out, cur string) (model, task string, ok bool) {
	r, ok := parseRouteFull(out, cur, nil)
	return r.Model, r.Task, ok
}

// parseRouteFull also reads kind and worker; worker must be one of cands (else it is dropped = fresh session).
func parseRouteFull(out, cur string, cands []haikuWorker) (rr routeResult, ok bool) {
	var topic, size, kind, worker, task string
	for _, l := range strings.Split(out, "\n") {
		l = strings.TrimSpace(l)
		lo := strings.ToLower(l)
		switch {
		case strings.HasPrefix(lo, "topic:"):
			topic = strings.TrimSpace(lo[6:])
		case strings.HasPrefix(lo, "size:"):
			size = strings.TrimSpace(lo[5:])
		case strings.HasPrefix(lo, "kind:") && task == "":
			kind = strings.TrimSpace(lo[5:])
		case strings.HasPrefix(lo, "worker:") && task == "":
			worker = strings.TrimSpace(l[7:])
		case strings.HasPrefix(lo, "task:"):
			task = strings.TrimSpace(l[5:])
		case task != "" && l != "":
			task += "\n" + l
		}
	}
	if task == "" || topic != "same" && topic != "new" || size != "small" && size != "large" {
		return rr, false
	}
	if kind != "lookup" && kind != "edit" {
		kind = "other"
	}
	model, del := pickTarget(cur, topic, size, kind)
	if del {
		task = delegateHint + task
	} else if model == "haiku" && size == "large" {
		task = planHint + task
	}
	rr = routeResult{Model: model, Task: task, Kind: kind}
	if model == "haiku" && kind == "edit" {
		for _, c := range cands {
			if c.ID == worker {
				rr.Worker = worker
			}
		}
	}
	return rr, true
}

// classify makes one short Sonnet call; returns model, terse task, cost and token usage.
func classify(line, cur, lastUser string, cands []haikuWorker) (rr routeResult, cost float64, usage map[string]any, err error) {
	in := "current session model: " + cur + " (keep it unless topic is new)\nprevious user prompt: " + trunc(lastUser, 200) + "\n"
	if ct := candidateText(cands); ct != "" {
		in += "haiku worker candidates (sessions that already edited files):\n" + ct
	}
	in += "user prompt:\n" + line
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
	rr, ok := parseRouteFull(str(j["result"]), cur, cands)
	if !ok {
		return routeResult{}, cost, usage, os.ErrInvalid
	}
	return rr, cost, usage, nil
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
	rr, cost, usage, err := classify(line, r.a.Model, last, r.pool.candidates())
	model, task := rr.Model, rr.Task
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
	f["kind"] = rr.Kind
	if rr.Worker != "" {
		f["worker"] = rr.Worker
	}
	_ = r.metrics.Log(f)
	r.curTask = task
	if model == "haiku" && (r.a.Model != "haiku" || rr.Worker != "") {
		r.p(dim+"[route → haiku"+map[bool]string{true: " (기존 세션 " + trunc(rr.Worker, 8) + ")", false: ""}[rr.Worker != ""]+"]"+rst, "\n")
		r.switchSession("haiku", rr.Worker)
		return task
	}
	r.p(dim+"[route → "+model+"]"+rst, "\n")
	if model != r.a.Model {
		if pid := r.parked[model]; pid != "" && r.a.Model == "haiku" {
			// back from a Haiku session: resume the parked session instead of a handoff rotation
			delete(r.parked, model)
			r.switchSession(model, pid)
			return task
		}
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

// switchSession replaces the child with a session on model, resuming resume (a session id) when set.
// The session being left is parked (its id is kept, no handoff) so a later switch back can resume it.
func (r *Repl) switchSession(model, resume string) {
	if r.sess != nil && r.a.Model != model {
		if id := r.sess.SessionID; id != "" {
			if r.parked == nil {
				r.parked = map[string]string{}
			}
			r.parked[r.a.Model] = id
		}
	}
	if r.sess != nil {
		r.sess.Close()
	}
	r.a.Model = model
	if resume != "" {
		r.pendingSnapshot = "" // resumed session already has its context
	}
	r.resumeID = resume
	if err := r.start(); err != nil {
		fmt.Fprintln(os.Stderr, "chaude: cannot start claude:", err)
	}
}
