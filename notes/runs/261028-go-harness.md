part: harness (internal/harness/{core,repl,unit_test,acceptance_test}.go; old harness.go stub removed)
build/vet: go vet ./... clean, gofmt clean
unit (samples/probe1.jsonl parse, ctxTokens, snapshot, cleanEnv): pass
live HARNESS_LIVE=1 go test ./internal/harness -run . -v: TestLiveRotationCarriesContext PASS 13.75s, TestLiveStreamingMultipleDeltas PASS 2.61s
  rotation test builds ./cmd/harness and runs `harness repl` (ANTHROPIC_API_KEY unset)
known diffs: transcript "t" float printed %.6f; Main args exclude "repl" word; Ctrl-C during turn -> interrupt via signal channel (not live-tested); idle Ctrl-C kills process; --max-auto-rotations default -1=unlimited; usage 5h utilization printed via Go %v
