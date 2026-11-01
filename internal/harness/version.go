package harness

import (
	"context"
	"os/exec"
	"strings"
	"sync"
	"time"
)

// ChaudeVersion is the harness release version; cmd/chaude sets it from its own version.
var ChaudeVersion = "unknown"

var (
	claudeVerOnce sync.Once
	claudeVerStr  string
)

// claudeVersion returns the output of `claude --version`, looked up once (3s timeout).
// "unknown" when claude is missing, fails, or times out.
func claudeVersion() string {
	claudeVerOnce.Do(func() {
		claudeVerStr = "unknown"
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		defer cancel()
		out, err := exec.CommandContext(ctx, "claude", "--version").Output()
		if err == nil {
			if v := strings.TrimSpace(string(out)); v != "" {
				claudeVerStr = v
			}
		}
	})
	return claudeVerStr
}
