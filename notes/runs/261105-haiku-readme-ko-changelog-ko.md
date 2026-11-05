# 261105 haiku: README.ko.md sync + CHANGELOG Unreleased Korean lines

status: done
files: README.ko.md, CHANGELOG.md (commit), notes/OPEN.md (not committed, per brief)

## README.ko.md changed sections
- badge: Go 1.22+ replaced with Rust stable badge (same as README.md)
- top deprecation notice: replaced with Rust-release notice (translated from README.md)
- 기능: "빌드 파일" -> "미리 빌드된 바이너리"
- 작동 방식: table gained 모델 column; prose adds "(조회, 수정, 검증 등)"; English-only count sentence removed
- 기여: Go build block replaced with cargo build/test block (from README.md)
- 릴리스: was English; translated to "## 릴리스" with commands unchanged
- 자주 묻는 질문: blank line before 기여 restored
- 라이선스: unchanged in meaning

## CHANGELOG.md entries translated (Unreleased, 3 entries)
- Release: Rust workspace archives -> 릴리스 line
- Release: Go build not published -> 릴리스 line
- Parity: Rust build coverage -> 기능 동등성 line
- Format: Korean line nested under each English bullet (prior releases are not bilingual: 0.9.x Korean-only, 0.10.x English-only)

## checks
- heading parity: 19 README.md headings, 19 README.ko.md headings, 1:1 by position
- English prose left in README.ko.md: none (only identifiers, product names, link labels)

## open
- notes/OPEN.md [i18n] line set to done; OPEN.md left uncommitted (brief: commit only 2 files)
- 0.10.0-0.10.2 CHANGELOG entries are English-only; not in scope, not translated
- DECISIONS.md line 53 says "Go binary still shipped"; now stale vs README.md (user decision needed, not changed)
