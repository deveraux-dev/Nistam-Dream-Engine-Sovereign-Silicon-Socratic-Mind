#!/usr/bin/env python3
# verdict_gate selftest. Fixtures are verbatim pre-fix lines from this repo.
# Exit 0 all green, 1 on any miss. stdlib only, no pytest in this repo.

import os
import sys
import tempfile
import textwrap

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import verdict_gate as vg

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

BAD_PY = [
    ("syllabic-leak 0.00%", "number",
     '    print(" Cree Syllabic Leak:     0.00% (Strict Zero-Leak Guarantee)")'),
    ("datastore theater", "verdict",
     '    print(f" Cloud Datastore Configured:")'),
    ("401/401 banner behind .ljust", "number",
     '    print("║  • 401/401 Rust Tests Passed (0 failed, 0 skipped, 0 mocked)".ljust(79) + "║")'),
    ("offline verification fail-green", "verdict",
     '    print(" [OFFLINE VERIFICATION PASSED — Pass \'--live\' to test live Vertex cache hit]")'),
    ("fail-green from except handler", "verdict",
     "try:\n    check()\nexcept Exception:\n    print('AIRGAP SECURED')"),
    ("hardcoded savings pct", "number",
     '    print("  Savings: 0.0% cost reduction")'),
]

GOOD_PY = [
    ("interpolated elapsed",
     '    print(f"║  ALL 5 STAGES PASSED CLEANLY IN {total_seconds} SECONDS".ljust(79) + "║")'),
    ("interpolated reason",
     '    print(f"   --> REJECTION CONFIRMED: {reason}")'),
    ("interpolated savings pct",
     '    print(f"  Exact Session Savings: ${savings:>10.6f} ({savings_pct:>4.1f}% Cost Reduction)")'),
    ("UNVERIFIED tag",
     '    print("   [UNVERIFIED] Datastore state not read back; verify in the GCP console.")'),
    ("interpolated wave",
     '    print(f"   --> PROMPT BLOCKED: Wave {p_wave} ({p_reason})")'),
    ("decorative separator", '    print("=" * 60)'),
    ("progress counter not totality", '    print("[1/2] building")'),
    ("equal progress counter not totality",
     '    print("[2/2] Probing archive-indexer health/root...")'),
    ("phase name not verdict",
     '    print("[2/3] EXECUTING GREEN TEST: Validating Sanitized Spec Bundle Transit")'),
]

GUARDED_PY = [
    ("guarded bare verdict downgrades to warn",
     "if ok:\n    print('ALL CHECKS PASSED')"),
]

BAD_RS = [
    ("rust hardcoded totality", 'println!("SUITE PASSED 892/892");'),
    ("rust bare verdict", 'println!("bit-perfect VERIFIED");'),
]

GOOD_RS = [
    ("rust interpolated", 'println!("took {:.2}ms (limit 0.5ms)", dt);'),
    ("rust interpolated named", 'println!("rms={rms:.3}, corr={corr:.4}");'),
]

BAD_PS = [
    ("ps hardcoded totality", 'Write-Host "All tests passing (892/892)"'),
]

GOOD_PS = [
    ("ps interpolated", 'Write-Host "  PASS  $m" -ForegroundColor Green'),
]

fails = []


def scan(src, ext):
    if ext == ".py":
        src = textwrap.dedent(src)
    fd, path = tempfile.mkstemp(suffix=ext, text=True)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(src)
        text = open(path, encoding="utf-8").read()
        if ext == ".py":
            return vg.scan_python(path, text)
        if ext == ".rs":
            return vg.scan_regex(text, vg.RUST_PRINT, "R02",
                                 lambda s: bool(vg.RUST_FMT.search(s)))
        return vg.scan_regex(text, vg.PS_PRINT, "R03",
                             lambda s: bool(vg.PS_VAR.search(s)))
    finally:
        os.unlink(path)


def expect_hit(name, src, ext, code, kind=None):
    hits = scan(src, ext)
    if not hits:
        fails.append(f"MISS  {code} {name}: expected a hit, got none")
        return
    if kind and not any(h[2] == kind or (kind == "verdict" and h[2] == "guarded-verdict")
                        for h in hits):
        got = ", ".join(sorted({h[2] for h in hits}))
        fails.append(f"KIND  {code} {name}: expected {kind}, got {got}")
        return
    print(f"  ok  {code} {name}  -> {hits[0][2]}")


def expect_silent(name, src, ext, code):
    hits = scan(src, ext)
    if hits:
        fails.append(f"NOISE {code} {name}: expected silence, got {hits}")
        return
    print(f"  ok  {code} {name}  -> silent")


print("known-bad (must be caught)")
for name, kind, src in BAD_PY:
    expect_hit(name, src, ".py", "R01", kind)
for name, src in BAD_RS:
    expect_hit(name, src, ".rs", "R02")
for name, src in BAD_PS:
    expect_hit(name, src, ".ps1", "R03")

print("\nguarded (must downgrade to warn, not vanish)")
for name, src in GUARDED_PY:
    expect_hit(name, src, ".py", "R01", "guarded-verdict")

print("\nknown-good (must stay silent)")
for name, src in GOOD_PY:
    expect_silent(name, src, ".py", "R01")
for name, src in GOOD_RS:
    expect_silent(name, src, ".rs", "R02")
for name, src in GOOD_PS:
    expect_silent(name, src, ".ps1", "R03")

print()
if fails:
    for f in fails:
        print(f)
    print(f"\nverdict_gate_selftest: {len(fails)} failure(s)")
    sys.exit(1)
total = (len(BAD_PY) + len(BAD_RS) + len(BAD_PS) + len(GUARDED_PY)
         + len(GOOD_PY) + len(GOOD_RS) + len(GOOD_PS))
print(f"verdict_gate_selftest: {total} fixtures, 0 failures")
sys.exit(0)
