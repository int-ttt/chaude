package harness

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"
)

// recordProject is one project folder under the record root, for the /resume list.
type recordProject struct {
	Name  string
	Dir   string
	Files int
	Last  time.Time // newest file mod time in the folder
}

// recordLegacyRe matches the old date/topic file names (YYMMDD-<slug>-NNN.md) that sit directly in the root.
var recordLegacyRe = regexp.MustCompile(`^\d{6}-.+-\d{3,}\.md$`)

// folderName is a project folder name from a path: its basename, sanitized as a slug, never hidden.
func folderName(path string) string {
	name := strings.TrimLeft(slugify(filepath.Base(path)), ".")
	if name == "" {
		return "misc"
	}
	return name
}

// recordProjectName is the folder name of the project at cwd (cwd basename).
func recordProjectName(cwd string) string {
	if cwd == "" {
		cwd, _ = os.Getwd()
	}
	return folderName(cwd)
}

// recordProjectDir is the folder of the project at cwd under the record root.
func recordProjectDir(root, cwd string) string {
	return filepath.Join(root, recordProjectName(cwd))
}

// migrateRecordRoot moves the old date/topic files that sit directly in root into one project folder.
// Old files carry no cwd; they were written for the checkout that owns root (root's parent), so that
// checkout's name is the folder. Files already in a folder are not touched. No-op when there are none.
func migrateRecordRoot(root string) error {
	ents, err := os.ReadDir(root)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var legacy []string
	for _, e := range ents {
		if !e.IsDir() && recordLegacyRe.MatchString(e.Name()) {
			legacy = append(legacy, e.Name())
		}
	}
	if len(legacy) == 0 {
		return nil
	}
	dst := filepath.Join(root, folderName(filepath.Dir(root)))
	if err := os.MkdirAll(dst, 0o755); err != nil {
		return err
	}
	for _, name := range legacy {
		if err := os.Rename(filepath.Join(root, name), filepath.Join(dst, name)); err != nil {
			return err
		}
	}
	return nil
}

// listRecordProjects returns the project folders under root (old files migrated first), most recently
// written first. Folders without a record file are skipped.
func listRecordProjects(root string) ([]recordProject, error) {
	if err := migrateRecordRoot(root); err != nil {
		return nil, err
	}
	ents, err := os.ReadDir(root)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	var out []recordProject
	for _, e := range ents {
		if !e.IsDir() || strings.HasPrefix(e.Name(), ".") {
			continue
		}
		dir := filepath.Join(root, e.Name())
		files, err := listRecordFiles(dir)
		if err != nil {
			return nil, err
		}
		if len(files) == 0 {
			continue
		}
		p := recordProject{Name: e.Name(), Dir: dir, Files: len(files)}
		for _, f := range files {
			if f.mod.After(p.Last) {
				p.Last = f.mod
			}
		}
		out = append(out, p)
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].Last.After(out[j].Last) })
	return out, nil
}

// filterRecordProjects keeps the folders whose name, or any block topic or text, contains q (case-insensitive).
func filterRecordProjects(projects []recordProject, q string) ([]recordProject, error) {
	q = strings.ToLower(q)
	var out []recordProject
	for _, p := range projects {
		if strings.Contains(strings.ToLower(p.Name), q) {
			out = append(out, p)
			continue
		}
		entries, err := loadRecordProject(p.Dir)
		if err != nil {
			return nil, err
		}
		for _, e := range entries {
			if strings.Contains(strings.ToLower(e.Text), q) || strings.Contains(strings.ToLower(e.Topic), q) {
				out = append(out, p)
				break
			}
		}
	}
	return out, nil
}

// loadRecordProject returns every block of a project folder, all its files as one history, in rec-number order.
func loadRecordProject(dir string) ([]recordEntry, error) {
	files, err := listRecordFiles(dir)
	if err != nil {
		return nil, err
	}
	var out []recordEntry
	for _, f := range files {
		data, err := os.ReadFile(filepath.Join(dir, f.name))
		if err != nil {
			return nil, err
		}
		out = append(out, parseRecordEntries(string(data))...)
	}
	sort.SliceStable(out, func(i, j int) bool { return out[i].Num < out[j].Num })
	return out, nil
}

// recordProjectLines formats folder rows for /resume. numbered rows can be picked with /resume <번호>.
func recordProjectLines(title string, ps []recordProject, numbered bool) []string {
	lines := []string{title}
	for i, p := range ps {
		prefix := "  - "
		if numbered {
			prefix = fmt.Sprintf("%2d. ", i+1)
		}
		lines = append(lines, fmt.Sprintf("%s%s · 파일 %d개 · %s", prefix, p.Name, p.Files, p.Last.Format("2006-01-02")))
	}
	return lines
}
