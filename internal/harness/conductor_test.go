package harness

import (
	"os"
	"strings"
	"testing"
)

func TestParseRoute(t *testing.T) {
	m, task, ok := parseRoute("model: Haiku\ntask: count funcs in a.go\nreply: Korean 존댓말")
	if !ok || m != "haiku" || task != "count funcs in a.go\nreply: Korean 존댓말" {
		t.Fatal(m, task, ok)
	}
	for _, bad := range []string{"", "model: gpt\ntask: x", "model: opus", "hello"} {
		if _, _, ok := parseRoute(bad); ok {
			t.Errorf("accepted %q", bad)
		}
	}
}

func TestForcedModel(t *testing.T) {
	cases := map[string]string{
		"fable로 세션 회전 설계를 다시 잡아줘": "fable",
		"이건 opus로 해줘":             "opus",
		"haiku로.":                 "haiku",
		"Sonnet으로 정리해":            "sonnet",
		"페이블로 해봐":                 "fable",
		"오퍼스로 보내":                 "opus",
		"haiku로 보고 어려우면 opus로 넘겨": "opus",
		"README의 로그인 설명을 고쳐":      "",
		"opus로그를 지워":              "",
		"어느 쪽이 나을지 골라줘":           "",
	}
	for in, want := range cases {
		if got := forcedModel(in); got != want {
			t.Errorf("forcedModel(%q)=%q want %q", in, got, want)
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
		{"cmd/chaude/main.go 의 version 값이 뭔지 알려줘", "haiku,sonnet"},
		{"README.md 에서 'foo' 를 'bar' 로 전부 바꿔줘", "haiku,sonnet"},
		{"fable로 세션 회전 설계를 다시 잡아줘. opus가 두 번 실패했어", "fable"},
		{"세션 회전 설계를 다시 잡아줘. 어느 쪽이 나을지 골라줘", "opus"}, // never fable without being asked
	}
	for _, c := range cases {
		m, task, _, _, err := classify(c.prompt, "haiku", "")
		if err != nil || !strings.Contains(c.want, m) {
			t.Errorf("%q -> %q (%v), want %s", c.prompt, m, err, c.want)
		}
		t.Logf("%s -> %s | %s", c.prompt[:12], m, task)
	}
}
