# input-repro (2026-10-29)
result: 측정 불가 (실제 IME 경로) / 부분 측정: NFC 입력은 재현 안 됨, NFD 입력은 모델 계산상 열 어긋남 재현됨

## code path
repl.go: 입력 처리 없음 (plain stdin 경로만). 실제 입력 편집은 input.go inputBox + tui.go (syncCursor, cursorWriter)
input.go: Insert(rune slice, pos += len(in)), cells()=runewidth 합, rows() 셀 기준 wrap, Cursor() col=cells(행 시작..pos), View() 역상 커서 셀
tui.go: 실제 터미널 커서를 ESC[nA ESC[colG 로 이동 (IME 프리에딧 위치용), col = col+4 (input-fix.md 기준)
repl.go diff 조사 시점: conductor 기본값 true 1줄만 (조합과 무관); 이후 git status 에서 repl.go 가 더 이상 M 아님 (내가 건드리지 않음)

## measure (임시 Go 테스트 internal/harness/zz_repro_test.go 로 실행 후 삭제, 코드/커밋 변경 없음)
cmd: go test ./internal/harness -run TestZZRepro -v
NFC "한글" (U+D55C U+AE00): cursorCol=4, lipgloss 실제 폭=4 -> 어긋남 없음
NFD "한글" (U+1112 U+1161 U+11AB U+1100 U+1173 U+11AF): cursorCol=8, lipgloss 폭=4 -> 모델이 4칸 과대 (음절당 +2)
IME-steps(ㅎ->하->한...) 시뮬레이션: 내 스캐폴드가 부정확해 "글"만 남음 -> 결과 무효, 증거로 쓰지 않음

## limits (측정 불가)
- 실제 macOS IME 조합(프리에딧을 터미널이 커서 위치에 그림, 확정 전 KeyRunes 없음)은 자동화 불가
- pty/script 로 bubbletea 전체 구동 + 터미널 에뮬레이터 화면 열 측정 안 함 (에뮬레이터가 없으면 실제 그려진 열을 알 수 없음)
- cursorWriter 의 부분 프레임 쓰기와 프리에딧 겹침은 미검증
- 결론: 사용자가 보는 "한 칸 밀림"(NFC 타이핑 시)은 모델 계산으로는 설명되지 않음. 원인 후보는 터미널측 커서 이동/프리에딧 겹침. 
next: 실제 터미널에서 curCol=0 으로 끄고 비교, 키마다 Cursor()/Value() 로그
