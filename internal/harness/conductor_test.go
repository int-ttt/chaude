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
	for _, bad := range []string{"", "model: gpt\ntask: x", "topic: new\nsize: small", "hello"} {
		if _, _, ok := parseRoute(bad, "opus"); ok {
			t.Errorf("accepted %q", bad)
		}
	}
}

// TestParseRouteSize: tier is normalized; a missing or unknown tier routes as sonnet (ok), topic and task are still required.
// A legacy size-only line maps small -> haiku, large -> sonnet and is marked defaulted. Size is derived from tier.
func TestParseRouteSize(t *testing.T) {
	for _, c := range []struct {
		name, in, tier, kind string
		defaulted            bool
	}{
		{"missing tier -> sonnet, kind other", "topic: new\ntask: x", "sonnet", "other", true},
		{"same topic, no tier -> sonnet", "topic: same\ntask: x", "sonnet", "other", true},
		{"legacy Small. size -> haiku", "topic: same\nsize: Small.\nkind: lookup\ntask: x", "haiku", "lookup", true},
		{"tier Haiku. -> haiku", "topic: same\ntier: Haiku.\nkind: lookup\ntask: x", "haiku", "lookup", false},
		{"tier medium -> sonnet", "topic: same\ntier: medium\nkind: other\ntask: x", "sonnet", "other", true},
		{"tier opus", "topic: same\ntier: opus\nkind: other\ntask: x", "opus", "other", false},
		{"tier wins over legacy size", "topic: same\ntier: sonnet\nsize: small\ntask: x", "sonnet", "other", false},
	} {
		rr, ok := parseRouteFull(c.in, "haiku", nil)
		wantSize := "large"
		if c.tier == "haiku" {
			wantSize = "small"
		}
		if !ok || rr.Tier != c.tier || rr.Size != wantSize || rr.SizeDefaulted != c.defaulted || rr.Kind != c.kind {
			t.Errorf("%s: tier=%q size=%q defaulted=%v kind=%q ok=%v", c.name, rr.Tier, rr.Size, rr.SizeDefaulted, rr.Kind, ok)
		}
	}
	for _, bad := range []string{"size: small\ntask: x", "topic: same\nsize: small"} {
		if _, ok := parseRouteFull(bad, "haiku", nil); ok {
			t.Errorf("accepted %q", bad)
		}
	}
	// forced opus main, topic new, tier missing (sonnet): stays opus and is not delegated
	rr, ok := parseRouteFull("topic: new\ntask: x", "opus", nil)
	if !ok || rr.Model != "opus" || strings.HasPrefix(rr.Task, delegateHint) {
		t.Errorf("forced opus, missing tier: model=%q delegated=%v ok=%v", rr.Model, strings.HasPrefix(rr.Task, delegateHint), ok)
	}
	// haiku main, tier missing (sonnet): sonnet hint is attached, no plan workflow
	rr, ok = parseRouteFull("topic: same\ntask: x", "haiku", nil)
	if !ok || !strings.HasPrefix(rr.Task, sonnetHint) || strings.HasPrefix(rr.Task, planHint) {
		t.Errorf("haiku main, missing tier: sonnetHint missing or planHint present, ok=%v", ok)
	}
}

func TestClassifyPromptSizeRule(t *testing.T) {
	if !strings.Contains(classifyPromptSonnet, "tier: haiku|sonnet|opus") || !strings.Contains(classifyPromptSonnet, "when unsure between haiku and sonnet, or sonnet and opus, sonnet") {
		t.Error("classifier prompt must keep tier: haiku|sonnet|opus and the unsure rule")
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
		{"same haiku -> haiku", "topic: same\ntier: haiku\ntask: fix typo", "haiku", "haiku", false},
		{"empty cur -> haiku", "topic: same\ntier: haiku\ntask: x", "", "haiku", false},
		{"unrelated haiku on haiku -> haiku, no hint", "topic: new\ntier: haiku\ntask: run tests", "haiku", "haiku", false},
		{"forced opus unrelated haiku -> delegate", "topic: new\ntier: haiku\ntask: run tests", "opus", "opus", true},
		{"forced opus same opus tier stays", "topic: same\ntier: opus\ntask: x", "opus", "opus", false},
		{"forced opus same sonnet tier stays, no hint", "topic: same\ntier: sonnet\ntask: analyze", "opus", "opus", false},
		{"forced sonnet stays", "topic: new\ntier: opus\ntask: redesign", "sonnet", "sonnet", false},
	} {
		m, task, ok := parseRoute(c.in, c.cur)
		if !ok || m != c.want || strings.HasPrefix(task, delegateHint) != c.hint {
			t.Errorf("%s: got %q hint=%v ok=%v", c.name, m, strings.HasPrefix(task, delegateHint), ok)
		}
		if c.want != "haiku" && (strings.HasPrefix(task, sonnetHint) || strings.HasPrefix(task, planHint)) {
			t.Errorf("%s: forced main must not get sonnet/plan hint", c.name)
		}
	}
	if routerModelDefault != "haiku" {
		t.Error("classifier default model must be haiku")
	}
	if !strings.Contains(classifyPromptSonnet, "topic: same|new") || !strings.Contains(classifyPromptSonnet, "tier: haiku|sonnet|opus") {
		t.Error("classifier prompt must ask topic and tier")
	}
}

// TestHaikuPlanWorkflow: the main session is Haiku for every prompt; opus-tier tasks carry the
// plan -> Opus review -> Haiku execute workflow hint, sonnet-tier tasks carry the sonnet-analyst hint, haiku-tier neither.
func TestHaikuPlanWorkflow(t *testing.T) {
	cands := []haikuWorker{{ID: "w-readme", Task: "README foo->bar", Files: []string{"README.md"}}}
	for _, c := range []struct {
		name, in, cur, worker string
		plan, sonnet          bool
	}{
		{"lookup", "topic: new\ntier: haiku\nkind: lookup\nworker: new\ntask: find X", "haiku", "", false, false},
		{"edit matching worker", "topic: new\ntier: haiku\nkind: edit\nworker: w-readme\ntask: replace b", "haiku", "w-readme", false, false},
		{"unknown worker -> fresh", "topic: new\ntier: haiku\nkind: edit\nworker: w-ghost\ntask: replace b", "haiku", "", false, false},
		{"opus tier -> haiku + plan workflow", "topic: same\ntier: opus\nkind: other\nworker: new\ntask: weigh A vs B", "haiku", "", true, false},
		{"sonnet tier -> haiku + sonnet hint, no plan", "topic: same\ntier: sonnet\nkind: other\nworker: new\ntask: review diff", "haiku", "", false, true},
		{"opus tier from stale opus cur stays opus (forced)", "topic: new\ntier: opus\nkind: lookup\ntask: roadmap", "opus", "", false, false},
		{"opus tier from empty cur -> haiku + plan", "topic: new\ntier: opus\nkind: other\ntask: design", "", "", true, false},
		{"haiku other -> haiku, no plan", "topic: same\ntier: haiku\nkind: other\ntask: pushback", "haiku", "", false, false},
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
		if has := strings.HasPrefix(rr.Task, sonnetHint); has != c.sonnet {
			t.Errorf("%s: sonnetHint=%v want %v", c.name, has, c.sonnet)
		}
	}
	for _, w := range []string{"opus-architect", "notes/runs/", "Haiku agents"} {
		if !strings.Contains(planHint, w) {
			t.Errorf("planHint missing %q", w)
		}
	}
}

// TestSonnetTierHint: a sonnet tier on a Haiku main gets only the sonnet-analyst hint (no plan step);
// forced opus/sonnet mains and haiku tier get no hint.
func TestSonnetTierHint(t *testing.T) {
	for _, w := range []string{"sonnet-analyst", "ESCALATE", "opus-architect"} {
		if !strings.Contains(sonnetHint, w) {
			t.Errorf("sonnetHint missing %q", w)
		}
	}
	for _, w := range []string{"plan-<topic>", "write the full design"} {
		if strings.Contains(sonnetHint, w) {
			t.Errorf("sonnetHint must not contain %q", w)
		}
	}
	for _, c := range []struct {
		name, in, cur, want string
		sonnet, plan        bool
	}{
		{"sonnet tier on haiku main", "topic: same\ntier: sonnet\ntask: analyze X", "haiku", "haiku", true, false},
		{"sonnet tier on forced opus main", "topic: same\ntier: sonnet\ntask: analyze X", "opus", "opus", false, false},
		{"sonnet tier on forced sonnet main", "topic: same\ntier: sonnet\ntask: analyze X", "sonnet", "sonnet", false, false},
		{"haiku tier on haiku main", "topic: same\ntier: haiku\ntask: analyze X", "haiku", "haiku", false, false},
	} {
		rr, ok := parseRouteFull(c.in, c.cur, nil)
		if !ok || rr.Model != c.want {
			t.Errorf("%s: model=%q ok=%v", c.name, rr.Model, ok)
		}
		if has := strings.HasPrefix(rr.Task, sonnetHint); has != c.sonnet {
			t.Errorf("%s: sonnetHint=%v want %v", c.name, has, c.sonnet)
		}
		if has := strings.HasPrefix(rr.Task, planHint); has != c.plan {
			t.Errorf("%s: planHint=%v want %v", c.name, has, c.plan)
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
