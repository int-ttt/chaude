package harness

import "testing"

func TestWorkerPool(t *testing.T) {
	p := newWorkerPool()
	p.noteEdit("", "t", "a.go")
	p.noteEdit("s1", "task one", "a.go")
	p.noteEdit("s1", "task one", "a.go")
	p.noteEdit("s2", "task two", "b.go")
	if !p.has("s1") || !p.has("s2") || p.has("") || p.has("nope") {
		t.Fatalf("has: %+v", p.l)
	}
	if n := len(p.l["s1"].Files); n != 1 {
		t.Fatalf("s1 files=%d, want 1 (dedup)", n)
	}
	var np *workerPool
	np.noteEdit("x", "t", "f")
	if np.has("x") {
		t.Fatal("nil pool")
	}
}

// TestThresholdFor: opus (including full model IDs) rotates at opusThreshold; every other model
// (haiku, empty, unknown, and a sonnet value left in a saved session) uses the -threshold value.
func TestThresholdFor(t *testing.T) {
	if haikuThreshold != 80_000 || opusThreshold != 200_000 {
		t.Fatalf("haiku=%d opus=%d", haikuThreshold, opusThreshold)
	}
	for _, c := range []struct {
		model string
		flag  int
		want  int
	}{
		{"haiku", haikuThreshold, 80_000},
		{"opus", haikuThreshold, 200_000},
		{"claude-opus-5-5", haikuThreshold, 200_000},
		{"", haikuThreshold, 80_000},
		{"sonnet", haikuThreshold, 80_000},
		{"sonnet", 150_000, 150_000},
	} {
		r := &Repl{a: Args{Model: c.model, Threshold: c.flag}}
		if got := r.thresholdFor(c.model); got != c.want {
			t.Errorf("thresholdFor(%q) flag=%d = %d, want %d", c.model, c.flag, got, c.want)
		}
	}
}

// TestShouldRotateBoundary: rotation fires at the threshold itself (>=) for haiku and opus.
func TestShouldRotateBoundary(t *testing.T) {
	haiku := &Repl{a: Args{Model: "haiku", Threshold: haikuThreshold}}
	opus := &Repl{a: Args{Model: "opus", Threshold: haikuThreshold}}
	for _, c := range []struct {
		r    *Repl
		ctx  int
		want bool
	}{
		{haiku, 79_999, false},
		{haiku, 80_000, true},
		{opus, 199_999, false},
		{opus, 200_000, true},
	} {
		if got := shouldRotate(c.ctx, c.r.effectiveThreshold()); got != c.want {
			t.Errorf("model=%s ctx=%d shouldRotate=%v, want %v", c.r.a.Model, c.ctx, got, c.want)
		}
	}
}
