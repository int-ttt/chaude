package harness

import "testing"

func TestMdBold(t *testing.T) {
	const on, off = "\x1b[1m", "\x1b[22m"
	cases := []struct{ in, want string }{
		{"**가**나", on + "가" + off + "나"},
		{"**가", "**가"},
		{"```\n**가**\n```", "```\n**가**\n```"},
		{"**a** x **b", on + "a" + off + " x **b"},
	}
	for _, c := range cases {
		if got := mdBold(c.in); got != c.want {
			t.Errorf("mdBold(%q)=%q want %q", c.in, got, c.want)
		}
	}
}
