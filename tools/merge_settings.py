#!/usr/bin/env python3
"""Install/upgrade helper shared by install.sh (macOS) and install.ps1 (Windows).

usage: merge_settings.py <package-dir> <project-dir> <python-for-hooks> <main-model or ""> <python-for-Bash> <backup-dir>

- CLAUDE.md: keeps the project's own text; inserts or REPLACES the policy block (upgrades older installs).
- settings.json: merges keys/env/permissions/hooks; replaces our hooks in place; removes settings the package dropped.
- notes/: removes empty template lines left by older templates.
- Moves files that older package versions installed but this one no longer ships into <backup-dir>.
- Records the installed version in .claude/claude-lean.version.
"""
import glob, json, os, re, shutil, sys

for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

src, dst, py, main, bashpy, backup = sys.argv[1:7]
START, END = "<!-- claude-lean:start -->", "<!-- claude-lean:end -->"
OLD_HEAD = "# Policy: Opus listens and routes; subagents do all work"
DROPPED_ENV = {"CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "70"}          # removed only if still at our old value
OBSOLETE = [".claude/agents/senior.md", ".claude/agents/advisor.md",
            ".claude/skills/doc-revise", ".claude/skills/long-job", ".claude/skills/debug", ".claude/skills/verify"]
# Agents renamed with their model as prefix (2026.10.28). Moved only if the file is ours (description starts with Haiku./Sonnet.).
RENAMED_AGENTS = ["scout", "researcher", "drafter", "checker", "batch-worker", "runner", "coder", "extractor", "analyst", "reviewer"]
done = []


def read(p, enc="utf-8-sig"):
    return open(p, encoding=enc).read() if os.path.exists(p) else ""


def fill(o):
    if isinstance(o, dict):
        return {k: fill(v) for k, v in o.items()}
    if isinstance(o, list):
        return [fill(v) for v in o]
    if isinstance(o, str):
        return o.replace("__PYTHON__", py).replace("__BASHPY__", bashpy)
    return o


def hook_key(h):
    """A hook is identified by the script it runs, so a changed interpreter path replaces the old entry."""
    args = h.get("args") or []
    target = args[-1] if args else h.get("command", "")
    m = re.search(r"\.claude/hooks/[\w-]+\.py", target)
    return m.group(0) if m else target


# 1) CLAUDE.md policy block
block = START + "\n" + read(os.path.join(src, "CLAUDE.md"), "utf-8").strip() + "\n" + END
md_path = os.path.join(dst, "CLAUDE.md")
md = read(md_path)
if START in md and END in md:
    new = md[: md.index(START)] + block + md[md.index(END) + len(END):]
    state = "정책 갱신"
elif OLD_HEAD in md:
    s = md.index(OLD_HEAD)
    c = md.find("# Compact instructions", s)
    e = md.find("\n\n", md.find("\n", c) + 1) if c != -1 else -1
    e = len(md) if e == -1 else e
    new = md[:s] + block + md[e:]
    state = "예전 정책 → 최신 정책으로 교체"
else:
    new = block + ("\n\n" + md if md.strip() else "\n")
    state = "정책 추가" + (" (기존 내용은 아래에 유지)" if md.strip() else "")
if new != md:
    open(md_path, "w", encoding="utf-8").write(new)
    done.append("CLAUDE.md: " + state)
else:
    done.append("CLAUDE.md: 최신")

# 2) settings.json
ours = fill(json.load(open(os.path.join(src, ".claude", "settings.json"), encoding="utf-8")))
sp = os.path.join(dst, ".claude", "settings.json")
cur = json.loads(read(sp) or "{}")
changed = [k for k in ("model", "effortLevel", "autoCompactWindow") if k in cur and cur[k] != ours[k]]
for k in ("model", "effortLevel", "autoCompactWindow"):
    cur[k] = ours[k]
if main:
    cur["model"] = main
env = cur.setdefault("env", {})
env.update(ours["env"])
removed = [k for k, v in DROPPED_ENV.items() if env.get(k) == v]
for k in removed:
    del env[k]
allow = cur.setdefault("permissions", {}).setdefault("allow", [])
allow += [r for r in ours.get("permissions", {}).get("allow", []) if r not in allow]
hooks = cur.setdefault("hooks", {})
for event, groups in ours["hooks"].items():
    existing = hooks.setdefault(event, [])
    for g in groups:
        k = hook_key(g["hooks"][0])
        for i, old in enumerate(existing):
            if any(hook_key(h) == k for h in old.get("hooks", [])):
                existing[i] = g
                break
        else:
            existing.append(g)
with open(sp, "w", encoding="utf-8") as f:
    json.dump(cur, f, indent=2, ensure_ascii=False)
    f.write("\n")
msg = "settings.json: 병합 완료 (메인 " + cur["model"] + ")"
if changed:
    msg += " (기존 값 교체: " + ", ".join(changed) + ")"
if removed:
    msg += " (더 이상 쓰지 않는 설정 제거: " + ", ".join(removed) + ")"
done.append(msg)

# 3) Script command used by agents (Windows: python / py -3)
if bashpy not in ("python", "python3"):
    for f in glob.glob(os.path.join(dst, ".claude", "agents", "*.md")) + glob.glob(os.path.join(dst, ".claude", "skills", "*", "SKILL.md")):
        s = read(f, "utf-8")
        t = s.replace("python scripts/", bashpy + " scripts/")
        if t != s:
            open(f, "w", encoding="utf-8").write(t)
    done.append(f"스크립트 실행 명령: '{bashpy}'")

# 4) notes/: drop empty template lines such as "- [대회] "
cleaned = 0
for name in ("FACTS.md", "DECISIONS.md"):
    p = os.path.join(dst, "notes", name)
    s = read(p)
    if not s:
        continue
    lines = s.splitlines(keepends=True)
    keep = [l for l in lines if not re.fullmatch(r"- \[[^\]]*\]\s*", l.rstrip("\r\n") + " ")]
    if len(keep) != len(lines):
        cleaned += len(lines) - len(keep)
        open(p, "w", encoding="utf-8").writelines(keep)
if cleaned:
    done.append(f"notes: 빈 양식 줄 {cleaned}개 삭제")

# 5) Files older versions installed that this version no longer ships
moved = []
for rel in OBSOLETE:
    p = os.path.join(dst, *rel.split("/"))
    if os.path.exists(p):
        t = os.path.join(backup, "obsolete", *rel.split("/"))
        os.makedirs(os.path.dirname(t), exist_ok=True)
        shutil.move(p, t)
        moved.append(rel)
kept = []
for name in RENAMED_AGENTS:
    rel = f".claude/agents/{name}.md"
    p = os.path.join(dst, ".claude", "agents", name + ".md")
    if not os.path.exists(p):
        continue
    if re.search(r"^description: (Haiku|Sonnet)\.", read(p, "utf-8"), re.M):
        t = os.path.join(backup, "obsolete", ".claude", "agents", name + ".md")
        os.makedirs(os.path.dirname(t), exist_ok=True)
        shutil.move(p, t)
        moved.append(rel)
    else:
        kept.append(rel)
if moved:
    done.append("예전 버전 파일을 백업으로 이동: " + ", ".join(moved))
if kept:
    done.append("같은 이름의 사용자 에이전트는 그대로 둠: " + ", ".join(kept))

# 6) Version stamp
ver = read(os.path.join(src, "VERSION"), "utf-8").strip() or "unknown"
vp = os.path.join(dst, ".claude", "claude-lean.version")
old = read(vp, "utf-8").strip()
open(vp, "w", encoding="utf-8").write(ver + "\n")
done.append(f"버전: {old} → {ver}" if old and old != ver else f"버전: {ver}")

print("\n".join(done))
