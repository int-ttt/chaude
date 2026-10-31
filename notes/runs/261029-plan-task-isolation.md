# 261029 plan: task isolation for multi-window Claude Code

## Evidence (확인된 사실)
- 병렬 headless claude 3개(각자 별도 하위 디렉터리) 동시 실행 PASS: 전부 rc 0, 실행 구간 겹침, wall 3.1s vs 합 9.1s (261029-test-parallel-terminals.md).
- 위 테스트는 haiku, max-turns 1, 저장소 밖(/tmp). 같은 저장소 파일 편집 충돌은 검증하지 않음.
- 브리프 제공 안전 노트: 같은 폴더 편집 충돌 가능 / ~/.claude 공유 상태 / harness가 notes/LEDGER.md와 metrics를 hook 쪽에서 기록 / 구독 한도 공유 / 캐시는 세션별.
- git status: 현재 master 브랜치(main 아님), 미커밋 변경 다수. 가정: worktree 생성 시 미커밋 변경은 따라가지 않음(git 동작상 일반적 사실이나 이 저장소에서 직접 확인 안 함).

## 공통 한계 (어느 옵션도 해결 못 함)
- 구독 사용 한도: 모든 창이 공유. 창 수만큼 소진 속도 증가. (A~D 동일)
- ~/.claude 공유 상태(설정, 세션 기록, 히스토리): 디렉터리를 나눠도 공유. 동시 쓰기 충돌 정도는 가정(미검증).
- 캐시: 세션별이라 창을 나누면 각 창이 캐시를 따로 만든다. 격리의 비용이지 결함이 아님.
- metrics/ledger가 hook-side라 기록 경로가 repo 상대인지 절대인지에 따라 결과가 갈림. 가정: 상대경로이면 worktree별로 자동 분리됨. 확인 필요.

## A. Task당 git worktree (브랜치 분리, notes/ worktree별, 브랜치로 병합)
- 격리: 소스·notes·LEDGER·metrics 파일 전부 물리적으로 분리. 편집 충돌이 병합 시점으로 이동.
- 비격리: ~/.claude, 구독 한도, 캐시(어차피 세션별). .git 객체 저장소는 공유(동시 git 명령 락 가능, 드묾).
- 병합 비용: 중간. notes/LEDGER.md, metrics/*.jsonl, DECISIONS.md는 append 성격이라 같은 줄 영역 충돌 빈발 예상(가정). 소스는 브랜치 병합으로 정상 처리. jsonl은 union 병합 가능(.gitattributes merge=union, 가정: 순서 무관할 때만).
- 유실 위험: 낮음. 커밋만 하면 브랜치에 남음. 위험은 미커밋 상태로 worktree 삭제, 또는 현재처럼 미커밋 변경이 많은 master에서 분기할 때 기준점 혼동.
- 구현 노력: 낮음~중간. `git worktree add ../claude-harness-<id> -b task/<id>` + 창에서 그 경로로 cd. harness 코드 변경 없음이 목표. 단 harness가 cwd나 repo 루트를 가정하는 경로(설정, notes 위치)가 있으면 수정 필요(미확인).

## B. Task당 디렉터리 + notes/ 복사 또는 symlink
- 격리: 복사본이면 소스까지 분리하나 git 이력 연결 없음. symlink notes/면 notes는 공유라 notes 충돌은 그대로.
- 비격리: ~/.claude, 한도, 캐시. symlink 쓰면 notes 동시 쓰기 비격리.
- 병합 비용: 높음. 복사본은 수동 diff/복사, 기준점 없음. symlink는 병합은 없으나 충돌이 실시간 발생.
- 유실 위험: 높음. 복사본끼리 어긋남, 마지막 쓴 쪽이 덮어쓰기, symlink 대상 삭제 시 연쇄.
- 구현 노력: 낮음(스크립트 몇 줄)이나 안전장치를 직접 만들어야 해서 A의 열화판. git이 이미 주는 것을 재구현.

## C. 한 repo + notes/tasks/<id>/ 네임스페이스 + lock 파일
- 격리: notes 쓰기만 분리(작업별 폴더). 소스 파일은 격리 안 됨.
- 비격리: 소스 동시 편집(가장 큰 위험), ~/.claude, 한도, 캐시. lock은 협조적(advisory)이라 우회 가능하고 stale lock 위험.
- 병합 비용: 없음(한 작업 트리). 대신 충돌은 편집 시점에 조용히 덮어쓰기로 나타남.
- 유실 위험: 중간~높음. 두 창이 같은 파일을 편집하면 마지막 저장 승리. git 커밋이 한 인덱스를 공유해 서로의 변경을 섞어 커밋할 수 있음.
- 구현 노력: 높음. hook이 LEDGER/metrics 경로를 task id별로 바꾸도록 harness 수정 + lock 획득/해제/타임아웃 로직 + task id 전달 방법(환경변수 등). 코드 변경 범위가 가장 큼.

## D. 아무것도 안 함 (한 창 편집, 나머지 읽기 전용)
- 격리: 쓰기 경합 자체가 없음. 규율에 의존.
- 비격리: 동일. 읽기 전용 창이 실수로 편집/커밋하면 막을 장치 없음(권한 설정으로 보강 가능, 가정).
- 병합 비용: 0.
- 유실 위험: 낮음(규율 지키면). 병렬 처리량 이득은 읽기/분석/조사에 한정. 편집 병렬화 불가.
- 구현 노력: 0. 보강하려면 읽기 창에 plan/read-only 권한 모드 사용(이미 있는 기능이라 가정).

## 비교 요약
| 항목 | A worktree | B 디렉터리 | C 네임스페이스+lock | D 무조치 |
|---|---|---|---|---|
| 소스 편집 격리 | 완전 | 복사본만 | 없음 | 규율 |
| notes/ledger 격리 | 완전 | 복사본만 | 부분 | 규율 |
| 병합 비용 | 중 | 높음 | 없음(대신 덮어쓰기) | 0 |
| 유실 위험 | 낮음 | 높음 | 중~높음 | 낮음 |
| 구현 노력 | 낮~중 | 낮음 | 높음 | 0 |
| 편집 병렬화 | 가능 | 가능 | 위험 | 불가 |

## 판단
- 지금 채택: D를 기본으로 하되, 편집 병렬이 실제로 필요한 작업만 A(worktree). 즉 "기본 D, 필요 시 A". B와 C는 기각: B는 git의 열화판, C는 harness 코드 변경이 가장 크면서 소스 충돌을 못 막음.
- 근거: 현재 검증된 것은 headless 병렬 동작뿐이고 편집 충돌 빈도는 미측정. 미커밋 변경이 많은 master에서 곧바로 worktree 체제로 가면 기준점 혼동이 더 큼. A는 코드 변경 없이 시작 가능해 되돌리기 쉬움.
- 판단 전환 조건: 편집 병렬 작업이 주 1회 이상 반복되거나 D에서 실제 덮어쓰기/섞인 커밋이 한 번이라도 발생하면 A를 기본으로 격상. LEDGER/metrics 병합 충돌이 매번 수작업이면 그때 hook 경로 분리(C의 notes 부분만) 검토.
- 한도가 병목이면 어떤 옵션도 도움 안 됨: 창 수를 줄이는 것이 답.

## 먼저 확인할 것 (구체 1건)
임시 worktree(`git worktree add /tmp/wt-test -b task/test`)에서 harness 환경(hook 포함)으로 짧은 headless 호출을 1회 실행한 뒤, LEDGER.md와 metrics jsonl이 어느 경로(worktree 안 vs 원본 repo)에 기록됐는지 확인. worktree 안이면 A는 코드 변경 없이 가능, 원본이면 hook 경로 수정 필요. 확인 후 `git worktree remove /tmp/wt-test`, 브랜치 삭제.
