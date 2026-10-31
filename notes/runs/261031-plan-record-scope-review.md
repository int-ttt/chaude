# review: 261031-plan-record-scope.md
status: done
reason: -
fmt: 3.4 R-decide 형식 검토 (spec plan/caveman-structure.md 3.1-3.6)
verdict: 수정 후 실행 가능 (v2 병합)
evidence: S0 scout: repl.go:224 p -> r.out; tui.go:1152-1153 r.out = tuiWriter -> noteMsg 블록; record.go:61 recordDir(cwd string), cwd "" 허용; repl.go:942-956 fields 키 start_ms,end_ms,model,ctx,session,draining,cost_usd,duration_ms,rotations,delegations + 조건부 cost_turn_usd, result_*, result_model_usage(map), spend 키; repl.go:898-900 budgetStop 조기 return 만; record_turn.go:24 recordTopicFor 가 r.lastReply 비움
fix1_D7: 블랙리스트 -> 화이트리스트. start_ms/end_ms/model/draining/map 값이 줄을 오염시킴
fix2_reset: recordSave 끝에서 r.lastMeasure = "" (plain loop repl.go:1075 도 같은 경로)
fix3_order: measureLine 은 숫자 값만 (int, int64, float64); map/string 무시
fix4_auto_cwd: a.Cwd "" 그대로 recordDir 에 넘김 (os.Getwd 분기 불필요)
fix5_test_env: tests/hooks 에서 subprocess env 에 HARNESS_RECORD_DIR pop
fix6_noteMsg: TUI 측정 줄은 noteMsg 블록 (dim 제거). 별도 tui.go 수정 불필요
risks_left: notes/metrics 기존 데이터 기본 요약 제외 / worktree 경로 차이
open_q: -
fmt_markers: 3.1 brief(B형식) / 3.2 R-read / 3.3 R-write / 3.4 R-decide / 3.5 빈 필드 "-" / 3.6 status 헤더
paths: as above
limits: no other files
return: R-write, 5 lines: .gitignore line number of record/ + changed or not, check-ignore output, review line count
lang: caveman
