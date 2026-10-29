package settings

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestModelGetSet(t *testing.T) {
	src := []byte("{\n  \"model\": \"opus\",\n  \"effortLevel\": \"low\",\n  \"env\": {\"A\": \"1\"},\n  \"hooks\": {\"Stop\": [{\"hooks\": [{\"type\": \"command\", \"command\": \"echo \\\"$X\\\"\"}]}]}\n}\n")
	if m := Model(src); m != "opus" {
		t.Fatalf("Model = %q", m)
	}
	out, err := SetModel(src, "fable")
	if err != nil {
		t.Fatal(err)
	}
	if Model(out) != "fable" || !json.Valid(out) || !strings.HasPrefix(string(out), "{\n  \"model\": \"fable\",\n  \"effortLevel\": \"low\"") {
		t.Fatalf("SetModel output:\n%s", out)
	}
	if !strings.Contains(string(out), `"command": "echo \"$X\""`) {
		t.Fatalf("other keys changed:\n%s", out)
	}
	// missing key: inserted first
	out, err = SetModel([]byte(`{"effortLevel": "low"}`), "opus")
	if err != nil || !strings.HasPrefix(string(out), "{\n  \"model\": \"opus\",\n  \"effortLevel\"") {
		t.Fatalf("insert first: %v\n%s", err, out)
	}
	if Model([]byte("not json")) != "" {
		t.Fatal("garbage must give empty model")
	}
	if _, err := SetModel([]byte("[1]"), "opus"); err == nil {
		t.Fatal("array must be rejected")
	}
}
