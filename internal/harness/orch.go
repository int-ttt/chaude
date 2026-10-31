// Package harness: orchestration modes for the A/B test (--orch m0|b|c|d|e|f).
package harness

import (
	"fmt"
	"os"
	"strings"
)

// parseOrch normalizes an --orch value; "" means the baseline m0.
func parseOrch(s string) (string, error) {
	mode := strings.ToLower(strings.TrimSpace(s))
	switch mode {
	case "":
		return "m0", nil
	case "m0", "b", "c", "d", "e", "f":
		return mode, nil
	}
	return "", fmt.Errorf("unknown orch mode %q (m0|b|c|d|e|f)", s)
}

// orchDefaultModel returns the main model a mode expects ("" = keep the --model default).
func orchDefaultModel(mode string) string {
	switch mode {
	case "b", "c", "d", "f":
		return "opus"
	case "e":
		return "haiku"
	}
	return ""
}

// applyOrchEnv exports the mode for child processes and hooks.
func applyOrchEnv(mode string) {
	_ = os.Setenv("CHAUDE_ORCH", mode)
}

const orchRuleB = "[orch B] Opus main decides inline. ALL file reads/writes go through haiku-scout/haiku-coder/haiku-batch-worker. sonnet-analyst only for judgment over >=3 sources or >~8k tokens of material; no sonnet relay hop. One haiku-scout call per decision (batch all reads), RETURN cap 4000 chars. Merge would-be short spawns (<=3 msgs) into one brief. Reuse the same haiku-coder instance per workstream."

const orchRuleC = orchRuleB + " [orch C] haiku-scout returns a structured digest (key: value, <=4000 chars); decide from the digest. sonnet-analyst only on escalation or after Haiku fails twice."

const orchRuleD = "[orch D] Write one task brief, then issue ALL Haiku calls (scout reads, coder edits on disjoint files, checker) in ONE message in parallel. No intermediate agent, no sequential ping-pong. Coder briefs must name disjoint file sets; sum of RETURN caps <= 8000 chars per round; max 2 rounds per task; second round only on FAIL."

const orchRuleE = "[orch E] Haiku main does all work itself; no Sonnet/Opus agents except escalation after 2 Haiku failures (put ESCALATE in that brief). Spawning a reader is allowed only if the brief names absolute file paths and declares 'RETURN: <=N chars' with N <= 0.25 x total bytes of those files; otherwise the hook blocks it and you do the work inline with your own tools."

const orchRuleF = "[orch F] Turns prefixed [ws:<id>] belong to one workstream; the harness routes them to the same live Haiku child, rotating at 40k tokens, 20 turns or idle >270s with a <=1500-char handoff. Keep one workstream id per task."

// orchRule returns the system-prompt rule for a mode ("" for m0 or unknown).
func orchRule(mode string) string {
	switch mode {
	case "b":
		return orchRuleB
	case "c":
		return orchRuleC
	case "d":
		return orchRuleD
	case "e":
		return orchRuleE
	case "f":
		return orchRuleF
	}
	return ""
}
