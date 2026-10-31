task: rateMu lock in repl.go (rate, lastFiveHour, fiveHourSeen)
changed: internal/harness/repl.go (field rateMu; setRate decide-under-lock, log after unlock; usageLine copies r.rate under lock)
not committed.
gofmt -l internal/harness: (no output) exit=0
go vet ./internal/harness/...: (no output) exit=0
go build ./...: (no output) exit=0
go test ./internal/harness/...: ok  	chaude/internal/harness	1.029s exit=0
go test -race ./internal/harness/...: ok  	chaude/internal/harness	2.052s exit=0
git diff --stat -- internal/harness:
 internal/harness/ratestatus_test.go | 44 +++++++++++++++++++++++++++++++++++++
 internal/harness/repl.go            | 20 +++++++++++++++--
 2 files changed, 62 insertions(+), 2 deletions(-)
note: ratestatus_test.go was already modified before this task (not by me).
