// Package chaude embeds the per-project package (agents, hooks, skills, settings, CLAUDE.md policy)
// so that a single chaude binary can install it anywhere: `chaude install <project>`.
package chaude

import "embed"

// Assets holds .claude/{agents,hooks,skills}, .claude/settings.json (posix form, the source of truth)
// and CLAUDE.md (the policy block between the claude-lean markers is what gets installed).
//
//go:embed all:.claude/agents all:.claude/hooks all:.claude/skills .claude/settings.json CLAUDE.md
var Assets embed.FS
