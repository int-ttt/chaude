package harness

import "testing"

func TestParseCacheTTL(t *testing.T) {
	for _, s := range []string{"auto", "1h", "5m"} {
		got, err := parseCacheTTL(s)
		if err != nil || got != s {
			t.Fatalf("parseCacheTTL(%q) = %q, %v; want %q, nil", s, got, err, s)
		}
	}
	for _, s := range []string{"", "10m", "1H", "AUTO"} {
		if got, err := parseCacheTTL(s); err == nil {
			t.Fatalf("parseCacheTTL(%q) = %q, nil; want error", s, got)
		}
	}
}
