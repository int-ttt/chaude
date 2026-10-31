"""Tests for the CHAUDE_ORCH=e gate in .claude/hooks/agent-model-pin.py and main-thread-guard.py.

Run: cd <repo root> && python3 -I -m unittest discover -s tests/hooks -p 'test_*.py' -v
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from typing import Optional

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PIN = os.path.join(ROOT, ".claude", "hooks", "agent-model-pin.py")
GUARD = os.path.join(ROOT, ".claude", "hooks", "main-thread-guard.py")


def run_hook(hook: str, payload: dict, orch: Optional[str] = None, lean: Optional[str] = "1") -> str:
    env = dict(os.environ)
    if lean is None:
        env.pop("CLAUDE_LEAN", None)
    else:
        env["CLAUDE_LEAN"] = lean
    env.pop("CLAUDE_ALLOW_MAIN_TOOLS", None)
    env.pop("CHAUDE_ORCH", None)
    if orch is not None:
        env["CHAUDE_ORCH"] = orch
    proc = subprocess.run(
        [sys.executable, "-I", hook],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        env=env,
        cwd=ROOT,
    )
    return proc.stdout.strip()


class OrchGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmpdir = tempfile.mkdtemp(prefix="orch-gate-")
        cls.big = os.path.join(cls.tmpdir, "big.txt")
        with open(cls.big, "wb") as fh:
            fh.write(b"x" * 10000)

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmpdir, ignore_errors=True)

    def agent_payload(self, subagent_type, prompt, agent_id=None):
        p = {"tool_name": "Agent", "tool_input": {"subagent_type": subagent_type, "prompt": prompt}}
        if agent_id:
            p["agent_id"] = agent_id
        return p

    def reason(self, out):
        return json.loads(out)["hookSpecificOutput"]["permissionDecisionReason"] if out else ""

    def test_1_haiku_small_cap_allowed(self):
        prompt = f"Summarize {self.big} briefly.\nRETURN: <=2000 chars"
        out = run_hook(PIN, self.agent_payload("haiku-scout", prompt), orch="e")
        self.assertNotIn("E gate", out)

    def test_2_haiku_cap_too_large_denied(self):
        prompt = f"Summarize {self.big} briefly.\nRETURN: <=4000 chars"
        out = run_hook(PIN, self.agent_payload("haiku-scout", prompt), orch="e")
        self.assertIn("deny", out)
        self.assertIn("E gate", out)

    def test_3_no_return_cap_denied(self):
        prompt = f"Summarize {self.big} briefly."
        out = run_hook(PIN, self.agent_payload("haiku-scout", prompt), orch="e")
        self.assertIn("deny", out)
        self.assertIn("E gate", out)

    def test_4a_opus_without_escalate_denied(self):
        out = run_hook(PIN, self.agent_payload("opus-architect", "Decide the trade-off."), orch="e")
        self.assertIn("deny", out)
        self.assertIn("E gate", out)

    def test_4b_opus_with_escalate_not_gated(self):
        out = run_hook(PIN, self.agent_payload("opus-architect", "ESCALATE: decide the trade-off."), orch="e")
        self.assertNotIn("E gate", out)

    def test_5_orch_unset_no_gate(self):
        prompt = f"Summarize {self.big} briefly.\nRETURN: <=4000 chars"
        out = run_hook(PIN, self.agent_payload("haiku-scout", prompt), orch=None)
        self.assertNotIn("E gate", out)

    def test_6_subagent_call_not_gated(self):
        out = run_hook(PIN, self.agent_payload("opus-architect", "no escalate here", agent_id="a1"), orch="e")
        self.assertNotIn("E gate", out)

    def test_7_main_guard_e_allows_read(self):
        out = run_hook(GUARD, {"tool_name": "Read", "tool_input": {"file_path": self.big}}, orch="e")
        self.assertNotIn("deny", out)

    def test_7_record_main_guard_without_orch(self):
        # Recorded only, not asserted: CLAUDE_LEAN unset (default lean mode), CHAUDE_ORCH unset.
        out = run_hook(GUARD, {"tool_name": "Read", "tool_input": {"file_path": self.big}}, orch=None, lean=None)
        print(f"\n[record] main-thread-guard, no CHAUDE_ORCH, Read: stdout={out[:120]!r}", file=sys.stderr)

    def test_8_line_suffix_path_counted(self):
        # "<file>:12-40" must resolve to the file, so raw size (10000) is counted and the 4000 cap is denied.
        prompt = f"Summarize {self.big}:12-40 briefly.\nRETURN: <=4000 chars"
        out = run_hook(PIN, self.agent_payload("haiku-scout", prompt), orch="e")
        self.assertIn("deny", out)
        self.assertIn("E gate", out)


if __name__ == "__main__":
    unittest.main()
