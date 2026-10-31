#!/usr/bin/env python3
"""usage_sync.py - 하네스 사용량 기록(metrics, LEDGER, Claude 세션 jsonl)을 PC 간에 옮긴다.

사용법 (3단계):
  1. 보내는 PC: python3 -I scripts/usage_sync.py export --root . [--include-claude-projects] [--include-conversation-text]
     --include-claude-projects: ~/.claude/projects 를 사용량 필드만 남긴 행(metrics-only)으로 복사한다.
       message.content 와 텍스트는 빠지며, usage 가 없는 줄은 버린다. 기본값은 복사하지 않음.
     --include-conversation-text: --include-claude-projects 와 함께만 쓴다. 원본 jsonl(대화 텍스트 포함)을
       이메일 마스킹만 하고 그대로 복사한다. 단독으로 쓰면 오류.
  2. 생성된 usage-bundle-<host>-<YYYYMMDD>.zip 파일을 받는 PC로 복사한다.
  3. 받는 PC: python3 -I scripts/usage_sync.py merge usage-bundle-<host>-<YYYYMMDD>.zip --root . [--label NAME]
     -> notes/metrics/imported/<label>/ 에 줄 단위 중복 없이 추가된다. 다시 실행하면 added 0.

stdlib only. 이메일 주소는 <email>로 마스킹된다. 출력은 파일별 개수만 보여 주며 기록 내용은 출력하지 않는다.
"""
import argparse
import datetime as dt
import json
import re
import socket
import sys
import zipfile
from pathlib import Path

EMAIL_RE = re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+")
SAFE_RE = re.compile(r"[^A-Za-z0-9._-]")
CLAUDE_PREFIX = "claude-projects/"


def safe_name(s):
    return SAFE_RE.sub("_", s)


def to_text(raw):
    return raw.decode("utf-8", errors="replace")


def mask(text):
    return EMAIL_RE.subn("<email>", text)


def split_lines(text):
    return [ln.rstrip("\r") for ln in text.split("\n") if ln.strip()]


def classify(name):
    """Return (kind, rel) for a bundle member, None to ignore; raise ValueError if unsafe."""
    if ".." in name or "\\" in name or name.startswith("/") or re.match(r"^[A-Za-z]:", name):
        raise ValueError("unsafe member name: %r" % name)
    if name == "ledger/LEDGER.md":
        return ("ledger", "LEDGER.md")
    if re.fullmatch(r"metrics/[^/]+\.jsonl", name):
        return ("metrics", name[len("metrics/"):])
    if name.startswith(CLAUDE_PREFIX):
        rel = name[len(CLAUDE_PREFIX):]
        if rel.endswith(".jsonl") and rel[:-len(".jsonl")]:
            return ("claude", rel)
    return None


KEEP_TOP = ("type", "timestamp", "sessionId", "uuid", "cwd", "isSidechain", "requestId")
KEEP_MSG = ("id", "model", "usage")


def reduce_line(obj):
    """Keep only metric fields of a claude-projects line; None if it has no message.usage."""
    if not isinstance(obj, dict):
        return None
    msg = obj.get("message")
    if not isinstance(msg, dict) or not isinstance(msg.get("usage"), dict):
        return None
    out = {k: obj[k] for k in KEEP_TOP if k in obj}
    out["message"] = {k: msg[k] for k in KEEP_MSG if k in msg}
    return out


def cmd_export(args):
    if args.include_conversation_text and not args.include_claude_projects:
        raise ValueError("--include-conversation-text requires --include-claude-projects")
    root = Path(args.root).expanduser()
    now = dt.datetime.now().astimezone()
    host = socket.gethostname()
    if args.out:
        out = Path(args.out).expanduser()
    else:
        out = Path("usage-bundle-%s-%s.zip" % (safe_name(host), now.strftime("%Y%m%d")))
    names = []
    masked_total = 0
    if not args.include_claude_projects:
        mode = "off"
    elif args.include_conversation_text:
        mode = "raw"
    else:
        mode = "metrics-only"

    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:

        def add(arc, path):
            nonlocal masked_total
            try:
                text, n = mask(to_text(path.read_bytes()))
            except OSError as e:
                print("skip %s: read error (%s)" % (arc, e.strerror))
                return None
            masked_total += n
            zf.writestr(arc, text.encode("utf-8"))
            names.append(arc)
            return len(split_lines(text))

        def add_reduced(arc, path):
            nonlocal masked_total
            try:
                raw = path.read_bytes()
            except OSError as e:
                print("skip %s: read error (%s)" % (arc, e.strerror))
                return None
            kept = []
            for ln in split_lines(to_text(raw)):
                try:
                    obj = json.loads(ln)
                except ValueError:
                    continue
                red = reduce_line(obj)
                if red is not None:
                    kept.append(json.dumps(red, ensure_ascii=False))
            text, n = mask("".join(x + "\n" for x in kept))
            masked_total += n
            zf.writestr(arc, text.encode("utf-8"))
            names.append(arc)
            return len(kept)

        def add_group(title, items, fn=add):
            files = lines = 0
            for arc, path in items:
                n = fn(arc, path)
                if n is not None:
                    files += 1
                    lines += n
            print("%s: %d files, %d lines" % (title, files, lines))

        mdir = root / "notes" / "metrics"
        if mdir.is_dir():
            items = [("metrics/" + p.name, p) for p in sorted(mdir.glob("*.jsonl")) if p.is_file()]
            add_group("metrics", items)
        else:
            print("skip metrics: %s not found" % mdir)

        ledger = root / "notes" / "LEDGER.md"
        if ledger.is_file():
            add_group("ledger", [("ledger/LEDGER.md", ledger)])
        else:
            print("skip ledger: %s not found" % ledger)

        if args.include_claude_projects:
            pdir = Path.home() / ".claude" / "projects"
            if pdir.is_dir():
                items = [
                    (CLAUDE_PREFIX + p.relative_to(pdir).as_posix(), p)
                    for p in sorted(pdir.rglob("*.jsonl")) if p.is_file()
                ]
                add_group("claude-projects", items, add if args.include_conversation_text else add_reduced)
            else:
                print("skip claude-projects: %s not found" % pdir)

        manifest = {"host": host, "created": now.isoformat(timespec="seconds"), "files": names}
        zf.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False, indent=2).encode("utf-8"))

    print("claude-projects mode: %s" % mode)
    print("wrote %s: %d members, masked emails: %d" % (out, len(names), masked_total))
    return 0


def cmd_merge(args):
    bundle = Path(args.bundle).expanduser()
    root = Path(args.root).expanduser()
    with zipfile.ZipFile(bundle) as zf:
        manifest = {}
        if "manifest.json" in zf.namelist():
            try:
                manifest = json.loads(to_text(zf.read("manifest.json")))
            except ValueError as e:
                raise ValueError("bad manifest.json: %s" % e)
        if not isinstance(manifest, dict):
            raise ValueError("bad manifest.json: not an object")

        # Validate every member before writing anything.
        plan = []
        ignored = 0
        for info in zf.infolist():
            if info.is_dir():
                continue
            cat = classify(info.filename)
            if cat is None:
                if info.filename != "manifest.json":
                    ignored += 1
                continue
            plan.append((cat[0], cat[1], info.filename))

        raw_label = args.label or manifest.get("host")
        if not raw_label:
            raise ValueError("no label: manifest has no host; pass --label NAME")
        label = safe_name(str(raw_label))
        if not label.strip("."):
            raise ValueError("label is empty after sanitizing: %r" % raw_label)
        base = root / "notes" / "metrics" / "imported" / label
        print("label: %s -> %s" % (label, base.as_posix()))

        tot_files = tot_add = tot_skip = 0
        for kind, rel, name in plan:
            if kind == "metrics":
                target = base / rel
            elif kind == "ledger":
                target = base / "LEDGER.md"
            else:
                target = base / "claude-projects" / Path(*rel.split("/"))

            incoming = split_lines(mask(to_text(zf.read(name)))[0])
            old_raw = target.read_bytes() if target.is_file() else None
            seen = set(split_lines(to_text(old_raw))) if old_raw else set()

            new = []
            skipped = 0
            for ln in incoming:
                if ln in seen:
                    skipped += 1
                else:
                    seen.add(ln)
                    new.append(ln)

            if new or old_raw is None:
                target.parent.mkdir(parents=True, exist_ok=True)
                with open(str(target), "a", encoding="utf-8", newline="\n") as f:
                    if new and old_raw and not old_raw.endswith(b"\n"):
                        f.write("\n")
                    for ln in new:
                        f.write(ln + "\n")

            tot_files += 1
            tot_add += len(new)
            tot_skip += skipped
            print("%s: added %d, skipped %d" % (target.relative_to(base).as_posix(), len(new), skipped))

    print("total: %d files, added %d, skipped %d, ignored members %d" % (tot_files, tot_add, tot_skip, ignored))
    return 0


def main(argv=None):
    p = argparse.ArgumentParser(description="Export/merge harness usage records between computers.")
    sub = p.add_subparsers(dest="cmd", required=True)

    e = sub.add_parser("export", help="write usage-bundle zip")
    e.add_argument("--root", default=".")
    e.add_argument("--out", default=None)
    e.add_argument(
        "--include-claude-projects",
        action="store_true",
        help="copy ~/.claude/projects as metrics-only rows (no message text)",
    )
    e.add_argument(
        "--include-conversation-text",
        action="store_true",
        help="with --include-claude-projects: copy raw jsonl including conversation text",
    )

    m = sub.add_parser("merge", help="merge a usage-bundle zip into notes/metrics/imported/<label>/")
    m.add_argument("bundle")
    m.add_argument("--root", default=".")
    m.add_argument("--label", default=None)

    args = p.parse_args(argv)
    try:
        if args.cmd == "export":
            return cmd_export(args)
        return cmd_merge(args)
    except (OSError, ValueError, zipfile.BadZipFile) as err:
        print("error: %s" % err, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
