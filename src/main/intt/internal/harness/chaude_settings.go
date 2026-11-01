package harness

import (
	"encoding/json"
	"os"
	"path/filepath"
)

// chaudeSettings is <project>/.chaude/settings.json. Changes to extra_coders and main_model apply on the next
// session start; session_retention_days applies at the next start (prune).
type chaudeSettings struct {
	SessionRetentionDays int    `json:"session_retention_days"` // 0 = keep forever
	ExtraCoders          int    `json:"extra_coders"`           // extra haiku coders (coder2...) on a 270s idle TTL, 0-8
	MainModel            string `json:"main_model"`             // opus | sonnet | fable
}

const (
	settingsMaxExtraCoders     = 8
	settingsDefaultRetain      = 30
	settingsDefaultExtraCoders = 4
	settingsDefaultModel       = "opus"
)

// mainModelIDs maps the settings alias to the claude model id used for the main session.
var mainModelIDs = map[string]string{
	"opus":   "claude-opus-5-5",
	"sonnet": "claude-sonnet-5-5",
	"fable":  "claude-fable-5-1",
}

// defaultChaudeSettings returns the values used when the file is missing or a field is absent.
func defaultChaudeSettings() chaudeSettings {
	return chaudeSettings{SessionRetentionDays: settingsDefaultRetain, ExtraCoders: settingsDefaultExtraCoders, MainModel: settingsDefaultModel}
}

// clamp fixes out-of-range values: retention >= 0, extra coders 0-8, unknown main model -> opus.
func (s chaudeSettings) clamp() chaudeSettings {
	if s.SessionRetentionDays < 0 {
		s.SessionRetentionDays = 0
	}
	if s.ExtraCoders < 0 {
		s.ExtraCoders = 0
	}
	if s.ExtraCoders > settingsMaxExtraCoders {
		s.ExtraCoders = settingsMaxExtraCoders
	}
	if _, ok := mainModelIDs[s.MainModel]; !ok {
		s.MainModel = settingsDefaultModel
	}
	return s
}

// modelID returns the claude model id for MainModel.
func (s chaudeSettings) modelID() string {
	return mainModelIDs[s.clamp().MainModel]
}

// settingsFileFor is <project cwd>/.chaude/settings.json, one file per project folder ("" when the cwd is
// unknown). An empty cwd means the process working directory, as recordDir does.
func settingsFileFor(cwd string) string {
	if cwd == "" {
		wd, err := os.Getwd()
		if err != nil {
			return ""
		}
		cwd = wd
	}
	return filepath.Join(cwd, ".chaude", "settings.json")
}

// loadChaudeSettings reads path. A missing or unreadable file yields the defaults; absent fields keep
// their defaults; the result is clamped. The 0.9.7 key coder_slots is read when extra_coders is absent.
func loadChaudeSettings(path string) chaudeSettings {
	s := defaultChaudeSettings()
	if path == "" {
		return s
	}
	b, err := os.ReadFile(path)
	if err != nil {
		return s
	}
	_ = json.Unmarshal(b, &s)
	var keys struct {
		ExtraCoders *int `json:"extra_coders"`
		CoderSlots  *int `json:"coder_slots"` // legacy name
	}
	if json.Unmarshal(b, &keys) == nil && keys.ExtraCoders == nil && keys.CoderSlots != nil {
		s.ExtraCoders = *keys.CoderSlots
	}
	return s.clamp()
}

// saveChaudeSettings writes the clamped settings to path through a tmp file and rename.
func saveChaudeSettings(path string, s chaudeSettings) error {
	s = s.clamp()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return err
	}
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, append(b, '\n'), 0o600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}
