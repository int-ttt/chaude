package harness

import (
	"path/filepath"
	"testing"
	"time"
)

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

func TestRegistryNilWhenNoFile(t *testing.T) {
	r := &Repl{a: Args{CacheTTL: "5m", IdleRotate: time.Minute}}
	r.wireClients()
	if r.reg != nil || r.children != nil {
		t.Fatalf("reg=%v children=%v; want both nil with empty ClientsFile", r.reg, r.children)
	}
	if r.ttl != "5m" || r.idleRotate != time.Minute {
		t.Fatalf("ttl=%q idleRotate=%v; want 5m, 1m", r.ttl, r.idleRotate)
	}
	r = &Repl{a: Args{ClientsFile: filepath.Join(t.TempDir(), "clients.json")}}
	r.wireClients()
	if r.reg == nil || r.children == nil {
		t.Fatalf("reg=%v children=%v; want both set with ClientsFile", r.reg, r.children)
	}
}
