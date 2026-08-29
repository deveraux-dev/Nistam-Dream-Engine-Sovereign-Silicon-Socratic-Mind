#!/usr/bin/env python3
# receipt_run - one host-pinned receipt for every published figure. Exit 1 on any failure.
# Emits _proof/RECEIPT-<utc>.md. A dirty tree marks the receipt PROVISIONAL.

import argparse
import os
import platform
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

ROOT = Path(__file__).parent.parent.resolve()
PROOF = ROOT / "_proof"

MEASURES = [
    ("mtok_throughput",
     "single-core cache-resident; conjugate sign inversion, BQ MetaRouter routing, host staging, tile planning; excludes end-to-end inference",
     ["cargo", "run", "--release", "--example", "mtok_throughput_bench", "-p", "forge-gpu-warden-v3"]),
    ("trit_dist",
     "single-core; LUT vs per-trit decode; excludes routing dispatch",
     ["cargo", "run", "--release", "--example", "trit_dist_bench", "-p", "forge-core-v3"]),
    ("astrolabe_runtime",
     "single-core zero-heap star-lock attitude resolve over HYG catalog; excludes render",
     ["cargo", "run", "--release", "--example", "probe_astrolabe_runtime", "-p", "gemma-s13"]),
    ("clockspine_contention",
     "multi-core contention under load; excludes single-core figures above",
     ["cargo", "run", "--release", "--example", "trit_dist_contention", "-p", "forge-hal-clockspine"]),
]

GPU_MEASURES = [
    ("gpu_dispatch_floor",
     "WebGPU dispatch floor; synthetic weights; excludes weight load and host norm/attention",
     ["cargo", "run", "--release", "--example", "gpu_dispatch_floor", "-p", "gemma-s13"]),
    ("gpu_decode_timed",
     "9B geometry, synthetic weights; GEMV-dominant; excludes host norm/attention application",
     ["cargo", "run", "--release", "--example", "gpu_decode_timed", "-p", "gemma-s13"]),
]

INVARIANTS = [
    ("workspace_tests",
     "machine-independent; excludes crates/studio-tauri and shell (Cargo.toml:44)",
     ["cargo", "test", "--workspace"]),
    ("blind_oracle_stress",
     "machine-independent pass count; timing figures in output are host-bound",
     ["cargo", "test", "--manifest-path", "crates/gemma-s13/Cargo.toml",
      "--test", "stress_blind_oracle", "--", "--nocapture"]),
    ("sovereign_airgap",
     "local filter assertions over generated vectors; proves filter integrity, not production leak rate; makes no cloud call",
     [sys.executable, str(ROOT / "crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py")]),
    ("mma_nostr",
     "BIP-340 attestation, O(1) Merkle gate, Byzantine injection refusal, ADR-0026 scrub",
     ["cargo", "run", "--release", "--example", "mma_nostr_live_demo", "-p", "forge-daemon-door"]),
]


def sh(cmd, timeout=None):
    try:
        r = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                           encoding="utf-8", errors="replace", timeout=timeout)
        return r.returncode, (r.stdout or "") + (r.stderr or "")
    except FileNotFoundError:
        return 127, f"not found: {cmd[0]}"
    except subprocess.TimeoutExpired:
        return 124, f"timeout after {timeout}s"


def one_line(cmd):
    rc, out = sh(cmd)
    return out.strip().splitlines()[0].strip() if rc == 0 and out.strip() else "unavailable"


def gpu_state():
    if not shutil.which("nvidia-smi"):
        return ["gpu: nvidia-smi unavailable"]
    q = ("name,memory.total,persistence_mode,clocks.applications.graphics,"
         "clocks.max.graphics,clocks.current.graphics,clocks.current.memory,"
         "power.limit,temperature.gpu")
    rc, out = sh(["nvidia-smi", f"--query-gpu={q}", "--format=csv,noheader"])
    if rc != 0:
        return ["gpu: nvidia-smi query failed"]
    keys = q.split(",")
    rows = []
    for line in out.strip().splitlines():
        vals = [v.strip() for v in line.split(",")]
        rows.append("gpu: " + " | ".join(f"{k}={v}" for k, v in zip(keys, vals)))
    return rows


def host_block():
    uname = platform.uname()
    ram = "unknown"
    rc, out = sh(["powershell", "-NoProfile", "-Command",
                  "[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory/1GB,1)"])
    if rc == 0 and out.strip():
        ram = out.strip() + " GB"
    lines = [
        f"utc: {datetime.now(timezone.utc).isoformat(timespec='seconds')}",
        f"host: {uname.node}",
        f"cpu: {platform.processor() or uname.processor}",
        f"cores_logical: {os.cpu_count()}",
        f"ram: {ram}",
        f"os: {uname.system} {uname.release} {uname.version}",
        f"python: {platform.python_version()}",
        f"rustc: {one_line(['rustc', '--version'])}",
        f"cargo: {one_line(['cargo', '--version'])}",
    ]
    lines += gpu_state()
    head = one_line(["git", "rev-parse", "HEAD"])
    rc, out = sh(["git", "status", "--short"])
    dirty = len([l for l in out.splitlines() if l.strip()]) if rc == 0 else -1
    lines.append(f"git_head: {head}")
    lines.append(f"git_dirty_paths: {dirty}")
    return lines, head, dirty


def section(fh, title, rows, timeout):
    failed = []
    fh.write(f"\n## {title}\n")
    for name, scope, cmd in rows:
        printable = " ".join(str(c) for c in cmd)
        print(f"--> {name}", flush=True)
        rc, out = sh(cmd, timeout=timeout)
        status = "PASS" if rc == 0 else f"FAIL(exit {rc})"
        if rc != 0:
            failed.append(name)
        fh.write(f"\n### {name} — {status}\n")
        fh.write(f"scope: {scope}\n\n")
        fh.write(f"```\n$ {printable}\n\n{out.strip()}\n```\n")
        print(f"    {status}", flush=True)
    return failed


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gpu", action="store_true", help="include WebGPU adapter measures")
    ap.add_argument("--timeout", type=int, default=1800)
    args = ap.parse_args()

    PROOF.mkdir(parents=True, exist_ok=True)
    host, head, dirty = host_block()
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")
    path = PROOF / f"RECEIPT-{stamp}.md"

    print("\n".join(host), flush=True)
    print(f"\nwriting {path}\n", flush=True)

    failed = []
    with open(path, "w", encoding="utf-8") as fh:
        state = "PROVISIONAL" if dirty != 0 else "PINNED"
        fh.write(f"# Receipt {stamp} — {state}\n\n")
        if dirty != 0:
            fh.write(f"PROVISIONAL: {dirty} uncommitted path(s) at capture time. "
                     f"Not reproducible from {head} alone. Commit, then re-run before citing.\n\n")
        fh.write("Every figure published anywhere in this repository must appear in this file "
                 "or be tagged [ASSUMED] / [INFERRED] / UNVERIFIED.\n\n")
        fh.write("## Host\n\n```\n" + "\n".join(host) + "\n```\n")
        failed += section(fh, "Invariants (machine-independent)", INVARIANTS, args.timeout)
        failed += section(fh, "Host-bound measures", MEASURES, args.timeout)
        if args.gpu:
            failed += section(fh, "GPU measures (adapter-bound)", GPU_MEASURES, args.timeout)
        fh.write(f"\n## Result\n\n{'FAILED: ' + ', '.join(failed) if failed else 'all sections passed'}\n")

    print(f"\nreceipt: {path}")
    if failed:
        print(f"FAILED: {', '.join(failed)}", file=sys.stderr)
        return 1
    if dirty != 0:
        print(f"PROVISIONAL: {dirty} uncommitted path(s); commit and re-run before citing.", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
