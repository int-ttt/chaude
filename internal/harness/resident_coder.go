package harness

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"strings"
	"time"
)

// coderRoles are the resident coder slots in status-line order: coder = C0, coder-spare-i = Ci.
var coderRoles = []string{"coder", "coder-spare-1", "coder-spare-2", "coder-spare-3", "coder-spare-4"}

// residentCoderTimeout bounds one resident_coder call; the caller gets an error after it.
var residentCoderTimeout = 30 * time.Minute

// coderSummaryKeep is how many summaries per coder resident_coders_status lists.
const coderSummaryKeep = 3

// coderReplyFormat is appended to every task so the coder ends with the lines the summary keeps.
const coderReplyFormat = "\n\n[reply] End with `status: done|partial|fail` and one `changed: <path>: <what>` line per file you edited."

// coderRule is appended to the main system prompt when the resident coder tools are served.
const coderRule = "[resident coder] For important code or user-document work call the resident_coder MCP tool. " +
	"Default slot is coder (C0); use coder-spare-N only for a clearly different parallel context; same-context follow-ups go to the same coder. " +
	"Check resident_coders_status before routing. Simple lookups and small edits go to Task haiku agents instead. " +
	"The tool returns only the coder's change summary; relay it to the user in Korean."

func isCoderRole(role string) bool {
	for _, rl := range coderRoles {
		if rl == role {
			return true
		}
	}
	return false
}

// runResidentCoder sends task to the coder slot (default C0) and blocks until its reply. A slot that is
// not running (a lazy spare) is spawned by this call. A busy coder is not reused: the caller picks another slot.
func (r *Repl) runResidentCoder(slot, task string) (string, error) {
	role := strings.TrimSpace(slot)
	if role == "" {
		role = coderRoles[0]
	}
	if !isCoderRole(role) {
		return "", fmt.Errorf("unknown coder slot %q (use coder or coder-spare-1..4)", role)
	}
	task = strings.TrimSpace(task)
	if task == "" {
		return "", errors.New("task is required")
	}
	def, ok := r.residentSlotByRole(role)
	if !ok {
		return "", fmt.Errorf("no coder slot %q in the resident preset", role)
	}
	if c, live := r.residentChild(role); live {
		if c.busyRec() != nil {
			return "", fmt.Errorf("resident coder %q is busy; pick another slot", role)
		}
	} else if _, err := r.spawnSlot(def); err != nil {
		return "", err
	}
	ch, err := r.sendResident(role, task+coderReplyFormat, firstLine(task))
	if err != nil {
		return "", err
	}
	select {
	case reply := <-ch:
		sum := summarizeReply(reply)
		r.recordCoderSummary(role, sum)
		return sum, nil
	case <-time.After(residentCoderTimeout):
		return "", fmt.Errorf("resident coder %q timed out", role)
	}
}

// summarizeReply keeps the status and changed lines of a coder reply; a reply without them is cut to 300 runes.
func summarizeReply(reply string) string {
	var keep []string
	for _, ln := range strings.Split(reply, "\n") {
		t := strings.TrimSpace(strings.TrimPrefix(strings.TrimSpace(ln), "- "))
		if strings.HasPrefix(t, "status:") || strings.HasPrefix(t, "changed:") {
			keep = append(keep, t)
		}
	}
	if len(keep) == 0 {
		return trunc(strings.TrimSpace(reply), 300)
	}
	return trunc(strings.Join(keep, "\n"), 1200)
}

// recordCoderSummary keeps the last coderSummaryKeep summaries of role.
func (r *Repl) recordCoderSummary(role, sum string) {
	r.coderMu.Lock()
	defer r.coderMu.Unlock()
	if r.coderSums == nil {
		r.coderSums = map[string][]string{}
	}
	l := append(r.coderSums[role], sum)
	if len(l) > coderSummaryKeep {
		l = l[len(l)-coderSummaryKeep:]
	}
	r.coderSums[role] = l
}

// coderStatus lists each coder slot of the preset with its state and last summaries (oldest first).
func (r *Repl) coderStatus() string {
	var b strings.Builder
	for _, role := range coderRoles {
		if _, ok := r.residentSlotByRole(role); !ok {
			continue
		}
		state := "not started"
		if c, live := r.residentChild(role); live {
			state = "idle"
			if bu := c.busyRec(); bu != nil {
				state = "busy: " + bu.Prompt
			}
		}
		fmt.Fprintf(&b, "%s (%s): %s\n", role, residentRoleKey(role), state)
		r.coderMu.Lock()
		sums := r.coderSums[role]
		r.coderMu.Unlock()
		if len(sums) == 0 {
			b.WriteString("  (no summaries)\n")
		}
		for _, s := range sums {
			fmt.Fprintf(&b, "  - %s\n", strings.ReplaceAll(s, "\n", "\n    "))
		}
	}
	return b.String()
}

// coderMayUse: a busy resident coder may use Read/Edit/Write on paths inside cwd and Bash.
// Idle parked children and other roles get no approval here.
// Note: Bash is approved without checking the command's paths.
func coderMayUse(c *liveChild, cwd, tool string, input map[string]any) bool {
	if c.busyRec() == nil || !strings.HasPrefix(c.Role, "coder") {
		return false
	}
	switch tool {
	case "Bash":
		return true
	case "Read", "Edit", "Write":
		p := str(input["file_path"])
		if p == "" || cwd == "" {
			return false
		}
		if !filepath.IsAbs(p) {
			p = filepath.Join(cwd, p)
		}
		rel, err := filepath.Rel(cwd, p)
		return err == nil && rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator))
	}
	return false
}

// mcpReply answers one JSON-RPC line of the chaude MCP server (initialize, tools/list, tools/call for
// resident_coder and resident_coders_status). It returns nil for notifications.
func (r *Repl) mcpReply(line []byte) []byte {
	var req struct {
		ID     json.RawMessage `json:"id"`
		Method string          `json:"method"`
		Params struct {
			Name      string `json:"name"`
			Arguments struct {
				Slot string `json:"slot"`
				Task string `json:"task"`
			} `json:"arguments"`
		} `json:"params"`
	}
	if json.Unmarshal(line, &req) != nil || len(req.ID) == 0 {
		return nil
	}
	resp := map[string]any{"jsonrpc": "2.0", "id": req.ID}
	switch req.Method {
	case "initialize":
		resp["result"] = map[string]any{"protocolVersion": "2024-11-05", "capabilities": map[string]any{"tools": map[string]any{}},
			"serverInfo": map[string]any{"name": "chaude", "version": "0"}}
	case "tools/list":
		resp["result"] = map[string]any{"tools": []any{
			map[string]any{
				"name":        "resident_coder",
				"description": "Run a bounded code or document task on a resident haiku coder and return only its change summary. Blocks until done. Default slot is coder (C0); coder-spare-1..4 for a clearly different parallel context.",
				"inputSchema": map[string]any{"type": "object", "required": []string{"task"},
					"properties": map[string]any{
						"task": map[string]any{"type": "string"},
						"slot": map[string]any{"type": "string", "enum": coderRoles},
					}},
			},
			map[string]any{
				"name":        "resident_coders_status",
				"description": "List each resident coder slot: busy or idle, and its last 3 change summaries. Use it to route same-context follow-ups to the same coder.",
				"inputSchema": map[string]any{"type": "object", "properties": map[string]any{}},
			},
		}}
	case "tools/call":
		var text string
		isErr := false
		switch req.Params.Name {
		case "resident_coder":
			reply, err := r.runResidentCoder(req.Params.Arguments.Slot, req.Params.Arguments.Task)
			text, isErr = reply, err != nil
			if err != nil {
				text = err.Error()
			}
		case "resident_coders_status":
			text = r.coderStatus()
		default:
			resp["error"] = map[string]any{"code": -32602, "message": "unknown tool " + req.Params.Name}
			b, _ := json.Marshal(resp)
			return b
		}
		res := map[string]any{"content": []any{map[string]any{"type": "text", "text": text}}}
		if isErr {
			res["isError"] = true
		}
		resp["result"] = res
	default:
		resp["error"] = map[string]any{"code": -32601, "message": "method not found: " + req.Method}
	}
	b, _ := json.Marshal(resp)
	return b
}
