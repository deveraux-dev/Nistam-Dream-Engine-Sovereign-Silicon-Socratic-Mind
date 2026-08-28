#!/usr/bin/env python3
# forge_lint - register + receipt linter for authored prose (acimowina laws, Drop Law, L03).
# Usage: python forge_lint.py <file-or-dir> [...] [--wpm] [--quiet]
# Exit 1 if any ERROR fires. WARN never fails the run.

import sys
import os
import re

HEDGE = (
    "should i", "would you like", "shall i", "do you want me to",
    "let me know if", "feel free to", "i hope this", "great question",
)

UNRECEIPTED = {
    "6.42 gtok": "no bench in tree; only prose in 08-17/08-19 handoffs",
    "856.16": "no bench in tree",
    "40.66 gtrits": "no bench in tree",
    "57.48 gb": "no bench in tree",
    "879.51": "no bench in tree",
    "17.89 ns": "no bench in tree",
    "1.17 ns": "no bench in tree",
    "35.12 ns": "measured value is 37.3633 ns (live_scale_telemetry.json:8)",
    "3.1 ns": "no zeroize bench exists",
    "gemini 3.7": "no such model; live receipt says gemini-2.5-flash",
    "gemini-3.7": "no such model; live receipt says gemini-2.5-flash",
    "gemma 4": "no such release",
    "1,562,500": "compression ratio unbenched",
    "440.6": "arithmetic disagrees with its own inputs by ~77x",
}

MEASURED = ("37.36", "37.3633", "49/49", "12,048,323", "12.05m", "40,000", "0.0004", "0.083")

SPEECH_WPM = 155.0


def sentences(text):
    return [s.strip() for s in re.split(r"(?<=[.?])\s+", text) if s.strip()]


def lint_line(path, n, line, state):
    hits = []
    low = line.lower()

    table_rule = bool(re.fullmatch(r"\s*\|(\s*:?-{2,}:?\s*\|)+\s*", line)) or \
        bool(re.fullmatch(r"\s*([-*_])\1{2,}\s*", line))
    prose = re.sub(r"`[^`]*`", "", line)
    if not table_rule:
        if "—" in prose or "--" in prose.replace("<!--", "").replace("-->", ""):
            hits.append(("ERROR", "R01", "em dash or double hyphen"))
    if ";" in line and not state["code"] and "&" not in line:
        hits.append(("ERROR", "R02", "semicolon"))
    if "!" in line and not state["code"] and "!=" not in line and "![" not in line:
        hits.append(("ERROR", "R03", "exclamation mark"))

    for h in HEDGE:
        if h in low:
            hits.append(("ERROR", "R04", f"hedge: {h!r}"))

    if not state["code"]:
        for bad, why in UNRECEIPTED.items():
            if bad in low:
                near = any(m.lower() in low for m in MEASURED)
                tagged = any(t in line for t in ("[ASSUMED]", "[INFERRED]", "UNVERIFIED", "RECEIPT("))
                if not (near or tagged or state["banned_block"]):
                    hits.append(("ERROR", "R05", f"unreceipted {bad!r}: {why}"))

    if state["code"] and re.match(r"\s*(#|//)\s*\S", line):
        state["comment_run"] += 1
        if state["comment_run"] == 4:
            hits.append(("WARN", "R07", "comment block over 3 lines (code-poetry line)"))
    else:
        state["comment_run"] = 0

    return hits


def lint_file(path, want_wpm):
    try:
        text = open(path, encoding="utf-8", errors="replace").read()
    except OSError as e:
        return [("ERROR", "R00", f"unreadable: {e}", 0)], 0

    out = []
    state = {"code": False, "comment_run": 0, "banned_block": False}
    section = None
    section_start = 0
    body = []
    vo = []

    for n, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith("```"):
            state["code"] = not state["code"]
            continue

        if re.match(r"\s*-?\s*(BANNED|CLEARED)\b", line):
            state["banned_block"] = line.strip().startswith(("BANNED", "- BANNED"))
        elif line.startswith("#"):
            state["banned_block"] = False

        m = re.match(r"\s*-\s*([LRC]):\s*\"(.+)\"", line)
        if m:
            vo.append(m.group(2))

        if line.startswith("#") and not state["code"]:
            for lvl, code, msg in lint_line(path, n, line, state):
                out.append((lvl, code, msg, n))
            if section and body:
                first = sentences(" ".join(body))
                if first and len(first[0].split()) > 28:
                    out.append(("WARN", "R06",
                                f"section {section!r} opens with a {len(first[0].split())}-word "
                                "sentence, point may be buried", section_start))
            section, section_start, body = line.strip("# ").strip(), n, []
            continue

        if line.strip() and not state["code"] and not line.lstrip().startswith(("-", "|", ">", "*")):
            body.append(line.strip())

        for lvl, code, msg in lint_line(path, n, line, state):
            out.append((lvl, code, msg, n))

    if vo:
        per = {}
        for v in vo:
            per.setdefault(len(re.findall(r"\w+", v)), 0)
        spoken = max(sum(len(re.findall(r"\w+", v)) for v in vo[i::3]) for i in range(3))
    else:
        spoken = len(re.findall(r"\w+", text))
    return out, spoken


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    want_wpm = "--wpm" in sys.argv
    quiet = "--quiet" in sys.argv
    if not args:
        print("usage: forge_lint.py <file-or-dir> [...] [--wpm] [--quiet]")
        return 2

    targets = []
    for a in args:
        if os.path.isdir(a):
            targets += [os.path.join(a, f) for f in sorted(os.listdir(a))
                        if f.endswith((".md", ".py", ".txt"))]
        elif os.path.isfile(a):
            targets.append(a)

    errors = warns = 0
    for t in targets:
        hits, words = lint_file(t, want_wpm)
        hits = [h for h in hits if not (quiet and h[0] == "WARN")]
        if hits:
            print(f"\n{t}")
            for lvl, code, msg, n in sorted(hits, key=lambda h: h[3]):
                print(f"  {lvl:5} {code} line {n}: {msg}")
        errors += sum(1 for h in hits if h[0] == "ERROR")
        warns += sum(1 for h in hits if h[0] == "WARN")
        if want_wpm and t.endswith(".md"):
            print(f"  INFO  R08 {words} words = {words / SPEECH_WPM * 60:.0f}s at {SPEECH_WPM:.0f} wpm")

    print(f"\n{len(targets)} file(s): {errors} error(s), {warns} warning(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
