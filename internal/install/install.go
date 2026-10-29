// Package install copies the chaude package into a project: agents, hooks, skills, settings.json (merged with
// what is there), the CLAUDE.md policy block (prepended or replaced in place), notes/ and logs/ folders, the
// version marker and the chaude binary itself under bin/. Re-running upgrades in place; user settings and
// user hooks are kept. OS differences stay in internal/settings (windows hook form) and internal/osenv.
package install

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"chaude"
	"chaude/internal/lean"
	"chaude/internal/osenv"
	"chaude/internal/settings"
)

const (
	markStart = "<!-- claude-lean:start -->"
	markEnd   = "<!-- claude-lean:end -->"
	bom       = "\xef\xbb\xbf"
)

// Options configures one installation.
type Options struct {
	Dir     string    // project folder (must exist)
	Main    string    // main-session model written to settings.json: opus|sonnet|fable|haiku|<full id>; "" = keep/default
	Python  string    // windows hook interpreter override (see settings.Windows)
	LeanOff bool      // start in normal mode (.claude/lean-off)
	Version string    // written to .claude/claude-lean.version
	Exe     string    // chaude binary to copy into bin/ ("" = this executable; "-" = skip)
	Windows bool      // write the windows settings form (default: set from the running OS by Main)
	Out     io.Writer // progress lines ("" = silent)
}

var pkgDirs = []string{".claude/agents", ".claude/hooks", ".claude/skills"}

var notesTemplates = map[string]string{
	"notes/FACTS.md": "# Confirmed Facts\n<!-- One line each. Only facts the user stated or confirmed. Update immediately on correction. -->\n" +
		"<!-- format: - [area] fact (source: user / file path) -->\n<!-- areas e.g.: people / project / env / schedule / constraint. Terse English; keep Korean values verbatim -->\n",
	"notes/DECISIONS.md": "# Decisions & Style Rules\n<!-- One line each. Newest at bottom. -->\n<!-- format: - [area] rule -->\n" +
		"<!-- areas e.g.: version / tone / format / banned. Terse English; keep Korean values verbatim -->\n",
}

var mkDirs = []string{"notes/runs", "notes/research", "notes/pastes", "logs"}

// launcherBat / launcherSh are written next to the project as chaude-start.bat / chaude-start.command.
const launcherBat = `@echo off
rem chaude launcher (written by "chaude install"): double-click to open this project's REPL.
setlocal
set "PROJDIR=%~dp0"
if "%PROJDIR:~-1%"=="\" set "PROJDIR=%PROJDIR:~0,-1%"
cd /d "%PROJDIR%"
if not exist "bin\chaude.exe" (
  echo [!] bin\chaude.exe not found. Run: chaude install "%PROJDIR%"
  pause
  exit /b 1
)
where wt >nul 2>nul
if not errorlevel 1 (
  start "" wt -d "%PROJDIR%" cmd /k "bin\chaude.exe"
  exit /b 0
)
echo [i] Windows Terminal not found: Korean input may not work in this window.
echo     Install it:  winget install Microsoft.WindowsTerminal
bin\chaude.exe
pause
`

const launcherSh = `#!/bin/sh
# chaude launcher (written by "chaude install"): double-click in Finder to open this project's REPL.
cd "$(dirname "$0")" && exec bin/chaude
`

// gitignoreLines are appended (once) to an existing .gitignore: runtime state that must not be committed.
var gitignoreLines = []string{"logs/", "notes/pastes/", "notes/metrics/", "bin/chaude", "bin/chaude.exe", "bin/chaude.exe.old", ".claude/lean-off", ".claude-backup-*/"}

// Run installs or upgrades the package in o.Dir.
func Run(o Options) error {
	if o.Out == nil {
		o.Out = io.Discard
	}
	dir, err := filepath.Abs(o.Dir)
	if err != nil {
		return err
	}
	if st, err := os.Stat(dir); err != nil || !st.IsDir() {
		return fmt.Errorf("not a folder: %s", dir)
	}
	say := func(f string, a ...any) { fmt.Fprintf(o.Out, f+"\n", a...) }

	// 1. backup what we overwrite in place
	stamp := time.Now().Format("20060102-150405")
	for _, rel := range []string{"CLAUDE.md", ".claude/settings.json"} {
		src := filepath.Join(dir, filepath.FromSlash(rel))
		if b, err := os.ReadFile(src); err == nil {
			dst := filepath.Join(dir, ".claude-backup-"+stamp, filepath.FromSlash(rel))
			if err := writeFile(dst, b); err != nil {
				return err
			}
			say("backup  %s -> %s", rel, filepath.ToSlash(filepath.Join(".claude-backup-"+stamp, rel)))
		}
	}

	// 2. package files (agents, hooks, skills): package-owned, always overwritten
	n := 0
	err = fs.WalkDir(chaude.Assets, ".", func(p string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() || !isPkgFile(p) {
			return err
		}
		b, err := fs.ReadFile(chaude.Assets, p)
		if err != nil {
			return err
		}
		n++
		dst := filepath.Join(dir, filepath.FromSlash(p))
		if err := writeFile(dst, lf(b)); err != nil {
			return err
		}
		if path.Ext(p) == "" { // launcher scripts (pyrun) must stay executable on mac/linux
			_ = os.Chmod(dst, 0o755)
		}
		return nil
	})
	if err != nil {
		return err
	}
	say("files   %d package files under .claude/ (agents, hooks, skills)", n)

	// 3. settings.json: merge, then the OS form
	pkgSet, err := fs.ReadFile(chaude.Assets, ".claude/settings.json")
	if err != nil {
		return err
	}
	setPath := filepath.Join(dir, ".claude", "settings.json")
	existing, _ := os.ReadFile(setPath)
	merged, err := MergeSettings(existing, lf(pkgSet), o.Main, hookNames())
	if err != nil {
		return fmt.Errorf(".claude/settings.json: %w", err)
	}
	if o.Windows {
		py := o.Python
		if py == "" {
			py = settings.WindowsPython()
		}
		if merged, err = settings.Windows(merged, py); err != nil {
			return err
		}
		say("settings .claude/settings.json (windows hook form, interpreter: %s)", py)
		if py == "python" && osenv.IsWindows {
			say("[!]     no Python 3 found (py launcher or python.exe): the hooks will not run until it is installed: winget install Python.Python.3.13")
		}
	} else {
		say("settings .claude/settings.json")
	}
	if err := writeFile(setPath, merged); err != nil {
		return err
	}

	// 4. CLAUDE.md policy block
	policy, err := Policy()
	if err != nil {
		return err
	}
	mdPath := filepath.Join(dir, "CLAUDE.md")
	old, _ := os.ReadFile(mdPath)
	had := bytes.Contains(old, []byte(markStart))
	if err := writeFile(mdPath, MergePolicy(old, policy)); err != nil {
		return err
	}
	switch {
	case had:
		say("policy  CLAUDE.md: policy block replaced, project content kept")
	case len(old) > 0:
		say("policy  CLAUDE.md: policy block added above the existing content")
	default:
		say("policy  CLAUDE.md created")
	}

	// 5. notes/ and logs/
	for _, d := range mkDirs {
		if err := os.MkdirAll(filepath.Join(dir, filepath.FromSlash(d)), 0o755); err != nil {
			return err
		}
	}
	for rel, body := range notesTemplates {
		p := filepath.Join(dir, filepath.FromSlash(rel))
		if _, err := os.Stat(p); err == nil {
			continue
		}
		if err := writeFile(p, []byte(body)); err != nil {
			return err
		}
	}
	say("notes   notes/ (FACTS, DECISIONS, runs, research, pastes) and logs/")

	// 6. version marker (session-context.py / style-adapter.py report upgrades from it)
	if o.Version != "" {
		if err := writeFile(filepath.Join(dir, ".claude", "claude-lean.version"), []byte(o.Version+"\n")); err != nil {
			return err
		}
	}

	// 7. the binary: hooks and agents call bin/chaude[.exe]
	if o.Exe != "-" {
		src := o.Exe
		if src == "" {
			if src, err = os.Executable(); err != nil {
				return err
			}
		}
		name := "chaude"
		if o.Windows {
			name += ".exe"
		}
		dst := filepath.Join(dir, "bin", name)
		_ = os.Remove(dst + ".old") // left by an earlier upgrade while chaude was running there
		if same, _ := sameFile(src, dst); !same {
			b, err := os.ReadFile(src)
			if err != nil {
				return fmt.Errorf("copy binary: %w", err)
			}
			if err := writeExe(dst, b); err != nil {
				return err
			}
			_ = os.Chmod(dst, 0o755)
		}
		say("binary  bin/%s", name)
	}

	// 7b. double-click launcher: opens the project's REPL (Windows Terminal when present: Korean IME needs it)
	if o.Windows {
		if err := writeFile(filepath.Join(dir, "chaude-start.bat"), []byte(strings.ReplaceAll(launcherBat, "\n", "\r\n"))); err != nil {
			return err
		}
		say("launch  chaude-start.bat (double-click)  ·  right-click menu: chaude context-menu install")
	} else {
		p := filepath.Join(dir, "chaude-start.command")
		if err := writeFile(p, []byte(launcherSh)); err != nil {
			return err
		}
		_ = os.Chmod(p, 0o755)
		say("launch  chaude-start.command (double-click in Finder)")
	}

	// 8. .gitignore (only when the project has one): runtime state stays out of git
	giPath := filepath.Join(dir, ".gitignore")
	if gi, err := os.ReadFile(giPath); err == nil {
		if added := addGitignore(&gi); len(added) > 0 {
			if err := writeFile(giPath, gi); err != nil {
				return err
			}
			say("git     .gitignore += %s", strings.Join(added, " "))
		}
	}

	// 9. mode
	off := filepath.Join(dir, ".claude", "lean-off")
	if o.LeanOff {
		if err := writeFile(off, nil); err != nil {
			return err
		}
		say("mode    normal (일반모드); type 절약모드 in claude to switch on")
	} else if _, err := os.Stat(off); err == nil {
		say("mode    normal (existing .claude/lean-off kept); type 절약모드 in claude to switch on")
	}
	return nil
}

// isPkgFile: a file inside one of pkgDirs, excluding Python cache leftovers.
func isPkgFile(p string) bool {
	if strings.Contains(p, "__pycache__") || strings.HasSuffix(p, ".pyc") {
		return false
	}
	for _, d := range pkgDirs {
		if strings.HasPrefix(p, d+"/") {
			return true
		}
	}
	return false
}

// hookNames lists the package hooks (file names) so merged settings can recognise package-owned entries.
func hookNames() map[string]bool {
	names := map[string]bool{}
	ents, _ := fs.ReadDir(chaude.Assets, ".claude/hooks")
	for _, e := range ents {
		if strings.HasSuffix(e.Name(), ".py") {
			names[e.Name()] = true
		}
	}
	return names
}

// lf normalises line endings of embedded text (a CRLF checkout must not leak into installed files).
func lf(b []byte) []byte { return bytes.ReplaceAll(b, []byte("\r\n"), []byte("\n")) }

// Policy returns the text between the claude-lean markers of the embedded CLAUDE.md (no markers, trimmed).
func Policy() (string, error) {
	b, err := fs.ReadFile(chaude.Assets, "CLAUDE.md")
	if err != nil {
		return "", err
	}
	s := string(lf(b))
	i, j := strings.Index(s, markStart), strings.Index(s, markEnd)
	if i < 0 || j < i {
		return "", fmt.Errorf("embedded CLAUDE.md has no policy markers")
	}
	return strings.TrimSpace(s[i+len(markStart) : j]), nil
}

// MergePolicy puts the policy block (markers + policy) into an existing CLAUDE.md: replaces the block between
// the markers when present, else prepends it. A UTF-8 BOM and CRLF line endings of the existing file are kept.
func MergePolicy(existing []byte, policy string) []byte {
	body := string(existing)
	hasBOM := strings.HasPrefix(body, bom)
	body = strings.TrimPrefix(body, bom)
	nl := "\n"
	if strings.Contains(body, "\r\n") {
		nl = "\r\n"
	}
	block := strings.ReplaceAll(markStart+"\n"+policy+"\n"+markEnd, "\n", nl)
	var out string
	i := strings.Index(body, markStart)
	j := -1
	if i >= 0 {
		if k := strings.Index(body[i:], markEnd); k >= 0 {
			j = i + k + len(markEnd)
		}
	}
	switch {
	case i >= 0 && j >= 0:
		out = body[:i] + block + body[j:]
	case strings.TrimSpace(body) == "":
		out = block + nl
	default:
		out = block + nl + nl + body
	}
	if hasBOM {
		out = bom + out
	}
	return []byte(out)
}

// MergeSettings merges the package settings (posix form) into an existing settings.json:
// scalar keys are set only when absent (user tuning survives upgrades); env is merged per key the same way;
// permissions.allow is a union; hook groups owned by the package (commands pointing at .claude/hooks/<pkg>.py)
// are replaced, other hooks are kept; main != "" sets model. Output: posix form, stable key order.
func MergeSettings(existing, pkg []byte, main string, pkgHooks map[string]bool) ([]byte, error) {
	ex := map[string]any{}
	if len(bytes.TrimSpace(existing)) > 0 {
		if err := json.Unmarshal(existing, &ex); err != nil {
			return nil, fmt.Errorf("existing file is not valid JSON (%v); fix or remove it", err)
		}
	}
	var pk map[string]any
	if err := json.Unmarshal(pkg, &pk); err != nil {
		return nil, err
	}
	for k, v := range pk {
		switch k {
		case "env":
			env := asMap(ex["env"])
			for kk, vv := range asMap(v) {
				if _, ok := env[kk]; !ok {
					env[kk] = vv
				}
			}
			ex["env"] = env
		case "permissions":
			perms := asMap(ex["permissions"])
			allow := asList(perms["allow"])
			for _, it := range asList(asMap(v)["allow"]) {
				if !containsAny(allow, it) {
					allow = append(allow, it)
				}
			}
			perms["allow"] = allow
			ex["permissions"] = perms
		case "hooks":
			hooks := asMap(ex["hooks"])
			for ev, groups := range hooks { // drop package-owned groups everywhere (a hook may have moved events)
				var keep []any
				for _, g := range asList(groups) {
					if !pkgOwned(g, pkgHooks) {
						keep = append(keep, g)
					}
				}
				if len(keep) == 0 {
					delete(hooks, ev)
				} else {
					hooks[ev] = keep
				}
			}
			for ev, groups := range asMap(v) {
				hooks[ev] = append(asList(hooks[ev]), asList(groups)...)
			}
			ex["hooks"] = hooks
		default:
			if _, ok := ex[k]; !ok {
				ex[k] = v
			}
		}
	}
	if main != "" {
		ex["model"] = main
	}
	return formatSettings(ex), nil
}

// pkgOwned: a hook group whose command (shell form) or args (exec form) name one of the package hooks.
func pkgOwned(group any, pkgHooks map[string]bool) bool {
	for _, h := range asList(asMap(group)["hooks"]) {
		hm := asMap(h)
		strs := []string{str(hm["command"])}
		for _, a := range asList(hm["args"]) {
			strs = append(strs, str(a))
		}
		for _, s := range strs {
			if i := strings.Index(s, ".claude/hooks/"); i >= 0 && pkgHooks[path.Base(strings.Trim(s[i:], `"' `))] {
				return true
			}
		}
	}
	return false
}

// formatSettings writes the map as indented JSON with a readable top-level key order.
func formatSettings(m map[string]any) []byte {
	pref := []string{"model", "effortLevel", "autoCompactWindow", "env", "permissions", "hooks"}
	var keys []string
	seen := map[string]bool{}
	for _, k := range pref {
		if _, ok := m[k]; ok {
			keys, seen[k] = append(keys, k), true
		}
	}
	var rest []string
	for k := range m {
		if !seen[k] {
			rest = append(rest, k)
		}
	}
	sort.Strings(rest)
	keys = append(keys, rest...)
	var b bytes.Buffer
	b.WriteString("{\n")
	for i, k := range keys {
		b.WriteString("  " + jsonStr(k) + ": ")
		var vb bytes.Buffer
		enc := json.NewEncoder(&vb)
		enc.SetEscapeHTML(false)
		enc.SetIndent("  ", "  ")
		_ = enc.Encode(m[k])
		b.Write(bytes.TrimRight(vb.Bytes(), "\n"))
		if i < len(keys)-1 {
			b.WriteString(",")
		}
		b.WriteString("\n")
	}
	b.WriteString("}\n")
	return b.Bytes()
}

func jsonStr(s string) string {
	var b bytes.Buffer
	enc := json.NewEncoder(&b)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(s)
	return strings.TrimRight(b.String(), "\n")
}

func asMap(v any) map[string]any {
	if m, ok := v.(map[string]any); ok {
		return m
	}
	return map[string]any{}
}

func asList(v any) []any {
	l, _ := v.([]any)
	return l
}

func str(v any) string {
	s, _ := v.(string)
	return s
}

func containsAny(l []any, v any) bool {
	for _, x := range l {
		if x == v {
			return true
		}
	}
	return false
}

// addGitignore appends the missing gitignoreLines under a "# chaude" heading; returns what was added.
func addGitignore(gi *[]byte) []string {
	have := map[string]bool{}
	for _, l := range strings.Split(string(*gi), "\n") {
		have[strings.TrimSpace(strings.TrimRight(l, "\r"))] = true
	}
	var add []string
	for _, l := range gitignoreLines {
		if !have[l] {
			add = append(add, l)
		}
	}
	if len(add) == 0 {
		return nil
	}
	nl := "\n"
	if bytes.Contains(*gi, []byte("\r\n")) {
		nl = "\r\n"
	}
	s := string(*gi)
	if s != "" && !strings.HasSuffix(s, "\n") {
		s += nl
	}
	s += nl + "# chaude (runtime state)" + nl + strings.Join(add, nl) + nl
	*gi = []byte(s)
	return add
}

func writeFile(p string, b []byte) error {
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	return os.WriteFile(p, b, 0o644)
}

// writeExe replaces the project's chaude binary. Windows locks a running executable against writes but
// allows renaming it, so a REPL that is open in that project keeps running on the renamed file.
func writeExe(dst string, b []byte) error {
	err := writeFile(dst, b)
	if err == nil {
		return nil
	}
	if _, st := os.Stat(dst); st != nil {
		return err
	}
	old := dst + ".old"
	_ = os.Remove(old)
	if rerr := os.Rename(dst, old); rerr != nil {
		return fmt.Errorf("copy binary: %w (close the chaude REPL running in that project and retry)", err)
	}
	return writeFile(dst, b)
}

func sameFile(a, b string) (bool, error) {
	sa, err := os.Stat(a)
	if err != nil {
		return false, err
	}
	sb, err := os.Stat(b)
	if err != nil {
		return false, err
	}
	return os.SameFile(sa, sb), nil
}

// Main: chaude install <project> [--main MODEL] [--python CMD] [--lean-off] [--os windows|mac|linux]
func Main(args []string, version string) int {
	fs := flag.NewFlagSet("install", flag.ContinueOnError)
	var o Options
	fs.StringVar(&o.Main, "main", "", "main-session model written to settings.json (opus|sonnet|fable|haiku); default: keep existing, else opus")
	fs.StringVar(&o.Python, "python", "", "windows hook interpreter (default: py -3 if found, else a real python.exe)")
	fs.BoolVar(&o.LeanOff, "lean-off", false, "start in normal mode (creates .claude/lean-off)")
	osName := fs.String("os", osenv.Name(), "settings form to write: windows|mac|linux")
	pos, err := lean.ParseInterspersed(fs, args)
	if err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	if len(pos) != 1 {
		fmt.Fprintln(os.Stderr, "usage: chaude install <project folder> [--main opus|sonnet|fable|haiku] [--python CMD] [--lean-off] [--os windows|mac|linux]")
		return 2
	}
	switch *osName {
	case "windows", "mac", "linux":
	default:
		fmt.Fprintln(os.Stderr, "--os must be windows, mac or linux")
		return 2
	}
	o.Dir, o.Version, o.Windows, o.Out = pos[0], version, *osName == "windows", os.Stdout
	if err := Run(o); err != nil {
		fmt.Fprintln(os.Stderr, "chaude install:", err)
		return 1
	}
	abs, _ := filepath.Abs(o.Dir)
	fmt.Printf("\ninstalled chaude %s into %s\nnext: cd into the project and start a NEW claude session (hooks/agents load at launch), or run bin/chaude for the REPL.\n", version, abs)
	return 0
}
