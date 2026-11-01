// Package clean removes the claude-lean harness from a project: the reverse of what the old install step wrote
// (git 28d26c6 internal/install). Manifest = the package files under .claude/ (agents, hooks, skills), the
// claude-lean marker block in CLAUDE.md, and the package hook entries in .claude/settings.json. Everything else
// (notes/, record/, user files, user settings keys, user hooks) is left alone. Dry-run unless --yes.
package clean

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"strings"
	"time"

	"chaude/internal/lean"
)

const (
	markStart    = "<!-- claude-lean:start -->"
	markEnd      = "<!-- claude-lean:end -->"
	bom          = "\xef\xbb\xbf"
	backupPrefix = ".chaude-backup-"
	hooksDir     = ".claude/hooks/"
)

// pkgFiles is the manifest: every file the harness installed under .claude/ (slash paths, relative to the project).
// TestManifestMatchesRepo keeps it equal to the repo's .claude/ tree (minus __pycache__).
var pkgFiles = []string{
	".claude/agents/haiku-batch-worker.md",
	".claude/agents/haiku-checker.md",
	".claude/agents/haiku-coder.md",
	".claude/agents/haiku-drafter.md",
	".claude/agents/haiku-extractor.md",
	".claude/agents/haiku-prompter.md",
	".claude/agents/haiku-researcher.md",
	".claude/agents/haiku-runner.md",
	".claude/agents/haiku-scout.md",
	".claude/agents/haiku-tui.md",
	".claude/agents/opus-architect.md",
	".claude/hooks/_lean.py",
	".claude/hooks/agent-model-pin.py",
	".claude/hooks/caveman-format.py",
	".claude/hooks/context-alert.py",
	".claude/hooks/correction-guard.py",
	".claude/hooks/haiku-context-guard.py",
	".claude/hooks/history-export.py",
	".claude/hooks/ledger.py",
	".claude/hooks/main-thread-guard.py",
	".claude/hooks/session-context.py",
	".claude/hooks/style-adapter.py",
	".claude/skills/check/SKILL.md",
	".claude/skills/correction/SKILL.md",
	".claude/skills/distill/SKILL.md",
	".claude/skills/fact/SKILL.md",
	".claude/skills/migrate-session/SKILL.md",
	".claude/skills/research/SKILL.md",
	".claude/skills/status/SKILL.md",
}

const cleanUsage = `usage: chaude clean [dir] [--yes]
  claude-lean 하네스를 프로젝트에서 제거한다. 기본은 dry-run(대상만 출력, 변경 없음).
  제거 대상: CLAUDE.md의 claude-lean 정책 블록(마커 사이만; 남는 내용이 없으면 파일 삭제),
            .claude/{agents,hooks,skills}의 설치 파일, .claude/settings.json의 훅 항목(다른 키·사용자 훅 유지)
  건드리지 않음: notes/, record/, 목록에 없는 파일
  --yes                  실제로 제거. 먼저 바뀔 파일을 전부 .chaude-backup-<YYYYMMDD-HHMMSS>/ 에 복사한다`

// Plan is what clean would change in one project. Nothing is written until Apply.
type Plan struct {
	Dir         string
	removeFiles []string          // slash paths under .claude/ that exist and are in the manifest
	mdTouched   bool              // CLAUDE.md contains the marker block
	mdDelete    bool              // nothing but the block was in CLAUDE.md: delete the file
	mdNew       []byte            // CLAUDE.md content without the block (when !mdDelete)
	setTouched  bool              // settings.json had package hook entries
	setNew      []byte            // settings.json without them
	hookRemoved int               // number of package hook entries removed
	orig        map[string][]byte // slash path -> original bytes of every touched file (backup source)
}

// MakePlan reads dir and decides every change. It fails (and changes nothing) on files it cannot interpret,
// e.g. a CLAUDE.md with only a start marker or a settings.json that is not JSON.
func MakePlan(dir string) (*Plan, error) {
	p := &Plan{Dir: dir, orig: map[string][]byte{}}
	for _, rel := range pkgFiles {
		st, err := os.Stat(filepath.Join(dir, filepath.FromSlash(rel)))
		if err != nil || st.IsDir() {
			continue
		}
		p.removeFiles = append(p.removeFiles, rel)
		b, err := os.ReadFile(filepath.Join(dir, filepath.FromSlash(rel)))
		if err != nil {
			return nil, err
		}
		p.orig[rel] = b
	}
	if err := p.planClaudeMd(); err != nil {
		return nil, err
	}
	if err := p.planSettings(); err != nil {
		return nil, err
	}
	return p, nil
}

// Lines describes the planned changes (one line per target).
func (p *Plan) Lines() []string {
	var ls []string
	for _, rel := range p.removeFiles {
		ls = append(ls, "remove  "+rel)
	}
	if p.mdTouched {
		if p.mdDelete {
			ls = append(ls, "delete  CLAUDE.md (only the claude-lean block was in it)")
		} else {
			ls = append(ls, "policy  CLAUDE.md: claude-lean block removed, project content kept")
		}
	}
	if p.setTouched {
		ls = append(ls, fmt.Sprintf("settings .claude/settings.json: %d claude-lean hook entries removed (other keys kept)", p.hookRemoved))
	}
	return ls
}

// Apply backs up every touched file to <dir>/.chaude-backup-<stamp>/ (same relative paths), then applies the plan.
// It returns the backup folder ("" when there was nothing to do).
func (p *Plan) Apply(now time.Time) (string, error) {
	if len(p.Lines()) == 0 {
		return "", nil
	}
	bdir := filepath.Join(p.Dir, backupPrefix+now.Format("20060102-150405"))
	for rel, b := range p.orig {
		if err := writeFile(filepath.Join(bdir, filepath.FromSlash(rel)), b); err != nil {
			return "", err
		}
	}
	if p.mdTouched {
		md := filepath.Join(p.Dir, "CLAUDE.md")
		if p.mdDelete {
			if err := os.Remove(md); err != nil {
				return bdir, err
			}
		} else if err := writeFile(md, p.mdNew); err != nil {
			return bdir, err
		}
	}
	if p.setTouched {
		if err := writeFile(filepath.Join(p.Dir, ".claude", "settings.json"), p.setNew); err != nil {
			return bdir, err
		}
	}
	for _, rel := range p.removeFiles {
		if err := os.Remove(filepath.Join(p.Dir, filepath.FromSlash(rel))); err != nil {
			return bdir, err
		}
		pruneEmptyDirs(p.Dir, path.Dir(rel))
	}
	return bdir, nil
}

// pruneEmptyDirs removes rel and its parents while they are empty, up to and including .claude.
// os.Remove fails on non-empty dirs, so user files are never deleted here.
func pruneEmptyDirs(root, rel string) {
	for rel != "." && strings.HasPrefix(rel, ".claude") {
		_ = os.Remove(filepath.Join(root, filepath.FromSlash(rel)))
		rel = path.Dir(rel)
	}
}

func (p *Plan) planClaudeMd() error {
	const rel = "CLAUDE.md"
	b, err := os.ReadFile(filepath.Join(p.Dir, rel))
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	out, found, err := stripPolicy(b)
	if err != nil {
		return fmt.Errorf("CLAUDE.md: %w (fix by hand first; nothing changed)", err)
	}
	if !found {
		return nil
	}
	p.orig[rel] = b
	p.mdTouched = true
	if strings.TrimSpace(string(out)) == "" {
		p.mdDelete = true
		return nil
	}
	p.mdNew = out
	return nil
}

// stripPolicy removes the claude-lean block (markers included). found=false when there is no block.
// install put the block first and separated it from the old content by one blank line, so that
// blank line is dropped when the block was at the top; a UTF-8 BOM and CRLF endings are kept.
func stripPolicy(b []byte) (out []byte, found bool, err error) {
	body := string(b)
	hasBOM := strings.HasPrefix(body, bom)
	body = strings.TrimPrefix(body, bom)
	i := strings.Index(body, markStart)
	if i < 0 {
		if strings.Contains(body, markEnd) {
			return nil, false, fmt.Errorf("end marker without start marker")
		}
		return nil, false, nil
	}
	k := strings.Index(body[i:], markEnd)
	if k < 0 {
		return nil, false, fmt.Errorf("start marker without end marker")
	}
	j := i + k + len(markEnd)
	head, tail := body[:i], body[j:]
	if strings.TrimSpace(head) == "" {
		tail = strings.TrimLeft(tail, "\r\n")
	}
	res := head + tail
	if strings.TrimSpace(res) == "" {
		return []byte{}, true, nil
	}
	if hasBOM {
		res = bom + res
	}
	return []byte(res), true, nil
}

func (p *Plan) planSettings() error {
	const rel = ".claude/settings.json"
	b, err := os.ReadFile(filepath.Join(p.Dir, filepath.FromSlash(rel)))
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	m := map[string]any{}
	if len(bytes.TrimSpace(b)) > 0 {
		dec := json.NewDecoder(bytes.NewReader(b))
		dec.UseNumber()
		if err := dec.Decode(&m); err != nil {
			return fmt.Errorf("%s is not valid JSON (%v); fix it by hand first (nothing changed)", rel, err)
		}
	}
	n := stripHooks(m, pkgHookNames())
	if n == 0 {
		return nil
	}
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "  ")
	if err := enc.Encode(m); err != nil {
		return err
	}
	p.orig[rel] = b
	p.setTouched = true
	p.setNew = buf.Bytes()
	p.hookRemoved = n
	return nil
}

// stripHooks removes the hook entries that run a package hook from m["hooks"]. A group keeps its other
// entries; a group left empty is dropped; an event left empty is dropped; "hooks" is dropped when it ends up empty.
// Returns the number of entries removed.
func stripHooks(m map[string]any, names map[string]bool) int {
	hooks, ok := m["hooks"].(map[string]any)
	if !ok {
		return 0
	}
	total := 0
	for ev, v := range hooks {
		groups, ok := v.([]any)
		if !ok {
			continue
		}
		removed := 0
		var keep []any
		for _, g := range groups {
			gm, ok := g.(map[string]any)
			if !ok {
				keep = append(keep, g)
				continue
			}
			inner, _ := gm["hooks"].([]any)
			var kept []any
			for _, h := range inner {
				if isPkgHook(h, names) {
					removed++
					continue
				}
				kept = append(kept, h)
			}
			switch {
			case len(kept) == len(inner):
				keep = append(keep, g)
			case len(kept) == 0:
			default:
				gm["hooks"] = kept
				keep = append(keep, gm)
			}
		}
		if removed == 0 {
			continue
		}
		total += removed
		if len(keep) == 0 {
			delete(hooks, ev)
		} else {
			hooks[ev] = keep
		}
	}
	if total > 0 && len(hooks) == 0 {
		delete(m, "hooks")
	}
	return total
}

// isPkgHook: a hook entry whose command (shell form) or args (exec form, windows) run .claude/hooks/<pkg>.py.
func isPkgHook(h any, names map[string]bool) bool {
	hm, ok := h.(map[string]any)
	if !ok {
		return false
	}
	strs := []string{str(hm["command"])}
	if args, ok := hm["args"].([]any); ok {
		for _, a := range args {
			strs = append(strs, str(a))
		}
	}
	for _, s := range strs {
		if n := hookName(s); n != "" && names[n] {
			return true
		}
	}
	return false
}

// hookName is the file name after ".claude/hooks/" in a command string ("" when absent). Accepts / and \ separators.
func hookName(s string) string {
	s = strings.ReplaceAll(s, `\`, "/")
	i := strings.Index(s, hooksDir)
	if i < 0 {
		return ""
	}
	rest := s[i+len(hooksDir):]
	if end := strings.IndexAny(rest, `"' `); end >= 0 {
		rest = rest[:end]
	}
	return rest
}

// pkgHookNames: the package hook file names (.claude/hooks/<name>) from the manifest.
func pkgHookNames() map[string]bool {
	names := map[string]bool{}
	for _, rel := range pkgFiles {
		if strings.HasPrefix(rel, hooksDir) && strings.HasSuffix(rel, ".py") {
			names[strings.TrimPrefix(rel, hooksDir)] = true
		}
	}
	return names
}

func str(v any) string {
	s, _ := v.(string)
	return s
}

func writeFile(p string, b []byte) error {
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	return os.WriteFile(p, b, 0o644)
}

// Main: clean [dir] [--yes]. Returns the process exit code.
func Main(args []string) int {
	fs := flag.NewFlagSet("clean", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	fs.Usage = func() { fmt.Fprintln(os.Stderr, cleanUsage) }
	yes := fs.Bool("yes", false, "actually remove (default: dry-run)")
	pos, err := lean.ParseInterspersed(fs, args)
	if err != nil {
		if err == flag.ErrHelp {
			return 0
		}
		return 2
	}
	if len(pos) > 1 {
		fs.Usage()
		return 2
	}
	dir := "."
	if len(pos) == 1 {
		dir = pos[0]
	}
	abs, err := filepath.Abs(dir)
	if err != nil {
		fmt.Fprintln(os.Stderr, "clean:", err)
		return 1
	}
	if st, err := os.Stat(abs); err != nil || !st.IsDir() {
		fmt.Fprintf(os.Stderr, "not a folder: %s\n", abs)
		return 1
	}
	p, err := MakePlan(abs)
	if err != nil {
		fmt.Fprintln(os.Stderr, "clean:", err)
		return 1
	}
	lines := p.Lines()
	if len(lines) == 0 {
		fmt.Printf("nothing to remove: no claude-lean harness files in %s\n", abs)
		return 0
	}
	for _, l := range lines {
		fmt.Println(l)
	}
	if !*yes {
		fmt.Printf("dry-run: %d targets listed, nothing changed. Run with --yes to remove (backup first to %s<time>/)\n", len(lines), backupPrefix)
		return 0
	}
	bdir, err := p.Apply(time.Now())
	if err != nil {
		fmt.Fprintf(os.Stderr, "clean: %v (backup: %s)\n", err, bdir)
		return 1
	}
	fmt.Printf("backup  %s (%d files)\ndone    %d targets applied\n", filepath.Base(bdir), len(p.orig), len(lines))
	return 0
}
