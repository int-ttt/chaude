# chaude 사용자 가이드

`chaude`는 Claude Code를 감싸서 긴 작업을 이어 가도록 돕는 단일 실행 파일입니다. 인자 없이 `chaude`를 실행하면 대화형 화면(REPL)이 열립니다. 대화가 길어지면 자동으로 세션을 넘기고, 서브에이전트와 사용량을 한 화면에서 보여 줍니다.

## 문서 목록

- [repl.md](repl.md): 대화형 화면의 모든 기능. 입력 키, 상태 줄, 권한 모드, `/` 명령, `/resume`, 에이전트 보기, 설정 화면을 설명합니다.
- [cli.md](cli.md): 설치 방법(macOS, Linux, Windows), 하위 명령, 옵션, 환경변수, 도우미 스크립트를 설명합니다.
- [models-and-agents.md](models-and-agents.md): 모델과 에이전트 구성을 설명합니다.
- [hooks-and-skills.md](hooks-and-skills.md): 훅과 스킬을 설명합니다.

## 처음 시작하기

1. 설치합니다. 방법은 [cli.md](cli.md#설치)를 보세요.
2. 프로젝트 폴더에서 `chaude`를 실행합니다.
3. 입력창에 질문을 쓰고 Enter를 누릅니다. 기본 모델은 opus입니다.
4. 화면 아래 상태 줄에서 컨텍스트 사용량과 5시간 사용량을 확인합니다.

처음 쓰는 분은 [repl.md](repl.md)의 "시작하기"와 "입력 키"부터 읽으세요.

## 자주 찾는 기능

| 하고 싶은 일 | 명령 또는 키 | 자세히 |
|---|---|---|
| 권한 모드 바꾸기 | Shift+Tab | [repl.md](repl.md) |
| 이전 대화 불러오기 | `/resume` | [repl.md](repl.md) |
| 설정 바꾸기 | `/settings` | [repl.md](repl.md) |
| 대화 기록 검색하기 | `chaude search-history "검색어"` | [cli.md](cli.md) |
| Claude Code 세션 이어받기 | `chaude --from-claude latest` | [cli.md](cli.md) |

## 문서 규칙

- 명령과 키는 코드 기준으로 적었습니다. 버전이 바뀌면 `chaude --help`와 함께 확인하세요.
- 이 가이드의 기준 버전은 0.9.13입니다.
