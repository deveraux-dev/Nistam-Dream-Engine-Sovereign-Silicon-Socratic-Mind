#!/usr/bin/env python3
# Shim. Canonical module is crates/forge-envelope/scripts/vertex_flash_cache.py,
# pinned there by cree_parity.rs embedding that path at compile time.
# Loaded under its own __file__ so its REPO_ROOT/WORKSPACE_ROOT still resolve.

import importlib.util
import os
import sys

_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_CANONICAL = os.path.join(_ROOT, "crates", "forge-envelope", "scripts",
                          "vertex_flash_cache.py")

if not os.path.isfile(_CANONICAL):
    sys.stderr.write(f"canonical module missing: {_CANONICAL}\n")
    raise SystemExit(2)

_spec = importlib.util.spec_from_file_location("vertex_flash_cache", _CANONICAL)
_mod = importlib.util.module_from_spec(_spec)
sys.modules["vertex_flash_cache"] = _mod
_spec.loader.exec_module(_mod)

globals().update({k: v for k, v in vars(_mod).items() if not k.startswith("__")})

if __name__ == "__main__":
    _mod.main()
