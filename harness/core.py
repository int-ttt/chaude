"""Child-process driver for `claude -p` stream-json. Never touches credentials."""
from __future__ import annotations

import json
import os
import queue
import subprocess
import threading
import time
import uuid


def clean_env():
    # ANTHROPIC_API_KEY must be absent: with -p a key silently switches billing to API.
    return {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}


def ctx_tokens(usage):
    """Context-size proxy = input + cache_read + cache_creation of one API call."""
    return (usage.get("input_tokens", 0) + usage.get("cache_read_input_tokens", 0)
            + usage.get("cache_creation_input_tokens", 0))


class Session:
    """One `claude -p` child. send() a user message, then iterate turn() until result."""

    def __init__(self, model="haiku", cwd=None, resume=None, extra=None, claude_bin="claude",
                 permission_tool=True):
        self.session_id = None
        cmd = [claude_bin, "-p", "--input-format", "stream-json", "--output-format", "stream-json",
               "--verbose", "--include-partial-messages", "--model", model]
        if permission_tool:
            cmd += ["--permission-prompt-tool", "stdio"]
        if resume:
            cmd += ["--resume", resume]
        cmd += list(extra or [])
        self.proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, text=True, encoding="utf-8",
                                     env=clean_env(), cwd=cwd)
        self.q = queue.Queue()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        for line in self.proc.stdout:
            line = line.strip()
            if line:
                self.q.put(line)
        self.q.put(None)

    def _write(self, obj):
        self.proc.stdin.write(json.dumps(obj, ensure_ascii=False) + "\n")
        self.proc.stdin.flush()

    def send(self, text):
        self._write({"type": "user", "message": {"role": "user", "content": text}})

    def respond_permission(self, request_id, allow, tool_input=None, message="denied by user"):
        body = ({"behavior": "allow", "updatedInput": tool_input or {}} if allow
                else {"behavior": "deny", "message": message})
        self._write({"type": "control_response", "response": {
            "subtype": "success", "request_id": request_id, "response": body}})

    def interrupt(self):
        self._write({"type": "control_request", "request_id": str(uuid.uuid4()),
                     "request": {"subtype": "interrupt"}})

    def events(self, timeout=300):
        """Yield parsed events until the turn's `result` (inclusive) or process end."""
        end = time.time() + timeout
        while time.time() < end:
            try:
                line = self.q.get(timeout=1)
            except queue.Empty:
                continue
            if line is None:
                return
            try:
                ev = json.loads(line)
            except ValueError:
                ev = {"type": "_unparsed", "raw": line}  # keep unknown output verbatim
            if ev.get("session_id"):
                self.session_id = ev["session_id"]
            yield ev
            if ev.get("type") == "result":
                return

    def close(self):
        try:
            self.proc.stdin.close()
            self.proc.wait(timeout=15)
        except Exception:
            self.proc.kill()
