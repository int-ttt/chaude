task: 삭제 후보 목록 (read-only, 삭제 안 함)
scope: tracked + untracked(ignored 포함). 크기는 du/wc 기준.

## safe (참조 없음, 재생성 가능 또는 중복)
- path: bin/harness | tracked: no (ignored bin/) | size: 5.2M(bin 전체) | why: 구 바이너리, 코드는 bin/chaude만 사용 | risk: 낮음. 참조는 notes/runs/261028-*.md 기록뿐
- path: tests/e2e-rebuild/ | tracked: no (ignored; .gitkeep만 ignore 예외) | size: 92K (t.jsonl 61K) | why: 일회성 e2e 재구성 산출물, 자체 go.mod | risk: 낮음. 참조는 notes/runs/261029-e2e-rebuild.md만. 단 해당 run 노트가 결과를 담고 있으면 노트 확인 후 삭제

## check (참조 있거나 사용자 확인 필요)
- path: ./chaude (repo root) | tracked: no (ignored /chaude) | size: 5.3M, Mach-O arm64, mtime 10-29 00:34 | why: bin/chaude와 중복된 빌드 산출물, 가장 최신 빌드 | risk: 중간. 사용자가 직접 실행 중일 수 있음. README는 bin/chaude 기준
- path: dist/ | tracked: no | size: 없음 (디렉토리 부재) | why: 삭제할 것 없음 | risk: -
- path: notes/pastes/ (38 files) | tracked: no (ignored) | size: 156K | why: 임시 붙여넣기 저장소 | risk: 중간. notes/ 내 참조 여부 미확인 (grep 결과 없음), 사용자 원문 보관 가능성
- path: logs/ | tracked: no (ignored) | size: 840K (logs/history) | why: 기록 산출물 | risk: 높음. .claude/hooks/history-export.py, context-alert.py, .claude/skills/status/SKILL.md 참조 → keep 권장
- path: samples/probe1.jsonl, probe_interrupt.jsonl, probe_misc.jsonl, probe_perm.jsonl, probe_resume.jsonl, probe1.err (0B) | tracked: yes | size: 27K+28K+45K+27K+11K+0 ≈ 138K | why: 스파이크 probe 녹화 | risk: 중간. notes/spike-results.md:4 "raw recordings"로 참조, notes/runs/261028-go-harness.md:3 probe1.jsonl 파싱 테스트에 사용 → 삭제 시 테스트 깨짐 가능
- path: samples/accept_transcript.jsonl, accept_run.txt | tracked: yes | size: 4K+0.4K | why: 수용 테스트 녹화 | risk: 중간. notes/spike-results.md:29 및 .claude/hooks/style-adapter.py 등에서 참조 → keep 권장
- path: notes/runs/261028-tui-capture.txt ~ capture5-scroll.txt (6 files) | tracked: yes | size: ≈ 94K | why: TUI 캡처 임시 출력 | risk: 낮음. 참조 없음
- path: notes/runs/261028-usage-bar-capture.txt | tracked: yes | size: 254B | why: 임시 캡처 | risk: 낮음. 참조 없음
- path: notes/runs/261029-impl-as-test.jsonl | tracked: yes | size: 1.9K | why: 임시 jsonl (동명 .md는 노트) | risk: 낮음. 참조 없음
- path: scripts/*.py (charcount, claude_history, compress_archive, export_history, extract_transcript, search_history, usage_report) | tracked: yes | size: ≈ 24K | why: Go 포팅본 internal/scripts/*.go 존재 (scripts.go 주석 "ports of scripts/*.py") | risk: 중간. 코드 참조 없음. 단 scripts/build-release.sh는 keep. Go 포팅 동등성 확인 후 삭제
- path: tools/usage_by_task.py | tracked: yes | size: 3.9K | why: 배포 제외 대상(tools/build_release.py EXCLUDE) | risk: 낮음. 자체 docstring만 참조
- path: tools/build_release.py, tools/configure.py, tools/merge_settings.py | tracked: yes | size: 4.4K, 9.9K, 10.8K | why: 구 Python 빌드/설치 도구 | risk: 높음. configure.py는 win/install.ps1 및 win/build_win.py 경로 참조, merge_settings.py는 win/install.ps1:64 사용 → keep
- path: win/build_win.py | tracked: yes | size: 11.8K | why: Windows 패키지 빌드 | risk: 높음. tools/build_release.py:25에서 호출 → keep
- path: notes/spike-results.md | tracked: yes | size: 6.3K | why: 스파이크 결과 노트 (spike/ 디렉토리 이미 없음, 참조 깨짐) | risk: 중간. notes/DECISIONS.md 등 참조 여부 미확인
- path: ANALYSIS.md, PLAN_0.5.0.md, DESIGN_HANDOFF.md | tracked: yes | size: 9.1K, 21K, 16K | why: 구 계획·인수인계 문서 | risk: 중간. DESIGN_HANDOFF.md:78-82가 파일 목록을 기술, README.md:82 분석 문서로 안내 → 사용자 확인 후 삭제
- path: docs/claude-lean/ (APPLY_GUIDE.md, FRESH_START.md, MIGRATE_NOW_PROMPT.md, CHANGELOG.md, README.md) | tracked: yes | size: 13.7K, 5.8K, 3K, 12K, 6.8K | why: 루트 파일과 APPLY_GUIDE/FRESH_START/MIGRATE_NOW_PROMPT는 cmp 동일(중복), CHANGELOG만 상이, README는 원본 없음 | risk: 중간. win/install.ps1:53 및 win/install.ps1 메시지에서 docs\claude-lean\ 경로 참조 → 삭제 시 Windows 설치 깨짐. keep 또는 win 경로 수정 후 삭제
- path: notes/runs/research/ | tracked: no | size: 없음 (디렉토리 부재) | why: 중복 없음 | risk: -

## keep (참조 또는 현행 소스)
- path: history/20261027-0444-36fb8dbb-01.txt, history/INDEX.txt | tracked: yes | size: 255K, 105B | why: 대화 기록 보관 | risk: 높음. 사용자 기록
- path: fresh_start.sh, install.sh, platform_setup/*, mac/launcher.sh | tracked: yes | why: 설치 경로, tools/configure.py가 fresh_start.sh 호출 | risk: 높음
- path: scripts/build-release.sh | tracked: yes | why: .github/workflows/release.yml:16 사용 | risk: 높음
- path: templates/*, CLAUDE.md, README.md, CHANGELOG.md, VERSION, FRESH_START.md, APPLY_GUIDE.md, MIGRATE_NOW_PROMPT.md (루트) | tracked: yes | why: 현행 문서/템플릿 | risk: 높음
- path: .idea/* | tracked: no (ignored; .idea/.gitignore만 tracked) | size: 작음 | why: IDE 설정 | risk: 낮음 (삭제 대상 아님, 필요 시 IDE 재생성)

## not found / 미확인
- 'remote-only old claude-lean files' 목록: 작업 트리에서 특정 불가. git diff origin/main 필요 (run 노트 261029-release-v061.md에 목록 없음)
- dist/, notes/runs/research/: 부재
- spike/ 디렉토리(notes/spike-results.md 참조): 부재
- tests/test_acceptance.py (notes/spike-results.md 참조): 부재, tests/에는 test_hooks.py만 있음
- 바이트 단위 정확 크기 대신 du/wc 근사치
