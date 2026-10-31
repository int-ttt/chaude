---
spec: plan/caveman-structure.md 6절 (:152-160, :171-175)
baseline: 0ba338b
rewritten: .claude/hooks/caveman-format.py (check_jondaetmal: haiku-tui 출력 '다'(비'니다') 문장끝 검출, 코드블록 제외; check_brief: haiku-prompter 브리프 B형식 5키; 기존 R-read/R-write/R-decide 검사·log|warn|off·exit 0 유지)
tests: .claude/hooks/tests/test_caveman_format.py(14) -> tests/hooks/test_caveman_format.py 로 병합(기존3+이동14+신규6=23); .claude/hooks/tests/ 삭제
settings.json: 변경 없음 (PostToolUse Agent|Task caveman-format.py 이미 plan과 일치)
not_in_plan: correction-guard, style-adapter, context-alert, haiku-context-guard, ledger, session-context, history-export, _lean.py (plan 미언급, 미수정); main-thread-guard, agent-model-pin (plan:183 불변)
deleted_by_plan: 없음 (plan에 삭제 지정 파일 없음)
check: python3 -m unittest discover tests/hooks -> Ran 33, OK; smoke warn/haiku-scout -> additionalContext+행1 exit0 PASS; smoke log/haiku-tui "완료했다." -> missing ["존댓말"] PASS; smoke "not json" -> 무출력 exit0 PASS; settings.json json 파싱 PASS
note: 첫 스모크 a 실패는 zsh echo 가 JSON 내 \n 을 개행으로 바꿔 입력이 깨진 탓, python json.dumps 입력으로 재실행 PASS
open_q: plan:160(prompter 전달문 B형식 검사) vs :42/:126/:133(전달문=원문+요약) 모순 -> :160 따라 log 전용 구현; plan:171 S1만 vs 현 TARGETS 10종 유지(S4 단계로 간주)
---
