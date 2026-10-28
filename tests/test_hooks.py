"""Regression tests for hooks: python3 tests/test_hooks.py"""
import json, os, subprocess, sys, tempfile, threading, time, unittest
H = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".claude", "hooks")


def run(hook, payload, root):
    r = subprocess.run([sys.executable, os.path.join(H, hook)], input=json.dumps(payload), text=True,
                       capture_output=True, env={**os.environ, "CLAUDE_PROJECT_DIR": root})
    return r.stdout


def user(text):
    return json.dumps({"type": "user", "message": {"role": "user", "content": text}}) + "\n"


class Ledger(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.d, "notes"))
        self.sub = os.path.join(self.d, "proj", "sid", "subagents")
        os.makedirs(self.sub)
        self.pl = {"agent_id": "A1", "agent_type": "haiku-runner", "transcript_path": os.path.join(self.d, "proj", "sid.jsonl"),
                   "last_assistant_message": "작업이 진행 중입니다. 로그를 확인했고 아직 실행 중입니다."}

    def entries(self):
        p = os.path.join(self.d, "notes", "LEDGER.md")
        return open(p).read() if os.path.exists(p) else ""

    def test_late_transcript_ping_is_skipped(self):   # transcript not on disk when the hook starts
        t = threading.Timer(0.5, lambda: open(os.path.join(self.sub, "agent-A1.jsonl"), "w").write(user("진행상황")))
        t.start(); run("ledger.py", self.pl, self.d); t.join()
        self.assertEqual(self.entries().strip(), "")

    def test_real_task_is_recorded(self):
        open(os.path.join(self.sub, "agent-A1.jsonl"), "w").write(user("로그를 확인하고 요약해줘"))
        run("ledger.py", self.pl, self.d)
        self.assertIn("haiku-runner", self.entries())

    def test_missing_transcript_worst_case_under_2s(self):   # review MAJOR: two sequential retry loops took 3.3s
        t0 = time.time(); run("ledger.py", self.pl, self.d)
        self.assertLess(time.time() - t0, 2.5)
        self.assertIn("haiku-runner", self.entries())


class Update(unittest.TestCase):
    def test_resume_new_session_id_notifies(self):
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, ".claude")); os.makedirs(os.path.join(d, "logs"))
        open(os.path.join(d, ".claude", "claude-lean.version"), "w").write("0.5.1\n")
        open(os.path.join(d, "logs", ".lean_ver_last"), "w").write("0.5.0")
        out = json.loads(run("session-context.py", {"source": "resume", "session_id": "new"}, d))
        self.assertIn("0.5.0", out.get("systemMessage", ""))
        again = run("session-context.py", {"source": "resume", "session_id": "new2"}, d)
        self.assertNotIn("업데이트됨", again)

    def test_root_version_fallback(self):         # dev repo: no .claude/claude-lean.version
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, "logs"))
        open(os.path.join(d, "VERSION"), "w").write("0.5.1\n")
        open(os.path.join(d, "logs", ".lean_ver_last"), "w").write("0.5.0")
        out = json.loads(run("session-context.py", {"source": "resume", "session_id": "n"}, d))
        self.assertIn("업데이트됨", out.get("systemMessage", ""))
        self.assertEqual(open(os.path.join(d, "logs", ".lean_ver_last")).read(), "0.5.1")

    def test_downgrade_or_older_is_silent(self):   # last recorded version newer than current: no "업데이트됨"
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, "logs"))
        open(os.path.join(d, "VERSION"), "w").write("0.5.0\n")
        open(os.path.join(d, "logs", ".lean_ver_last"), "w").write("0.5.1")
        self.assertNotIn("업데이트됨", run("session-context.py", {"source": "resume", "session_id": "n"}, d))

    def test_numeric_compare(self):               # 0.5.10 > 0.5.9 (not string order)
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, "logs"))
        open(os.path.join(d, "VERSION"), "w").write("0.5.10\n")
        open(os.path.join(d, "logs", ".lean_ver_last"), "w").write("0.5.9")
        self.assertIn("업데이트됨", run("session-context.py", {"source": "resume", "session_id": "n"}, d))

    def test_first_run_no_record_is_silent(self):  # previous version unknown: no notice, record created
        d = tempfile.mkdtemp()
        open(os.path.join(d, "VERSION"), "w").write("0.5.1\n")
        out = run("session-context.py", {"source": "startup", "session_id": "n"}, d)
        self.assertNotIn("업데이트됨", out)
        self.assertEqual(open(os.path.join(d, "logs", ".lean_ver_last")).read(), "0.5.1")


if __name__ == "__main__":
    unittest.main()
