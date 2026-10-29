package harness

import (
	"os"
	"strings"
	"testing"
)

func TestParseRoute(t *testing.T) {
	m, task, ok := parseRoute("topic: New\nsize: Small\ntask: count funcs in a.go\nreply: Korean 존댓말", "opus")
	if !ok || m != "opus" || task != delegateHint+"count funcs in a.go\nreply: Korean 존댓말" { // forced opus main keeps delegation
		t.Fatal(m, task, ok)
	}
	for _, bad := range []string{"", "model: gpt\ntask: x", "topic: same\ntask: x", "topic: new\nsize: small", "hello"} {
		if _, _, ok := parseRoute(bad, "opus"); ok {
			t.Errorf("accepted %q", bad)
		}
	}
}

// TestClassifierLive runs the real classifier (costs ~$0.007 per prompt): CHAUDE_LIVE=1 go test -run ClassifierLive ./internal/harness
func TestClassifierLive(t *testing.T) {
	if os.Getenv("CHAUDE_LIVE") == "" {
		t.Skip("set CHAUDE_LIVE=1")
	}
	cases := []struct{ prompt, want string }{
		{"캐시 TTL을 1시간으로 고정하는 안과 5분으로 두는 안 중에 어느 쪽이 나을지 장단점 비교해서 골라줘", "haiku"},
		{"아니 그게 아니라 내가 말한 건 세션 회전이 아니라 카테고리 분류였어. 다시 생각해봐", "haiku"},
		{"방금 수정한 category.go 의 isAck 로직이 틀렸어. 두 글자 이하 문장이 왜 ack 처리되는지 고쳐야 해", "haiku"},
		{"앞으로 한 달 동안 하네스 개선 우선순위를 어떻게 잡는 게 좋을지 전략을 세워줘", "haiku"},
		{"cmd/chaude/main.go 의 version 값이 뭔지 알려줘", "haiku"},
		{"README.md 에서 'foo' 를 'bar' 로 전부 바꿔줘", "haiku"},
	}
	for _, c := range cases {
		rr, _, _, err := classify(c.prompt, "haiku", "", nil)
		m, task := rr.Model, rr.Task
		if err != nil || !strings.Contains(c.want, m) {
			t.Errorf("%q -> %q (%v), want %s", c.prompt, m, err, c.want)
		}
		t.Logf("%s -> %s | %s", c.prompt[:12], m, task)
	}
}

func TestMainRouteSwitchPolicy(t *testing.T) {
	for _, c := range []struct {
		name, in, cur, want string
		hint                bool
	}{
		{"same small -> haiku", "topic: same\nsize: small\ntask: fix typo", "haiku", "haiku", false},
		{"empty cur -> haiku", "topic: same\nsize: small\ntask: x", "", "haiku", false},
		{"unrelated small on haiku -> haiku, no hint", "topic: new\nsize: small\ntask: run tests", "haiku", "haiku", false},
		{"forced opus unrelated small -> delegate", "topic: new\nsize: small\ntask: run tests", "opus", "opus", true},
		{"forced opus same large stays", "topic: same\nsize: large\ntask: x", "opus", "opus", false},
		{"forced sonnet stays", "topic: new\nsize: large\ntask: redesign", "sonnet", "sonnet", false},
	} {
		m, task, ok := parseRoute(c.in, c.cur)
		if !ok || m != c.want || strings.HasPrefix(task, delegateHint) != c.hint {
			t.Errorf("%s: got %q hint=%v ok=%v", c.name, m, strings.HasPrefix(task, delegateHint), ok)
		}
	}
	if conductorModel != "sonnet" {
		t.Error("classifier model must be fixed to sonnet")
	}
	if !strings.Contains(classifyPrompt, "topic: same|new") || !strings.Contains(classifyPrompt, "size: small|large") {
		t.Error("classifier prompt must ask topic and size")
	}
}

// TestHaikuPlanWorkflow: the main session is Haiku for every prompt; large/judgment tasks carry the
// plan -> Opus/Sonnet review -> Haiku execute workflow hint, small ones do not.
func TestHaikuPlanWorkflow(t *testing.T) {
	cands := []haikuWorker{{ID: "w-readme", Task: "README foo->bar", Files: []string{"README.md"}}}
	for _, c := range []struct {
		name, in, cur, worker string
		plan                  bool
	}{
		{"lookup", "topic: new\nsize: small\nkind: lookup\nworker: new\ntask: find X", "haiku", "", false},
		{"edit matching worker", "topic: new\nsize: small\nkind: edit\nworker: w-readme\ntask: replace b", "haiku", "w-readme", false},
		{"unknown worker -> fresh", "topic: new\nsize: small\nkind: edit\nworker: w-ghost\ntask: replace b", "haiku", "", false},
		{"judgment large -> haiku + plan workflow", "topic: same\nsize: large\nkind: other\nworker: new\ntask: weigh A vs B", "haiku", "", true},
		{"strategy from stale opus cur stays opus (forced)", "topic: new\nsize: large\nkind: lookup\ntask: roadmap", "opus", "", false},
		{"large from empty cur -> haiku + plan", "topic: new\nsize: large\nkind: other\ntask: design", "", "", true},
		{"small other -> haiku, no plan", "topic: same\nsize: small\nkind: other\ntask: pushback", "haiku", "", false},
	} {
		rr, ok := parseRouteFull(c.in, c.cur, cands)
		want := "haiku"
		if c.cur == "opus" {
			want = "opus"
		}
		if !ok || rr.Model != want || rr.Worker != c.worker {
			t.Errorf("%s: got model=%q worker=%q ok=%v", c.name, rr.Model, rr.Worker, ok)
		}
		if has := strings.HasPrefix(rr.Task, planHint); has != c.plan {
			t.Errorf("%s: planHint=%v want %v", c.name, has, c.plan)
		}
	}
	for _, w := range []string{"opus-architect", "sonnet-analyst", "notes/runs/", "Haiku agents"} {
		if !strings.Contains(planHint, w) {
			t.Errorf("planHint missing %q", w)
		}
	}
}

func TestWorkerPool(t *testing.T) {
	p := newWorkerPool()
	p.noteEdit("", "t", "a.go")
	p.noteEdit("s1", "task one", "a.go")
	p.noteEdit("s1", "task one", "a.go")
	p.noteEdit("s2", "task two", "b.go")
	c := p.candidates()
	if len(c) != 2 || c[0].ID != "s2" || len(c[1].Files) != 1 || !p.has("s1") || p.has("") {
		t.Fatalf("%+v", c)
	}
	var np *workerPool
	np.noteEdit("x", "t", "f")
	if np.candidates() != nil {
		t.Fatal("nil pool")
	}
	if !strings.Contains(candidateText(c), "s2 | task: task two | edited: b.go") {
		t.Fatal(candidateText(c))
	}
}
