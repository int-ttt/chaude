#!/usr/bin/env python3
"""Build the Windows package from the shared (macOS) package.

usage: python3 win/build_win.py [<out-dir>]   (default: ../claude-opus-lean-win next to the package)
Copies shared files, swaps in the PowerShell installers, rewrites settings.json hooks to exec form
(python.exe path filled in at install time), and converts docs/commands to Windows. Fails on leftovers.
"""
import json, os, re, shutil, sys

PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.abspath(sys.argv[1]) if len(sys.argv) > 1 else os.path.join(os.path.dirname(PKG), "claude-opus-lean-win")
SKIP = {"install.sh", "fresh_start.sh", "win", "logs", "notes", "__pycache__", ".git", ".github", "dist", "history", ".gitignore", "mac"}

if os.path.exists(OUT):
    shutil.rmtree(OUT)
shutil.copytree(PKG, OUT, ignore=lambda d, names: [n for n in names if n in SKIP])

# PowerShell files need a UTF-8 BOM so Windows PowerShell 5.1 reads the Korean text correctly.
W = os.path.join(PKG, "win")
for rel in ("install.ps1", "fresh_start.ps1", "tools/common.ps1"):
    text = open(os.path.join(W, rel), encoding="utf-8").read().replace("\r\n", "\n").replace("\n", "\r\n")
    os.makedirs(os.path.dirname(os.path.join(OUT, rel)), exist_ok=True)
    open(os.path.join(OUT, rel), "w", encoding="utf-8-sig", newline="").write(text)

# settings.json: exec-form hooks (no shell quoting), python path filled in by install.ps1.
sp = os.path.join(OUT, ".claude", "settings.json")
s = json.load(open(sp, encoding="utf-8"))
for event, groups in s["hooks"].items():
    for g in groups:
        for h in g["hooks"]:
            script = re.search(r"\.claude/hooks/([\w-]+\.py)", h["command"]).group(1)
            timeout = h.get("timeout", 5)
            h.clear()
            h.update({"type": "command", "command": "__PYTHON__",
                      "args": ["-X", "utf8", "${CLAUDE_PROJECT_DIR}/.claude/hooks/" + script], "timeout": timeout})
s["env"]["PYTHONUTF8"] = "1"
s["permissions"]["allow"] = [r.replace("python3 ", "__BASHPY__ ") for r in s["permissions"]["allow"]]
json.dump(s, open(sp, "w", encoding="utf-8"), indent=2, ensure_ascii=False)
open(sp, "a").write("\n")


def edit(rel, pairs, generic=True):
    p = os.path.join(OUT, rel)
    t = open(p, encoding="utf-8").read()
    for a, b in pairs:
        if a not in t:
            raise SystemExit(f"{rel}: 찾을 수 없음 → {a[:70]!r}")
        t = t.replace(a, b)
    if generic:
        t = t.replace("python3 ", "python ")
    open(p, "w", encoding="utf-8").write(t)


# Agents, skills, policy: only the interpreter name changes.
for root, _, files in os.walk(os.path.join(OUT, ".claude")):
    for f in files:
        if f.endswith(".md"):
            edit(os.path.relpath(os.path.join(root, f), OUT), [])
edit("CLAUDE.md", [])
edit(".claude/agents/haiku-runner.md", [(
    "`nohup <cmd> > logs/<job>.log 2>&1 &`",
    "Git Bash `nohup <cmd> > logs/<job>.log 2>&1 &`, or PowerShell `Start-Process -WindowStyle Hidden -FilePath <exe> -ArgumentList '<args>' -RedirectStandardOutput logs/<job>.log -RedirectStandardError logs/<job>.err -PassThru` (record the PID)")])

PS_INSTALL = "powershell -ExecutionPolicy Bypass -File .\\install.ps1"
PS_FRESH = "powershell -ExecutionPolicy Bypass -File .\\fresh_start.ps1"

edit("APPLY_GUIDE.md", [
    ("""```bash
claude --version        # 2.1.296 이상 권장 (에이전트별 autoCompactWindow 지원). 낮으면: claude update
python3 --version       # 없으면: xcode-select --install
unzip ~/Downloads/claude-opus-lean.zip -d ~/claude-opus-lean
```""", """PowerShell에서 실행합니다.
```powershell
claude --version        # 최신 버전 권장 (훅의 실행 방식 지원). 낮으면: claude update
py -3 --version         # 또는 python --version. 없으면 python.org에서 설치 ('Add python.exe to PATH' 체크)
git --version           # Git for Windows 권장 (Claude의 Bash 도구용). 없어도 PowerShell 도구로 동작
Expand-Archive "$HOME\\Downloads\\claude-opus-lean-win.zip" -DestinationPath "$HOME\\claude-opus-lean"
```"""),
    ("""```bash
python3 ~/claude-opus-lean/scripts/usage_report.py > ~/usage-before.txt
```""", """```powershell
py -3 -X utf8 "$HOME\\claude-opus-lean\\scripts\\usage_report.py" | Out-File -Encoding utf8 "$HOME\\usage-before.txt"
```"""),
    ("""```bash
cd ~/claude-opus-lean
chmod +x install.sh fresh_start.sh
./install.sh <프로젝트 폴더>                 # 메인 Opus
./install.sh <프로젝트 폴더> --main sonnet   # 판단이 적은 프로젝트는 메인 Sonnet
```""", f"""```powershell
cd "$HOME\\claude-opus-lean"
{PS_INSTALL} "<프로젝트 폴더>"               # 메인 Opus
{PS_INSTALL} "<프로젝트 폴더>" -Main sonnet  # 판단이 적은 프로젝트는 메인 Sonnet
```
`-ExecutionPolicy Bypass`는 이 실행에만 적용되며, 시스템 설정은 바꾸지 않습니다."""),
    ("`./fresh_start.sh <폴더> <폴더> ...`", "`fresh_start.ps1 \"<폴더>\" \"<폴더>\" ...`"),
    ("(`fresh_start.sh`가 설치까지", "(`fresh_start.ps1`이 설치까지"),
    ("""```bash
cd <프로젝트 폴더> && claude
```""", """```powershell
cd "<프로젝트 폴더>"; claude
```"""),
    ("""```bash
echo '{"prompt":"ㅇㅇ"}' | python3 .claude/hooks/style-adapter.py      # [ack] … 출력
echo '{"tool_name":"Read"}' | python3 .claude/hooks/main-thread-guard.py  # deny JSON 출력
```""", """```powershell
$OutputEncoding = [Text.Encoding]::UTF8
'{"prompt":"ㅇㅇ"}' | py -3 -X utf8 .claude\\hooks\\style-adapter.py         # [ack] … 출력
'{"tool_name":"Read"}' | py -3 -X utf8 .claude\\hooks\\main-thread-guard.py   # deny JSON 출력
```"""),
    ("| 실행 시 `CLAUDE_LEAN=0 claude` | — | 그 세션만 일반 모드로 시작 (프로젝트 상태는 그대로) |",
     "| 실행 시 `$env:CLAUDE_LEAN=0; claude` | — | 그 세션만 일반 모드로 시작 (프로젝트 상태는 그대로). 끝나면 `Remove-Item Env:CLAUDE_LEAN` |"),
    ("`--main sonnet`", "`-Main sonnet`"),
    ("""```bash
python3 ~/claude-opus-lean/scripts/usage_report.py > ~/usage-after.txt
diff ~/usage-before.txt ~/usage-after.txt
```""", """```powershell
py -3 -X utf8 "$HOME\\claude-opus-lean\\scripts\\usage_report.py" | Out-File -Encoding utf8 "$HOME\\usage-after.txt"
Compare-Object (Get-Content "$HOME\\usage-before.txt") (Get-Content "$HOME\\usage-after.txt")
```"""),
    ("""```bash
cd <프로젝트 폴더>
cp .claude-backup-<시각>/CLAUDE.md . ; cp .claude-backup-<시각>/.claude/settings.json .claude/
rm -f .claude/hooks/{correction-guard,style-adapter,main-thread-guard,haiku-context-guard,agent-model-pin,ledger,session-context,context-alert,history-export,_lean}.py
```""", """```powershell
cd "<프로젝트 폴더>"
Copy-Item .claude-backup-<시각>\\CLAUDE.md . ; Copy-Item .claude-backup-<시각>\\.claude\\settings.json .claude\\
'correction-guard','style-adapter','main-thread-guard','haiku-context-guard','agent-model-pin','ledger','session-context','context-alert','history-export','_lean' | ForEach-Object { Remove-Item ".claude\\hooks\\$_.py" }
```"""),
    ("| 훅 오류 메시지 | `python3 .claude/hooks/<훅>.py` 직접 실행해 오류 확인, `/hooks`에서 등록 확인 |",
     "| 훅 오류 메시지 | `py -3 -X utf8 .claude\\hooks\\<훅>.py` 직접 실행해 오류 확인, `/hooks`에서 등록 확인. 설치 후 Python을 옮겼다면 `install.ps1`을 다시 실행 |"),
    ("터미널에서 직접: `python3 scripts/search_history.py \"<단어>\"`", "터미널에서 직접: `py -3 -X utf8 scripts\\search_history.py \"<단어>\"`"),
])

edit("FRESH_START.md", [
    ("""```bash
cd ~/claude-opus-lean
./fresh_start.sh <프로젝트 폴더> [<프로젝트 폴더> ...]
```""", f"""```powershell
cd "$HOME\\claude-opus-lean"
{PS_FRESH} "<프로젝트 폴더>" ["<프로젝트 폴더>" ...]
```"""),
    ("""```bash
./fresh_start.sh ~/Documents/report ~/code/webapp ~/code/analysis --main sonnet
```""", f"""```powershell
{PS_FRESH} "$HOME\\Documents\\report" "D:\\code\\webapp" -Main sonnet
```"""),
    ("| 설치 | `install.sh` 실행", "| 설치 | `install.ps1` 실행"),
    ("`~/claude-archive/<프로젝트명>-<날짜>`", "`%USERPROFILE%\\claude-archive\\<프로젝트명>-<날짜>`"),
    ("- **옵션 `--main sonnet`:**", "- **옵션 `-Main sonnet`:**"),
    ("- **옵션 `--force`:** 다른 프로젝트의 Claude만 열려 있을 때, 실행 중 확인을 건너뜁니다.",
     "- **옵션 `-Force`:** 다른 프로젝트의 Claude나 Claude 데스크톱 앱만 열려 있을 때, 실행 중 확인을 건너뜁니다."),
    ("`~/usage-before.txt`", "`%USERPROFILE%\\usage-before.txt`"),
    ("""```bash
cd <프로젝트 폴더> && claude
```""", """```powershell
cd "<프로젝트 폴더>"; claude
```"""),
    ("""```bash
P=<프로젝트 폴더>
cd ~/claude-opus-lean && ./install.sh "$P"
cd "$P" && python3 scripts/extract_transcript.py "$P" --out notes/distill
ENC=$(python3 -c "import re,sys;print(re.sub(r'[^A-Za-z0-9]','-',sys.argv[1]))" "$(cd "$P" && pwd)")
mkdir -p ~/claude-archive && mv ~/.claude/projects/"$ENC" ~/claude-archive/"$(basename "$P")"-$(date +%Y%m%d)
```
기록 폴더를 못 찾으면 `ls ~/.claude/projects`에서 이름을 확인해 `extract_transcript.py`에 그 폴더를 직접 지정하세요.""",
     f"""```powershell
$P = (Resolve-Path "<프로젝트 폴더>").Path
cd "$HOME\\claude-opus-lean"; {PS_INSTALL} $P
cd $P; py -3 -X utf8 scripts\\extract_transcript.py $P --out notes\\distill
$H = py -3 -X utf8 scripts\\claude_history.py $P
New-Item -ItemType Directory -Force "$HOME\\claude-archive" | Out-Null
Move-Item $H "$HOME\\claude-archive\\$(Split-Path $P -Leaf)-$(Get-Date -Format yyyyMMdd)"
```
기록 폴더를 못 찾으면 `dir $HOME\\.claude\\projects`에서 이름을 확인해 `extract_transcript.py`에 그 폴더를 직접 지정하세요."""),
    ("그다음 세션을 닫고 A-1(`fresh_start.sh`)을 실행한 뒤", "그다음 세션을 닫고 A-1(`fresh_start.ps1`)을 실행한 뒤"),
])

edit("README.md", [
    ("# Claude Lean\n",
     "# Claude Lean (Windows)\n\n"
     "> **Windows 요구 사항:** Python 3 (python.org, 'Add python.exe to PATH' 체크), Git for Windows 권장(없으면 PowerShell 도구로 동작), 최신 Claude Code. "
     "설치는 `claude-lean-<버전>.exe`를 실행하거나, 이 폴더에서 `install.ps1`을 실행하세요. 훅은 설치 시 찾은 `python.exe` 경로로 직접 실행되므로 셸 종류와 무관합니다.\n"),
])

edit("MIGRATE_NOW_PROMPT.md", [
    ("`./install.sh <프로젝트 폴더>`로 먼저 설치합니다", f"`{PS_INSTALL} \"<프로젝트 폴더>\"`로 먼저 설치합니다"),
])

# Version label in README and the top CHANGELOG entry must match VERSION.
ver = open(os.path.join(PKG, "VERSION"), encoding="utf-8").read().strip()
for rel, needle in (("README.md", f"**버전 {ver}**"), ("CHANGELOG.md", f"## {ver}\n")):
    if needle not in open(os.path.join(PKG, rel), encoding="utf-8").read():
        raise SystemExit(f"{rel}: 버전 {ver} 표기가 없습니다")

# Leftover check: nothing macOS-only may remain in what ships.
BAD = re.compile(r"\./install\.sh|\./fresh_start\.sh|install\.sh|fresh_start\.sh|python3 |chmod|xcode|unzip |shasum|&& claude|--main|--force|~/claude|~/\.claude/projects/<|/Users/")
left = []
for root, _, files in os.walk(OUT):
    for f in files:
        if f.endswith((".md", ".json", ".ps1")) and f not in ("ANALYSIS.md", "DESIGN_HANDOFF.md"):
            p = os.path.join(root, f)
            for i, line in enumerate(open(p, encoding="utf-8-sig"), 1):
                if BAD.search(line):
                    left.append(f"{os.path.relpath(p, OUT)}:{i}: {line.strip()[:110]}")
print(f"built {OUT}")
if left:
    print("남은 macOS 표현:")
    print("\n".join(left))
    sys.exit(1)
