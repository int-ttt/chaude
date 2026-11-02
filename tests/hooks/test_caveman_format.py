"""Tests for .claude/hooks/caveman-format.py (PostToolUse, never blocks, always exit 0).

Run: cd repo && python3 -m unittest discover tests/hooks -v
Each run uses a temporary CLAUDE_PROJECT_DIR so record/metrics in the repo is not written."""
import glob, json, os, subprocess, sys, tempfile, unittest
from pathlib import Path

HOOK = str(Path(__file__).resolve().parents[2] / ".claude" / "hooks" / "caveman-format.py")

BRIEF_READ = "task: 목록 찾기\npaths: /abs/x\nreturn: R-read, 30줄\nlang: caveman\n"
BRIEF_KO = BRIEF_READ.replace("lang: caveman", "lang: ko")
BRIEF_PROMPTER_5 = "task: x\npaths: /abs\nlimits: -\nreturn: R-write, 300\nlang: caveman\n"
RESP_READ = (
    "status: done\n"
    "reason: -\n"
    "claim: /abs/x.md:12 | foo 원문 [확실]\n"
    "searched: grep -rn foo /abs\n"
    "not_found: bar\n"
    "not_checked: /abs/other\n"
    "truncated: no ; omitted: -\n"
)
VALID_RESPONSE = (
    "status: done\n"
    "reason: -\n"
    "changed: notes/x.md\n"
    "diffstat: 1 file changed\n"
    "check: python3 -m unittest -> PASS\n"
    "ws: none\n"
    "summary_ko: 완료\n"
)


def payload(agent="haiku-scout", prompt=BRIEF_READ, response=RESP_READ, tool="Agent"):
    return {"tool_name": tool, "tool_input": {"subagent_type": agent, "prompt": prompt},
            "tool_response": response}


class CavemanFormatHookTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = self._tmp.name

    def tearDown(self):
        self._tmp.cleanup()

    def run_hook(self, stdin, guard="log"):
        env = dict(os.environ, CLAUDE_PROJECT_DIR=self.root, CAVEMAN_GUARD=guard)
        env.pop("HARNESS_RECORD_DIR", None)
        raw = stdin if isinstance(stdin, bytes) else json.dumps(stdin, ensure_ascii=False).encode("utf-8")
        p = subprocess.run([sys.executable, "-I", HOOK], input=raw, env=env, capture_output=True, timeout=20)
        self.assertEqual(p.returncode, 0, p.stderr.decode("utf-8", "ignore"))
        return p.stdout.decode("utf-8")

    def rows(self):
        out = []
        for f in glob.glob(os.path.join(self.root, "record", "metrics", "*.jsonl")):
            with open(f, encoding="utf-8") as fh:
                out += [json.loads(line) for line in fh if line.strip()]
        return out

    # --- from the original tests/hooks suite ---

    def test_malformed_stdin_exits_zero(self):
        self.assertEqual(self.run_hook(b"{not json"), "")

    def test_valid_payload_guard_off_exits_zero_without_block(self):
        out = self.run_hook(payload(agent="haiku-coder", prompt="return: R-write, 300\nlang: ko\n",
                                    response=VALID_RESPONSE), guard="off")
        self.assertNotIn('"block"', out)
        self.assertEqual(out.strip(), "")

    def test_log_mode_never_blocks_and_records_missing_keys(self):
        bad = "status: done\nreason: -\nchanged: notes/x.md\n"  # diffstat/check/ws missing
        out = self.run_hook(payload(agent="haiku-coder", prompt="return: R-write, 300\nlang: ko\n",
                                    response=bad), guard="log")
        self.assertNotIn("block", out)
        self.assertEqual(out.strip(), "")  # log mode: metrics row only, no additionalContext
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["agent"], "haiku-coder")
        self.assertIn("check", rows[0]["missing"])

    # --- from the former .claude/hooks/tests suite ---

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
        self.assertIn("missing required fields", ctx)
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

    # --- new: 존댓말 (haiku-tui) and B형식 brief (haiku-prompter) ---

    def test_j_tui_informal_one_row(self):
        self.run_hook(payload(agent="haiku-tui", prompt="작업 요청", response="작업 완료했다."))
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["fmt"], "tui")
        self.assertEqual(rows[0]["missing"], ["존댓말"])

    def test_j2_tui_polite_no_row(self):
        self.run_hook(payload(agent="haiku-tui", prompt="작업 요청", response="작업을 완료했습니다."))
        self.assertEqual(self.rows(), [])

    def test_j3_tui_code_block_ignored(self):
        resp = "```\n이것은 있다.\n```\n작업을 완료했습니다.\n"
        self.run_hook(payload(agent="haiku-tui", prompt="작업 요청", response=resp))
        self.assertEqual(self.rows(), [])

    def test_j4_tui_caveman_leak_one_row(self):
        self.run_hook(payload(agent="haiku-tui", prompt="작업 요청", response="읽기 결과입니다.\nclaim: a.go:1 | x"))
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertIn("caveman_leak", rows[0]["missing"])

    def test_j5_tui_polite_list_no_row(self):
        self.run_hook(payload(agent="haiku-tui", prompt="작업 요청", response="확인했습니다.\n- a.go:1 — 내용입니다."))
        self.assertEqual(self.rows(), [])

    def test_k_prompter_missing_lang(self):
        prompt = BRIEF_PROMPTER_5.replace("lang: caveman\n", "")
        self.run_hook(payload(agent="haiku-prompter", prompt=prompt, response=VALID_RESPONSE))
        rows = self.rows()
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["fmt"], "prompter")
        self.assertEqual(rows[0]["missing"], ["brief:lang"])

    def test_k2_prompter_all_five_keys_no_row(self):
        self.run_hook(payload(agent="haiku-prompter", prompt=BRIEF_PROMPTER_5, response=VALID_RESPONSE))
        self.assertEqual(self.rows(), [])

    def test_l_warn_tui_violation_exit0_with_context(self):
        out = self.run_hook(payload(agent="haiku-tui", prompt="작업 요청", response="작업 완료했다."), guard="warn")
        ctx = json.loads(out)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("missing required fields", ctx)
        self.assertIn("존댓말", ctx)
        self.assertEqual(len(self.rows()), 1)


if __name__ == "__main__":
    unittest.main()
