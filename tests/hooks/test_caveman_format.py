#!/usr/bin/env python3
"""Tests for .claude/hooks/caveman-format.py (PostToolUse, never blocks).

Each run uses a temporary CLAUDE_PROJECT_DIR so notes/metrics in the repo is not written."""
import json, os, subprocess, sys, tempfile, unittest

HOOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".claude", "hooks", "caveman-format.py")
HOOK = os.path.normpath(HOOK)

VALID_RESPONSE = (
    "status: done\n"
    "reason: -\n"
    "changed: notes/x.md\n"
    "diffstat: 1 file changed\n"
    "check: python3 -m unittest -> PASS\n"
    "ws: none\n"
    "summary_ko: 완료\n"
)


def payload(response, guard_prompt="return: R-write, 300\nlang: ko\n"):
    return json.dumps({
        "tool_name": "Agent",
        "tool_input": {"subagent_type": "haiku-coder", "prompt": guard_prompt},
        "tool_response": response,
    }, ensure_ascii=False)


class CavemanFormatHookTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = self._tmp.name

    def tearDown(self):
        self._tmp.cleanup()

    def run_hook(self, stdin_text, guard=None):
        env = dict(os.environ)
        env["CLAUDE_PROJECT_DIR"] = self.root
        env.pop("CAVEMAN_GUARD", None)
        if guard is not None:
            env["CAVEMAN_GUARD"] = guard
        return subprocess.run([sys.executable, HOOK], input=stdin_text.encode("utf-8"),
                              capture_output=True, env=env, cwd=self.root, timeout=30)

    def test_malformed_stdin_exits_zero(self):
        r = self.run_hook("{not json")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.decode("utf-8").strip(), "")

    def test_valid_payload_guard_off_exits_zero_without_block(self):
        r = self.run_hook(payload(VALID_RESPONSE), guard="off")
        out = r.stdout.decode("utf-8")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn('"block"', out)
        self.assertEqual(out.strip(), "")

    def test_log_mode_never_blocks_and_records_missing_keys(self):
        bad = "status: done\nreason: -\nchanged: notes/x.md\n"  # diffstat/check/ws missing
        r = self.run_hook(payload(bad), guard="log")
        out = r.stdout.decode("utf-8")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("block", out)
        self.assertEqual(out.strip(), "")  # log mode: metrics row only, no additionalContext
        metrics_dir = os.path.join(self.root, "notes", "metrics")
        files = os.listdir(metrics_dir)
        self.assertEqual(len(files), 1)
        with open(os.path.join(metrics_dir, files[0]), encoding="utf-8") as f:
            row = json.loads(f.readline())
        self.assertEqual(row["agent"], "haiku-coder")
        self.assertIn("check", row["missing"])


if __name__ == "__main__":
    unittest.main()
