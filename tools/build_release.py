#!/usr/bin/env python3
"""Build release assets.

usage: python3 tools/build_release.py [out-dir] [--stage-only]   (default out-dir: dist)

Always: out/stage/pkg-unix (macOS/Linux package), out/stage/pkg-win (Windows package), out/NOTES.md.
Unless --stage-only: out/claude-lean-<ver>-terminal.zip (macOS/Linux terminal package: unzip, ./install.sh). The .app and .exe are built from the staged packages by PyInstaller
in .github/workflows/release.yml.
Fails if the git tag (GITHUB_REF_NAME, when it starts with v) doesn't match VERSION, or README/CHANGELOG lack it."""
import os, re, shutil, subprocess, sys, zipfile

PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
args = [a for a in sys.argv[1:] if not a.startswith("--")]
OUT = os.path.abspath(args[0] if args else os.path.join(PKG, "dist"))
EXCLUDE = {".git", ".github", "dist", "history", "logs", "notes", "__pycache__", "win", "mac", ".gitignore", "build"}
ver = open(os.path.join(PKG, "VERSION"), encoding="utf-8").read().strip()
tag = os.environ.get("GITHUB_REF_NAME", "")
if tag.startswith("v") and tag != f"v{ver}":
    sys.exit(f"tag {tag} != VERSION {ver}")

shutil.rmtree(OUT, ignore_errors=True)
stage = os.path.join(OUT, "stage")
unix = os.path.join(stage, "pkg-unix")
shutil.copytree(PKG, unix, ignore=lambda d, names: [n for n in names if n in EXCLUDE or n.endswith(".pyc")])
subprocess.run([sys.executable, os.path.join(PKG, "win", "build_win.py"), os.path.join(stage, "pkg-win")], check=True)

log = open(os.path.join(PKG, "CHANGELOG.md"), encoding="utf-8").read()
m = re.search(rf"^## {re.escape(ver)}\n(.*?)(?=^## |\Z)", log, re.S | re.M)
open(os.path.join(OUT, "NOTES.md"), "w", encoding="utf-8").write((m.group(1).strip() if m else "") + f"""

### 내려받기
- **macOS:** `Claude-Lean-{ver}-mac.zip` → 풀어서 `Claude Lean.app` 실행 (처음엔 우클릭 → 열기: 서명되지 않은 앱)
- **Windows:** `claude-lean-{ver}.exe` 실행 (SmartScreen이 막으면 추가 정보 → 실행)
- **macOS/Linux 터미널:** `claude-lean-{ver}-terminal.zip`을 풀고 `cd claude-lean && ./install.sh` (폴더 없이 실행하면 선택 창·질문, `./install.sh <프로젝트 폴더>`면 바로 설치)
- 모두 Python 3이 필요합니다 (설치 후 훅이 Python으로 동작).
""")
if "--stage-only" in sys.argv:
    print("staged", ver)
    sys.exit(0)

# macOS/Linux terminal package, the usual way: unzip, then ./install.sh
def zipdir(src, zp, top):
    with zipfile.ZipFile(zp, "w", zipfile.ZIP_DEFLATED) as z:
        for root, _, files in os.walk(src):
            for f in files:
                full = os.path.join(root, f)
                info = zipfile.ZipInfo.from_file(full, os.path.join(top, os.path.relpath(full, src)))
                info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, open(full, "rb").read())


zipdir(unix, os.path.join(OUT, f"claude-lean-{ver}-terminal.zip"), "claude-lean")

# macOS app: a plain bundle around mac/launcher.sh (native dialogs via osascript), no runtime bundled
app = os.path.join(stage, "Claude Lean.app", "Contents")
shutil.copytree(unix, os.path.join(app, "Resources", "pkg"))
os.makedirs(os.path.join(app, "MacOS"))
shutil.copy(os.path.join(PKG, "mac", "launcher.sh"), os.path.join(app, "MacOS", "claude-lean"))
os.chmod(os.path.join(app, "MacOS", "claude-lean"), 0o755)
open(os.path.join(app, "Info.plist"), "w", encoding="utf-8").write(f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>claude-lean</string>
  <key>CFBundleIdentifier</key><string>io.github.int-ttt.claude-lean</string>
  <key>CFBundleName</key><string>Claude Lean</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{ver}</string>
  <key>CFBundleVersion</key><string>{ver}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
""")
zipdir(os.path.dirname(app), os.path.join(OUT, f"Claude-Lean-{ver}-mac.zip"), "Claude Lean.app")

# Windows: the package the exe embeds (compiled on Windows from win/installer/Installer.cs)
zipdir(os.path.join(stage, "pkg-win"), os.path.join(stage, "pkg-win.zip"), "")
print("built", ver, sorted(n for n in os.listdir(OUT) if n != "stage"))
