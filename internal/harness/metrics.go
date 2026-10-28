package harness

import (
	"fmt"
	"os"
	"path/filepath"
	"time"
)

// Metrics appends one JSON object per line to <dir>/<YYYY-MM-DD>.jsonl.
// dir == "" disables it; all methods are no-ops, and a nil *Metrics is safe.
type Metrics struct {
	dir   string
	flags map[string]any
}

func newMetrics(dir string, flags map[string]any) *Metrics {
	return &Metrics{dir: dir, flags: flags}
}

// Log adds "ts" (RFC3339 UTC) and "flags" to a copy of rec and appends it as one line.
func (m *Metrics) Log(rec map[string]any) error {
	if m == nil || m.dir == "" {
		return nil
	}
	now := time.Now().UTC()
	out := make(map[string]any, len(rec)+2)
	for k, v := range rec {
		out[k] = v
	}
	out["ts"] = now.Format(time.RFC3339)
	out["flags"] = m.flags

	if err := os.MkdirAll(m.dir, 0o755); err != nil {
		return err
	}
	path := filepath.Join(m.dir, now.Format("2006-01-02")+".jsonl")
	f, err := os.OpenFile(path, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o644)
	if err != nil {
		return err
	}
	if _, err := fmt.Fprintln(f, jsonStr(out)); err != nil {
		_ = f.Close()
		return err
	}
	return f.Close()
}

// usageFields extracts token counts from an API usage object; ctx is the context-size proxy.
func usageFields(usage map[string]any) map[string]any {
	return map[string]any{
		"input_tokens":                num(usage["input_tokens"]),
		"cache_read_input_tokens":     num(usage["cache_read_input_tokens"]),
		"cache_creation_input_tokens": num(usage["cache_creation_input_tokens"]),
		"output_tokens":               num(usage["output_tokens"]),
		"ctx":                         ctxTokens(usage),
	}
}
