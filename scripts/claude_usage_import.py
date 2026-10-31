#!/usr/bin/env python3
"""claude_usage_import.py - 일반 Claude Code(~/.claude)의 사용량 기록을 다른 PC에서 옮긴다.

사용법 (3단계):
  1. 보내는 PC: python3 -I scripts/claude_usage_import.py export [--claude-dir ~/.claude] [--include-conversations]
  2. 생성된 claude-usage-<host>-<YYYYMMDD>.zip 파일을 받는 PC로 복사한다.
  3. 받는 PC: python3 -I scripts/claude_usage_import.py merge claude-usage-<host>-<YYYYMMDD>.zip [--dest DIR] [--label NAME] [--corrections FILE]
     -> <dest>/<label>/usage.jsonl 에 중복 없이 추가되고 summary.json 이 다시 만들어진다. 다시 실행하면 added 0.
     --corrections FILE: {"<sid>": {"model_corrected": "라벨", "note": "..."}} 형식의 JSON. 해당 세션 행에 model_corrected 만 붙인다(model 은 그대로).
     merge 는 중복 제거 건수를 dedup-check 줄로 출력한다(zip 의 manifest rows 와 대조).
  4. 요약 보기(파일 쓰기 없음): python3 -I scripts/claude_usage_import.py summary [--dest DIR]
     -> <dest>/*/usage.jsonl 을 호스트·모델·프로젝트·사이드체인·역할별 표로 stdout 에 출력한다(개수·토큰 합계만).
     sessions.jsonl 은 세션별 컨텍스트 크기·사용량(메인/사이드체인 구분)을 담으며, merge 때 usage.jsonl 에서 다시 계산된다.
     중복 제거는 message.id 우선(없으면 uuid)이며, 같은 키가 여러 번 나오면 output 토큰이 큰 행을 남긴다.

주의: --include-conversations 를 주면 대화 jsonl(프롬프트 텍스트 포함)과 history.jsonl 도 zip에 들어간다.
      기본값은 꺼짐이며, 꺼진 상태에서는 사용량 행(토큰 수, 모델, 시각)만 내보낸다.

stdlib only. 이메일 주소는 <email>로 마스킹된다. 출력은 개수만 보여 주며 기록 내용은 출력하지 않는다.
"""
import argparse
import datetime as dt
import json
import re
import socket
import sys
import unicodedata
import zipfile
from pathlib import Path

EMAIL_RE = re.compile(r"[\w.+-]+@[\w-]+\.[\w.-]+")
SAFE_RE = re.compile(r"[^A-Za-z0-9._-]")
DATE_RE = re.compile(r"(\d{4}-\d{2}-\d{2})")
CONV_PREFIX = "conversations/"
USAGE_MEMBER = "usage/rows.jsonl"
STATS_MEMBER = "stats-cache.json"
SESSIONS_MEMBER = "sessions.jsonl"
TOKEN_KEYS = ("in", "out", "cc", "cr")
ROW_STR_FIELDS = ("key", "sid", "ts", "project", "model", "rid", "mid", "agent", "subagent", "tools", "cwd", "model_corrected")


class UnsafeMemberError(ValueError):
    """zip 멤버 이름이 안전하지 않다. merge 전체를 중단한다 (exit 2)."""


def safe_name(s):
    return SAFE_RE.sub("_", s)


def to_text(raw):
    return raw.decode("utf-8", errors="replace")


def mask(text):
    return EMAIL_RE.subn("<email>", text)


def split_lines(text):
    return [ln.rstrip("\r") for ln in text.split("\n") if ln.strip()]


def text_of(v):
    if isinstance(v, str):
        return v
    return "" if v is None else str(v)


def num(v):
    return v if isinstance(v, (int, float)) and not isinstance(v, bool) else 0


def classify(name):
    """Return (kind, rel) for a bundle member, None to ignore; raise UnsafeMemberError if unsafe."""
    if ".." in name or "\\" in name or name.startswith("/") or re.match(r"^[A-Za-z]:", name):
        raise UnsafeMemberError("unsafe member name: %r" % name)
    if name == USAGE_MEMBER:
        return ("usage", None)
    if name == STATS_MEMBER:
        return ("stats", None)
    if name == SESSIONS_MEMBER:
        return ("sessions", None)
    if name.startswith(CONV_PREFIX):
        rel = name[len(CONV_PREFIX):]
        segs = rel.split("/")
        if rel.endswith(".jsonl") and len(rel) > len(".jsonl") and all(s and s != "." for s in segs):
            return ("conv", rel)
    return None


def inside(path, root):
    try:
        path.resolve().relative_to(root)
    except ValueError:
        return False
    return True


def row_key(line):
    try:
        obj = json.loads(line)
    except ValueError:
        return None
    if isinstance(obj, dict) and isinstance(obj.get("key"), str):
        return obj["key"]
    return None


def row_mid(line):
    try:
        obj = json.loads(line)
    except ValueError:
        return ""
    v = obj.get("mid") if isinstance(obj, dict) else None
    return v if isinstance(v, str) else ""


def first_str(*vals):
    for v in vals:
        if isinstance(v, str) and v:
            return v
    return ""


def subagent_of(obj, msg):
    tur = obj.get("toolUseResult")
    cands = [obj, msg, tur if isinstance(tur, dict) else {}]
    return first_str(*(c.get(k) for c in cands for k in ("subagent_type", "subagentType")))


def tool_names(msg):
    """message.content 의 tool_use 이름을 정렬·중복 제거해 쉼표로 잇는다(최대 200자)."""
    content = msg.get("content")
    if not isinstance(content, list):
        return ""
    names = {b.get("name") for b in content if isinstance(b, dict) and b.get("type") == "tool_use"}
    return ",".join(sorted(n for n in names if isinstance(n, str) and n))[:200]


def manifest_rows_of(manifest):
    """manifest 의 행 수: 최상위 rows 우선, 없으면 counts.rows. 없으면 None."""
    v = manifest.get("rows")
    if not isinstance(v, int) or isinstance(v, bool):
        c = manifest.get("counts")
        v = c.get("rows") if isinstance(c, dict) else None
    return v if isinstance(v, int) and not isinstance(v, bool) else None


def load_corrections(path):
    """--corrections JSON 을 {sid: model_corrected} 로 읽는다. 형식이 틀리면 ValueError."""
    p = Path(path).expanduser()
    try:
        obj = json.loads(to_text(p.read_bytes()))
    except OSError as e:
        raise ValueError("corrections file unreadable: %s" % e.strerror)
    except ValueError as e:
        raise ValueError("corrections file is not valid JSON: %s" % e)
    if not isinstance(obj, dict):
        raise ValueError('corrections file must be a JSON object: {"<sid>": {"model_corrected": "..."}}')
    out = {}
    for i, (sid, entry) in enumerate(obj.items(), 1):
        label = entry.get("model_corrected") if isinstance(entry, dict) else None
        if not isinstance(label, str) or not label.strip():
            raise ValueError("corrections entry #%d: model_corrected must be a non-empty string" % i)
        out[sid] = label.strip()
    return out


def annotate(line, corr):
    """sid 가 corr 에 있으면 model_corrected 를 붙인다. (줄, 매칭 여부) 를 돌려준다."""
    try:
        obj = json.loads(line)
    except ValueError:
        return line, False
    sid = obj.get("sid") if isinstance(obj, dict) else None
    if not isinstance(sid, str) or sid not in corr:
        return line, False
    label = corr[sid]
    if obj.get("model_corrected") == label:
        return line, True
    obj["model_corrected"] = label
    return json.dumps(obj, ensure_ascii=False), True


def cmd_export(args):
    claude_dir = Path(args.claude_dir).expanduser()
    projects = claude_dir / "projects"
    if not projects.is_dir():
        raise ValueError("projects dir not found: %s" % projects)
    now = dt.datetime.now().astimezone()
    host = socket.gethostname()
    if args.out:
        out = Path(args.out).expanduser()
    else:
        out = Path("claude-usage-%s-%s.zip" % (safe_name(host), now.strftime("%Y%m%d")))

    jsonl_files = sorted(p for p in projects.rglob("*.jsonl") if p.is_file())
    files = len(jsonl_files)
    bad = unreadable = masked = dups = 0
    rows = {}

    for path in jsonl_files:
        try:
            raw = path.read_bytes()
        except OSError:
            unreadable += 1
            continue
        parts = path.relative_to(projects).parts
        project = parts[0] if len(parts) > 1 else ""
        for line in to_text(raw).split("\n"):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except ValueError:
                bad += 1
                continue
            if not isinstance(obj, dict) or obj.get("type") != "assistant":
                continue
            msg = obj.get("message")
            if not isinstance(msg, dict):
                continue
            usage = msg.get("usage")
            if not isinstance(usage, dict):
                continue
            model = text_of(msg.get("model"))
            if model == "<synthetic>":
                continue
            sid = text_of(obj.get("sessionId"))
            ts = text_of(obj.get("timestamp"))
            uid = text_of(obj.get("uuid"))
            mid = text_of(msg.get("id"))
            rid = text_of(obj.get("requestId"))
            key = ("m:" + mid) if mid else (uid or "|".join((sid, ts, mid)))
            row = {
                "key": key,
                "sid": sid,
                "ts": ts,
                "project": project,
                "model": model,
                "in": num(usage.get("input_tokens")),
                "out": num(usage.get("output_tokens")),
                "cc": num(usage.get("cache_creation_input_tokens")),
                "cr": num(usage.get("cache_read_input_tokens")),
                "side": bool(obj.get("isSidechain")),
                "rid": rid,
                "mid": mid,
                "agent": first_str(obj.get("agentName"), obj.get("agentId"), obj.get("agent")),
                "subagent": subagent_of(obj, msg),
                "tools": tool_names(msg),
                "cwd": text_of(obj.get("cwd")),
            }
            row_masked = 0
            for k in ROW_STR_FIELDS:
                if k not in row:
                    continue
                row[k], n = mask(row[k])
                row_masked += n
            prev = rows.get(key)
            if prev is not None:
                dups += 1
                if row["out"] < prev["out"]:
                    continue
            masked += row_masked
            rows[key] = row

    ordered = sorted(rows.values(), key=lambda r: (r["ts"], r["key"]))
    usage_text = "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in ordered)
    sessions = build_sessions(ordered)
    sessions_text = "".join(json.dumps(s, ensure_ascii=False) + "\n" for s in sessions)

    include = bool(args.include_conversations)
    conv_files = 0
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.writestr(USAGE_MEMBER, usage_text.encode("utf-8"))
        zf.writestr(SESSIONS_MEMBER, sessions_text.encode("utf-8"))

        stats = claude_dir / STATS_MEMBER
        if stats.is_file():
            try:
                text, n = mask(to_text(stats.read_bytes()))
                masked += n
                zf.writestr(STATS_MEMBER, text.encode("utf-8"))
            except OSError:
                unreadable += 1

        if include:
            items = [(p, CONV_PREFIX + p.relative_to(projects).as_posix()) for p in jsonl_files]
            hist = claude_dir / "history.jsonl"
            if hist.is_file():
                items.append((hist, CONV_PREFIX + "history.jsonl"))
            for path, arc in items:
                try:
                    text, n = mask(to_text(path.read_bytes()))
                except OSError:
                    unreadable += 1
                    continue
                masked += n
                zf.writestr(arc, text.encode("utf-8"))
                conv_files += 1

        counts = {
            "files_scanned": files,
            "rows": len(ordered),
            "sessions": len(sessions),
            "duplicates_dropped": dups,
            "skipped_bad_lines": bad,
            "masked_emails": masked,
        }
        manifest = {
            "host": host,
            "created": now.isoformat(timespec="seconds"),
            "rows": len(ordered),
            "counts": counts,
            "include_conversations": include,
        }
        zf.writestr("manifest.json", json.dumps(manifest, ensure_ascii=False, indent=2).encode("utf-8"))

    print("files scanned: %d" % files)
    print("rows: %d" % len(ordered))
    print("skipped bad lines: %d" % bad)
    print("masked emails: %d" % masked)
    print("sessions: %d" % len(sessions))
    print("duplicates dropped (message.id/uuid): %d" % dups)
    if include:
        print("conversation files: %d" % conv_files)
    if unreadable:
        print("unreadable files: %d" % unreadable)
    print("wrote %s" % out)
    print("size: %d bytes" % out.stat().st_size)
    return 0


def _zero():
    return {"in": 0, "out": 0, "cc": 0, "cr": 0, "messages": 0}


def _add(bucket, row):
    for k in TOKEN_KEYS:
        bucket[k] += num(row.get(k))
    bucket["messages"] += 1


def _tokens(bucket):
    return sum(bucket[k] for k in TOKEN_KEYS)


def _day(ts):
    m = DATE_RE.match(ts) if isinstance(ts, str) else None
    return m.group(1) if m else "unknown"


def _name(v):
    return v if isinstance(v, str) and v else "unknown"


def _ctx(row):
    return num(row.get("in")) + num(row.get("cr")) + num(row.get("cc"))


def _sums(rs):
    rs = list(rs)
    return {k: sum(num(r.get(k)) for r in rs) for k in ("in", "out", "cr", "cc")}


def build_sessions(rows):
    """sid 별 세션 요약 dict 목록(sid 순). 컨텍스트 크기는 사이드체인이 아닌 행만 본다."""
    groups = {}
    for idx, r in enumerate(rows):
        groups.setdefault(_name(r.get("sid")), []).append((idx, r))
    out = []
    for sid in sorted(groups):
        items = groups[sid]
        main = [it for it in items if not it[1].get("side")]
        side = [it for it in items if it[1].get("side")]
        tss = [text_of(r.get("ts")) for _, r in items]
        nonempty = [t for t in tss if t]
        # latest: max ts, on tie the later list position wins
        latest = max(items, key=lambda it: (text_of(it[1].get("ts")), it[0]))
        last = max(main, key=lambda it: (text_of(it[1].get("ts")), it[0]), default=None)
        peak = max(main, key=lambda it: _ctx(it[1]), default=None)
        out.append({
            "sid": sid,
            "first": min(nonempty) if nonempty else "",
            "last": max(nonempty) if nonempty else "",
            "project": text_of(latest[1].get("project")),
            "models": sorted({text_of(r.get("model")) for _, r in items} - {""}),
            "turns": len(items),
            "turns_main": len(main),
            "turns_side": len(side),
            "main": _sums(r for _, r in main),
            "side": _sums(r for _, r in side),
            "last_ctx": _ctx(last[1]) if last else None,
            "last_ctx_ts": text_of(last[1].get("ts")) if last else None,
            "peak_ctx": _ctx(peak[1]) if peak else None,
            "peak_ctx_ts": text_of(peak[1].get("ts")) if peak else None,
        })
    return out


def cmd_merge(args):
    corr = load_corrections(args.corrections) if args.corrections else None
    bundle = Path(args.zip).expanduser()
    dest = Path(args.dest).expanduser()
    dest_root = dest.resolve()
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
        plan = {}
        ignored = 0
        for info in zf.infolist():
            if info.is_dir():
                continue
            cat = classify(info.filename)
            if cat is None:
                if info.filename != "manifest.json":
                    ignored += 1
                continue
            plan[info.filename] = cat

        raw_label = args.label or manifest.get("host")
        if not raw_label:
            raise ValueError("no label: manifest has no host; pass --label NAME")
        label = safe_name(str(raw_label))
        if not label.strip("."):
            raise ValueError("label is empty after sanitizing: %r" % raw_label)
        base = dest / label

        usage_t = base / "usage.jsonl"
        stats_t = base / "stats-cache.json"
        summary_t = base / "summary.json"
        sessions_t = base / "sessions.jsonl"
        conv_jobs = []
        for name, (kind, rel) in sorted(plan.items()):
            if kind == "conv":
                conv_jobs.append((name, base / "conversations" / Path(*rel.split("/"))))
        targets = [usage_t, stats_t, summary_t, sessions_t] + [t for _, t in conv_jobs]
        for t in targets:
            if not inside(t, dest_root):
                raise UnsafeMemberError("target escapes dest: %s" % t)
        print("label: %s -> %s" % (label, base.as_posix()))

        base.mkdir(parents=True, exist_ok=True)

        # usage rows
        usage_bytes = usage_t.read_bytes() if usage_t.is_file() else b""
        seen = set()
        seen_mid = set()
        for ln in split_lines(to_text(usage_bytes)):
            k = row_key(ln)
            if k is not None:
                seen.add(k)
            m = row_mid(ln)
            if m:
                seen_mid.add(m)
        new = []
        skipped = 0
        zip_keys, zip_mids = set(), set()
        zip_rows = zip_within = 0
        if USAGE_MEMBER in plan:
            for ln in split_lines(mask(to_text(zf.read(USAGE_MEMBER)))[0]):
                k = row_key(ln)
                if k is None:
                    continue
                m = row_mid(ln)
                zip_rows += 1
                if k in zip_keys or (m and m in zip_mids):
                    zip_within += 1
                zip_keys.add(k)
                if m:
                    zip_mids.add(m)
                if k in seen or (m and m in seen_mid):
                    skipped += 1
                else:
                    seen.add(k)
                    if m:
                        seen_mid.add(m)
                    new.append(ln)

        annotated = 0
        if corr is not None:
            fixed = []
            touched = False
            for ln in split_lines(to_text(usage_bytes)):
                out_ln, hit = annotate(ln, corr)
                annotated += hit
                touched = touched or out_ln != ln
                fixed.append(out_ln)
            if touched:
                text = "".join(x + "\n" for x in fixed)
                with open(str(usage_t), "w", encoding="utf-8", newline="\n") as f:
                    f.write(text)
                usage_bytes = text.encode("utf-8")
            annotated_new = []
            for ln in new:
                out_ln, hit = annotate(ln, corr)
                annotated += hit
                annotated_new.append(out_ln)
            new = annotated_new

        if new or not usage_t.is_file():
            with open(str(usage_t), "a", encoding="utf-8", newline="\n") as f:
                if new and usage_bytes and not usage_bytes.endswith(b"\n"):
                    f.write("\n")
                for ln in new:
                    f.write(ln + "\n")
        print("usage rows: added %d, skipped %d (duplicate), total %d" % (len(new), skipped, len(seen)))
        if corr is not None:
            print("corrections: rows annotated %d (sessions in file: %d)" % (annotated, len(corr)))

        # dedup-check: kept = unique rows in the zip; manifest rows should match that count.
        kept = zip_rows - zip_within
        manifest_rows = manifest_rows_of(manifest)
        if manifest_rows is None:
            print("dedup-check: removed=%d manifest_rows=n/a" % skipped)
        else:
            diff = manifest_rows - kept
            print("dedup-check: removed=%d manifest_rows=%d kept=%d diff=%d %s"
                  % (skipped, manifest_rows, kept, diff, "OK" if diff == 0 else "MISMATCH"))

        # stats-cache.json
        if STATS_MEMBER in plan:
            text = mask(to_text(zf.read(STATS_MEMBER)))[0]
            old = stats_t.read_bytes() if stats_t.is_file() else None
            if old == text.encode("utf-8"):
                print("stats-cache: unchanged")
            else:
                with open(str(stats_t), "w", encoding="utf-8", newline="\n") as f:
                    f.write(text)
                print("stats-cache: updated")
        else:
            print("stats-cache: not in bundle")

        # conversations
        conv_added = 0
        for name, target in conv_jobs:
            incoming = split_lines(mask(to_text(zf.read(name)))[0])
            old_bytes = target.read_bytes() if target.is_file() else b""
            line_seen = set(split_lines(to_text(old_bytes)))
            conv_new = []
            for ln in incoming:
                if ln not in line_seen:
                    line_seen.add(ln)
                    conv_new.append(ln)
            if conv_new or not target.is_file():
                target.parent.mkdir(parents=True, exist_ok=True)
                with open(str(target), "a", encoding="utf-8", newline="\n") as f:
                    if conv_new and old_bytes and not old_bytes.endswith(b"\n"):
                        f.write("\n")
                    for ln in conv_new:
                        f.write(ln + "\n")
            conv_added += len(conv_new)
        print("conversations: files %d, added lines %d" % (len(conv_jobs), conv_added))

    # summary.json regenerated from the full usage.jsonl
    rows = []
    for ln in split_lines(to_text(usage_t.read_bytes())):
        try:
            obj = json.loads(ln)
        except ValueError:
            continue
        if isinstance(obj, dict):
            rows.append(obj)

    totals = _zero()
    by_date, by_model, by_project = {}, {}, {}
    for row in rows:
        _add(totals, row)
        _add(by_date.setdefault(_day(row.get("ts")), _zero()), row)
        _add(by_model.setdefault(_name(row.get("model")), _zero()), row)
        _add(by_project.setdefault(_name(row.get("project")), _zero()), row)

    days = sorted(d for d in by_date if d != "unknown")
    summary = {
        "rows": len(rows),
        "date_range": {"first": days[0] if days else None, "last": days[-1] if days else None},
        "totals": totals,
        "by_date": by_date,
        "by_model": by_model,
        "by_project": by_project,
    }
    with open(str(summary_t), "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True) + "\n")

    # sessions.jsonl is recomputed from the merged usage rows. The bundle's own sessions.jsonl
    # is accepted by classify() but never read, so several exports combine correctly.
    sessions = build_sessions(rows)
    sessions_text = "".join(json.dumps(s, ensure_ascii=False) + "\n" for s in sessions)
    old_sessions = sessions_t.read_bytes() if sessions_t.is_file() else None
    if old_sessions == sessions_text.encode("utf-8"):
        print("sessions: %d (unchanged)" % len(sessions))
    else:
        with open(str(sessions_t), "w", encoding="utf-8", newline="\n") as f:
            f.write(sessions_text)
        print("sessions: %d (written to %s)" % (len(sessions), sessions_t))

    if days:
        print("date range: %s .. %s" % (days[0], days[-1]))
    else:
        print("date range: none")
    top = sorted(by_model.items(), key=lambda kv: (-_tokens(kv[1]), kv[0]))[:5]
    print("top models: %s" % (", ".join(name for name, _ in top) or "none"))
    print("summary: %s (rows %d)" % (summary_t, len(rows)))
    return 0


def _read_rows(path):
    rows = []
    for ln in split_lines(to_text(path.read_bytes())):
        try:
            obj = json.loads(ln)
        except ValueError:
            continue
        if isinstance(obj, dict):
            rows.append(obj)
    return rows


def _pick(row, *keys):
    for k in keys:
        v = row.get(k)
        if isinstance(v, str) and v:
            return v
    return ""


def _group(rows, keyfn):
    groups = {}
    for r in rows:
        b = groups.setdefault(keyfn(r), _zero())
        _add(b, r)
        b["corrected"] = b.get("corrected", 0) + (1 if _pick(r, "model_corrected") else 0)
    return groups


def _dw(s):
    """terminal display width: East Asian wide/fullwidth characters count as 2."""
    return sum(2 if unicodedata.east_asian_width(c) in ("W", "F") else 1 for c in s)


def _fmt_line(cells, widths):
    def pad(c, w, left):
        gap = " " * (w - _dw(c))
        return c + gap if left else gap + c

    out = [pad(cells[0], widths[0], True)] + [pad(c, w, False) for c, w in zip(cells[1:], widths[1:])]
    return "  ".join(out)


def _print_table(title, first, groups, extra_header=None):
    headers = [first, "rows", "in", "out", "cc", "cr"] + ([extra_header] if extra_header else [])
    body = []
    for key, b in sorted(groups.items(), key=lambda kv: (-kv[1]["messages"], kv[0])):
        cells = [key, str(b["messages"]), str(b["in"]), str(b["out"]), str(b["cc"]), str(b["cr"])]
        if extra_header:
            cells.append(str(b.get("corrected", 0)))
        body.append(cells)
    widths = [max([_dw(h)] + [_dw(r[i]) for r in body]) for i, h in enumerate(headers)]
    print(title)
    print(_fmt_line(headers, widths))
    print("  ".join("-" * w for w in widths))
    for r in body:
        print(_fmt_line(r, widths))
    print()


def cmd_summary(args):
    dest = Path(args.dest).expanduser()
    files = sorted(p for p in dest.glob("*/usage.jsonl") if p.is_file()) if dest.is_dir() else []
    if not files:
        print("no usage.jsonl under %s" % dest)
        return 0
    all_rows = []
    host_groups = {}
    for p in files:
        rows = _read_rows(p)
        all_rows.extend(rows)
        bucket = _zero()
        for r in rows:
            _add(bucket, r)
        host_groups[p.parent.name] = bucket
    print("dest: %s (hosts %d, rows %d)" % (dest, len(files), len(all_rows)))
    print()
    _print_table("by host", "host", host_groups)
    _print_table("by model (model_corrected if set)", "model",
                 _group(all_rows, lambda r: _name(_pick(r, "model_corrected", "model"))), "(corrected)")
    _print_table("by project (cwd if set, else project)", "project",
                 _group(all_rows, lambda r: _name(_pick(r, "cwd", "project"))))
    _print_table("by sidechain", "sidechain",
                 _group(all_rows, lambda r: "sidechain" if r.get("side") else "main"))
    _print_table("by role (agent / subagent / first tool)", "agent / subagent / tool",
                 _group(all_rows, lambda r: " / ".join([
                     _pick(r, "agent") or "(없음)",
                     _pick(r, "subagent") or "(없음)",
                     (_pick(r, "tools").split(",")[0]) or "(없음)",
                 ])))
    return 0


def main(argv=None):
    p = argparse.ArgumentParser(description="Export/merge plain Claude Code usage records between computers.")
    sub = p.add_subparsers(dest="cmd", required=True)

    e = sub.add_parser("export", help="write claude-usage zip from ~/.claude")
    e.add_argument("--claude-dir", default="~/.claude")
    e.add_argument("--out", default=None)
    e.add_argument(
        "--include-conversations",
        action="store_true",
        help="also include conversation jsonl and history.jsonl (they contain prompt text; default off)",
    )

    m = sub.add_parser("merge", help="merge a claude-usage zip into <dest>/<label>/")
    m.add_argument("zip")
    m.add_argument("--dest", default="./claude-usage-imported")
    m.add_argument("--label", default=None)
    m.add_argument(
        "--corrections",
        default=None,
        metavar="FILE",
        help='JSON {"<sid>": {"model_corrected": "label", "note": "..."}}; adds model_corrected to matching rows',
    )

    s = sub.add_parser("summary", help="print tables from <dest>/*/usage.jsonl (stdout only, no files written)")
    s.add_argument("--dest", default="./claude-usage-imported")

    args = p.parse_args(argv)
    try:
        if args.cmd == "export":
            return cmd_export(args)
        if args.cmd == "summary":
            return cmd_summary(args)
        return cmd_merge(args)
    except UnsafeMemberError as err:
        print("error: %s" % err, file=sys.stderr)
        return 2
    except (OSError, ValueError, zipfile.BadZipFile) as err:
        print("error: %s" % err, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
