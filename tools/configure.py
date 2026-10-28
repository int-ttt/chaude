#!/usr/bin/env python3
"""Interactive installer: pick project folders and options, then run the platform installer.

usage: python3 tools/configure.py [--tty]
Opens a settings window (tkinter) when one is available, otherwise asks in the terminal (--tty forces it).
Options are passed to the installer through CLAUDE_LEAN_OPTIONS and saved in each project
(.claude/claude-lean.options.json), so later upgrades keep them.
"""
import json, os, shutil, subprocess, sys, tempfile
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

if getattr(sys, "frozen", False):                  # Claude Lean.app / claude-lean.exe: the package is bundled as data
    PKG = os.path.join(sys._MEIPASS, "pkg")
else:
    PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WIN = os.name == "nt"
DEFAULTS = {"main": "opus", "style": "explain", "inbox": True, "reset_alert": 0, "history_export": True,
            "nesting": True, "start_mode": "lean"}
# (key, question, [(value, label), ...])
CHOICES = [
    ("main", "메인 모델", [("opus", "Opus (기본: 판단이 많은 작업)"), ("sonnet", "Sonnet (지시가 명확한 작업, 더 저렴)")]),
    ("style", "답변 방식", [("explain", "설명형 (일반 Opus처럼 풀어서 설명)"), ("concise", "간결형 (5줄 이내 요약)")]),
    ("inbox", "정보 문장 메모 (모델 호출 없이 보관했다가 다음 지시와 함께 전달)", [(True, "켜기"), (False, "끄기")]),
    ("reset_alert", "맥락이 커지면 /exit 후 다시 시작하라는 안내", [(0, "끄기 (기본)"), (100000, "100k에서 안내"), (120000, "120k에서 안내")]),
    ("history_export", "대화 기록을 logs/history에 txt로 자동 저장", [(True, "켜기"), (False, "끄기")]),
    ("nesting", "Sonnet·Opus 에이전트가 단순 작업을 Haiku에게 직접 넘기기", [(True, "켜기"), (False, "끄기")]),
    ("start_mode", "설치 직후 모드", [("lean", "절약 모드"), ("normal", "일반 모드 (절약 규칙 꺼짐, 나중에 '절약모드' 입력)")]),
]
ACTIONS = [("install", "설치·업그레이드만"), ("fresh", "기존 대화 정리 후 새로 시작 (/distill 준비, Claude 창을 닫고 실행)")]


def saved_options(folder):
    try:
        return json.load(open(os.path.join(folder, ".claude", "claude-lean.options.json"), encoding="utf-8"))
    except Exception:
        return {}


def run(folders, action, opts):
    tmp = tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8")
    json.dump(opts, tmp, ensure_ascii=False)
    tmp.close()
    env = dict(os.environ, CLAUDE_LEAN_OPTIONS=tmp.name, PYTHONUTF8="1")
    if not WIN:                                    # apps opened from Finder get a minimal PATH
        env["PATH"] = env.get("PATH", "") + ":/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"
        if not shutil.which("python3", path=env["PATH"]):
            return "python3가 필요합니다. macOS: 터미널에서 xcode-select --install, Linux: 패키지 관리자로 python3 설치 후 다시 실행하세요."
    main = ["-Main", opts["main"]] if WIN else ["--main", opts["main"]]
    if WIN:
        ps = ["powershell", "-ExecutionPolicy", "Bypass", "-File"]
        cmds = ([ps + [os.path.join(PKG, "fresh_start.ps1")] + folders + main] if action == "fresh"
                else [ps + [os.path.join(PKG, "install.ps1"), f] + main for f in folders])
    else:
        cmds = ([["bash", os.path.join(PKG, "fresh_start.sh")] + folders + main] if action == "fresh"
                else [["bash", os.path.join(PKG, "install.sh"), f] + main for f in folders])
    out = []
    for c in cmds:
        r = subprocess.run(c, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace",
                           creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0) if WIN else 0)
        out.append((r.stdout + r.stderr).strip())
    os.unlink(tmp.name)
    return "\n\n".join(out)


def tty():
    if WIN:
        os.system("")                       # turn on ANSI colors in the Windows console
    color = sys.stdout.isatty() and not os.environ.get("NO_COLOR")
    c = lambda code, s: f"\033[{code}m{s}\033[0m" if color else s
    title, ok, warn, err, dim, key = (lambda s: c("1;36", s)), (lambda s: c("32", s)), (lambda s: c("33", s)), \
                                     (lambda s: c("31", s)), (lambda s: c("2", s)), (lambda s: c("1;33", s))

    def ask(head, items, cur):
        print("\n" + title(head))
        for i, (v, label) in enumerate(items, 1):
            print(f"  {key(str(i))}. {label}" + (ok("  ← 현재") if v == cur else ""))
        k = input(dim("번호 (Enter = 현재값): ")).strip()
        return items[int(k) - 1][0] if k.isdigit() and 1 <= int(k) <= len(items) else cur

    print(title("━━ Claude Lean 설치 ━━") + dim(f"  (버전 {open(os.path.join(PKG, 'VERSION')).read().strip()})"))
    print(dim("프로젝트 폴더를 입력하세요. Finder/탐색기에서 폴더를 끌어다 놓아도 됩니다. 여러 개면 하나씩, 다 넣었으면 Enter."))
    folders = []
    while True:
        f = input(key(f"폴더 {len(folders) + 1}") + dim(" (끝내려면 Enter)" if folders else "") + ": ").strip()
        f = f.strip("'\"").replace("\\ ", " ").rstrip("/\\") if f not in ("/", "\\") else f
        if not f:
            if folders:
                break
            continue
        f = os.path.abspath(os.path.expanduser(f))
        if not os.path.isdir(f):
            print(err("  ✗ 폴더가 없습니다: ") + f)
        elif f in folders:
            print(warn("  이미 넣은 폴더입니다."))
        else:
            folders.append(f)
            old = saved_options(f)
            print(ok("  ✓ ") + f + (dim("  (이미 설치됨: 이전 선택사항을 기본값으로 씁니다)") if old else ""))
    opts = dict(DEFAULTS)
    opts.update(saved_options(folders[0]))

    action = ask("어떤 작업을 할까요?", ACTIONS, "install")
    how = ask("선택사항", [("keep", "지금 값 그대로 (처음이면 기본값)"), ("pick", "하나씩 고르기")], "keep")
    if how == "pick":
        for k_, head, items in CHOICES:
            opts[k_] = ask(head, items, opts[k_])

    print("\n" + title("확인"))
    print(f"  폴더: " + ", ".join(folders))
    print(f"  작업: " + dict(ACTIONS)[action])
    for k_, head, items in CHOICES:
        print(dim(f"  {head.split(' (')[0]}: ") + dict(items).get(opts[k_], str(opts[k_])))
    if input(key("\n진행할까요? [Y/n] ")).strip().lower() in ("n", "no", "아니", "ㄴ"):
        print(warn("취소했습니다."))
        return
    print(dim("\n설치 중…"))
    for line in run(folders, action, opts).splitlines():
        print(err(line) if line.lstrip().startswith(("✗", "Traceback", "Error")) else
              ok(line) if line.startswith(("설치 완료", "버전", "✓")) else line)
    print(ok("\n완료. ") + "각 프로젝트 폴더에서 claude를 새로 시작하세요.")


def gui():
    import tkinter as tk
    from tkinter import filedialog, ttk
    root = tk.Tk()
    root.title("Claude Lean 설치")
    root.geometry("900x820")
    frm = ttk.Frame(root, padding=12)
    frm.pack(fill="both", expand=True)
    folders = []
    lst = tk.Listbox(frm, height=4)
    vars_ = {}

    def add():
        f = filedialog.askdirectory(title="프로젝트 폴더 선택")
        if f and f not in folders:
            folders.append(os.path.abspath(f))
            lst.insert("end", folders[-1])
            if len(folders) == 1:
                for k, v in saved_options(folders[0]).items():
                    if k in vars_:
                        vars_[k].set(json.dumps(v))

    ttk.Label(frm, text="프로젝트 폴더").pack(anchor="w")
    lst.pack(fill="x")
    ttk.Button(frm, text="폴더 추가…", command=add).pack(anchor="w", pady=(4, 10))
    act = tk.StringVar(value="install")
    box = ttk.LabelFrame(frm, text="작업", padding=6)
    box.pack(fill="x", pady=3)
    for v, label in ACTIONS:
        ttk.Radiobutton(box, text=label, variable=act, value=v).pack(anchor="w")
    bottom = ttk.Frame(root, padding=(12, 0, 12, 12))
    bottom.pack(side="bottom", fill="x")            # packed before the options so it is never cut off
    grid = ttk.Frame(frm)
    grid.pack(fill="x")
    grid.columnconfigure((0, 1), weight=1)
    for n, (key, title, items) in enumerate(CHOICES):
        vars_[key] = tk.StringVar(value=json.dumps(DEFAULTS[key]))
        box = ttk.LabelFrame(grid, text=title if len(title) < 34 else title.split(" (")[0], padding=6)
        box.grid(row=n // 2, column=n % 2, sticky="nsew", padx=3, pady=3)
        for v, label in items:
            ttk.Radiobutton(box, text=label, variable=vars_[key], value=json.dumps(v)).pack(anchor="w")
    log = tk.Text(bottom, height=7)

    def go():
        if not folders:
            log.delete("1.0", "end")
            log.insert("end", "프로젝트 폴더를 먼저 추가하세요.")
            return
        opts = {k: json.loads(v.get()) for k, v in vars_.items()}
        log.delete("1.0", "end")
        log.insert("end", "설치 중…\n")
        root.update()
        log.insert("end", run(folders, act.get(), opts) + "\n\n완료. 프로젝트 폴더에서 claude를 새로 시작하세요.")

    ttk.Button(bottom, text="설치", command=go).pack(pady=(0, 6))
    log.pack(fill="x")
    root.mainloop()


if __name__ == "__main__":
    try:
        if "--tty" in sys.argv:
            tty()
        else:
            try:
                gui()
            except Exception as e:        # no tkinter / no display: fall back to the terminal
                print(f"(설정 창을 열 수 없어 터미널로 진행합니다: {type(e).__name__})")
                tty()
    except (EOFError, KeyboardInterrupt):
        print("\n취소했습니다.")
