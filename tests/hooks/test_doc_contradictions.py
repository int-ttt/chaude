"""S30: one final-reply owner, one agent count, one main-model rule across the docs.

Run: cd <repo root> && python3 -I -m unittest discover -s tests/hooks -p 'test_*.py' -v
"""
import glob
import os
import re
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def read(rel):
    with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
        return f.read()


def agent_names():
    """The 13 agent files under .claude/agents are the source of truth for the count."""
    paths = glob.glob(os.path.join(ROOT, ".claude", "agents", "*.md"))
    return sorted(os.path.basename(p)[:-3] for p in paths)


class AgentCount(unittest.TestCase):
    def test_there_are_13_agent_files(self):
        self.assertEqual(len(agent_names()), 13)

    def test_readme_tables_list_every_agent(self):
        for rel in ("README.md", "README.ko.md"):
            text = read(rel)
            for name in agent_names():
                with self.subTest(doc=rel, agent=name):
                    self.assertIn("| `%s` |" % name, text)

    def test_guide_table_lists_every_agent_and_count_is_13(self):
        text = read("docs/guide/models-and-agents.md")
        self.assertIn("에이전트는 모두 13개입니다.", text)
        self.assertNotIn("11개", text)
        for name in agent_names():
            with self.subTest(agent=name):
                self.assertRegex(text, r"(?m)^\| %s \|" % re.escape(name))

    def test_apply_guide_count_is_13(self):
        self.assertIn("에이전트 13개", read("docs/claude-lean/APPLY_GUIDE.md"))


class FinalReplyOwner(unittest.TestCase):
    def test_readme_names_haiku_output_as_final_reply_owner(self):
        self.assertIn("`haiku-output` writes the final reply", read("README.md"))
        self.assertIn("최종 답변은 `haiku-output`가 씁니다", read("README.ko.md"))
        self.assertNotIn("writes the final answer", read("README.md"))
        self.assertNotIn("최종 답변을 씁니다", read("README.ko.md"))

    def test_guide_names_haiku_output_as_final_reply_owner(self):
        text = read("docs/guide/models-and-agents.md")
        self.assertIn("`haiku-output`가 최종 답을 한 번 씁니다", text)
        self.assertIn("W --> F[haiku-output가 최종 답 한 번]", text)
        self.assertNotIn("메인이 답을 한 번 씁니다", text)
        self.assertNotIn("메인이 최종 답 한 번", text)

    def test_claude_md_final_answer_rule_uses_haiku_output(self):
        text = read("CLAUDE.md")
        self.assertIn("haiku-output owns the final reply", text)
        self.assertIn("main writes the English relay", text)


class MainModelRule(unittest.TestCase):
    def test_haiku_main_only_with_model_flag(self):
        claude = read("CLAUDE.md")
        self.assertIn("Haiku main (`--model haiku` only)", claude)
        self.assertNotIn("`haiku로`/`opus로`", claude)
        guide = read("docs/guide/models-and-agents.md")
        self.assertIn("`--model haiku`로 시작합니다. 대화 중에는 바꾸지 않습니다.", guide)
        self.assertNotIn('대화 중에 "haiku로"', guide)


if __name__ == "__main__":
    unittest.main()
