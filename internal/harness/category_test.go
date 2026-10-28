package harness

import "testing"

func TestParseCategory(t *testing.T) {
	c, sw := parseCategory("답변입니다\n\n[cat-switch: resume]\n")
	if c != "resume" || !sw {
		t.Fatalf("got %q %v", c, sw)
	}
	c, sw = parseCategory("x\n[cat: chaude]")
	if c != "chaude" || sw {
		t.Fatalf("got %q %v", c, sw)
	}
	if c, _ = parseCategory("no marker"); c != "" {
		t.Fatal(c)
	}
}

func TestCategoryRotation(t *testing.T) {
	reply := "done\n[cat-switch: resume]"
	if cat, rot := categoryRotation("이력서 써줘", reply, "chaude", false); !rot || cat != "resume" {
		t.Fatalf("switch should rotate: %q %v", cat, rot)
	}
	if _, rot := categoryRotation("ㅇㅇ", reply, "chaude", false); rot {
		t.Fatal("ack must not rotate")
	}
	if _, rot := categoryRotation("이력서 써줘", reply, "chaude", true); rot {
		t.Fatal("disabled must not rotate")
	}
	if _, rot := categoryRotation("계속 해", "ok\n[cat: chaude]", "chaude", false); rot {
		t.Fatal("same category must not rotate")
	}
	if _, rot := categoryRotation("이력서 써줘", reply, "resume", false); rot {
		t.Fatal("same label must not rotate")
	}
}

func TestCategoryRuleSize(t *testing.T) {
	if n := len(categoryRule); n > 1500 { // ~500 tokens upper bound
		t.Fatalf("rule too long: %d bytes", n)
	}
}
