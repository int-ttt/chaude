"""Tests for the correction tags in .claude/hooks/correction-guard.py.

True corrections keep their tag; questions alone, long messages, multi-line pastes and plain imperatives get none.
The routing tag letter is the class: C fact, D missing change, A misread, E pushback, G tone, "generic" = untyped [corr].

Run: cd <repo root> && python3 -I -m unittest discover -s tests/hooks -p 'test_correction_guard.py' -v
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CORRECTION = os.path.join(ROOT, ".claude", "hooks", "correction-guard.py")


def run_hook(script: str, prompt: str) -> str:
    with tempfile.TemporaryDirectory() as d:
        env = dict(os.environ, CLAUDE_PROJECT_DIR=d, CLAUDE_LEAN="1")
        env.pop("STYLE_INBOX", None)
        proc = subprocess.run([sys.executable, "-I", script],
                              input=json.dumps({"prompt": prompt, "session_id": "test"}),
                              capture_output=True, text=True, env=env)
        if proc.returncode != 0:
            raise AssertionError(proc.stderr)
        return proc.stdout.strip()


def tag(prompt: str) -> str:
    """'' (no tag), 'generic' (untyped [corr]) or the class letter of [corr X]. [corr x2] stop lines are ignored."""
    out = run_hook(CORRECTION, prompt)
    m = re.search(r"^\[corr (\w)\]", out, re.M)
    if m:
        return m.group(1)
    if re.search(r"^\[corr\] ", out, re.M):
        return "generic"
    return ""


# Pasted tool log: several lines, with words that would match markers if they were scanned line by line.
TOOL_LOG = (
    "$ cargo test -q -p chaude-core\n"
    "error[E0425]: cannot find value `x` in this scope\n"
    "  --> crates/core/src/lib.rs:3:5\n"
    "   |\n"
    "3  | x 아니라 y이다\n"
    "   | ^ 빠졌어? 안 바뀐 것 같은데\n"
)


class CorrectionFalsePositives(unittest.TestCase):
    def test_video_question_is_not_tagged(self):
        self.assertEqual(tag("그 영상에서 claude.md의 맨 위 200줄은 ... 연구결과? 가 있다고 하던데"), "")

    def test_multi_line_tool_log_is_not_tagged(self):
        self.assertEqual(tag(TOOL_LOG), "")

    def test_long_instruction_is_not_tagged(self):
        self.assertEqual(tag("그러니까 이거에 대해서 원인을 파악하고, 현재 존재하는 문서들에 대해서 고칠 부분이 있는가에 대해서 판단해."), "")

    def test_next_release_instruction_is_not_tagged(self):
        self.assertEqual(tag("그리고 다음 릴리즈 해"), "")

    def test_question_alone_is_not_tagged(self):
        self.assertEqual(tag("아니야?"), "")
        self.assertEqual(tag("그 파일 정리 했어?"), "")
        self.assertEqual(tag("이 함수는 로그 말고 파일에 씀."), "")

    def test_over_300_chars_is_not_tagged(self):
        self.assertEqual(tag("아니 " + "가나다라 " * 60), "")


class CorrectionTruePositives(unittest.TestCase):
    def test_fact_short_form(self):
        self.assertEqual(tag("아니 A가 아니라 B야"), "C")
        self.assertEqual(tag("그게 아니라 이거야"), "C")

    def test_missing_change(self):
        self.assertEqual(tag("바꾼거 맞아?"), "D")
        self.assertEqual(tag("안 바뀐 것 같은데"), "D")

    def test_untyped_no_at_start(self):
        self.assertEqual(tag("아니 그거 말고 다른거"), "generic")
        self.assertEqual(tag("아니 그냥 빼"), "generic")

    def test_misread_at_start(self):
        self.assertEqual(tag("그거 말고 저거 해"), "A")


if __name__ == "__main__":
    unittest.main()
