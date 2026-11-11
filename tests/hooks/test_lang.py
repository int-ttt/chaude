"""Tests for _lean.lang(): the project settings file wins over CHAUDE_LANG, and it is re-read on every call.

Run: cd <repo root> && python3 -I -m unittest discover -s tests/hooks -p 'test_*.py' -v
"""
import json
import os
import subprocess
import sys
import tempfile
import unittest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HOOKS = os.path.join(ROOT, ".claude", "hooks")


def live_lang(settings, env_lang):
    """Runs _lean.lang() in a fresh process with the given settings text (None = no file) and CHAUDE_LANG."""
    with tempfile.TemporaryDirectory() as d:
        if settings is not None:
            os.makedirs(os.path.join(d, ".chaude"))
            with open(os.path.join(d, ".chaude", "settings.json"), "w", encoding="utf-8") as f:
                f.write(settings)
        env = dict(os.environ, CLAUDE_PROJECT_DIR=d)
        env.pop("CHAUDE_LANG", None)
        if env_lang is not None:
            env["CHAUDE_LANG"] = env_lang
        code = "import sys; sys.path.insert(0, sys.argv[1]); from _lean import lang; print(lang())"
        proc = subprocess.run([sys.executable, "-I", "-c", code, HOOKS], env=env, capture_output=True, text=True)
        if proc.returncode != 0:
            raise AssertionError(proc.stderr)
        return proc.stdout.strip()


class LangTest(unittest.TestCase):
    def test_settings_file_wins_over_env(self):
        self.assertEqual(live_lang(json.dumps({"language": "en"}), "ko"), "en")
        self.assertEqual(live_lang(json.dumps({"language": "ko"}), "en"), "ko")

    def test_auto_missing_or_broken_falls_back_to_env_then_ko(self):
        self.assertEqual(live_lang(json.dumps({"language": "auto"}), "en"), "en")
        self.assertEqual(live_lang(json.dumps({"main_model": "opus"}), "en"), "en")
        self.assertEqual(live_lang("not json", "en"), "en")
        self.assertEqual(live_lang(None, "en"), "en")
        self.assertEqual(live_lang(None, None), "ko")
        self.assertEqual(live_lang(json.dumps({"language": "fr"}), None), "ko")


class StyleAdapterLangTagTest(unittest.TestCase):
    def adapter_out(self, settings, env_lang):
        with tempfile.TemporaryDirectory() as d:
            if settings is not None:
                os.makedirs(os.path.join(d, ".chaude"))
                with open(os.path.join(d, ".chaude", "settings.json"), "w", encoding="utf-8") as f:
                    f.write(settings)
            env = dict(os.environ, CLAUDE_PROJECT_DIR=d, HARNESS_RECORD_DIR=os.path.join(d, "record"), CLAUDE_LEAN="1")
            env.pop("CHAUDE_LANG", None)
            env.pop("STYLE_INBOX", None)
            if env_lang is not None:
                env["CHAUDE_LANG"] = env_lang
            proc = subprocess.run([sys.executable, "-I", os.path.join(HOOKS, "style-adapter.py")],
                                  input=json.dumps({"prompt": "ok", "session_id": "lang-test"}),
                                  capture_output=True, text=True, env=env)
            self.assertEqual(proc.returncode, 0, proc.stderr)
            return proc.stdout

    def test_tag_only_when_live_language_differs_from_spawn_env(self):
        self.assertIn("[lang]", self.adapter_out(json.dumps({"language": "en"}), "ko"))
        self.assertNotIn("[lang]", self.adapter_out(json.dumps({"language": "en"}), "en"))
        self.assertNotIn("[lang]", self.adapter_out(None, None))


if __name__ == "__main__":
    unittest.main()
