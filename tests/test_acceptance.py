"""Live acceptance (needs logged-in `claude`): streaming + one forced rotation with context carried.
Run: python3 -m unittest tests.test_acceptance -v"""
import json, os, subprocess, sys, tempfile, unittest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


class Acceptance(unittest.TestCase):
    def test_rotation_carries_context(self):
        tr = os.path.join(tempfile.mkdtemp(), "t.jsonl")
        script = "Remember this codeword: ZEBRA-77. Reply with just: stored.\nWhat was the codeword I asked you to remember? Reply with just the codeword.\n"
        env = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}
        out = subprocess.run([sys.executable, "-m", "harness", "--threshold", "100", "--max-auto-rotations", "1",
                              "--transcript", tr], input=script, capture_output=True, text=True, cwd=ROOT, env=env, timeout=300).stdout
        recs = [json.loads(l) for l in open(tr)]
        rot = [r for r in recs if r["kind"] == "rotate"]
        asst = [r for r in recs if r["kind"] == "assistant"]
        self.assertEqual(len(rot), 1)
        self.assertNotEqual(asst[0]["session"], asst[1]["session"])
        self.assertIn("ZEBRA-77", asst[1]["text"])
        self.assertIn("ZEBRA-77", rot[0]["snapshot"])

    def test_streaming_multiple_deltas(self):
        sys.path.insert(0, ROOT)
        from harness.core import Session
        s = Session(); s.send("Count from 1 to 30, one per line. No tools.")
        n = sum(1 for e in s.events() if e.get("type") == "stream_event"
                and e["event"].get("type") == "content_block_delta" and e["event"]["delta"].get("type") == "text_delta")
        s.close()
        self.assertGreater(n, 3)


if __name__ == "__main__":
    unittest.main()
