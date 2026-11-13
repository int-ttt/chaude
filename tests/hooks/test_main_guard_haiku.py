"""Tests for the Haiku-main allow-list in .claude/hooks/main-thread-guard.py.

Haiku main may call only opus-architect and haiku-output; Opus/fable/sonnet main keeps the old rules.
The main model comes from the newest assistant entry of transcript_path (or a "model" field in the input).

Run: cd <repo root> && /usr/local/bin/python3 -I -m unittest discover -s tests/hooks -p 'test_*.py' -v
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
GUARD = os.path.join(ROOT, ".claude", "hooks", "main-thread-guard.py")


def run_guard(payload: dict) -> str:
    env = dict(os.environ)
    env["CLAUDE_LEAN"] = "1"
    for k in ("CLAUDE_ALLOW_MAIN_TOOLS", "CHAUDE_ORCH"):
        env.pop(k, None)
    proc = subprocess.run(
        [sys.executable, "-I", GUARD],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        cwd=ROOT,
    )
    return proc.stdout.strip()


def decision(out: str) -> str:
    return json.loads(out)["hookSpecificOutput"]["permissionDecision"] if out else "allow"


class HaikuMainGuardTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="main-guard-haiku-")
        cls.haiku = cls._transcript("haiku", "claude-haiku-5-5")
        cls.opus = cls._transcript("opus", "claude-opus-5-5")
        cls.empty = os.path.join(cls.tmp, "empty.jsonl")
        open(cls.empty, "w", encoding="utf-8").close()

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    @classmethod
    def _transcript(cls, name, model):
        path = os.path.join(cls.tmp, f"{name}.jsonl")
        rows = [
            {"type": "user", "message": {"role": "user", "content": "hi"}},
            {"type": "assistant", "message": {"model": model, "content": [{"type": "text", "text": "ok"}]}},
            {"type": "user", "message": {"role": "user", "content": "x" * 50_000}},
        ]
        with open(path, "w", encoding="utf-8") as fh:
            for r in rows:
                fh.write(json.dumps(r, ensure_ascii=False) + "\n")
        return path

    def agent(self, kind, transcript):
        return {
            "tool_name": "Agent",
            "tool_input": {"subagent_type": kind, "prompt": "p"},
            "transcript_path": transcript,
        }

    def test_1_haiku_main_scout_denied(self):
        out = run_guard(self.agent("haiku-scout", self.haiku))
        self.assertEqual(decision(out), "deny")
        self.assertIn("opus-architect", json.loads(out)["hookSpecificOutput"]["permissionDecisionReason"])

    def test_2_haiku_main_coder_denied(self):
        self.assertEqual(decision(run_guard(self.agent("haiku-coder", self.haiku))), "deny")

    def test_3_haiku_main_default_agent_denied(self):
        self.assertEqual(decision(run_guard(self.agent("", self.haiku))), "deny")

    def test_4_haiku_main_opus_architect_allowed(self):
        self.assertEqual(decision(run_guard(self.agent("opus-architect", self.haiku))), "allow")

    def test_5_haiku_main_haiku_output_allowed(self):
        self.assertEqual(decision(run_guard(self.agent("haiku-output", self.haiku))), "allow")

    def test_6_opus_main_scout_allowed(self):
        self.assertEqual(decision(run_guard(self.agent("haiku-scout", self.opus))), "allow")

    def test_7_opus_main_architect_allowed(self):
        self.assertEqual(decision(run_guard(self.agent("opus-architect", self.opus))), "allow")

    def test_8_unknown_model_keeps_old_rules(self):
        self.assertEqual(decision(run_guard(self.agent("haiku-scout", self.empty))), "allow")

    def test_9_model_field_in_input_wins(self):
        payload = self.agent("haiku-scout", self.opus)
        payload["model"] = "claude-haiku-5-5"
        self.assertEqual(decision(run_guard(payload)), "deny")

    def test_10_haiku_main_work_tool_still_denied(self):
        out = run_guard({"tool_name": "Read", "tool_input": {"file_path": "x"}, "transcript_path": self.haiku})
        self.assertEqual(decision(out), "deny")


if __name__ == "__main__":
    unittest.main()
