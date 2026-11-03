package update

import (
	"archive/tar"
	"archive/zip"
	"bytes"
	"compress/gzip"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// fakeGitHub serves the latest-release JSON and the asset bytes, like api.github.com does (no real network).
type fakeGitHub struct {
	srv    *httptest.Server
	tag    string
	assets map[string][]byte   // asset name -> bytes (checksums.txt included when present)
	omit   map[string]bool     // asset names left out of the release JSON
	token  string              // when set, requests with another Authorization get 401 on the release read
	hits   map[string]int      // asset name -> download count
	authOn map[string][]string // asset name -> Authorization headers seen
}

func newFakeGitHub(t *testing.T, tag string) *fakeGitHub {
	t.Helper()
	fg := &fakeGitHub{tag: tag, assets: map[string][]byte{}, omit: map[string]bool{}, hits: map[string]int{}, authOn: map[string][]string{}}
	fg.srv = httptest.NewServer(http.HandlerFunc(fg.serve))
	t.Cleanup(fg.srv.Close)
	return fg
}

func (fg *fakeGitHub) serve(w http.ResponseWriter, r *http.Request) {
	switch {
	case r.URL.Path == "/repos/int-ttt/chaude/releases/latest":
		if auth := r.Header.Get("Authorization"); fg.token != "" && auth != "" && auth != "Bearer "+fg.token {
			http.Error(w, "bad credentials", http.StatusUnauthorized)
			return
		}
		type asset struct {
			Name string `json:"name"`
			URL  string `json:"url"`
		}
		rel := struct {
			Tag    string  `json:"tag_name"`
			Assets []asset `json:"assets"`
		}{Tag: fg.tag}
		for name := range fg.assets {
			if !fg.omit[name] {
				rel.Assets = append(rel.Assets, asset{Name: name, URL: fg.srv.URL + "/assets/" + name})
			}
		}
		_ = json.NewEncoder(w).Encode(rel)
	case strings.HasPrefix(r.URL.Path, "/assets/"):
		name := strings.TrimPrefix(r.URL.Path, "/assets/")
		data, ok := fg.assets[name]
		if !ok {
			http.NotFound(w, r)
			return
		}
		fg.hits[name]++
		fg.authOn[name] = append(fg.authOn[name], r.Header.Get("Authorization"))
		_, _ = w.Write(data)
	default:
		http.NotFound(w, r)
	}
}

func (fg *fakeGitHub) options(exe, goos, goarch string) Options {
	return Options{APIBase: fg.srv.URL, Repo: "int-ttt/chaude", GOOS: goos, GOARCH: goarch, Exe: exe, Client: fg.srv.Client()}
}

func tarGzWith(t *testing.T, name string, content []byte) []byte {
	t.Helper()
	var buf bytes.Buffer
	gz := gzip.NewWriter(&buf)
	tw := tar.NewWriter(gz)
	if err := tw.WriteHeader(&tar.Header{Name: name, Mode: 0o755, Size: int64(len(content)), Typeflag: tar.TypeReg}); err != nil {
		t.Fatal(err)
	}
	if _, err := tw.Write(content); err != nil {
		t.Fatal(err)
	}
	if err := tw.Close(); err != nil {
		t.Fatal(err)
	}
	if err := gz.Close(); err != nil {
		t.Fatal(err)
	}
	return buf.Bytes()
}

func zipWith(t *testing.T, name string, content []byte) []byte {
	t.Helper()
	var buf bytes.Buffer
	zw := zip.NewWriter(&buf)
	w, err := zw.Create(name)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := w.Write(content); err != nil {
		t.Fatal(err)
	}
	if err := zw.Close(); err != nil {
		t.Fatal(err)
	}
	return buf.Bytes()
}

func sumLine(data []byte, name string) string {
	h := sha256.Sum256(data)
	return fmt.Sprintf("%s  %s\n", hex.EncodeToString(h[:]), name)
}

// writeExe creates the "running" binary with old contents in a fresh dir and returns its path.
func writeExe(t *testing.T, name string) string {
	t.Helper()
	exe := filepath.Join(t.TempDir(), name)
	if err := os.WriteFile(exe, []byte("OLD"), 0o755); err != nil {
		t.Fatal(err)
	}
	return exe
}

func readFile(t *testing.T, p string) string {
	t.Helper()
	b, err := os.ReadFile(p)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

func listDir(t *testing.T, dir string) []string {
	t.Helper()
	ents, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	var names []string
	for _, e := range ents {
		names = append(names, e.Name())
	}
	return names
}

func TestCompareVersion(t *testing.T) {
	cases := []struct {
		a, b string
		want int
	}{
		{"0.9.13", "v0.9.14", -1},
		{"v0.9.14", "0.9.13", 1},
		{"0.9.13", "0.9.13", 0},
		{"0.10.0", "0.9.99", 1},
		{"1.0.0", "0.99.99", 1},
		{"0.9.13-rc1", "v0.9.13", 0}, // pre-release suffix ignored for ordering
	}
	for _, c := range cases {
		a, ok1 := parseVersion(c.a)
		b, ok2 := parseVersion(c.b)
		if !ok1 || !ok2 {
			t.Fatalf("parseVersion(%q, %q) failed", c.a, c.b)
		}
		if got := compareVersion(a, b); got != c.want {
			t.Errorf("compare(%s, %s) = %d, want %d", c.a, c.b, got, c.want)
		}
	}
	if _, ok := parseVersion("unknown"); ok {
		t.Error(`parseVersion("unknown") should not parse`)
	}
}

func TestUpToDateDownloadsNothing(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.13")
	fg.assets["chaude_0.9.13_linux_amd64.tar.gz"] = []byte("x")
	fg.assets["checksums.txt"] = []byte("x")
	exe := writeExe(t, "chaude")

	msg, err := Run(context.Background(), "0.9.13", fg.options(exe, "linux", "amd64"))
	if err != nil {
		t.Fatalf("Run: %v", err)
	}
	if !strings.Contains(msg, "up to date") || !strings.Contains(msg, "0.9.13") {
		t.Errorf("message = %q", msg)
	}
	if len(fg.hits) != 0 {
		t.Errorf("assets downloaded while up to date: %v", fg.hits)
	}
	if readFile(t, exe) != "OLD" {
		t.Error("binary changed while up to date")
	}
}

func TestUpdateUnixReplacesBinaryAtomically(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	archive := tarGzWith(t, "chaude", []byte("NEW"))
	name := "chaude_0.9.14_linux_amd64.tar.gz"
	fg.assets[name] = archive
	fg.assets["checksums.txt"] = []byte(sumLine(archive, name))
	exe := writeExe(t, "chaude")

	msg, err := Run(context.Background(), "0.9.13", fg.options(exe, "linux", "amd64"))
	if err != nil {
		t.Fatalf("Run: %v", err)
	}
	if !strings.Contains(msg, "0.9.13 -> 0.9.14") || !strings.Contains(msg, "Restart chaude to use it") {
		t.Errorf("message = %q", msg)
	}
	if got := readFile(t, exe); got != "NEW" {
		t.Errorf("binary = %q, want NEW", got)
	}
	st, err := os.Stat(exe)
	if err != nil {
		t.Fatal(err)
	}
	if st.Mode().Perm() != 0o755 {
		t.Errorf("mode = %v, want 0755", st.Mode().Perm())
	}
	if names := listDir(t, filepath.Dir(exe)); len(names) != 1 || names[0] != "chaude" {
		t.Errorf("leftover files next to binary: %v", names)
	}
}

func TestUpdateWindowsRenamesOldAndCleanupRemovesIt(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	archive := zipWith(t, "chaude.exe", []byte("NEW"))
	name := "chaude_0.9.14_windows_amd64.zip"
	fg.assets[name] = archive
	fg.assets["checksums.txt"] = []byte(sumLine(archive, name))
	exe := writeExe(t, "chaude.exe")

	if _, err := Run(context.Background(), "v0.9.13", fg.options(exe, "windows", "amd64")); err != nil {
		t.Fatalf("Run: %v", err)
	}
	if got := readFile(t, exe); got != "NEW" {
		t.Errorf("chaude.exe = %q, want NEW", got)
	}
	if got := readFile(t, exe+".old"); got != "OLD" {
		t.Errorf("chaude.exe.old = %q, want OLD", got)
	}
	removeOld(exe)
	if _, err := os.Stat(exe + ".old"); !os.IsNotExist(err) {
		t.Errorf("chaude.exe.old still present after cleanup: %v", err)
	}
}

func TestChecksumMismatchInstallsNothing(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	archive := tarGzWith(t, "chaude", []byte("NEW"))
	name := "chaude_0.9.14_linux_amd64.tar.gz"
	fg.assets[name] = archive
	fg.assets["checksums.txt"] = []byte(sumLine([]byte("tampered"), name))
	exe := writeExe(t, "chaude")

	_, err := Run(context.Background(), "0.9.13", fg.options(exe, "linux", "amd64"))
	if err == nil || !strings.Contains(err.Error(), "checksum mismatch") {
		t.Fatalf("err = %v, want checksum mismatch", err)
	}
	if readFile(t, exe) != "OLD" {
		t.Error("binary replaced despite checksum mismatch")
	}
	if names := listDir(t, filepath.Dir(exe)); len(names) != 1 {
		t.Errorf("leftover files after failure: %v", names)
	}
}

func TestChecksumsRequired(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	name := "chaude_0.9.14_linux_amd64.tar.gz"
	fg.assets[name] = tarGzWith(t, "chaude", []byte("NEW"))
	exe := writeExe(t, "chaude")

	_, err := Run(context.Background(), "0.9.13", fg.options(exe, "linux", "amd64"))
	if err == nil || !strings.Contains(err.Error(), "checksums.txt not found") {
		t.Fatalf("err = %v, want checksums.txt not found", err)
	}
	if readFile(t, exe) != "OLD" {
		t.Error("binary replaced without checksums")
	}
}

func TestNoAssetForPlatform(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	fg.assets["checksums.txt"] = []byte("x")
	exe := writeExe(t, "chaude")

	_, err := Run(context.Background(), "0.9.13", fg.options(exe, "linux", "riscv64"))
	if err == nil || !strings.Contains(err.Error(), "no prebuilt binary") {
		t.Fatalf("err = %v, want no prebuilt binary", err)
	}
}

func TestOfflineIsClearError(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	o := fg.options(writeExe(t, "chaude"), "linux", "amd64")
	fg.srv.Close() // nothing listens any more: transport error

	_, err := Run(context.Background(), "0.9.13", o)
	if !errors.Is(err, ErrOffline) {
		t.Fatalf("err = %v, want ErrOffline", err)
	}
	if !strings.Contains(err.Error(), "offline") {
		t.Errorf("message lacks offline hint: %v", err)
	}
}

func TestRejectedTokenIsDroppedAndRetried(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.13")
	fg.token = "good"
	fg.assets["checksums.txt"] = []byte("x")
	exe := writeExe(t, "chaude")
	o := fg.options(exe, "linux", "amd64")
	o.Token = "stale"

	msg, err := Run(context.Background(), "0.9.13", o)
	if err != nil {
		t.Fatalf("Run: %v", err)
	}
	if !strings.Contains(msg, "up to date") {
		t.Errorf("message = %q", msg)
	}
}

func TestUnknownCurrentVersionIsRefused(t *testing.T) {
	fg := newFakeGitHub(t, "v0.9.14")
	fg.assets["checksums.txt"] = []byte("x")
	exe := writeExe(t, "chaude")

	_, err := Run(context.Background(), "unknown", fg.options(exe, "linux", "amd64"))
	if !errors.Is(err, ErrUnknownVersion) {
		t.Fatalf("err = %v, want ErrUnknownVersion", err)
	}
	if readFile(t, exe) != "OLD" {
		t.Error("binary changed for unknown current version")
	}
}

func TestMainUsage(t *testing.T) {
	if code := Main("0.9.13", []string{"extra"}); code != 2 {
		t.Errorf("Main with bad args = %d, want 2", code)
	}
}

func TestAssetNamePlatforms(t *testing.T) {
	cases := []struct{ goos, goarch, want string }{
		{"windows", "amd64", "chaude_0.9.14_windows_amd64.zip"},
		{"windows", "arm64", "chaude_0.9.14_windows_amd64.zip"},
		{"darwin", "arm64", "chaude_0.9.14_darwin_arm64.tar.gz"},
		{"linux", "arm64", "chaude_0.9.14_linux_arm64.tar.gz"},
	}
	for _, c := range cases {
		got, err := assetName("0.9.14", c.goos, c.goarch)
		if err != nil || got != c.want {
			t.Errorf("assetName(%s/%s) = %q, %v; want %q", c.goos, c.goarch, got, err, c.want)
		}
	}
	if _, err := assetName("0.9.14", "darwin", "amd64"); err == nil {
		t.Errorf("assetName(darwin/amd64) returned no error, want Apple Silicon only error")
	}
}
