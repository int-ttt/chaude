"""Tests for .claude/hooks/haiku-read-guard.py: haiku-* subagents may not read a file over 400 lines in full, nor read
the same unchanged file in full twice in one session. Everything else passes; any error fails open.
The hook appends to logs/hooks.jsonl under the shared lock, so the lock file is checked too.

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
GUARD = os.path.join(ROOT, ".claude", "hooks", "haiku-read-guard.py")


def run_guard(payload, project, raw=None):
    env = dict(os.environ)
    env["CLAUDE_LEAN"] = "1"  # lean on: the guard is active
    env["CLAUDE_PROJECT_DIR"] = project
    for k in ("CLAUDE_ALLOW_MAIN_TOOLS", "CHAUDE_ORCH"):
        env.pop(k, None)
    return subprocess.run(
        [sys.executable, "-I", GUARD],
        input=raw if raw is not None else json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        cwd=ROOT,
    )


def read_payload(path, agent="haiku-scout", sid="s1", **tool_input):
    tin = {"file_path": path}
    tin.update(tool_input)
    return {"session_id": sid, "hook_event_name": "PreToolUse", "tool_name": "Read",
            "agent_type": agent, "tool_input": tin}


def decision(proc):
    if not proc.stdout.strip():
        return None
    return json.loads(proc.stdout)["hookSpecificOutput"]["permissionDecision"]


def reason(proc):
    return json.loads(proc.stdout)["hookSpecificOutput"]["permissionDecisionReason"]


class ReadGuardTest(unittest.TestCase):
    def setUp(self):
        self.project = tempfile.mkdtemp(prefix="readguard-")
        self.files = tempfile.mkdtemp(prefix="readguard-files-")
        self.big = os.path.join(self.files, "big.txt")
        with open(self.big, "w", encoding="utf-8") as f:
            f.write("line\n" * 500)
        self.small = os.path.join(self.files, "small.txt")
        with open(self.small, "w", encoding="utf-8") as f:
            f.write("a\nb\nc\n")

    def tearDown(self):
        shutil.rmtree(self.project, ignore_errors=True)
        shutil.rmtree(self.files, ignore_errors=True)

    def test_full_read_of_big_file_denied_for_haiku(self):
        proc = run_guard(read_payload(self.big), self.project)
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(decision(proc), "deny")
        self.assertIn("Grep", reason(proc))
        self.assertIn("offset/limit", reason(proc))

    def test_slice_of_big_file_allowed(self):
        self.assertIsNone(decision(run_guard(read_payload(self.big, offset=10, limit=50), self.project)))
        self.assertIsNone(decision(run_guard(read_payload(self.big, limit=50), self.project)))

    def test_big_file_allowed_for_main_thread_and_other_agents(self):
        main = {"session_id": "s1", "hook_event_name": "PreToolUse", "tool_name": "Read",
                "tool_input": {"file_path": self.big}}
        self.assertIsNone(decision(run_guard(main, self.project)))
        opus = read_payload(self.big, agent="opus-architect")
        self.assertIsNone(decision(run_guard(opus, self.project)))

    def test_repeat_full_read_denied_when_unchanged(self):
        self.assertIsNone(decision(run_guard(read_payload(self.small), self.project)))
        proc = run_guard(read_payload(self.small), self.project)
        self.assertEqual(decision(proc), "deny")
        self.assertIn("already read in full", reason(proc))

    def test_repeat_allowed_after_size_change(self):
        self.assertIsNone(decision(run_guard(read_payload(self.small), self.project)))
        with open(self.small, "a", encoding="utf-8") as f:
            f.write("d\n")
        self.assertIsNone(decision(run_guard(read_payload(self.small), self.project)))

    def test_repeat_allowed_after_mtime_change(self):
        self.assertIsNone(decision(run_guard(read_payload(self.small), self.project)))
        st = os.stat(self.small)
        os.utime(self.small, ns=(st.st_atime_ns, st.st_mtime_ns + 2_000_000_000))
        self.assertIsNone(decision(run_guard(read_payload(self.small), self.project)))

    def test_state_is_per_session(self):
        self.assertIsNone(decision(run_guard(read_payload(self.small, sid="s1"), self.project)))
        self.assertIsNone(decision(run_guard(read_payload(self.small, sid="s2"), self.project)))
        self.assertTrue(os.path.exists(os.path.join(self.project, "logs", ".read-seen-s1.json")))
        self.assertTrue(os.path.exists(os.path.join(self.project, "logs", ".read-seen-s2.json")))

    def test_no_session_id_skips_repeat_check(self):
        self.assertIsNone(decision(run_guard(read_payload(self.small, sid=""), self.project)))
        self.assertIsNone(decision(run_guard(read_payload(self.small, sid=""), self.project)))

    def test_lock_file_created_and_log_line_written(self):
        run_guard(read_payload(self.big), self.project)
        self.assertTrue(os.path.exists(os.path.join(self.project, "logs", ".hooks.lock")))
        with open(os.path.join(self.project, "logs", "hooks.jsonl"), encoding="utf-8") as f:
            rec = json.loads(f.readline())
        self.assertEqual(rec["hook"], "haiku-read-guard")
        self.assertEqual(rec["decision"], "deny")

    def test_malformed_input_fails_open(self):
        proc = run_guard(None, self.project, raw="{not json")
        self.assertEqual(proc.returncode, 0)
        self.assertIsNone(decision(proc))

    def test_missing_file_fails_open(self):
        proc = run_guard(read_payload(os.path.join(self.files, "nope.txt")), self.project)
        self.assertEqual(proc.returncode, 0)
        self.assertIsNone(decision(proc))


if __name__ == "__main__":
    unittest.main()
