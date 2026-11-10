# Caveman format (canonical)

source: plan/caveman-structure.md 3.1~3.6 (v7)

## 3.1 Brief format B (downward)

```
task: <one-line verb phrase>
paths: <absolute path>[:<line range>], ...
ctx: E<n>: <path>:<line> / <number or quote gist>   (no inheritance, C6)
limits: <paths not to edit>, <forbidden actions>
return: R-read | R-write | R-decide, <cap (chars or lines)>
lang: caveman | ko | en   (ko = summary_ko required, rule_4; en = plain English, no summary_ko)
```

- The E<n> evidence-line format in ctx reuses the earlier definition.
- User original text and Korean values stay verbatim inside ctx too.

## 3.2 Read return R-read

```
claim: <path>:<line> | <fact or verbatim quote> [sure|likely|unsure]
searched: <paths/patterns looked at>
not_found: <looked, confirmed absent>
not_checked: <ranges not looked at>
truncated: yes|no ; omitted: <omitted items or ->
summary_ko: <required only when lang: ko, 1-5 lines>
```

- not_found and not_checked are never merged ("confirmed absent" vs "not looked at", read-review.md:15).
- When the return cap is hit, always write truncated: yes and the omitted list (C12).
- Brief and R-read values are English only (claim, reason, searched, not_found, not_checked). Korean appears only inside quoted source text. Rendering the output in the CHAUDE_LANG language (ko summary + item translation, or en) is done by haiku-tui (plan/caveman-structure.md 2.1, 261101).

## 3.3 Write return R-write

```
changed: <path list>
diffstat: <gist of git diff --stat>
check: <command> -> PASS|FAIL
ws: <workspace id | none>
summary_ko: <required when lang: ko, 1-5 lines>
```

- The field list uses the flow.md:58-61 definition as-is.

## 3.4 Decision return R-decide

```
decision: <one line>
why: <see evidence E<n>>
rejected: <discarded option: reason>
risks: <items>
open_q: <question only the user can answer | ->
plan_path: <path | ->
summary_ko: <required when lang: ko, 1-5 lines, for L0 relay>
```

## 3.5 Caveman allowed and forbidden scope

- Allowed: drop particles, connectives, greetings, preambles; key: value; noun phrases; English keys.
- Forbidden: deleting qualifiers (C3), dropping number units or subjects, dropping paths or line numbers, deleting certainty tags, merging not_found and not_checked, translating or abbreviating Korean values.
- Empty fields keep the key with "-" (distinguishes a missing field from "not applicable").

## 3.6 Common status fields (2-line head prepended to each block in 3.2~3.4)

```
status: done | partial | fail | blocked
reason: <one line when partial/fail/blocked, else ->
```

- blocked: tool use blocked by a hook or permission denial. Quote the denial message verbatim and return without a workaround.
- fail: check FAIL or task impossible. Write the retry count in reason.
- Main must state any non-done status in the user answer, in the CHAUDE_LANG language (ko: polite Korean, en: plain English).
