"""Tests for .claude/hooks/caveman-format.py (run: python3 -I -m unittest discover -s .claude/hooks/tests -v)."""
import glob, json, os, subprocess, sys, tempfile, unittest

HOOK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "caveman-format.py")

BRIEF_READ = "task: 목록 찾기\npaths: /abs/x\nreturn: R-read, 30줄\nlang: caveman\n"
BRIEF_KO = BRIEF_READ.replace("lang: caveman", "lang: ko")
RESP_READ = (
    "status: done\n"
    "reason: -\n"
    "claim: /abs/x.md:12 | foo 원문 [확실]\n"
    "searched: grep -rn foo /abs\n"
    "not_found: bar\n"
    "not_checked: /abs/other\n"
    "truncated: no ; omitted: -\n"
)


def payload(agent="haiku-scout", prompt=BRIEF_READ, response=RESP_READ, tool="Agent"):
    return {"tool_name": tool, "tool_input": {"subagent_type": agent, "prompt": prompt},
            "tool_response": response}


class HookTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = self._tmp.name

    def tearDown(self):
        self._tmp.cleanup()

    def run_hook(self, stdin, guard="log"):
        env = dict(os.environ, CLAUDE_PROJECT_DIR=self.root, CAVEMAN_GUARD=guard)
        raw = stdin if isinstance(stdin, bytes) else json.dumps(stdin, ensure_ascii=False).encode("utf-8")
        p = subprocess.run([sys.executable, "-I", HOOK], input=raw, env=env, capture_output=True, timeout=20)
        self.assertEqual(p.returncode, 0, p.stderr.decode("utf-8", "ignore"))
        return p.stdout.decode("utf-8")

    def rows(self):
        out = []
        for f in glob.glob(os.path.join(self.root, "notes", "metrics", "*.jsonl")):
            with open(f, encoding="utf-8") as fh:
                out += [json.loads(line) for line in fh if line.strip()]
        return out

    def test_a_complete_rread_no_row(self):
        self.run_hook(payload())
        self.assertEqual(self.rows(), [])

    def test_b_missing_status_one_row(self):
        resp = RESP_READ.replace("status: done\n", "")
        self.run_hook(payload(response=resp))
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["hook"], "caveman-format")
        self.assertEqual(rows[0]["agent"], "haiku-scout")
        self.assertEqual(rows[0]["fmt"], "R-read")
        self.assertIn("status", rows[0]["missing"])

    def test_c_lang_ko_without_summary_ko(self):
        self.run_hook(payload(prompt=BRIEF_KO))
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertIn("summary_ko", rows[0]["missing"])

    def test_c2_lang_ko_with_summary_ko_no_row(self):
        self.run_hook(payload(prompt=BRIEF_KO, response=RESP_READ + "summary_ko: 완료했습니다.\n"))
        self.assertEqual(self.rows(), [])

    def test_d_garbage_stdin_exit0_no_row(self):
        self.run_hook(b"not json {{{")
        self.assertEqual(self.rows(), [])

    def test_e_non_target_agent_no_row(self):
        self.run_hook(payload(agent="general-purpose", response="status: done\n"))
        self.assertEqual(self.rows(), [])

    def test_e2_non_agent_tool_no_row(self):
        self.run_hook(payload(tool="Bash", response="status: done\n"))
        self.assertEqual(self.rows(), [])

    def test_e3_legacy_brief_not_checked(self):
        self.run_hook(payload(prompt="task: 옛 형식\n", response="아무 텍스트"))
        self.assertEqual(self.rows(), [])

    def test_f_warn_prints_additional_context(self):
        resp = RESP_READ.replace("truncated: no ; omitted: -\n", "")
        out = self.run_hook(payload(response=resp), guard="warn")
        data = json.loads(out)
        ctx = data["hookSpecificOutput"]["additionalContext"]
        self.assertIn("필수 필드 누락", ctx)
        self.assertIn("truncated", ctx)
        self.assertEqual(len(self.rows()), 1)

    def test_g_guard_off_no_row(self):
        resp = RESP_READ.replace("status: done\n", "")
        self.assertEqual(self.run_hook(payload(response=resp), guard="off"), "")
        self.assertEqual(self.rows(), [])

    def test_h_invalid_status_value(self):
        self.run_hook(payload(response=RESP_READ.replace("status: done", "status: ok")))
        self.assertEqual(self.rows()[0]["missing"], ["status(invalid)"])

    def test_h2_invalid_truncated_value(self):
        self.run_hook(payload(response=RESP_READ.replace("truncated: no", "truncated: maybe")))
        self.assertEqual(self.rows()[0]["missing"], ["truncated(invalid)"])

    def test_h3_claim_without_line_number(self):
        self.run_hook(payload(response=RESP_READ.replace("/abs/x.md:12", "/abs/x.md")))
        self.assertEqual(self.rows()[0]["missing"], ["claim(invalid)"])

    def test_i_tool_response_block_list(self):
        blocks = [{"type": "text", "text": RESP_READ}]
        self.run_hook(payload(response=blocks))
        self.assertEqual(self.rows(), [])
        self.run_hook(payload(response={"content": [{"type": "text", "text": RESP_READ.replace("status: done\n", "")}]}))
        self.assertEqual(self.rows()[0]["missing"], ["status"])


if __name__ == "__main__":
    unittest.main()
