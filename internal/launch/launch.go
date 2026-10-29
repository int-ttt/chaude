// Package launch: the "double-click / right-click" entry points.
//   - `chaude open [dir] [repl flags]`: installs the package into dir if it is not there yet (asks first),
//     then starts the REPL in dir. This is what the Explorer context menu and the launcher files call.
//   - `chaude context-menu install|remove` (windows): adds "chaude로 열기" to the folder right-click menu
//     (HKCU only, no admin), pointing at this executable.
package launch

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"chaude/internal/harness"
	"chaude/internal/install"
	"chaude/internal/osenv"
)

// Installed reports whether the chaude package is present in dir (the version marker install writes).
func Installed(dir string) bool {
	_, err := os.Stat(filepath.Join(dir, ".claude", "claude-lean.version"))
	return err == nil
}

// SplitOpenArgs separates the optional leading folder from REPL flags: `open C:\proj --model fable`.
func SplitOpenArgs(args []string) (dir string, rest []string) {
	if len(args) > 0 && !strings.HasPrefix(args[0], "-") {
		return args[0], args[1:]
	}
	return ".", args
}

// Open runs `chaude open`; version is written by the install step. Returns the exit code.
func Open(args []string, version string) int {
	dir, rest := SplitOpenArgs(args)
	abs, err := filepath.Abs(dir)
	if err == nil {
		var st os.FileInfo
		if st, err = os.Stat(abs); err == nil && !st.IsDir() {
			err = fmt.Errorf("not a folder")
		}
	}
	if err != nil {
		fmt.Fprintf(os.Stderr, "chaude open: %s: %v\n", dir, err)
		return 2
	}
	if !Installed(abs) {
		fmt.Printf("chaude is not installed in %s.\nInstall it there now (agents, hooks, settings, CLAUDE.md policy)? [Y/n] ", abs)
		line, _ := bufio.NewReader(os.Stdin).ReadString('\n')
		if a := strings.ToLower(strings.TrimSpace(line)); a != "" && a != "y" && a != "yes" && a != "ㅇ" && a != "ㅇㅇ" {
			fmt.Println("cancelled")
			return 1
		}
		if err := install.Run(install.Options{Dir: abs, Version: version, Windows: osenv.IsWindows, Out: os.Stdout}); err != nil {
			fmt.Fprintln(os.Stderr, "chaude install:", err)
			return 1
		}
		fmt.Println()
	}
	if err := os.Chdir(abs); err != nil {
		fmt.Fprintln(os.Stderr, "chaude open:", err)
		return 1
	}
	return harness.Main(append([]string{"--cwd", abs}, rest...))
}

var menuKeys = []string{
	`HKCU\Software\Classes\Directory\shell\chaude`,            // right-click on a folder
	`HKCU\Software\Classes\Directory\Background\shell\chaude`, // right-click inside a folder
}

const menuLabel = "chaude로 열기"

// ContextMenu runs `chaude context-menu install|remove|status` (windows only; HKCU, no admin).
func ContextMenu(args []string) int {
	if !osenv.IsWindows {
		fmt.Fprintln(os.Stderr, "context-menu is windows only (mac/linux: double-click chaude-start.command in the project)")
		return 2
	}
	sub := "install"
	if len(args) > 0 {
		sub = args[0]
	}
	switch sub {
	case "install":
		exe, err := os.Executable()
		if err == nil {
			exe, err = filepath.Abs(exe)
		}
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			return 1
		}
		cmdline := `"` + exe + `" open "%V"`
		if wt, err := exec.LookPath("wt"); err == nil { // Windows Terminal: Korean IME input works there
			cmdline = `"` + wt + `" -d "%V" "` + exe + `" open "%V"`
		}
		for _, k := range menuKeys {
			if err := regRun("add", k, "/ve", "/d", menuLabel, "/f"); err != nil {
				return regFail(err)
			}
			if err := regRun("add", k, "/v", "Icon", "/d", exe, "/f"); err != nil {
				return regFail(err)
			}
			if err := regRun("add", k+`\command`, "/ve", "/d", cmdline, "/f"); err != nil {
				return regFail(err)
			}
		}
		fmt.Printf("context menu \"%s\" added (folder and folder background) -> %s\nremove: chaude context-menu remove\n", menuLabel, cmdline)
	case "remove":
		for _, k := range menuKeys {
			_ = regRun("delete", k, "/f") // absent key is fine
		}
		fmt.Printf("context menu \"%s\" removed\n", menuLabel)
	case "status":
		for _, k := range menuKeys {
			out, err := exec.Command("reg", "query", k+`\command`, "/ve").Output()
			if err != nil {
				fmt.Printf("%s: not installed\n", k)
				continue
			}
			fmt.Printf("%s:\n%s", k, strings.TrimSpace(string(out)))
			fmt.Println()
		}
	default:
		fmt.Fprintln(os.Stderr, "usage: chaude context-menu install|remove|status")
		return 2
	}
	return 0
}

func regRun(args ...string) error {
	out, err := exec.Command("reg", args...).CombinedOutput()
	if err != nil {
		return fmt.Errorf("reg %s: %v: %s", strings.Join(args[:2], " "), err, strings.TrimSpace(string(out)))
	}
	return nil
}

func regFail(err error) int {
	fmt.Fprintln(os.Stderr, "chaude context-menu:", err)
	return 1
}
