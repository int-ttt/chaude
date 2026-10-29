package launch

import (
	"os"
	"path/filepath"
	"testing"
)

func TestSplitOpenArgs(t *testing.T) {
	cases := []struct {
		in   []string
		dir  string
		rest int
	}{
		{nil, ".", 0},
		{[]string{"C:\\proj"}, "C:\\proj", 0},
		{[]string{"C:\\proj", "--model", "fable"}, "C:\\proj", 2},
		{[]string{"--model", "fable"}, ".", 2},
	}
	for _, c := range cases {
		d, r := SplitOpenArgs(c.in)
		if d != c.dir || len(r) != c.rest {
			t.Errorf("SplitOpenArgs(%v) = %q, %v", c.in, d, r)
		}
	}
}

func TestInstalled(t *testing.T) {
	dir := t.TempDir()
	if Installed(dir) {
		t.Fatal("empty dir reported installed")
	}
	if err := os.MkdirAll(filepath.Join(dir, ".claude"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, ".claude", "claude-lean.version"), []byte("1\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if !Installed(dir) {
		t.Fatal("version marker not detected")
	}
}
