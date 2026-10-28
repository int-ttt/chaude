#!/usr/bin/env python3
"""Build release assets: claude-lean-mac.zip, claude-lean-win.zip and NOTES.md (this version's CHANGELOG section).

usage: python3 tools/build_release.py [out-dir]   (default: dist)
Fails if the git tag (GITHUB_REF_NAME) doesn't match VERSION, or README/CHANGELOG lack the version."""
import os, re, shutil, subprocess, sys, tempfile, zipfile

PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(PKG, "dist"))
EXCLUDE = {".git", ".github", "dist", "history", "logs", "notes", "__pycache__"}
ver = open(os.path.join(PKG, "VERSION"), encoding="utf-8").read().strip()
tag = os.environ.get("GITHUB_REF_NAME", "")
if tag.startswith("v") and tag != f"v{ver}":
    sys.exit(f"tag {tag} != VERSION {ver}")


def pack(src, dst):
    with zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as z:
        for root, dirs, files in os.walk(src):
            dirs[:] = [d for d in dirs if d not in EXCLUDE]
            for f in files:
                if not f.endswith(".pyc"):
                    p = os.path.join(root, f)
                    z.write(p, os.path.relpath(p, src))


shutil.rmtree(OUT, ignore_errors=True)
os.makedirs(OUT)
win = os.path.join(tempfile.mkdtemp(), "claude-lean-win")
subprocess.run([sys.executable, os.path.join(PKG, "win", "build_win.py"), win], check=True)   # also checks the version labels
pack(PKG, os.path.join(OUT, "claude-lean-mac.zip"))
pack(win, os.path.join(OUT, "claude-lean-win.zip"))
log = open(os.path.join(PKG, "CHANGELOG.md"), encoding="utf-8").read()
m = re.search(rf"^## {re.escape(ver)}\n(.*?)(?=^## |\Z)", log, re.S | re.M)
open(os.path.join(OUT, "NOTES.md"), "w", encoding="utf-8").write((m.group(1).strip() if m else "") +
    "\n\n설치: zip을 풀고 macOS는 `./install.sh`, Windows는 `install.ps1`을 인자 없이 실행하면 설정 창이 열립니다.\n")
print("built", ver, sorted(os.listdir(OUT)))
