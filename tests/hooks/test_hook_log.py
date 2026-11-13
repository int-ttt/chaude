"""Tests for the hook run log (D5): each hook appends one JSON line {ts, hook, event, decision}
to <CLAUDE_PROJECT_DIR>/logs/hooks.jsonl at exit, and a logging failure never changes the hook's output.

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
HOOKS = os.path.join(ROOT, ".claude", "hooks")


def run_hook(script: str, payload: dict, project: str) -> subprocess.CompletedProcess:
    env = dict(os.environ)
    env["CLAUDE_LEAN"] = "1"
    env["CLAUDE_PROJECT_DIR"] = project
    for k in ("CLAUDE_ALLOW_MAIN_TOOLS", "CHAUDE_ORCH"):
        env.pop(k, None)
    return subprocess.run(
        [sys.executable, "-I", os.path.join(HOOKS, script)],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        cwd=ROOT,
    )


def log_lines(project: str) -> list:
    path = os.path.join(project, "logs", "hooks.jsonl")
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


class HookLogTest(unittest.TestCase):
    def setUp(self):
        self.project = tempfile.mkdtemp(prefix="hooklog-")

    def tearDown(self):
        shutil.rmtree(self.project, ignore_errors=True)

    def test_deny_is_logged_with_event_and_decision(self):
        # main thread (no agent_id) using a work tool: main-thread-guard denies
        proc = run_hook("main-thread-guard.py",
                        {"session_id": "t1", "hook_event_name": "PreToolUse", "tool_name": "Read", "tool_input": {}},
                        self.project)
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(json.loads(proc.stdout)["hookSpecificOutput"]["permissionDecision"], "deny")
        lines = log_lines(self.project)
        self.assertEqual(len(lines), 1)
        self.assertEqual(set(lines[0]), {"ts", "hook", "event", "decision"})
        self.assertEqual(lines[0]["hook"], "main-thread-guard")
        self.assertEqual(lines[0]["event"], "PreToolUse")
        self.assertEqual(lines[0]["decision"], "deny")
        self.assertTrue(lines[0]["ts"].endswith("Z"))

    def test_pass_is_logged_as_pass(self):
        # subagent (agent_id set): main-thread-guard lets the call through silently
        proc = run_hook("main-thread-guard.py",
                        {"session_id": "t2", "hook_event_name": "PreToolUse", "tool_name": "Read",
                         "tool_input": {}, "agent_id": "a1"},
                        self.project)
        self.assertEqual(proc.stdout.strip(), "")
        lines = log_lines(self.project)
        self.assertEqual(len(lines), 1)
        self.assertEqual(lines[0]["decision"], "pass")

    def test_inject_is_logged_for_prompt_hook(self):
        # correction-guard on a plain prompt prints nothing: pass; on a correction it injects
        proc = run_hook("correction-guard.py",
                        {"session_id": "t3", "hook_event_name": "UserPromptSubmit", "prompt": "아니 그게 아니라 다시"},
                        self.project)
        self.assertEqual(proc.returncode, 0)
        self.assertTrue(proc.stdout.strip())
        lines = log_lines(self.project)
        self.assertEqual(lines[-1]["hook"], "correction-guard")
        self.assertEqual(lines[-1]["event"], "UserPromptSubmit")
        self.assertEqual(lines[-1]["decision"], "inject")

    def test_bad_input_still_exits_and_is_logged(self):
        # unparseable stdin exits early (before data is read); the record still lands with event ""
        env = dict(os.environ, CLAUDE_LEAN="1", CLAUDE_PROJECT_DIR=self.project)
        proc = subprocess.run([sys.executable, "-I", os.path.join(HOOKS, "agent-model-pin.py")],
                              input="not json", capture_output=True, text=True, env=env, cwd=ROOT)
        self.assertEqual(proc.returncode, 0)
        lines = log_lines(self.project)
        self.assertEqual(len(lines), 1)
        self.assertEqual(lines[0]["hook"], "agent-model-pin")
        self.assertEqual(lines[0]["event"], "")
        self.assertEqual(lines[0]["decision"], "pass")

    def test_unwritable_log_dir_does_not_change_output(self):
        # CLAUDE_PROJECT_DIR is a file, so logs/ cannot be created: the hook still denies, exit 0
        blocker = os.path.join(self.project, "not-a-dir")
        open(blocker, "w").close()
        proc = run_hook("main-thread-guard.py",
                        {"session_id": "t5", "hook_event_name": "PreToolUse", "tool_name": "Read", "tool_input": {}},
                        blocker)
        self.assertEqual(proc.returncode, 0)
        self.assertEqual(json.loads(proc.stdout)["hookSpecificOutput"]["permissionDecision"], "deny")


if __name__ == "__main__":
    unittest.main()
