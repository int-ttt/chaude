package scripts

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
)

func init() { register("claude-history", claudeHistory) }

var nonAlnum = regexp.MustCompile(`[^A-Za-z0-9]`)

// ProjectsRoot returns ~/.claude/projects.
func ProjectsRoot() string {
	h, _ := os.UserHomeDir()
	return filepath.Join(h, ".claude", "projects")
}

func normCase(p string) string {
	p = filepath.Clean(p)
	if runtime.GOOS == "windows" {
		return strings.ToLower(strings.ReplaceAll(p, "/", "\\"))
	}
	return p
}

// HistoryDir locates a project's transcript folder ("" if none).
func HistoryDir(project string) string {
	proj := absPath(project)
	root := ProjectsRoot()
	d := filepath.Join(root, nonAlnum.ReplaceAllString(proj, "-"))
	if st, err := os.Stat(d); err == nil && st.IsDir() {
		return d
	}
	want := normCase(proj)
	ents, err := os.ReadDir(root)
	if err != nil {
		return ""
	}
	for _, en := range ents {
		if strings.HasPrefix(en.Name(), ".") {
			continue
		}
		cand := filepath.Join(root, en.Name())
		fl := globDir(cand, ".jsonl")
		if len(fl) > 3 {
			fl = fl[:3]
		}
		for _, f := range fl {
			found := false
			n := 0
			_ = eachFileLine(f, func(line string) {
				n++
				if found || n > 50 || !strings.Contains(line, `"cwd"`) {
					return
				}
				e := parseLine(line)
				if e == nil {
					return
				}
				if cwd, ok := e["cwd"].(string); ok && cwd != "" && normCase(cwd) == want {
					found = true
				}
			})
			if found {
				return cand
			}
		}
	}
	return ""
}

func claudeHistory(args []string) int {
	fs := newFS("claude-history")
	pos, rc := parseArgs(fs, args)
	if rc >= 0 {
		return rc
	}
	if len(pos) > 1 {
		fs.Usage()
		return 2
	}
	proj := os.Getenv("CLAUDE_PROJECT_DIR")
	if proj == "" {
		proj, _ = os.Getwd()
	}
	if len(pos) == 1 {
		proj = pos[0]
	}
	d := HistoryDir(proj)
	if d == "" {
		return fail("기록 없음: %s 에 대한 대화 기록 폴더를 찾지 못했습니다 (확인 위치: %s)", absPath(proj), ProjectsRoot())
	}
	fmt.Println(d)
	return 0
}
