task: make installers work with no token (repo now PUBLIC)
verify: gh repo view -> visibility PUBLIC, url https://github.com/int-ttt/chaude
release: v0.9.13 assets = 6 archives (darwin/linux amd64+arm64, windows amd64+arm64 zip) + checksums.txt
anon api: GET api.github.com/repos/int-ttt/chaude/releases/latest -> 200

changed:
- install.sh: no token required (token optional, rejected token dropped + anonymous retry); curl --retry 3; prereq check curl/tar/awk/sed/install/mktemp + sha256sum|shasum (clear error); warning if claude not on PATH; error msgs no longer say private repo
- install.ps1: same flow; PS>=5 check; token fallback; claude warning; ASCII-only
- README.md: install section only (curl/irm lines without header, token section, gh auth line)
- docs/guide/cli.md: install section only (token wording, one-liners, tools bullet, claude bullet, env table GH_TOKEN row)
- not touched: Go src, .claude/, CLAUDE.md, platform_setup/ (old duplicate installers, untouched)
- git: no commit, no push

checks:
- bash -n install.sh -> PASS; sh -n install.sh -> PASS
- shellcheck -> not installed (not run)
- pwsh -> not installed; install.ps1 parse NOT run. Substitutes: non-ASCII lines 0, { } 50/50, ( ) 81/81
- local run, HOME=tmp, PATH=/usr/bin:/bin (no gh, no token), CHAUDE_INSTALL_DIR=tmp prefix:
  - run1 fresh -> curl reset (network flake) -> error exit 1; fixed by --retry 3
  - run2 fresh -> PASS, chaude 0.9.13 installed, claude warning printed
  - run3 fresh + GH_TOKEN=bogus -> warning "rejected the token", retry anonymous -> PASS exit 0
  - run4 same version re-run -> PASS, chaude.0.9.13.bak created, rc "# chaude" lines = 1 (no dup)
  - CHAUDE_VERSION=v0.0.1 -> clear error exit 1
  - PATH without curl -> "required tool not found: curl" exit 1
  - CHAUDE_REPO=int-ttt/nope -> clear error exit 1
- raw.githubusercontent one-liner NOT exercised (needs push; not allowed)
- install.ps1 runtime NOT exercised (no pwsh)

doubts:
- brief said "missing go/git/claude" errors: go and git not needed for prebuilt download, so not required; claude only warned (chaude needs it at run time, install still succeeds)
- Korean doc text: minimal edits in existing Korean style, flagged for haiku-tui review
- README.md already had uncommitted edits from other agents; only install lines changed by me
- install.ps1 Expand-Archive / HttpClient path unverified on 5.1 and 7
