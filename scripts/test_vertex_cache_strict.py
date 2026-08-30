#!/usr/bin/env python3
import os
import sys
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TARGET_SCRIPT = REPO_ROOT / "crates" / "forge-envelope" / "scripts" / "test_vertex_cache_strict.py"

if __name__ == "__main__":
    res = subprocess.run([sys.executable, str(TARGET_SCRIPT)] + sys.argv[1:], cwd=str(REPO_ROOT))
    sys.exit(res.returncode)
