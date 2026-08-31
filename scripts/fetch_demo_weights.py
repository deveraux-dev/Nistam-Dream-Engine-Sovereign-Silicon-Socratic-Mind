#!/usr/bin/env python3
"""
Verify and audit S13 Gemma quantized weights (2B + 9B).

The quantized .s13m/.s13n weights are tracked directly in the Git repository:
  - s13_gemma_2b_m3/ (182 .s13m files, 446.4 MB) -> 26 layers x 7 matrices
  - s13_gemma_9b_m3/ (294 .s13m + 169 .s13n files, 1.84 GB) -> 42 layers x 7 matrices + norms

Usage:
  python scripts/fetch_demo_weights.py
"""

import os
import sys
from pathlib import Path

WEIGHT_DIRS = [
    ("s13_gemma_2b_m3", 182, 0, "Baby Bear (Gemma 2B @ 1.58-bit)"),
    ("s13_gemma_9b_m3", 294, 169, "Mama Bear (Gemma 9B @ 1.58-bit)"),
]

def verify_weights():
    repo_root = Path(__file__).parent.parent.resolve()
    os.chdir(repo_root)

    print("===============================================================================")
    print("  S13 GEMMA QUANTIZED WEIGHTS AUDIT & VERIFICATION")
    print("===============================================================================")
    print(f"  Repo root: {repo_root}")
    print()

    all_ok = True

    for dir_name, exp_m, exp_n, label in WEIGHT_DIRS:
        target = repo_root / dir_name
        if not target.exists() or not target.is_dir():
            print(f"  [MISSING] {dir_name}/ ({label}) not found on disk.")
            print(f"            Run 'git checkout main -- {dir_name}' or 'git pull origin main'.")
            all_ok = False
            continue

        s13m_files = list(target.glob("*.s13m"))
        s13n_files = list(target.glob("*.s13n"))
        count_m = len(s13m_files)
        count_n = len(s13n_files)
        total_bytes = sum(f.stat().st_size for f in s13m_files) + sum(f.stat().st_size for f in s13n_files)
        mb = total_bytes / (1024 * 1024)

        if count_m >= exp_m and count_n >= exp_n:
            detail = f"{count_m} .s13m" + (f" + {count_n} .s13n" if exp_n > 0 else "")
            print(f"  [OK] {dir_name}/ : {detail} files ({mb:.1f} MB) - {label}")
        else:
            print(f"  [PARTIAL] {dir_name}/ : {count_m}/{exp_m} .s13m, {count_n}/{exp_n} .s13n ({mb:.1f} MB)")
            all_ok = False

    print()
    if all_ok:
        print("  SUCCESS: All S13 quantized weights verified on local disk.")
        print("  Ready to execute:")
        print("    $env:S13_GEMMA_DIR = 's13_gemma_2b_m3'; cargo run --release --example gpu_decode_real -p gemma-s13")
        print("    cargo run --release --example gpu_decode_timed -p gemma-s13")
        print("    cargo run --release --example full_inference -p gemma-s13")
        print("===============================================================================")
        return True
    else:
        print("  FAILURE: Missing weight files. Please pull latest main branch via git.")
        print("===============================================================================")
        return False

if __name__ == "__main__":
    if not verify_weights():
        sys.exit(1)
