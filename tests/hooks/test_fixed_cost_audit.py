"""Tests for scripts/fixed-cost-audit.py: fixed-context sources, bytes / 4 tokens, descending sort, total, and hooks
only with --hooks. Each test builds its own project and home under a temp dir; the real home is never read.

Run: cd <repo root> && python3 -I -m unittest discover tests/hooks -p 'test_fixed_cost_audit.py' -v
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SCRIPT = os.path.join(ROOT, "scripts", "fixed-cost-audit.py")
ROW = re.compile(r"^\s*(\d+)\s+(\d+)\s+(.*)$")


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def audit(project, home, *extra):
    proc = subprocess.run([sys.executable, "-I", SCRIPT, "--project", project, "--home", home, *extra],
                          capture_output=True, text=True)
    assert proc.returncode == 0, proc.stderr
    return proc.stdout


def rows(out):
    data = []
    for line in out.splitlines():
        m = ROW.match(line)
        if m and not m.group(3).startswith("total"):
            data.append((int(m.group(1)), int(m.group(2)), m.group(3).strip()))
    return data


class FixedCostAuditTest(unittest.TestCase):
    def setUp(self):
        self.base = tempfile.mkdtemp(prefix="fixedcost-")
        self.project = os.path.join(self.base, "proj")
        self.home = os.path.join(self.base, "home")
        os.makedirs(self.project)
        os.makedirs(self.home)

    def tearDown(self):
        shutil.rmtree(self.base, ignore_errors=True)

    def test_sources_counted_and_agent_body_excluded(self):
        write(os.path.join(self.project, "CLAUDE.md"), "x" * 800)
        write(os.path.join(self.project, ".claude", "agents", "haiku-scout.md"),
              "---\nname: haiku-scout\ndescription: Finds files fast.\n---\n" + "body " * 2000)
        out = audit(self.project, self.home)
        by_label = {label: (b, t) for t, b, label in rows(out)}
        self.assertEqual(by_label["CLAUDE.md (project)"], (800, 200))
        agent_bytes, agent_tokens = by_label["agent project haiku-scout.md"]
        self.assertEqual(agent_bytes, len("haiku-scout: Finds files fast.".encode()))
        self.assertEqual(agent_tokens, agent_bytes // 4)

    def test_skill_and_memory_index_counted(self):
        write(os.path.join(self.project, ".claude", "skills", "check", "SKILL.md"),
              "---\nname: check\ndescription: Runs the gates.\n---\nlong body " * 50)
        encoded = re.sub(r"[^A-Za-z0-9]", "-", os.path.abspath(self.project))
        write(os.path.join(self.home, ".claude", "projects", encoded, "memory", "MEMORY.md"), "m" * 1200)
        out = audit(self.project, self.home)
        labels = {label for _, _, label in rows(out)}
        self.assertIn("skill project check", labels)
        self.assertIn("memory MEMORY.md", labels)
        by_label = {label: t for t, _, label in rows(out)}
        self.assertEqual(by_label["memory MEMORY.md"], 300)

    def test_sorted_descending_with_total(self):
        write(os.path.join(self.project, "CLAUDE.md"), "x" * 400)
        write(os.path.join(self.home, ".claude", "CLAUDE.md"), "y" * 4000)
        write(os.path.join(self.project, ".claude", "agents", "a.md"), "---\nname: a\ndescription: d\n---\n")
        out = audit(self.project, self.home)
        data = rows(out)
        tokens = [t for t, _, _ in data]
        self.assertEqual(tokens, sorted(tokens, reverse=True))
        self.assertEqual(data[0][2], "CLAUDE.md (user)")
        total_line = [ln for ln in out.splitlines() if "total" in ln][0]
        self.assertIn(f"{sum(tokens)}", total_line.split()[0])
        self.assertIn(f"({len(data)} sources)", total_line)

    def test_empty_project_reports_zero_sources(self):
        out = audit(self.project, self.home)
        self.assertEqual(rows(out), [])
        self.assertIn("(0 sources)", out)

    def test_hooks_not_run_without_flag_and_counted_with_flag(self):
        marker = os.path.join(self.project, "ran.txt")
        cmd = f"{sys.executable} -c \"open('{marker}', 'w').close(); print('x' * 400)\""
        settings = '{"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": %s}]}]}}' % (
            __import__("json").dumps(cmd))
        write(os.path.join(self.project, ".claude", "settings.json"), settings)
        out = audit(self.project, self.home)
        self.assertFalse(os.path.exists(marker))
        self.assertIn("hook output not counted", out)
        out = audit(self.project, self.home, "--hooks")
        self.assertTrue(os.path.exists(marker))
        hook = [r for r in rows(out) if r[2].startswith("hook SessionStart project")]
        self.assertEqual(len(hook), 1)
        self.assertEqual(hook[0][1], 401)  # 400 x plus newline
        self.assertEqual(hook[0][0], 401 // 4)


if __name__ == "__main__":
    unittest.main()
