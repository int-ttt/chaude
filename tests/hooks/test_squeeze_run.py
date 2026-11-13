"""Tests for scripts/squeeze-run.py: head/errors/tail filter, (xN) collapse, full log saved, exit code passed through.
Each test runs the script in a temp directory, so logs/runs/ is created there and never in the repo.

Run: cd <repo root> && python3 -I -m unittest discover tests/hooks -p 'test_squeeze_run.py' -v
"""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SCRIPT = os.path.join(ROOT, "scripts", "squeeze-run.py")


def run_sq(cwd, *cmd):
    return subprocess.run([sys.executable, "-I", SCRIPT, "--", *cmd], cwd=cwd, capture_output=True, text=True)


def py(code):
    return [sys.executable, "-c", code]


class SqueezeRunTest(unittest.TestCase):
    def setUp(self):
        self.cwd = tempfile.mkdtemp(prefix="squeeze-")

    def tearDown(self):
        shutil.rmtree(self.cwd, ignore_errors=True)

    def log_path(self, proc):
        line = [ln for ln in proc.stdout.splitlines() if ln.startswith("log: ")][0]
        return line[len("log: "):]

    def test_full_output_saved_and_summary_printed(self):
        proc = run_sq(self.cwd, *py("print('hello')"))
        self.assertEqual(proc.returncode, 0)
        self.assertIn("exit: 0", proc.stdout.splitlines())
        log = self.log_path(proc)
        self.assertTrue(os.path.isfile(log))
        self.assertTrue(log.startswith(os.path.join(os.path.realpath(self.cwd), "logs", "runs")))
        with open(log, encoding="utf-8") as f:
            self.assertEqual(f.read(), "hello\n")

    def test_exit_code_passed_through(self):
        proc = run_sq(self.cwd, *py("import sys; sys.exit(3)"))
        self.assertEqual(proc.returncode, 3)
        self.assertIn("exit: 3", proc.stdout.splitlines())

    def test_missing_command_exits_127(self):
        proc = run_sq(self.cwd, "no-such-command-squeeze-xyz")
        self.assertEqual(proc.returncode, 127)
        self.assertIn("exit: 127", proc.stdout.splitlines())

    def test_head_errors_with_context_and_tail(self):
        code = "for i in range(1, 201):\n    print('row', i)\n    if i == 100: print('error: boom')"
        proc = run_sq(self.cwd, *py(code))
        lines = set(proc.stdout.splitlines())
        self.assertIn("row 1", lines)  # head
        self.assertIn("row 8", lines)
        self.assertNotIn("row 9", lines)
        self.assertIn("row 99", lines)  # two lines of context before the error (it follows row 100)
        self.assertIn("row 100", lines)
        self.assertIn("error: boom", lines)
        self.assertIn("row 101", lines)  # and after
        self.assertIn("row 102", lines)
        self.assertNotIn("row 98", lines)
        self.assertNotIn("row 50", lines)
        self.assertNotIn("row 180", lines)
        self.assertIn("row 181", lines)  # tail
        self.assertIn("row 200", lines)
        self.assertIn("...", lines)
        with open(self.log_path(proc), encoding="utf-8") as f:
            self.assertIn("row 50\n", f.read())  # the full output is still in the log

    def test_repeated_lines_collapse(self):
        proc = run_sq(self.cwd, *py("for _ in range(30): print('same')"))
        self.assertIn("same (x30)", proc.stdout.splitlines())
        self.assertEqual(proc.stdout.count("same"), 1)

    def test_failure_word_and_panic_are_kept(self):
        code = "print('x'); print('test a ... FAILED'); print('thread main panicked'); print('y')"
        proc = run_sq(self.cwd, *py(code))
        self.assertIn("test a ... FAILED", proc.stdout)
        self.assertIn("thread main panicked", proc.stdout)

    def test_no_command_is_usage_error(self):
        proc = subprocess.run([sys.executable, "-I", SCRIPT], cwd=self.cwd, capture_output=True, text=True)
        self.assertEqual(proc.returncode, 2)


if __name__ == "__main__":
    unittest.main()
