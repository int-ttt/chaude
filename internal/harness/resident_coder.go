package harness

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"strings"
	"time"
)

// residentCoderTimeout bounds one resident_coder call; the caller gets an error after it.
var residentCoderTimeout = 30 * time.Minute

// coderReplyFormat is appended to every task so the coder ends with the lines the summary keeps.
const coderReplyFormat = "\n\n[reply] End with `status: done|partial|fail` and one `changed: <path>: <what>` line per file you edited."

// coderRule is appended to the main system prompt when the resident coder tools are served.
const coderRule = "For important files (main code, user documents) call the resident_coder tool: one resident haiku-coder, same session, long context. " +
	"For lookups and small edits use Task haiku-scout / haiku-batch-worker."

// runResidentCoder sends task to the resident coder and blocks until its reply. A coder that is not
// running is spawned by this call. A busy coder is refused.
func (r *Repl) runResidentCoder(task string) (out string, err error) {
	task = strings.TrimSpace(task)
	if task == "" {
		return "", errors.New("task is required")
	}
	r.stage("· coder 처리중")
	defer func() {
		if err != nil {
			r.stage("· coder 실패: " + err.Error())
		} else {
			r.stage("· coder 완료")
		}
	}()
	role, err := r.pickCoderRole()
	if err != nil {
		return "", err
	}
	def, _ := r.residentSlotByRole(role)
	if _, live := r.residentChild(role); !live {
		if _, err := r.spawnSlot(def); err != nil {
			return "", err
		}
	}
	ch, err := r.sendResident(role, task+coderReplyFormat, firstLine(task))
	if err != nil {
		return "", err
	}
	select {
	case reply := <-ch:
		return summarizeReply(reply), nil
	case <-time.After(residentCoderTimeout):
		return "", fmt.Errorf("resident coder %q timed out", role)
	}
}

// subagentLabel is the subagent_type of a Task/Agent tool_use input, "agent" when it has none.
func subagentLabel(in map[string]any) string {
	if t := str(in["subagent_type"]); t != "" {
		return t
	}
	return "agent"
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
// resident_coder). It returns nil for notifications.
func (r *Repl) mcpReply(line []byte) []byte {
	var req struct {
		ID     json.RawMessage `json:"id"`
		Method string          `json:"method"`
		Params struct {
			Name      string `json:"name"`
			Arguments struct {
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
				"description": "Run a bounded code or document task on the resident haiku coder and return only its change summary. Blocks until done.",
				"inputSchema": map[string]any{"type": "object", "required": []string{"task"},
					"properties": map[string]any{
						"task": map[string]any{"type": "string"},
					}},
			},
		}}
	case "tools/call":
		var text string
		isErr := false
		switch req.Params.Name {
		case "resident_coder":
			reply, err := r.runResidentCoder(req.Params.Arguments.Task)
			text, isErr = reply, err != nil
			if err != nil {
				text = err.Error()
			}
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

// pickCoderRole returns the first idle coder: the always-on coder, then extra coders up to extraCoders
// (not started yet counts as idle; they are spawned by the caller). Every coder busy -> error.
func (r *Repl) pickCoderRole() (string, error) {
	roles := []string{"coder"}
	for i := 0; i < r.extraCoders; i++ {
		roles = append(roles, extraCoderRole(i))
	}
	for _, role := range roles {
		if c, live := r.residentChild(role); !live || c.busyRec() == nil {
			return role, nil
		}
	}
	return "", errors.New("all resident coders are busy")
}
