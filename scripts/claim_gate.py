#!/usr/bin/env python3
# claim_gate - repo-wide unreceipted-claim gate. Exit 1 on any hit.
# Reads its banned table from forge_lint.UNRECEIPTED so there is one source of truth.

import importlib.util
import os
import re
import sys

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LINT = os.path.join(ROOT, "crates", "forge-envelope", "scripts", "forge_lint.py")

TEXT_EXT = {".md", ".rs", ".py", ".ps1", ".txt", ".json", ".html", ".toml",
            ".bat", ".sh", ".yaml", ".yml", ".wgsl", ".vixi"}
SKIP_DIRS = {"target", "target_env", "node_modules", ".git", "vcs", "objects",
             "_archive-benchmarks-2026-08-27", "_archive-2026-08-27", "__pycache__"}
SKIP_FILES = {"forge_lint.py", "claim_gate.py", "MODEL-STRING-SWEEP-2026-08-28.md",
              "WITHDRAWN-FIGURE-SWEEP-2026-08-28.md", "mtok_bench_receipt.txt"}

# A line carrying any of these is declaring its own uncertainty, or is a line
# that exists to debunk the number it quotes. Either way it passes.
TAGS = ("[ASSUMED]", "[INFERRED]", "UNVERIFIED", "UNRECEIPTED", "RECEIPT(",
        "no such model", "withdrawn", "superseded", "do not appear",
        "does not appear", "no bench", "not of token", "claimed", "overstated")

# A literal key is benign when the line also matches its exception.
EXCEPTIONS = {}


def banned_table():
    spec = importlib.util.spec_from_file_location("forge_lint", LINT)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.UNRECEIPTED


def walk(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for f in filenames:
            if f in SKIP_FILES:
                continue
            if os.path.splitext(f)[1].lower() in TEXT_EXT:
                yield os.path.join(dirpath, f)


def scan(path, table):
    try:
        text = open(path, encoding="utf-8", errors="replace").read()
    except OSError:
        return []
    hits = []
    for n, line in enumerate(text.splitlines(), 1):
        low = line.lower()
        if any(t.lower() in low for t in TAGS):
            continue
        for bad, why in table.items():
            if bad in low:
                exc = EXCEPTIONS.get(bad)
                if exc and exc.search(line):
                    continue
                hits.append((n, bad, why, line.strip()[:100]))
    return hits


def main():
    root = next((a for a in sys.argv[1:] if not a.startswith("--")), ROOT)
    table = banned_table()
    total = files = 0
    for path in walk(root):
        hits = scan(path, table)
        if not hits:
            continue
        files += 1
        print(f"\n{os.path.relpath(path, root)}")
        for n, bad, why, line in hits:
            print(f"  line {n}: {bad!r} - {why}")
            print(f"    {line}")
        total += len(hits)

    print(f"\nclaim_gate: {total} unreceipted claim(s) in {files} file(s)")
    if total:
        print("clear each one by re-measuring it, deleting it, or tagging the line "
              "[ASSUMED] / [INFERRED] / UNVERIFIED.")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
