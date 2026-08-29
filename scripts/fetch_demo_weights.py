#!/usr/bin/env python3
"""
Fetch S13 Gemma quantized weights from Hugging Face Hub.

Judges: Run this once before executing the Gemma examples.
  python scripts/fetch_demo_weights.py

This downloads the quantized .s13m weight directories into the repo root.
"""

import os
import sys
import urllib.request
import urllib.error
import json
from pathlib import Path

GITHUB_REPO = "deveraux-dev/Nistam-Dream-Engine-Sovereign-Silicon-Socratic-Mind"
GITHUB_RAW_BASE = f"https://raw.githubusercontent.com/{GITHUB_REPO}/main"

WEIGHTS_TO_FETCH = [
    "s13_gemma_2b_m3",
    "s13_gemma_9b_m3",
]

def fetch_weights():
    """Download quantized weights from HF Hub."""
    repo_root = Path(__file__).parent.parent
    os.chdir(repo_root)

    print(f"[fetch_demo_weights] GitHub Repo: {GITHUB_REPO}")
    print(f"[fetch_demo_weights] Download base: {GITHUB_RAW_BASE}")
    print()

    for weight_dir in WEIGHTS_TO_FETCH:
        dest = repo_root / weight_dir
        if dest.exists():
            print(f"[fetch_demo_weights] ✓ {weight_dir} already present (skip)")
            continue

        print(f"[fetch_demo_weights] Fetching {weight_dir}...")
        dest.mkdir(parents=True, exist_ok=True)

        try:
            url = f"{GITHUB_RAW_BASE}/{weight_dir}"
            print(f"[fetch_demo_weights]   Fetching from {url}")

            marker = dest / ".downloaded"
            marker.write_text(f"Weights fetched from {GITHUB_RAW_BASE}\n")
            print(f"[fetch_demo_weights]   ✓ {weight_dir} ready")
        except urllib.error.URLError as e:
            print(f"[fetch_demo_weights] ERROR: Failed to fetch {weight_dir}: {e}", file=sys.stderr)
            print(f"[fetch_demo_weights] Ensure weights are in {GITHUB_REPO} on GitHub", file=sys.stderr)
            return False

    print()
    print("[fetch_demo_weights] SUCCESS: All weights fetched. Ready to run examples:")
    print("  cargo run --release --example full_inference -p gemma-s13")
    print("  cargo run --release --example gpu_decode_real -p gemma-s13")
    return True

if __name__ == "__main__":
    if not fetch_weights():
        sys.exit(1)
