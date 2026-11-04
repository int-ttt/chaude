# run: 261104 haiku release rust-build + parity checklist
status: done
scope: rust-port phase 6 prep
changed:
- .github/workflows/release.yml: added job rust-build (+57 lines); Go release job unchanged
- plan/rust-parity-checklist.md: new, 63 rows
check: ruby -ryaml YAML.load_file(release.yml) -> ok; jobs = [release, rust-build]; matrix include = 4
not run: actionlint, a real GitHub Actions run (no push, no tag)
checklist counts: done 39, differs 7, missing 17 (rows; repl flags as one row F3 covering 26)
top missing: repl subcommand + 26 flags; --from-claude/--import-dry-run CLI; clean; update CLI; 7 helper scripts; CHAUDE_MAX_WORKERS; status bar P/T/C (blocked)
doubts:
- Go test count 320 (grep) vs plan 312
- E3, E4, F3 Go refs not re-read
- S11-S19 passthrough rows not verified per command
- rust-build does not check Cargo version against tag
- windows Compress-Archive and macOS runner untested
- K5 clipboard: no non-macOS tool
