package harness

import (
	"os"
	"path/filepath"

	"chaude/internal/settings"
)

// The project's main model ("주력 모델") lives in .claude/settings.json `model` so that a plain `claude`
// session and the chaude REPL agree. The REPL starts on it unless --model is given; `/model X` saves it.

// mainModels are the choices offered by /model (any other alias or full model id is still accepted).
var mainModels = []string{"opus", "fable", "sonnet", "haiku"}

func settingsPath(cwd string) string {
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	return filepath.Join(cwd, ".claude", "settings.json")
}

// loadMainModel returns the model saved for the project folder, or "" when there is none.
func loadMainModel(cwd string) string {
	b, err := os.ReadFile(settingsPath(cwd))
	if err != nil {
		return ""
	}
	return settings.Model(b)
}

// saveMainModel writes model into the project's settings.json (created with just that key when missing).
func saveMainModel(cwd, model string) error {
	p := settingsPath(cwd)
	src, err := os.ReadFile(p)
	if err != nil {
		src = []byte("{}")
	}
	out, err := settings.SetModel(src, model)
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	return os.WriteFile(p, out, 0o644)
}
