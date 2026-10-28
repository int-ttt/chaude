#!/usr/bin/env python3
"""Interactive installer: pick project folders and options, then run the platform installer.

usage: python3 tools/configure.py [--tty]
Opens a settings window (tkinter) when one is available, otherwise asks in the terminal (--tty forces it).
Options are passed to the installer through CLAUDE_LEAN_OPTIONS and saved in each project
(.claude/claude-lean.options.json), so later upgrades keep them.
"""
import json, os, subprocess, sys, tempfile
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

PKG = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WIN = os.name == "nt"
DEFAULTS = {"main": "opus", "style": "explain", "inbox": True, "reset_alert": 150000, "history_export": True,
            "haiku_cap": 80000, "nesting": True, "start_mode": "lean"}
# (key, question, [(value, label), ...])
CHOICES = [
    ("main", "메인 모델", [("opus", "Opus (기본: 판단이 많은 작업)"), ("sonnet", "Sonnet (지시가 명확한 작업, 더 저렴)")]),
    ("style", "답변 방식", [("explain", "설명형 (일반 Opus처럼 풀어서 설명)"), ("concise", "간결형 (5줄 이내 요약)")]),
    ("inbox", "정보 문장 메모 (모델 호출 없이 보관했다가 다음 지시와 함께 전달)", [(True, "켜기"), (False, "끄기")]),
    ("reset_alert", "맥락이 커지면 /exit 후 다시 시작하라는 안내", [(150000, "150k에서 안내"), (250000, "250k에서 안내"), (0, "끄기")]),
    ("history_export", "대화 기록을 logs/history에 txt로 자동 저장", [(True, "켜기"), (False, "끄기")]),
    ("haiku_cap", "Haiku 에이전트 맥락 상한 (넘기 전에 새 에이전트로 교대)", [(80000, "80k (기본)"), (50000, "50k (더 저렴, 교대 잦음)"), (95000, "95k (교대 적음)")]),
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
        r = subprocess.run(c, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace")
        out.append((r.stdout + r.stderr).strip())
    os.unlink(tmp.name)
    return "\n\n".join(out)


def tty():
    print("Claude Lean 설치 (Enter = 괄호 안 기본값)\n")
    folders = []
    while True:
        f = input(f"프로젝트 폴더 경로{' (끝내려면 Enter)' if folders else ''}: ").strip().strip('"')
        if not f:
            if folders:
                break
            continue
        f = os.path.abspath(os.path.expanduser(f))
        if os.path.isdir(f):
            folders.append(f)
        else:
            print("  폴더가 없습니다:", f)
    opts = dict(DEFAULTS)
    opts.update(saved_options(folders[0]))

    def ask(title, items, cur):
        print("\n" + title)
        for i, (v, label) in enumerate(items, 1):
            print(f"  {i}. {label}{'  ← 현재' if v == cur else ''}")
        k = input("번호: ").strip()
        return items[int(k) - 1][0] if k.isdigit() and 1 <= int(k) <= len(items) else cur

    action = ask("작업", ACTIONS, "install")
    for key, title, items in CHOICES:
        opts[key] = ask(title, items, opts[key])
    print("\n" + run(folders, action, opts))


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
