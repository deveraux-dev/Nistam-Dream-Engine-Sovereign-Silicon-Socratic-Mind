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
    "6.42 gtok": "withdrawn 2026-08-20; L1 lookup relabelled as token gen x invented 7.5 factor. use 20.64 mtok/s 8-core",
    "856.16": "withdrawn 2026-08-20; ~311x over. use 2.75 mtok/s single-core (mtok_bench_receipt.txt:16)",
    "40.66 gtrits": "withdrawn 2026-08-21; LLVM hoisted the loop, 14x over. use 37.06 gtrits/s avx2",
    "879.51": "withdrawn 2026-08-21; const-folded, 2.4x over. use 358.17 m plans/s (RECEIPT-RUN-2026-08-27.txt:32)",
    "17.89 ns": "superseded; measured 17.25 ns (RECEIPT-RUN-2026-08-27.txt:26)",
    "57.48 gb": "superseded; measured 59.62 gb/s (RECEIPT-RUN-2026-08-27.txt:27)",
    "1.17 ns": "real L1-lookup measurement, wrong label. it is not per-token. use 363.40 ns/token",
    "35.12 ns": "measured value is 37.3633 ns (live_scale_telemetry.json:8)",
    "3.1 ns": "no zeroize bench exists",
    "gemini 3.7": "real model; this repo calls gemini-2.5-flash (vertex_flash_cache.py:65)",
    "gemini-3.7": "real model; this repo calls gemini-2.5-flash (vertex_flash_cache.py:65)",
    "gemini 3.5": "real model; this repo calls gemini-2.5-flash (vertex_flash_cache.py:65)",
    "gemini-3.5": "real model; this repo calls gemini-2.5-flash (vertex_flash_cache.py:65)",
    "gemini 3.1": "real model; this repo calls gemini-2.5-flash (vertex_flash_cache.py:65)",
    "gemini-3.1": "real model; this repo calls gemini-2.5-flash (vertex_flash_cache.py:65)",
    "1,562,500": "a size ratio, not a timed benchmark; nothing reconstructs the 25 mb",
    "440.6": "arithmetic disagrees with its own inputs by ~77x",
    "60 million gemini audits": "projection, never run at that volume",
    "60m audits": "projection, never run at that volume",
}

# Figures with a dated machine receipt behind them.
# Sources: docs/_archive-benchmarks-2026-08-27/RECEIPT-RUN-2026-08-27.txt
#          crates/forge-envelope/surfaceledger/mtok_bench_receipt.txt
MEASURED = (
    "37.36", "37.3633", "49/49", "12,048,323", "12.05m", "40,000", "0.083",
    "1.76 million", "1.76m", "568.28", "2.57 gtrits", "2,570.07", "37.06 gtrits",
    "17.25", "59.62", "59,615.01", "358.17", "2.79 ns", "2.75 mtok", "20.64 mtok",
    "363.40", "874.13", "338", "191 passed", "122 passed", "25 passed",
)

# The real VARS cache is a ~40k-55k token bundle (vertex_flash_cache.py:12,168).
# $0.0004 is a real measured single audit (3,094 in / 472 out,
# HANDOFF-2026-08-19-BILLING-AND-SYSTEM-REVIEW.md:20).
# live_scale_telemetry.json's 450,000-token / 25,000-query / $212 block is a
# PROJECTION, never a spend, and its cache size is ~10x the real bundle.
UNRECEIPTED["450,000-token"] = "cache bundle is ~40k-55k tokens; 450k is a projection"
UNRECEIPTED["450,000 tokens"] = "cache bundle is ~40k-55k tokens; 450k is a projection"
UNRECEIPTED["212.20"] = "projected spend, never billed"
UNRECEIPTED["843.75"] = "projected spend, never billed"

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
