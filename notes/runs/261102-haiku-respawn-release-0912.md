# 261102 haiku respawn fix + release 0.9.12
status: done
fix: sendResident (resident.go) respawns missing/dead slot once via spawnSlot, re-checks residentChild, else error -> raw fallback
tests:
- new: TestSendResidentRespawnsDeadSlot (dead prompter -> respawned, msg delivered, reply ok) PASS
- new: TestViaPrompterRawWhenRespawnFails (spawn error -> raw text) PASS
- stubbed failRespawn() in TestViaPrompterFailureNotice, TestViaTUIFailureNotice, TestViaPrompterFallsBack, TestRunTurnShowsTUIOutputOnly (they assumed no respawn; without stub they spawned real claude)
- go vet ./... (src/main/intt) exit 0; go test ./... -count=1: clean, harness ok; gofmt -l empty
docs: sonnet-reviewer removed from docs/claude-lean/README.md (row), docs/claude-lean/APPLY_GUIDE.md (/agents row); CHANGELOG history kept
commit: 520a0ff (master, pushed 0ce88cf..520a0ff); tag v0.9.12 pushed
trailer: Co-Authored-By: Claude Opus 5.5 (per brief); system attribution said Haiku 5.5 -> doubt, check
release run: 41323027047 success (~6 min)
assets: 7 (6 archives + checksums.txt)
install: darwin arm64 -> ~/.local/bin/chaude ; old -> ~/.local/bin/chaude.0.9.11.bak
checksum: archive sha256 d351f479fd8a3222204ad1e87191ac1ff423f92de88f07c59308e852ffdd2936 OK vs checksums.txt; installed bytes cmp == extracted
chaude --version -> chaude 0.9.12
untouched: .gitignore (pre-existing M), untracked notes/e2e/.chaude etc.
notes: 2 claude procs (pid 34972, 68549) pre-existing, user's; not touched
