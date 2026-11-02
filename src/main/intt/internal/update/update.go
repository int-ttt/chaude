// Package update implements `chaude update` and the /update slash command.
// It follows install.sh / install.ps1: the same GitHub latest-release API, the same asset names
// (chaude_<ver>_<os>_<arch>.tar.gz, .zip on windows), and checksums.txt is required before anything is replaced.
package update

import (
	"archive/tar"
	"archive/zip"
	"compress/gzip"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"net/http"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"
)

const (
	defaultRepo = "int-ttt/chaude"
	defaultAPI  = "https://api.github.com"
	usage       = "usage: chaude update   check GitHub for a newer release and replace this binary\n"
)

// ErrOffline marks a failure to reach GitHub at all (no network, DNS, proxy).
var ErrOffline = errors.New("cannot reach GitHub")

// ErrUnknownVersion means the running binary has no release version (a dev build), so no comparison is possible.
var ErrUnknownVersion = errors.New("current version is not a release version")

// Options configures a check. Zero fields take defaults. Tests set APIBase, GOOS, GOARCH, Exe and Client directly.
type Options struct {
	Repo    string       // owner/name; default int-ttt/chaude
	APIBase string       // default https://api.github.com
	Token   string       // optional; a rejected token is dropped and the request retried anonymously (as install.sh does)
	GOOS    string       // asset platform; default runtime.GOOS
	GOARCH  string       // asset platform; default runtime.GOARCH
	Exe     string       // binary to replace; default the running executable (symlinks resolved)
	Client  *http.Client // default: 10-minute timeout
}

// DefaultOptions reads CHAUDE_REPO, then GH_TOKEN / GITHUB_TOKEN, else `gh auth token` (same order as install.sh).
func DefaultOptions() Options {
	o := Options{Repo: defaultRepo}
	if r := os.Getenv("CHAUDE_REPO"); r != "" {
		o.Repo = r
	}
	switch {
	case os.Getenv("GH_TOKEN") != "":
		o.Token = os.Getenv("GH_TOKEN")
	case os.Getenv("GITHUB_TOKEN") != "":
		o.Token = os.Getenv("GITHUB_TOKEN")
	default:
		ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
		defer cancel()
		if out, err := exec.CommandContext(ctx, "gh", "auth", "token").Output(); err == nil {
			o.Token = strings.TrimSpace(string(out))
		}
	}
	return o
}

// runningExe returns the path of the running binary with symlinks resolved.
func runningExe() (string, error) {
	exe, err := os.Executable()
	if err != nil {
		return "", fmt.Errorf("cannot find the running binary: %w", err)
	}
	if r, err := filepath.EvalSymlinks(exe); err == nil {
		exe = r
	}
	return exe, nil
}

func (o Options) withDefaults() (Options, error) {
	if o.Repo == "" {
		o.Repo = defaultRepo
	}
	if o.APIBase == "" {
		o.APIBase = defaultAPI
	}
	if o.GOOS == "" {
		o.GOOS = runtime.GOOS
	}
	if o.GOARCH == "" {
		o.GOARCH = runtime.GOARCH
	}
	if o.Client == nil {
		o.Client = &http.Client{Timeout: 10 * time.Minute}
	}
	if o.Exe == "" {
		exe, err := runningExe()
		if err != nil {
			return o, err
		}
		o.Exe = exe
	}
	return o, nil
}

// Run checks the latest release and, when it is newer than current, installs it over the running binary.
// The returned string is the text to show when nothing failed (up to date, or updated). A non-nil error means nothing was installed.
func Run(ctx context.Context, current string, o Options) (string, error) {
	o, err := o.withDefaults()
	if err != nil {
		return "", err
	}
	ctx, cancel := context.WithTimeout(ctx, 10*time.Minute)
	defer cancel()

	rel, err := fetchRelease(ctx, &o)
	if err != nil {
		if errors.Is(err, ErrOffline) {
			return "", fmt.Errorf("%w (offline? try again when the network is up)", err)
		}
		return "", fmt.Errorf("cannot read the latest release of %s: %w", o.Repo, err)
	}
	newV := strings.TrimPrefix(rel.Tag, "v")
	oldV := strings.TrimPrefix(current, "v")
	latest, ok := parseVersion(rel.Tag)
	if !ok {
		return "", fmt.Errorf("release tag %q is not a version (vX.Y.Z expected)", rel.Tag)
	}
	cur, ok := parseVersion(current)
	if !ok {
		return "", fmt.Errorf("%w: %q. Reinstall with install.sh to get updates (latest is %s)", ErrUnknownVersion, current, newV)
	}
	if compareVersion(cur, latest) >= 0 {
		return fmt.Sprintf("chaude %s is up to date (latest %s).", oldV, newV), nil
	}

	// probe the install dir before any download: an unwritable dir changes nothing
	if err := checkWritable(filepath.Dir(o.Exe), o.GOOS); err != nil {
		return "", err
	}

	name := assetName(newV, o.GOOS, o.GOARCH)
	asset, ok := rel.asset(name)
	if !ok {
		return "", fmt.Errorf("%s not found in release %s (no prebuilt binary for %s/%s); nothing installed", name, rel.Tag, o.GOOS, o.GOARCH)
	}
	sums, ok := rel.asset("checksums.txt")
	if !ok {
		return "", fmt.Errorf("checksums.txt not found in release %s; refusing to install", rel.Tag)
	}

	tmp, err := os.MkdirTemp("", "chaude-update-")
	if err != nil {
		return "", fmt.Errorf("cannot create a temp dir: %w", err)
	}
	defer os.RemoveAll(tmp)

	archive := filepath.Join(tmp, name)
	if err := download(ctx, &o, asset.URL, "application/octet-stream", archive); err != nil {
		return "", fmt.Errorf("download failed: %s: %w", name, err)
	}
	sumsPath := filepath.Join(tmp, "checksums.txt")
	if err := download(ctx, &o, sums.URL, "application/octet-stream", sumsPath); err != nil {
		return "", fmt.Errorf("download failed: checksums.txt: %w", err)
	}
	if err := verify(archive, name, sumsPath); err != nil {
		return "", fmt.Errorf("%w; nothing installed", err)
	}
	bin := binName(o.GOOS)
	newBin := filepath.Join(tmp, bin)
	if err := extract(archive, bin, newBin); err != nil {
		return "", fmt.Errorf("cannot extract %s: %w", name, err)
	}
	if err := replace(o, newBin); err != nil {
		return "", err
	}
	return fmt.Sprintf("chaude updated: %s -> %s (%s). Restart chaude to use it.", oldV, newV, o.Exe), nil
}

// Text runs Run for the REPL and returns the message to show; failures are prefixed with "update failed:".
func Text(ctx context.Context, current string) string {
	msg, err := Run(ctx, current, DefaultOptions())
	if err != nil {
		return "update failed: " + err.Error()
	}
	return msg
}

// CleanupOld removes the chaude.exe.old that a windows self-update leaves behind (the old file is locked while it ran),
// and migrates an old ~/.local/bin install (see migrateLegacy). Call it at startup; errors are ignored.
func CleanupOld() {
	exe, err := runningExe()
	if err != nil {
		return
	}
	removeOld(exe)
	if home, err := os.UserHomeDir(); err == nil {
		migrateLegacy(home, runtime.GOOS, exe)
	}
}

func removeOld(exe string) {
	_ = os.Remove(exe + ".old")
}

// checkWritable creates and removes a probe file in dir. On unix a permission error means the install dir was
// set up with sudo (/usr/local/bin); the user is told to run the update with sudo.
func checkWritable(dir, goos string) error {
	f, err := os.CreateTemp(dir, ".chaude-probe-*")
	if err != nil {
		if goos != "windows" && errors.Is(err, fs.ErrPermission) {
			return fmt.Errorf("%s is not writable; nothing changed. run: sudo chaude update", dir)
		}
		return fmt.Errorf("cannot write to %s: %w; nothing changed", dir, err)
	}
	name := f.Name()
	_ = f.Close()
	_ = os.Remove(name)
	return nil
}

// migrateLegacy removes an install left by the old installers in ~/.local/bin: chaude, chaude.exe and chaude.<ver>.bak.
// On unix it also removes the exact PATH line the old installer appended to an rc file. Nothing is touched when there is
// no legacy file, or when the running binary is itself a legacy file (it must keep working).
func migrateLegacy(home, goos, exe string) {
	dir := filepath.Join(home, ".local", "bin")
	candidates := []string{filepath.Join(dir, "chaude"), filepath.Join(dir, "chaude.exe")}
	baks, _ := filepath.Glob(filepath.Join(dir, "chaude.*.bak"))
	candidates = append(candidates, baks...)

	var found []string
	for _, c := range candidates {
		if _, err := os.Lstat(c); err != nil {
			continue
		}
		if exe != "" {
			if a, err1 := os.Stat(c); err1 == nil {
				if b, err2 := os.Stat(exe); err2 == nil && os.SameFile(a, b) {
					return
				}
			}
		}
		found = append(found, c)
	}
	if len(found) == 0 {
		return
	}
	for _, c := range found {
		_ = os.Remove(c)
	}
	if goos == "windows" {
		return
	}
	line := `export PATH="` + dir + `:$PATH"`
	for _, rc := range []string{".zshrc", ".bash_profile", ".bashrc", ".profile"} {
		removeLine(filepath.Join(home, rc), line)
	}
}

// removeLine deletes every line that equals line exactly from file. A missing file or no match changes nothing.
func removeLine(file, line string) {
	data, err := os.ReadFile(file)
	if err != nil {
		return
	}
	lines := strings.Split(string(data), "\n")
	kept := lines[:0]
	for _, l := range lines {
		if l != line {
			kept = append(kept, l)
		}
	}
	if len(kept) == len(lines) {
		return
	}
	st, err := os.Stat(file)
	if err != nil {
		return
	}
	_ = os.WriteFile(file, []byte(strings.Join(kept, "\n")), st.Mode().Perm())
}

// Main implements `chaude update`. Exit 0 when up to date or updated, 1 on failure, 2 on bad arguments.
func Main(current string, args []string) int {
	if len(args) > 0 {
		if args[0] == "-h" || args[0] == "--help" {
			fmt.Print(usage)
			return 0
		}
		fmt.Fprint(os.Stderr, usage)
		return 2
	}
	msg, err := Run(context.Background(), current, DefaultOptions())
	if err != nil {
		fmt.Fprintln(os.Stderr, "update failed: "+err.Error())
		return 1
	}
	fmt.Println(msg)
	return 0
}

// parseVersion reads "v0.9.13", "0.9.13" or "0.9.13-rc1" (a pre-release suffix is ignored for ordering).
func parseVersion(s string) ([3]int, bool) {
	var v [3]int
	s = strings.TrimPrefix(s, "v")
	if i := strings.IndexAny(s, "-+"); i >= 0 {
		s = s[:i]
	}
	parts := strings.Split(s, ".")
	if len(parts) != 3 {
		return v, false
	}
	for i, p := range parts {
		n, err := strconv.Atoi(p)
		if err != nil || n < 0 {
			return v, false
		}
		v[i] = n
	}
	return v, true
}

func compareVersion(a, b [3]int) int {
	for i := range a {
		if a[i] != b[i] {
			if a[i] < b[i] {
				return -1
			}
			return 1
		}
	}
	return 0
}

// assetName is the release archive install.sh / install.ps1 would download for this platform.
func assetName(ver, goos, goarch string) string {
	if goos == "windows" {
		return fmt.Sprintf("chaude_%s_windows_%s.zip", ver, goarch)
	}
	return fmt.Sprintf("chaude_%s_%s_%s.tar.gz", ver, goos, goarch)
}

func binName(goos string) string {
	if goos == "windows" {
		return "chaude.exe"
	}
	return "chaude"
}

type release struct {
	Tag    string `json:"tag_name"`
	Assets []struct {
		Name string `json:"name"`
		URL  string `json:"url"` // API url; downloaded with Accept: application/octet-stream
	} `json:"assets"`
}

type asset struct{ Name, URL string }

func (r *release) asset(name string) (asset, bool) {
	for _, a := range r.Assets {
		if a.Name == name {
			return asset{a.Name, a.URL}, true
		}
	}
	return asset{}, false
}

// fetchRelease reads the latest release JSON. On failure with a token it drops the token (o.Token = "") and retries anonymously.
func fetchRelease(ctx context.Context, o *Options) (*release, error) {
	url := o.APIBase + "/repos/" + o.Repo + "/releases/latest"
	body, err := getJSON(ctx, o, o.Token, url)
	if err != nil && o.Token != "" {
		o.Token = ""
		body, err = getJSON(ctx, o, "", url)
	}
	if err != nil {
		return nil, err
	}
	var rel release
	if err := json.Unmarshal(body, &rel); err != nil {
		return nil, fmt.Errorf("unreadable release JSON: %w", err)
	}
	if rel.Tag == "" {
		return nil, errors.New("could not resolve the release tag")
	}
	return &rel, nil
}

func getJSON(ctx context.Context, o *Options, token, url string) ([]byte, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Accept", "application/vnd.github+json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	resp, err := o.Client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrOffline, err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, statusError(resp.StatusCode)
	}
	return io.ReadAll(io.LimitReader(resp.Body, 4<<20))
}

func statusError(code int) error {
	if code == http.StatusForbidden {
		return fmt.Errorf("GitHub returned HTTP 403 (rate limited?); set GH_TOKEN and try again")
	}
	return fmt.Errorf("GitHub returned HTTP %d", code)
}

// download fetches url into dst. Asset API urls need Accept: application/octet-stream and redirect to storage.
func download(ctx context.Context, o *Options, url, accept, dst string) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("Accept", accept)
	if o.Token != "" {
		req.Header.Set("Authorization", "Bearer "+o.Token)
	}
	resp, err := o.Client.Do(req)
	if err != nil {
		return fmt.Errorf("%w: %v", ErrOffline, err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return statusError(resp.StatusCode)
	}
	f, err := os.Create(dst)
	if err != nil {
		return err
	}
	_, err = io.Copy(f, resp.Body)
	if cerr := f.Close(); err == nil {
		err = cerr
	}
	return err
}

// verify checks the sha256 of file against the line for name in checksums.txt ("<hex>  <name>" or "<hex> *<name>").
func verify(file, name, sumsPath string) error {
	sums, err := os.ReadFile(sumsPath)
	if err != nil {
		return err
	}
	want := ""
	for _, line := range strings.Split(string(sums), "\n") {
		f := strings.Fields(line)
		if len(f) >= 2 && strings.TrimPrefix(f[1], "*") == name {
			want = strings.ToLower(f[0])
			break
		}
	}
	if want == "" {
		return fmt.Errorf("%s is not listed in checksums.txt", name)
	}
	h, err := sha256File(file)
	if err != nil {
		return err
	}
	if h != want {
		return fmt.Errorf("checksum mismatch for %s", name)
	}
	return nil
}

func sha256File(file string) (string, error) {
	f, err := os.Open(file)
	if err != nil {
		return "", err
	}
	defer f.Close()
	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return "", err
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

// extract writes the entry named bin from archive to dst. Only the base name is matched; nothing else is written.
func extract(archive, bin, dst string) error {
	if strings.HasSuffix(archive, ".zip") {
		return extractZip(archive, bin, dst)
	}
	return extractTarGz(archive, bin, dst)
}

func extractTarGz(archive, bin, dst string) error {
	f, err := os.Open(archive)
	if err != nil {
		return err
	}
	defer f.Close()
	gz, err := gzip.NewReader(f)
	if err != nil {
		return err
	}
	defer gz.Close()
	tr := tar.NewReader(gz)
	for {
		h, err := tr.Next()
		if err == io.EOF {
			return fmt.Errorf("%s not found in archive", bin)
		}
		if err != nil {
			return err
		}
		if h.Typeflag == tar.TypeReg && path.Base(h.Name) == bin {
			return writeExecutable(dst, tr)
		}
	}
}

func extractZip(archive, bin, dst string) error {
	r, err := zip.OpenReader(archive)
	if err != nil {
		return err
	}
	defer r.Close()
	for _, f := range r.File {
		if f.FileInfo().IsDir() || path.Base(f.Name) != bin {
			continue
		}
		rc, err := f.Open()
		if err != nil {
			return err
		}
		defer rc.Close()
		return writeExecutable(dst, rc)
	}
	return fmt.Errorf("%s not found in archive", bin)
}

func writeExecutable(dst string, src io.Reader) error {
	out, err := os.OpenFile(dst, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0o755)
	if err != nil {
		return err
	}
	_, err = io.Copy(out, src)
	if cerr := out.Close(); err == nil {
		err = cerr
	}
	return err
}

// stageCopy copies src into a new temp file in dir (same filesystem as the target, so the later rename is atomic).
func stageCopy(dir, src string) (string, error) {
	in, err := os.Open(src)
	if err != nil {
		return "", err
	}
	defer in.Close()
	out, err := os.CreateTemp(dir, ".chaude-update-*")
	if err != nil {
		return "", err
	}
	name := out.Name()
	_, err = io.Copy(out, in)
	if err == nil {
		err = out.Sync()
	}
	if cerr := out.Close(); err == nil {
		err = cerr
	}
	if err == nil {
		err = os.Chmod(name, 0o755)
	}
	if err != nil {
		os.Remove(name)
		return "", err
	}
	return name, nil
}

// replace puts newBin in place of o.Exe.
// Unix: stage in the same directory, then rename over the target (atomic).
// Windows: a running exe cannot be overwritten, so it is renamed to exe.old, the staged file is renamed into place,
// and CleanupOld removes exe.old on the next start.
func replace(o Options, newBin string) error {
	dir := filepath.Dir(o.Exe)
	stage, err := stageCopy(dir, newBin)
	if err != nil {
		return fmt.Errorf("cannot write to %s: %w (check permissions, or reinstall with install.sh / install.ps1)", dir, err)
	}
	if o.GOOS != "windows" {
		if err := os.Rename(stage, o.Exe); err != nil {
			os.Remove(stage)
			return fmt.Errorf("cannot replace %s: %w", o.Exe, err)
		}
		return nil
	}
	old := o.Exe + ".old"
	_ = os.Remove(old)
	if err := os.Rename(o.Exe, old); err != nil {
		os.Remove(stage)
		return fmt.Errorf("cannot move the running %s aside: %w", o.Exe, err)
	}
	if err := os.Rename(stage, o.Exe); err != nil {
		_ = os.Rename(old, o.Exe)
		os.Remove(stage)
		return fmt.Errorf("cannot put the new binary in place: %w", err)
	}
	return nil
}
