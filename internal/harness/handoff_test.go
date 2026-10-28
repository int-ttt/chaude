package harness

import (
	"strings"
	"testing"
)

func TestExtractRecoMarkdownMultiline(t *testing.T) {
	reply := "앞부분 설명\n추천: 첫 번째 (무시됨)\n\n## 결론\n" +
		"**추천:** 테스트를 먼저 실행하세요.\n두 번째 줄 세부 내용\n- 항목 A\n\n\n"
	want := "**추천:** 테스트를 먼저 실행하세요.\n두 번째 줄 세부 내용\n- 항목 A"
	if got := extractReco(reply); got != want {
		t.Fatalf("extractReco = %q, want %q", got, want)
	}
}

func TestExtractRecoNone(t *testing.T) {
	reply := "추천 없이 그냥 텍스트\n추천은 없음\n"
	if got := extractReco(reply); got != "" {
		t.Fatalf("extractReco = %q, want empty", got)
	}
}

func goodHandoffNote() string {
	return `## Goal
Implement handoff validation for the claude-harness REPL so that a fresh session can resume work without losing context.

## Decision
Use stdlib only. Keep the recommendation line verbatim.

## Files
- internal/harness/handoff.go: extractReco, validateHandoff, withReco, recoPreserved.
- internal/harness/handoff_test.go: unit tests for the four helpers.

## In-progress
Tests are written; verification command is pending.

## Open question
None.

## Preference
User prefers Korean replies and minimal diffs. Do not touch other files.

## Exact
Verify with go vet and go test -run Reco|Handoff in the module root.
`
}

var handoffChanged = []string{
	"internal/harness/handoff.go",
	"internal/harness/handoff_test.go",
}

func hasProblem(problems []string, want string) bool {
	for _, p := range problems {
		if p == want {
			return true
		}
	}
	return false
}

func TestValidateHandoffGood(t *testing.T) {
	if problems := validateHandoff(goodHandoffNote(), "", handoffChanged); len(problems) != 0 {
		t.Fatalf("validateHandoff good note: unexpected problems %q", problems)
	}
}

func TestValidateHandoffTooShort(t *testing.T) {
	problems := validateHandoff("## Goal\nshort", "", nil)
	if !hasProblem(problems, "too short") {
		t.Fatalf("validateHandoff short note: want \"too short\", got %q", problems)
	}
}

func TestValidateHandoffMissingSection(t *testing.T) {
	note := strings.Split(goodHandoffNote(), "## Exact")[0]
	problems := validateHandoff(note, "", handoffChanged)
	if !hasProblem(problems, "missing section: exact") {
		t.Fatalf("validateHandoff missing section: want \"missing section: exact\", got %q", problems)
	}
}

func TestValidateHandoffUnmentionedFile(t *testing.T) {
	changed := append([]string{"internal/harness/repl.go"}, handoffChanged...)
	problems := validateHandoff(goodHandoffNote(), "", changed)
	if !hasProblem(problems, "file not mentioned: repl.go") {
		t.Fatalf("validateHandoff unmentioned file: want \"file not mentioned: repl.go\", got %q", problems)
	}
}

func TestWithRecoRoundTripKorean(t *testing.T) {
	reply := "설명입니다.\n> **추천:** 다음 단계는 테스트 실행입니다.\n- 이유: 빠른 검증\n"
	reco := extractReco(reply)
	if reco != "> **추천:** 다음 단계는 테스트 실행입니다.\n- 이유: 빠른 검증" {
		t.Fatalf("extractReco = %q", reco)
	}
	snapshot := "## Goal\n핸드오프 테스트 작업 진행 중.\n"

	out := withReco(reco, snapshot)
	wantPrefix := "## Previous recommendation (verbatim)\n" + reco + "\n\n"
	if !strings.HasPrefix(out, wantPrefix) || !strings.HasSuffix(out, snapshot) {
		t.Fatalf("withReco = %q", out)
	}
	if !recoPreserved(reco, out) {
		t.Fatalf("recoPreserved(reco, withReco output) = false")
	}
	if recoPreserved(reco, snapshot) {
		t.Fatalf("recoPreserved(reco, snapshot without reco) = true, want false")
	}
}

func TestWithRecoEmpty(t *testing.T) {
	snapshot := "## Goal\n본문\n"
	if got := withReco("", snapshot); got != snapshot {
		t.Fatalf("withReco empty reco = %q, want snapshot unchanged", got)
	}
	if !recoPreserved("", snapshot) {
		t.Fatalf("recoPreserved empty reco = false, want true")
	}
}
