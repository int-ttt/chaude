"""Terminal REPL: streams replies, own transcript, per-turn usage, auto session rotation."""
from __future__ import annotations

import argparse
import json
import os
import sys
import time

from .core import Session, ctx_tokens

DIM, RST = "\033[2m", "\033[0m"
HANDOFF_PROMPT = (
    "Write a handoff note for a fresh session that will continue this work. No tools. "
    "Sections: Goal; Decisions; Files created/changed; In-progress work; Open questions; "
    "User preferences/instructions; Any exact codes, names or numbers the user gave. Be exact and brief.")
PREAMBLE = ("[Context carried over from a previous session that was rotated to save tokens. "
            "Treat it as your memory of the conversation so far; do not mention the rotation.]")


class Transcript:
    """Own append-only JSONL, one thread across any number of claude sessions."""

    def __init__(self, path):
        self.path = path
        self.turns = []  # (user, assistant) pairs for snapshot part B
        os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)

    def add(self, kind, **kw):
        rec = {"t": time.time(), "kind": kind, **kw}
        with open(self.path, "a", encoding="utf-8") as f:
            f.write(json.dumps(rec, ensure_ascii=False) + "\n")


class Repl:
    def __init__(self, a, out=sys.stdout, inp=sys.stdin):
        self.a, self.out, self.inp = a, out, inp
        self.tr = Transcript(a.transcript)
        self.sess = None
        self.pending_snapshot = None
        self.rotations = 0
        self.files = []
        self.last_ctx = 0
        self.rate = None

    def p(self, s="", end="\n"):
        self.out.write(s + end)
        self.out.flush()

    def start(self):
        self.sess = Session(model=self.a.model, cwd=self.a.cwd)

    def ask_permission(self, req):
        r = req["request"]
        self.p(f"\n{DIM}[permission] {r.get('tool_name')} {json.dumps(r.get('input', {}), ensure_ascii=False)[:200]}{RST}")
        if self.a.auto_allow:
            ans = "y"
        else:
            self.p("allow? [y/N] ", end="")
            ans = (self.inp.readline() or "n").strip().lower()
        self.tr.add("permission", tool=r.get("tool_name"), allowed=ans.startswith("y"))
        self.sess.respond_permission(req["request_id"], ans.startswith("y"), r.get("input"))

    def run_turn(self, text, show=True):
        """Send text, stream reply. Returns (reply_text, result_event|None, last_main_usage)."""
        self.sess.send(text)
        reply, last_usage, result = [], None, None
        try:
            for ev in self.sess.events():
                t = ev.get("type")
                if t == "stream_event" and not ev.get("parent_tool_use_id"):
                    e = ev["event"]
                    if e.get("type") == "content_block_delta" and e["delta"].get("type") == "text_delta":
                        reply.append(e["delta"]["text"])
                        if show:
                            self.p(e["delta"]["text"], end="")
                elif t == "assistant" and not ev.get("parent_tool_use_id"):
                    last_usage = ev["message"].get("usage") or last_usage
                    for b in ev["message"].get("content", []):
                        if b.get("type") == "tool_use" and b.get("name") in ("Write", "Edit", "NotebookEdit"):
                            fp = b.get("input", {}).get("file_path")
                            if fp and fp not in self.files:
                                self.files.append(fp)
                elif t == "control_request" and ev["request"].get("subtype") == "can_use_tool":
                    self.ask_permission(ev)
                elif t == "rate_limit_event":
                    self.rate = ev.get("rate_limit_info")
                elif t == "result":
                    result = ev
                elif t == "_unparsed":
                    self.tr.add("unparsed", raw=ev["raw"])
        except KeyboardInterrupt:
            self.sess.interrupt()
            self.p(f"\n{DIM}[interrupted]{RST}")
            for ev in self.sess.events(timeout=20):
                if ev.get("type") == "result":
                    result = ev
        if show:
            self.p()
        return "".join(reply), result, last_usage

    def usage_line(self, result, usage):
        if not result:
            return
        u = usage or {}
        ctx = ctx_tokens(u)
        self.last_ctx = ctx
        ru = result.get("usage", {})
        five = ((self.rate or {}).get("unifiedWindows", {}).get("five_hour", {}) or {}).get("utilization")
        self.p(f"{DIM}[ctx {ctx:,} | in {ru.get('input_tokens', 0)} cr {ru.get('cache_read_input_tokens', 0):,} "
               f"cc {ru.get('cache_creation_input_tokens', 0):,} out {ru.get('output_tokens', 0)} | "
               f"${result.get('total_cost_usd', 0):.4f} | 5h {five if five is not None else '?'} | "
               f"sess {str(self.sess.session_id)[:8]} rot {self.rotations}]{RST}")

    def build_snapshot(self, note):
        parts = [PREAMBLE, "## Handoff note (written by previous session)", note.strip() or "(empty)"]
        if self.files:
            parts += ["## Files changed", "\n".join(f"- {f}" for f in self.files)]
        tail = self.tr.turns[-self.a.tail_turns:]
        if tail:
            parts += ["## Last turns (verbatim, truncated)"]
            for u, r in tail:
                parts.append(f"User: {u[:1500]}\nAssistant: {r[:1500]}")
        snap = "\n\n".join(parts)
        return snap[: self.a.snapshot_chars]

    def rotate(self, reason):
        old = self.sess.session_id
        note, _, _ = self.run_turn(HANDOFF_PROMPT, show=False)
        snap = self.build_snapshot(note)
        self.sess.close()
        self.rotations += 1
        self.tr.add("rotate", n=self.rotations, reason=reason, old_session=old, snapshot=snap)
        self.pending_snapshot = snap
        self.start()
        if self.a.show_rotation:
            self.p(f"{DIM}── context rotated #{self.rotations} ({reason}; {old and old[:8]} → new) ──{RST}")

    def handle(self, line):
        text = line
        if self.pending_snapshot:
            text = f"{self.pending_snapshot}\n\n## Current user message\n{line}"
            self.pending_snapshot = None
        self.tr.add("user", text=line, session=self.sess.session_id)
        reply, result, usage = self.run_turn(text)
        self.tr.turns.append((line, reply))
        self.tr.add("assistant", text=reply, session=self.sess.session_id,
                    usage=(result or {}).get("usage"), ctx=ctx_tokens(usage or {}),
                    cost=(result or {}).get("total_cost_usd"))
        self.usage_line(result, usage)
        auto_ok = self.a.max_auto_rotations is None or self.rotations < self.a.max_auto_rotations
        if auto_ok and self.last_ctx > self.a.threshold:
            self.rotate(f"ctx {self.last_ctx:,} > {self.a.threshold:,}")

    def loop(self):
        self.start()
        interactive = self.inp.isatty() if hasattr(self.inp, "isatty") else False
        try:
            while True:
                if interactive:
                    self.p("\n> ", end="")
                line = self.inp.readline()
                if not line:
                    break
                line = line.rstrip("\n")
                if not line.strip():
                    continue
                if line == "/quit":
                    break
                if line == "/rotate":
                    self.rotate("manual")
                    continue
                if not interactive:
                    self.p(f"> {line}")
                self.handle(line)
        finally:
            if self.sess:
                self.sess.close()


def main(argv=None):
    ap = argparse.ArgumentParser(prog="harness")
    ap.add_argument("--model", default="haiku")
    ap.add_argument("--cwd", default=None)
    ap.add_argument("--threshold", type=int, default=100_000, help="rotate when context tokens exceed this")
    ap.add_argument("--max-auto-rotations", type=int, default=None)
    ap.add_argument("--transcript", default=os.path.expanduser("~/.claude-harness/transcript.jsonl"))
    ap.add_argument("--tail-turns", type=int, default=4)
    ap.add_argument("--snapshot-chars", type=int, default=24000)
    ap.add_argument("--auto-allow", action="store_true", help="allow all permission prompts (testing)")
    ap.add_argument("--show-rotation", action="store_true", help="print a one-line rotation marker")
    Repl(ap.parse_args(argv)).loop()


if __name__ == "__main__":
    main()
