package harness

import (
	"os"
	"strings"
	"testing"
)

func TestParseRoute(t *testing.T) {
	m, task, ok := parseRoute("topic: New\nsize: Small\ntask: count funcs in a.go\nreply: Korean 존댓말", "opus")
	if !ok || m != "opus" || task != delegateHint+"count funcs in a.go\nreply: Korean 존댓말" {
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
		{"캐시 TTL을 1시간으로 고정하는 안과 5분으로 두는 안 중에 어느 쪽이 나을지 장단점 비교해서 골라줘", "opus"},
		{"아니 그게 아니라 내가 말한 건 세션 회전이 아니라 카테고리 분류였어. 다시 생각해봐", "opus"},
		{"방금 수정한 category.go 의 isAck 로직이 틀렸어. 두 글자 이하 문장이 왜 ack 처리되는지 고쳐야 해", "opus"},
		{"앞으로 한 달 동안 하네스 개선 우선순위를 어떻게 잡는 게 좋을지 전략을 세워줘", "opus"},
		{"cmd/chaude/main.go 의 version 값이 뭔지 알려줘", "opus"},
		{"README.md 에서 'foo' 를 'bar' 로 전부 바꿔줘", "opus"},
	}
	for _, c := range cases {
		m, task, _, _, err := classify(c.prompt, "opus", "")
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
		{"same small stays opus, no delegation", "topic: same\nsize: small\ntask: fix typo", "opus", "opus", false},
		{"unrelated small on opus -> delegate", "topic: new\nsize: small\ntask: run tests", "opus", "opus", true},
		{"unrelated large on opus -> main", "topic: new\nsize: large\ntask: redesign", "opus", "opus", false},
		{"same large stays opus", "topic: same\nsize: large\ntask: x", "opus", "opus", false},
		{"empty cur -> opus", "topic: same\nsize: small\ntask: x", "", "opus", false},
		{"sonnet main same large stays", "topic: same\nsize: large\ntask: x", "sonnet", "sonnet", false},
		{"sonnet main new small stays, no hint", "topic: new\nsize: small\ntask: x", "sonnet", "sonnet", false},
		{"sonnet main new large -> opus", "topic: new\nsize: large\ntask: redesign A/B/C", "sonnet", "opus", false},
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
