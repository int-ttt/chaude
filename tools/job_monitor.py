#!/usr/bin/env python3
"""Monitor a background job by pid and/or a status/log file.

Usage:
  python3 tools/job_monitor.py [--pid N] [--status PATH | --log PATH]
                               [--interval SEC] [--once]

Without --status/--log, the newest logs/*.status file is watched.
Prints a line only when the state changes; exits when the pid ends
(or, without a pid, when the last line contains STOP).
"""

import argparse
import os
import re
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOGS = ROOT / "logs"


def win_pid_alive(pid):
    # OpenProcess + GetExitCodeProcess via ctypes (no os.kill on Windows: signal 0 is not a probe there)
    import ctypes
    from ctypes import wintypes
    k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    k32.OpenProcess.restype = wintypes.HANDLE
    k32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    k32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    k32.CloseHandle.argtypes = [wintypes.HANDLE]
    PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
    ERROR_ACCESS_DENIED = 5
    STILL_ACTIVE = 259
    h = k32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not h:
        return ctypes.get_last_error() == ERROR_ACCESS_DENIED  # exists but not ours
    try:
        code = wintypes.DWORD()
        if not k32.GetExitCodeProcess(h, ctypes.byref(code)):
            return True
        return code.value == STILL_ACTIVE
    finally:
        k32.CloseHandle(h)


def pid_alive(pid):
    if os.name == "nt":
        return win_pid_alive(pid)
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def proc_etime(pid):
    if os.name == "nt":  # no ps on Windows
        return ""
    try:
        r = subprocess.run(["ps", "-o", "etime=", "-p", str(pid)],
                           capture_output=True, text=True, timeout=5)
        return r.stdout.strip()
    except Exception:
        return ""


def last_line(path):
    if not path.exists():
        return None
    try:
        with open(path, "rb") as f:
            f.seek(0, 2)
            size = f.tell()
            f.seek(max(0, size - 65536))
            data = f.read()
    except OSError:
        return None
    lines = [ln.strip() for ln in data.decode("utf-8", errors="replace").splitlines()
             if ln.strip()]
    return lines[-1] if lines else ""


def progress(line):
    m = re.search(r"(\d+)\s*/\s*(\d+)", line)
    if m and int(m.group(2)) > 0:
        n, d = int(m.group(1)), int(m.group(2))
        return f"{n}/{d} ({n * 100 // d}%)"
    m = re.search(r"(\d+(?:\.\d+)?)\s*%", line)
    if m:
        return f"{m.group(1)}%"
    return "-"


def hms(seconds):
    s = int(seconds)
    return f"{s // 3600}:{(s % 3600) // 60:02d}:{s % 60:02d}"


def resolve_target(args):
    if args.status:
        return Path(args.status)
    if args.log:
        return Path(args.log)
    cands = list(LOGS.glob("*.status"))
    if cands:
        return max(cands, key=lambda p: p.stat().st_mtime)
    return None


def snapshot(pid, target):
    alive = pid_alive(pid) if pid is not None else None
    last = last_line(target) if target is not None else None
    return (alive, last, progress(last or ""))


def render(start, pid, target, snap):
    alive, last, prog = snap
    parts = [f"[{datetime.now():%H:%M:%S}]", f"elapsed={hms(time.monotonic() - start)}"]
    if pid is not None:
        parts.append(f"pid={pid} alive={'yes' if alive else 'no'}")
        et = proc_etime(pid)
        if et:
            parts.append(f"etime={et}")
    if target is not None:
        parts.append(f"file={target.name if last is not None else 'missing'}")
    parts.append(f"progress={prog}")
    parts.append(f'last="{(last or "")[:160]}"')
    return " ".join(parts)


def run(args):
    target = resolve_target(args)
    if args.pid is None and target is None:
        print("job_monitor: no target (use --pid/--status/--log)", file=sys.stderr)
        return 1
    start = time.monotonic()
    if args.once:
        snap = snapshot(args.pid, target)
        print(render(start, args.pid, target, snap), flush=True)
        return 0
    prev = None
    while True:
        snap = snapshot(args.pid, target)
        if snap != prev:
            print(render(start, args.pid, target, snap), flush=True)
            prev = snap
        alive, last, _ = snap
        if args.pid is not None:
            if not alive:
                print("... process ended", flush=True)
                return 0
        elif last is not None and "STOP" in last:
            print(last, flush=True)
            print("job_monitor: STOP seen, exiting", flush=True)
            return 0
        time.sleep(args.interval)


def main(argv=None):
    try:
        ap = argparse.ArgumentParser(description="Monitor a background job.")
        ap.add_argument("--pid", type=int, help="process id to watch")
        ap.add_argument("--status", help="status file to read")
        ap.add_argument("--log", help="log file to read")
        ap.add_argument("--interval", type=float, default=10,
                        help="seconds between checks (default 10)")
        ap.add_argument("--once", action="store_true", help="print one line and exit")
        args = ap.parse_args(argv)
        return run(args)
    except KeyboardInterrupt:
        print("job_monitor: interrupted", flush=True)
        return 0


if __name__ == "__main__":
    sys.exit(main())
