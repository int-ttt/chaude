# 완전 초기화 가이드 — 중요한 내용만 남기고 새로 시작

어떤 프로젝트에도 같은 방식으로 적용합니다. 오래된 대화 기록에서 **나중에도 필요한 내용만** 그 프로젝트의 `notes/`로 뽑아내고, 기존 기록은 보관한 뒤 깨끗한 상태에서 시작합니다. 프로젝트마다 `notes/`가 따로 생기므로 내용이 섞이지 않습니다.

## 무엇이 남는가
| 파일 | 내용 | 예 |
|---|---|---|
| `notes/FACTS.md` | 사용자·프로젝트·환경에 대한 사실 | 역할, 날짜, 수치, 버전, 하드웨어, 제약 |
| `notes/DECISIONS.md` | 정한 것, 규칙 | "A안 채택", "2줄 이내", "10pt", "PR은 squash" |
| `notes/DELIVERABLES.md` | 최종 산출물 위치 | 최종 문서·결과물 경로, 구버전 표시 |
| `notes/OPEN.md` | 아직 남은 일, 마감 | 마감일, 대기 중인 답변, 미해결 버그 |
| `notes/LESSONS.md` | 실패한 방법과 이유 | 서비스 제한, 설정 문제, 잘못된 접근 |

버려지는 것: 대화 흐름, 중간 초안, 해결된 논쟁, 진행 보고, 도구 출력.

## 비용 비교
| 방법 | 비용 | 빠짐 |
|---|---|---|
| **A. 기록 파일 추출 (권장)** | Haiku 여러 개 + Sonnet 1회. 대형 프로젝트도 약 $0.5 미만 | 적음. 자동 압축 전 원문까지 읽음 |
| B. 기존 세션에서 직접 정리 | 큰 세션을 다시 여는 비용(500k면 약 $4) + 출력 | 압축되어 사라진 내용은 못 살림 |

---

## A. 기록 파일 추출 (권장)

### 1. 설치·추출·보관 (터미널, 모델 호출 없음)
이 프로젝트들의 **Claude 창을 모두 닫고** 실행합니다. 프로젝트는 하나든 여러 개든 같은 명령입니다.
```bash
cd ~/claude-opus-lean
./fresh_start.sh <프로젝트 폴더> [<프로젝트 폴더> ...]
```
예:
```bash
./fresh_start.sh ~/Documents/report ~/code/webapp ~/code/analysis --main sonnet
```
프로젝트마다 하는 일:

| 순서 | 내용 |
|---|---|
| 설치 | `install.sh` 실행 (기존 설정 백업 후 병합) |
| 추출 | 그 프로젝트의 대화 기록 → `notes/distill/chunks/` (사용자 메시지·답변 본문만, 7천 자 조각) |
| 보관 | 기존 기록 폴더 → `~/claude-archive/<프로젝트명>-<날짜>` (삭제 아님) |
| 건너뛰기 | 대화 기록이 없으면 설치만 / 추출된 메시지가 0개면 보관하지 않고 멈춤 |

- **옵션 `--main sonnet`:** 메인 모델을 Sonnet으로 설치합니다 (기본 Opus). 판단이 적은 프로젝트에 씁니다. 프로젝트별로 다르게 하려면 나눠서 실행하세요.
- **옵션 `--force`:** 다른 프로젝트의 Claude만 열려 있을 때, 실행 중 확인을 건너뜁니다.
- 적용 전 사용량 기준값은 `~/usage-before.txt`에 처음 한 번만 저장됩니다.

### 2. 새 세션에서 정리 (프로젝트마다)
```bash
cd <프로젝트 폴더> && claude
```
```
/distill
```
1. Haiku `haiku-extractor`들이 조각 4개씩 나눠 동시에 읽고 후보를 `notes/distill/cand/`에 저장합니다.
2. Sonnet `sonnet-analyst`가 합칩니다. 정정과 최신 내용을 우선하고, 중복과 끝난 일은 버립니다. `haiku-batch-worker`가 결과를 `notes/*.md` 5개로 저장합니다.
3. Haiku `haiku-checker`가 산출물 경로가 실제로 있는지 확인합니다.
4. 애매하거나 충돌하는 항목은 `notes/distill/REVIEW.md`에 번호 질문으로 정리됩니다.

### 3. 확인 질문에 답하기
`notes/distill/REVIEW.md`를 보고 짧게 답합니다.
```
1 맞음, 2 B로, 3 삭제, 4 모름
```
반영된 뒤 다음 두 가지를 제안받습니다. "ㅇㅇ"이면 실행합니다.
- 구버전 파일을 `archive/`로 이동
- CLAUDE.md 아래에 남은 예전 내용 정리 (이미 notes로 옮겨졌으므로)

### 4. 마무리
- `notes/` 5개 파일을 한 번 훑어보세요. 앞으로의 기억은 이 파일들입니다.
- `/memory`로 함께 불러와지는 다른 메모리 파일(사용자 전역 CLAUDE.md 등)이 있는지 확인하고, 프로젝트 전용 내용이 섞여 있으면 정리하세요.
- 같은 세션에서 그대로 작업을 시작하면 됩니다. 이후로는 `/clear`가 필요 없습니다. 예:
```
notes/OPEN.md 기준으로 오늘 할 일부터 시작하자
```

---

## 수동으로 할 때 (스크립트 없이)
```bash
P=<프로젝트 폴더>
cd ~/claude-opus-lean && ./install.sh "$P"
cd "$P" && python3 scripts/extract_transcript.py "$P" --out notes/distill
ENC=$(python3 -c "import re,sys;print(re.sub(r'[^A-Za-z0-9]','-',sys.argv[1]))" "$(cd "$P" && pwd)")
mkdir -p ~/claude-archive && mv ~/.claude/projects/"$ENC" ~/claude-archive/"$(basename "$P")"-$(date +%Y%m%d)
```
기록 폴더를 못 찾으면 `ls ~/.claude/projects`에서 이름을 확인해 `extract_transcript.py`에 그 폴더를 직접 지정하세요.

---

## B. 기존 세션 안에서 바로 정리 (빠르지만 비쌈)
기록 파일이 이미 지워졌거나 한 번에 끝내고 싶을 때만 씁니다. 기존 세션에 아래를 **한 번만** 붙여넣습니다.
```
새 작업은 하지 마. 완전 초기화 전에 이 대화에서 나중에도 필요한 내용만 notes/DISTILL.md 한 파일에 한 번에 써.
섹션: FACTS(내가 말하거나 정정한 사실) / DECISIONS(정한 것, 규칙) / DELIVERABLES(최종 파일 경로, 구버전 표시) / OPEN(남은 일·마감) / LESSONS(실패한 방법과 이유) / REVIEW(애매하거나 충돌하는 것, 번호 질문).
한 줄씩, 정정된 내용은 최신 것만, 대화 요약·문서 본문·해결된 논쟁은 빼. 파일을 다시 읽거나 검증하지 말고 쓰고 바로 멈춰.
```
그다음 세션을 닫고 A-1(`fresh_start.sh`)을 실행한 뒤, A-2에서 `/distill` 대신 이렇게 입력합니다.
```
notes/DISTILL.md를 haiku-batch-worker로 notes/ 5개 파일과 notes/distill/REVIEW.md로 나눠
```
