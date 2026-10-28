part: scripts (internal/scripts/*.go)
files: common.go charcount.go claude_history.go compress_archive.go export_history.go extract_transcript.go search_history.go usage_report.go
vet: go vet ./internal/scripts clean; gofmt clean
harness: notes/runs/261028-go-scripts-diff.py (py original vs `harness <script>`; HOME=fixture, frozen snapshot of ~/.claude/projects {claude-harness,BaseX,UnitV2M12} + odd-name cwd-match project + claude-archive copy; compares stdout, rc, full file tree md5 incl. hidden; .gz compared decompressed)
result: 44/44 PASS
cases: charcount(stdin,--section,last section,--limit over,section missing,real file CRLF) · claude-history(encoded dir, cwd-match dir, none, no arg) · export-history(real project, --width 40 --split-kb 20 (35 files), temp project w/ tabs/long words/emoji/bad json, --file, --hook valid/invalid stdin, other projects) · extract-transcript(default, --chunk/--reply-chars 1, dir, jsonl file, temp project, missing) · search-history(10 variants: terms, --limit 0/-3/5/30, --who, --since, --width, --project temp, archive gz, none) · usage-report(default, --top, root arg, --top -2) · compress-archive(archive dir recursive, no files)
known diffs:
- file iteration order: python glob = scandir order (arbitrary); Go sorts. Only matters for ties on equal minute timestamps (chunk boundaries/hit order). Test patches python glob to sorted.
- compress-archive: gzip bytes differ from python (header/mtime/deflate impl) so MB figures may differ in last decimal; content identical (decompressed compared).
- python crash cases (non-dict json line, null text, empty-terms hit, invalid width<=0) are skipped/ignored rather than traceback; argparse abbreviations (--proj) unsupported.
- charcount stdin: no newline translation on posix (as python), translation on windows.
notes:
- export-history exposes `scripts.Stdin io.Reader` (default os.Stdin); --hook reads it. If the hook already consumed os.Stdin, set scripts.Stdin = strings.NewReader(raw) before scripts.Run("export-history", {"--hook","--project",root}). Returns 0 and prints nothing in hook mode.
- exported: scripts.HistoryDir(project), scripts.ProjectsRoot()
- side effect during testing: an early run wrote logs/history/* in the repo (git-ignored; hook regenerates it).
