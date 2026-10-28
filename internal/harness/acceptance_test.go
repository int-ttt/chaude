package harness

import (
	"bufio"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// Live acceptance (needs logged-in `claude`): HARNESS_LIVE=1 go test ./internal/harness -run . -v
func liveOnly(t *testing.T) {
	if os.Getenv("HARNESS_LIVE") != "1" {
		t.Skip("set HARNESS_LIVE=1")
	}
}

func TestLiveRotationCarriesContext(t *testing.T) {
	liveOnly(t)
	root, _ := filepath.Abs(filepath.Join("..", ".."))
	bin := filepath.Join(t.TempDir(), "chaude")
	if out, err := exec.Command("go", "build", "-o", bin, "./cmd/chaude").CombinedOutput(); err != nil {
		// build must run from module root
		cmd := exec.Command("go", "build", "-o", bin, "./cmd/chaude")
		cmd.Dir = root
		if out, err = cmd.CombinedOutput(); err != nil {
			t.Fatalf("build: %v\n%s", err, out)
		}
	}
	tr := filepath.Join(t.TempDir(), "t.jsonl")
	script := "Remember this codeword: ZEBRA-77. Reply with just: stored.\nWhat was the codeword I asked you to remember? Reply with just the codeword.\n"
	cmd := exec.Command(bin, "repl", "--threshold", "100", "--max-auto-rotations", "1", "--transcript", tr)
	cmd.Dir = root
	cmd.Stdin = strings.NewReader(script)
	cmd.Env = append(os.Environ(), "ANTHROPIC_API_KEY=") // harness must strip it
	done := make(chan struct{})
	go func() { _ = cmd.Run(); close(done) }()
	select {
	case <-done:
	case <-time.After(300 * time.Second):
		_ = cmd.Process.Kill()
		t.Fatal("timeout")
	}
	f, err := os.Open(tr)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	var rot, asst []map[string]any
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	for sc.Scan() {
		var r map[string]any
		if err := json.Unmarshal(sc.Bytes(), &r); err != nil {
			t.Fatal(err)
		}
		switch r["kind"] {
		case "rotate":
			rot = append(rot, r)
		case "assistant":
			asst = append(asst, r)
		}
	}
	if len(rot) != 1 || len(asst) < 2 {
		t.Fatalf("rot=%d asst=%d", len(rot), len(asst))
	}
	if asst[0]["session"] == asst[1]["session"] {
		t.Error("session did not change")
	}
	if !strings.Contains(asst[1]["text"].(string), "ZEBRA-77") {
		t.Errorf("answer lacks codeword: %v", asst[1]["text"])
	}
	if !strings.Contains(rot[0]["snapshot"].(string), "ZEBRA-77") {
		t.Error("snapshot lacks codeword")
	}
}

func TestLiveStreamingMultipleDeltas(t *testing.T) {
	liveOnly(t)
	s, err := NewSession(SessionOpts{})
	if err != nil {
		t.Fatal(err)
	}
	defer s.Close()
	_ = s.Send("Count from 1 to 30, one per line. No tools.")
	n := 0
	s.Events(300*time.Second, nil, func(e event) {
		if e["type"] == "stream_event" {
			ev := asMap(e["event"])
			if ev["type"] == "content_block_delta" && asMap(ev["delta"])["type"] == "text_delta" {
				n++
			}
		}
	})
	if n <= 3 {
		t.Fatalf("deltas=%d", n)
	}
}
