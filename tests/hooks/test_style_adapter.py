"""Tests for the inbox/imperative routing in .claude/hooks/style-adapter.py.

English statements ("The build is green now.", "Tests pass now.") go to the inbox like their Korean equivalents;
English imperatives need a sentence-initial verb or please/can you. Korean routing is covered by the second half.

Run: cd <repo root> && python3 -I -m unittest discover -s tests/hooks -p 'test_*.py' -v
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ADAPTER = os.path.join(ROOT, ".claude", "hooks", "style-adapter.py")


def route(prompt: str) -> str:
    """'inbox' (kept in record/INBOX.md, no model call), 'model' (no tag, sent to the model) or the tag name."""
    with tempfile.TemporaryDirectory() as d:
        env = dict(os.environ, CLAUDE_PROJECT_DIR=d, HARNESS_RECORD_DIR=os.path.join(d, "record"), CLAUDE_LEAN="1")
        env.pop("STYLE_INBOX", None)
        proc = subprocess.run([sys.executable, "-I", ADAPTER],
                              input=json.dumps({"prompt": prompt, "session_id": "test"}),
                              capture_output=True, text=True, env=env)
        if proc.returncode != 0:
            raise AssertionError(proc.stderr)
        if os.path.exists(os.path.join(d, "record", "INBOX.md")):
            return "inbox"
        out = proc.stdout.strip()
        if '"decision"' in out:
            return "block"
        if out.startswith("["):
            return out.split("]")[0][1:]
        return "model"


class EnglishRouting(unittest.TestCase):
    def test_bare_verb_at_end_is_not_imperative(self):
        # "build" as a keyword anywhere no longer makes a statement an instruction
        self.assertEqual(route("The build is green now."), "inbox")

    def test_test_status_statement_goes_to_inbox(self):
        self.assertEqual(route("Tests pass now."), "inbox")

    def test_sentence_initial_verb_is_imperative(self):
        self.assertNotEqual(route("Build it"), "inbox")
        self.assertEqual(route("Build it"), "frag")

    def test_please_is_imperative(self):
        self.assertNotEqual(route("please build"), "inbox")
        self.assertEqual(route("please build"), "frag")

    def test_question_still_goes_to_model(self):
        self.assertEqual(route("Tests pass now?"), "model")

    def test_can_you_goes_to_model(self):
        self.assertEqual(route("Can you build it"), "model")


class KoreanRouting(unittest.TestCase):
    def test_status_statement_goes_to_inbox(self):
        self.assertEqual(route("빌드 초록색이야."), "inbox")

    def test_past_change_goes_to_inbox(self):
        self.assertEqual(route("어제 설정 파일을 바꿨거야."), "inbox")

    def test_instruction_is_not_inboxed(self):
        self.assertEqual(route("이거 확인해봐"), "frag")


if __name__ == "__main__":
    unittest.main()
