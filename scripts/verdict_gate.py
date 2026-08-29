#!/usr/bin/env python3
# verdict_gate - announcement not wired to the check. Exit 1 on ERROR.
# R01 python/ast ERROR. R02 rust, R03 powershell, regex, WARN.

import ast
import os
import re
import sys

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

SCAN_EXT = {".py", ".rs", ".ps1"}
SKIP_DIRS = {"target", "target_env", "node_modules", ".git", "vcs", "objects",
             "__pycache__"}
SKIP_PREFIX = ("_archive-",)
SKIP_FILES = {"forge_lint.py", "claim_gate.py", "verdict_gate.py",
              "verdict_gate_selftest.py"}

VERDICT = re.compile(
    r"\b(PASSED|VERIFIED|GREEN|SECURED|CONFIRMED|INTACT|GUARANTEE[DS]?"
    r"|VALIDATED|COMPLETE|CONFIGURED)\b", re.I)
ESCAPE = ("[ASSUMED]", "[INFERRED]", "UNVERIFIED", "SYNTHETIC", "RECEIPT(")

PERCENT = re.compile(r"\d+(?:\.\d+)?\s*%")
UNIT = re.compile(r"\d+(?:\.\d+)?\s*(?:ms|ns|us|tok/s|MB/s|GB/s)\b")
RATIO = re.compile(r"\b(\d+)\s*/\s*(\d+)\b")
PROGRESS = re.compile(r"\[\s*\d+\s*/\s*\d+\s*\]")
PHASE = re.compile(r"\b(?:GREEN|RED)\s+(?:TEST|WAVE|VECTOR|PATH)\b", re.I)
RUST_FMT = re.compile(r"\{[^}]*\}")
PS_VAR = re.compile(r"\$[A-Za-z_(]")
RUST_PRINT = re.compile(r'\b(?:e)?println!\s*\(\s*"((?:[^"\\]|\\.)*)"')
PS_PRINT = re.compile(r'\bWrite-(?:Host|Output)\s+(?:@?["\'])((?:[^"\'\\]|\\.)*)["\']')


def walk(root):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames
                       if d not in SKIP_DIRS and not d.startswith(SKIP_PREFIX)]
        for f in filenames:
            if f in SKIP_FILES:
                continue
            if os.path.splitext(f)[1].lower() in SCAN_EXT:
                yield os.path.join(dirpath, f)


def escaped(s):
    return any(tag in s for tag in ESCAPE)


def totality_ratio(s):
    return any(a == b for a, b in RATIO.findall(s))


def hardcoded_number(s):
    return bool(PERCENT.search(s) or UNIT.search(s)) or totality_ratio(s)


def verdict_word(s):
    return bool(VERDICT.search(s))


def classify(s):
    if escaped(s):
        return None
    stripped = PHASE.sub("", PROGRESS.sub("", s))
    if hardcoded_number(stripped):
        return "number"
    if verdict_word(stripped):
        return "verdict"
    return None


def parent_map(tree):
    parents = {}
    for node in ast.walk(tree):
        for child in ast.iter_child_nodes(node):
            parents[child] = node
    return parents


def guarded(node, parents):
    cur = parents.get(node)
    while cur is not None:
        if isinstance(cur, (ast.If, ast.IfExp)) and not isinstance(cur.test, ast.Constant):
            return True
        cur = parents.get(cur)
    return False


def literal_strings(arg):
    if isinstance(arg, ast.JoinedStr):
        if any(isinstance(v, ast.FormattedValue) for v in arg.values):
            return []
        return ["".join(v.value for v in arg.values
                        if isinstance(v, ast.Constant) and isinstance(v.value, str))]
    if isinstance(arg, ast.BinOp) and isinstance(arg.op, ast.Mod):
        return []
    out = []
    if isinstance(arg, ast.Constant):
        if isinstance(arg.value, str):
            out.append(arg.value)
        return out
    for child in ast.iter_child_nodes(arg):
        out.extend(literal_strings(child))
    return out


def scan_python(path, text):
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return []
    parents = parent_map(tree)
    hits = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        if not (isinstance(node.func, ast.Name) and node.func.id == "print"):
            continue
        for arg in node.args:
            for s in literal_strings(arg):
                kind = classify(s)
                if kind is None:
                    continue
                if kind == "verdict" and guarded(node, parents):
                    kind = "guarded-verdict"
                hits.append((node.lineno, "R01", kind, s))
    return hits


def scan_regex(text, pattern, code, skip_interp):
    hits = []
    for n, line in enumerate(text.splitlines(), 1):
        if escaped(line):
            continue
        for m in pattern.finditer(line):
            s = m.group(1)
            if skip_interp(s):
                continue
            kind = classify(s)
            if kind is not None:
                hits.append((n, code, kind, s))
    return hits


def main():
    root = next((a for a in sys.argv[1:] if not a.startswith("--")), ROOT)
    root = os.path.abspath(root)
    errors = warns = files = 0

    for path in sorted(walk(root)):
        try:
            text = open(path, encoding="utf-8", errors="replace").read()
        except OSError:
            continue
        ext = os.path.splitext(path)[1].lower()
        if ext == ".py":
            hits = scan_python(path, text)
        elif ext == ".rs":
            hits = scan_regex(text, RUST_PRINT, "R02",
                              lambda s: bool(RUST_FMT.search(s)))
        else:
            hits = scan_regex(text, PS_PRINT, "R03",
                              lambda s: bool(PS_VAR.search(s)))
        if not hits:
            continue
        files += 1
        print(f"\n{os.path.relpath(path, root)}")
        seen = set()
        for n, code, kind, s in hits:
            key = (n, code, s)
            if key in seen:
                continue
            seen.add(key)
            level = "ERROR" if code == "R01" and kind != "guarded-verdict" else "WARN"
            if level == "ERROR":
                errors += 1
            else:
                warns += 1
            print(f"  {level:5} {code} line {n}: [{kind}] {s.strip()[:88]!r}")

    print(f"\nverdict_gate: {errors} error(s), {warns} warning(s) in {files} file(s)")
    print("wire the announcement to the check, interpolate the real value, "
          "or tag the line [ASSUMED] / UNVERIFIED.")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
