# category rotation + v0.6.6
design: internal/harness/category.go; rule appended via --append-system-prompt (with delegation rule); `[cat: X]`/`[cat-switch: X]` parsed in handle(); switch -> notice + rotate("category A -> B"); isAck guard; same label no rotate; flag --no-category-rotate; metrics stage=category
tests: category_test.go (parse, rotation decision incl. ack/disabled/same-label, rule size). Rotation decision tested as pure function; full live rotate not exercised (needs real claude).
doubts: marker line stays visible in streamed reply (not stripped); rule is a prompt, model compliance unverified live.
