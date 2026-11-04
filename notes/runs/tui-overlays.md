# core needs from TUI A (tui-diff gaps)
- stage-prompter: emit UiEvent::Stage{slot:"prompter"} Working before the prompter call and Done/Failed after it when residents run. Go: deprecated/src/main/intt/internal/harness/resident.go:289 ("· prompter 처리중"), :305 ("· prompter 완료"). Without it Rust lacks 2 rows in submit-stream/tool-use/scroll-up/rewind.
- stage-tui: emit Stage{slot:"tui"} Working ("· tui 번역중") and Done ("· tui 완료"). Go: resident.go:359, :371.
- turnstats-ctx: add to TurnStats: input_tokens, cache_read_input_tokens, cache_creation_input_tokens, output_tokens, five_hour (Option, else "?"), session id (8-char prefix, else "None"), rotations. Go line format: repl.go:874 `[ctx %s | in %d cr %s cc %s out %d | $%.4f | 5h %s | sess %s rot %d]`. Rust currently prints no ctx line.
- turnstats-result: add result_input_tokens, result_output_tokens, result_cache_read_input_tokens, result_cache_creation_input_tokens, result_ctx so the [측정] line matches Go keys.
- duration_ms: Go 1538 vs Rust 42 per run (timing noise). Not maskable TUI-side without changing the harness normalization; left unmasked.
