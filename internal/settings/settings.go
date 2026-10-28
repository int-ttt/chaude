// Package settings derives OS-specific .claude/settings.json from the posix source (mac/linux form = source of truth).
// windows: hooks use exec form (command=<exe>, args=[...]) -> no shell, no Git Bash/PowerShell dependency.
package settings

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"

	"chaude/internal/lean"
	"chaude/internal/osenv"
)

// node is an order-preserving JSON value: object (keys/vals), array (items) or scalar (raw).
type node struct {
	keys  []string
	vals  []*node
	items []*node
	raw   json.RawMessage
	kind  byte // 'o', 'a', 's'
}

func parse(b []byte) (*node, error) {
	b = bytes.TrimSpace(b)
	if len(b) == 0 {
		return nil, fmt.Errorf("empty JSON")
	}
	switch b[0] {
	case '{':
		n := &node{kind: 'o'}
		dec := json.NewDecoder(bytes.NewReader(b))
		if _, err := dec.Token(); err != nil {
			return nil, err
		}
		for dec.More() {
			t, err := dec.Token()
			if err != nil {
				return nil, err
			}
			var raw json.RawMessage
			if err := dec.Decode(&raw); err != nil {
				return nil, err
			}
			v, err := parse(raw)
			if err != nil {
				return nil, err
			}
			n.keys, n.vals = append(n.keys, t.(string)), append(n.vals, v)
		}
		return n, nil
	case '[':
		var raws []json.RawMessage
		if err := json.Unmarshal(b, &raws); err != nil {
			return nil, err
		}
		n := &node{kind: 'a'}
		for _, r := range raws {
			v, err := parse(r)
			if err != nil {
				return nil, err
			}
			n.items = append(n.items, v)
		}
		return n, nil
	}
	if !json.Valid(b) {
		return nil, fmt.Errorf("invalid JSON scalar %q", b)
	}
	return &node{kind: 's', raw: append(json.RawMessage(nil), b...)}, nil
}

func scalar(v any) *node {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(v)
	return &node{kind: 's', raw: bytes.TrimSpace(buf.Bytes())}
}

func (n *node) get(k string) *node {
	for i, key := range n.keys {
		if key == k {
			return n.vals[i]
		}
	}
	return nil
}

func (n *node) str() string {
	var s string
	if n != nil && n.kind == 's' {
		_ = json.Unmarshal(n.raw, &s)
	}
	return s
}

// insertAfter sets key k (replacing, or inserting right after key `after`).
func (n *node) insertAfter(after, k string, v *node) {
	for i, key := range n.keys {
		if key == k {
			n.vals[i] = v
			return
		}
	}
	for i, key := range n.keys {
		if key == after {
			n.keys = append(n.keys[:i+1], append([]string{k}, n.keys[i+1:]...)...)
			n.vals = append(n.vals[:i+1], append([]*node{v}, n.vals[i+1:]...)...)
			return
		}
	}
	n.keys, n.vals = append(n.keys, k), append(n.vals, v)
}

func (n *node) write(b *strings.Builder, ind string) {
	switch n.kind {
	case 's':
		b.Write(n.raw)
	case 'a':
		if len(n.items) == 0 {
			b.WriteString("[]")
			return
		}
		b.WriteString("[\n")
		for i, it := range n.items {
			b.WriteString(ind + "  ")
			it.write(b, ind+"  ")
			if i < len(n.items)-1 {
				b.WriteString(",")
			}
			b.WriteString("\n")
		}
		b.WriteString(ind + "]")
	case 'o':
		if len(n.keys) == 0 {
			b.WriteString("{}")
			return
		}
		b.WriteString("{\n")
		for i, k := range n.keys {
			b.WriteString(ind + "  ")
			b.Write(scalar(k).raw)
			b.WriteString(": ")
			n.vals[i].write(b, ind+"  ")
			if i < len(n.keys)-1 {
				b.WriteString(",")
			}
			b.WriteString("\n")
		}
		b.WriteString(ind + "}")
	}
}

var (
	posixHook = regexp.MustCompile(`^python3 "\$CLAUDE_PROJECT_DIR/([^"]+\.py)"$`)
	winHook   = regexp.MustCompile(`^\$\{CLAUDE_PROJECT_DIR\}/([^"]+\.py)$`)
)

// WindowsPython picks the hook interpreter for windows: `py -3` when the launcher is on PATH
// (only checked when running on windows), else `python` (python3.exe is usually absent there).
func WindowsPython() string {
	if osenv.IsWindows {
		if _, err := exec.LookPath("py"); err == nil {
			return "py -3"
		}
	}
	return "python"
}

// Windows converts the posix settings text to the windows form; py is the interpreter command
// line (e.g. "py -3", "python", "C:\Python312\python.exe"), split on spaces into command + args
// unless the whole string is an existing file (paths like C:\Program Files\...).
func Windows(src []byte, py string) ([]byte, error) {
	cfg, err := parse(src)
	if err != nil {
		return nil, err
	}
	pyf := strings.Fields(py)
	if st, err := os.Stat(py); err == nil && !st.IsDir() {
		pyf = []string{py}
	}
	if len(pyf) == 0 {
		pyf = []string{"python"}
	}
	if hooks := cfg.get("hooks"); hooks != nil {
		for _, groups := range hooks.vals {
			for _, g := range groups.items {
				hs := g.get("hooks")
				if hs == nil {
					continue
				}
				for _, h := range hs.items {
					script := ""
					if m := posixHook.FindStringSubmatch(h.get("command").str()); m != nil {
						script = m[1]
					} else if a := h.get("args"); a != nil && a.kind == 'a' && len(a.items) > 0 {
						// already windows form (install.ps1 re-run): re-target the interpreter
						if m := winHook.FindStringSubmatch(a.items[len(a.items)-1].str()); m != nil {
							script = m[1]
						}
					}
					if script == "" {
						continue
					}
					args := append(append([]string(nil), pyf[1:]...), "${CLAUDE_PROJECT_DIR}/"+script)
					h.insertAfter("", "command", scalar(pyf[0]))
					h.insertAfter("command", "args", scalar(args))
				}
			}
		}
	}
	if perms := cfg.get("permissions"); perms != nil {
		if allow := perms.get("allow"); allow != nil && allow.kind == 'a' {
			for _, it := range append([]*node(nil), allow.items...) {
				if s := it.str(); strings.HasPrefix(s, "Bash(bin/chaude ") {
					ps := "PowerShell(bin/chaude.exe " + strings.TrimPrefix(s, "Bash(bin/chaude ")
					if !hasStr(allow.items, ps) {
						allow.items = append(allow.items, scalar(ps))
					}
				}
			}
		}
	}
	var b strings.Builder
	cfg.write(&b, "")
	b.WriteString("\n")
	return []byte(b.String()), nil
}

func hasStr(items []*node, s string) bool {
	for _, it := range items {
		if it.str() == s {
			return true
		}
	}
	return false
}

// Main: gen-settings windows|mac|linux [--out FILE] [--src FILE] [--python CMD]  (default out: stdout).
func Main(args []string) int {
	fs := flag.NewFlagSet("gen-settings", flag.ContinueOnError)
	out := fs.String("out", "", "write here instead of stdout")
	py := fs.String("python", "", "windows hook interpreter (default: py -3 if found, else python)")
	exe, _ := os.Executable()
	defSrc := filepath.Join(filepath.Dir(exe), "..", ".claude", "settings.json")
	if r := os.Getenv("CLAUDE_PROJECT_DIR"); r != "" {
		defSrc = filepath.Join(r, ".claude", "settings.json")
	}
	src := fs.String("src", defSrc, "posix settings source")
	pos, err := lean.ParseInterspersed(fs, args)
	if err != nil {
		return 2
	}
	if len(pos) != 1 || (pos[0] != "windows" && pos[0] != "mac" && pos[0] != "linux") {
		fmt.Fprintln(os.Stderr, "usage: chaude gen-settings windows|mac|linux [--out FILE] [--src FILE] [--python CMD]")
		return 2
	}
	txt, err := os.ReadFile(*src)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	if pos[0] == "windows" {
		if *py == "" {
			*py = WindowsPython()
		}
		if txt, err = Windows(txt, *py); err != nil {
			fmt.Fprintln(os.Stderr, err)
			return 1
		}
	}
	if !json.Valid(txt) {
		fmt.Fprintln(os.Stderr, "generated settings is not valid JSON")
		return 1
	}
	if *out == "" {
		os.Stdout.Write(txt)
		return 0
	}
	if err := os.WriteFile(*out, txt, 0o644); err != nil {
		fmt.Fprintln(os.Stderr, err)
		return 1
	}
	return 0
}
