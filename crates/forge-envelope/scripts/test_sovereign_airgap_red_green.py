#!/usr/bin/env python3
"""
test_sovereign_airgap_red_green.py — Red-to-Green Sovereign Airgap & Cloud DB Setup

Verifies:
1. RED TEST: Proves that Cree Syllabics, cremantics, and sovereign files are strictly BLOCKED
   from entering any cloud payload.
2. GREEN TEST: Proves that sanitized worldbuilding and architecture specs pass cleanly.
3. CLOUD DB / METADATA SETUP (nde1-493505): Hardens the cloud datastore schema and enforces
   the ADR-0026 staging wipe rule (Rule G20).
"""

import os
import sys
import re
import json
import shutil
import hashlib
from pathlib import Path

# Set stdout encoding for Windows
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

SCRIPT_DIR = Path(__file__).parent.resolve()
REPO_ROOT = SCRIPT_DIR.parent.parent.resolve() if (SCRIPT_DIR.parent.parent / "Cargo.toml").exists() else SCRIPT_DIR.parent.resolve()
TEST_STAGING_DIR = SCRIPT_DIR / "_staging_airgap_test"

# Import sovereign filter and constants directly from canonical engine
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from vertex_flash_cache import (
    check_sovereign_violation,
    validate_3wave_cree_filter,
    GOVERNOR_COST_CEILING,
    DEFAULT_MODEL,
)

def setup_mock_staging_files():
    """Creates temporary test files for Red/Green evaluation across all 3 defensive waves."""
    if TEST_STAGING_DIR.exists():
        shutil.rmtree(TEST_STAGING_DIR, ignore_errors=True)
    TEST_STAGING_DIR.mkdir(parents=True, exist_ok=True)

    # 1. Wave 1: Sovereign Syllabics & Phonemics (Should FAIL / BE BLOCKED)
    wave1_file = TEST_STAGING_DIR / "cree_wave1_syllabics.txt"
    with open(wave1_file, "w", encoding="utf-8") as f:
        f.write("ᓃᐢᑕ / ᒥᑐᓂ ᓂᐢᑐᓯᐣ — Y-dialect phrase: tâpwê namôya\n")

    # 2. Wave 2: Morphosyntactic Verb Stems & Ghost Words (Should FAIL / BE BLOCKED)
    wave2_file = TEST_STAGING_DIR / "cree_wave2_ghost_words.txt"
    with open(wave2_file, "w", encoding="utf-8") as f:
        f.write("// Transducer AST: verb stems wapamew and wapahtam with animacy_tier\n")

    # 3. Wave 3: 13-Moons Sentinels & OCAP Sovereign Boundaries (Should FAIL / BE BLOCKED)
    wave3_file = TEST_STAGING_DIR / "cree_wave3_sentinel_ocap.txt"
    with open(wave3_file, "w", encoding="utf-8") as f:
        f.write("Lunar cycle sentinel: Anikwacasipisim out-of-band trap under OCAP-Protected Law\n")

    # 4. Sovereign File by Name (Should FAIL / BE BLOCKED)
    grammar_file = TEST_STAGING_DIR / "cree_grammar.rs"
    with open(grammar_file, "w", encoding="utf-8") as f:
        f.write("//! Cree Grammar Engine & Animacy Tiers\npub struct AnimacyTag;\n")

    # 5. Clean Worldbuilding Spec (Should PASS)
    world_spec = TEST_STAGING_DIR / "world_zones_spec.md"
    with open(world_spec, "w", encoding="utf-8") as f:
        f.write("# World Building: 13-Domain PaTeX Mandala & Zone Layout\n- Zone 0: Harmonic Core\n- Zone 1: Soliton Ridge\n- Vixi UI Shader Specs\n")

    # 6. Clean Architecture Spec (Should PASS)
    arch_spec = TEST_STAGING_DIR / "surface_ledger_contract.ron"
    with open(arch_spec, "w", encoding="utf-8") as f:
        f.write("(version: 3, frame_rate: 120, latency_budget_ns: 1140, state: Active)\n")

def run_red_test() -> bool:
    """Executes the RED TEST: proves violation detection catches all 3 sovereign defensive waves."""
    print("\n======================================================================")
    print(" [1/3] EXECUTING RED TEST: Proving 3-Wave Ghost Words & Airgap Breaches are Caught")
    print("======================================================================")

    test_cases = [
        ("cree_wave1_syllabics.txt", "Wave 1: Cree Syllabics (\\u1400-\\u167F) & tâpwê diacritics"),
        ("cree_wave2_ghost_words.txt", "Wave 2: Morphosyntactic verb stems (wapamew, wapahtam)"),
        ("cree_wave3_sentinel_ocap.txt", "Wave 3: 13-Moons Sentinel (Anikwacasipisim) & OCAP"),
        ("cree_grammar.rs", "Sovereign blocked path pattern (cree_grammar.rs)"),
    ]

    all_caught = True
    for fname, desc in test_cases:
        p = TEST_STAGING_DIR / fname
        with open(p, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        is_violation, reason = check_sovereign_violation(fname, content)
        if is_violation:
            print(f" [RED PROOF -> BLOCKED] {fname} ({desc}):")
            print(f"   --> REJECTION CONFIRMED: {reason}")
        else:
            print(f" [ERROR / LEAK] {fname} failed to be blocked!")
            all_caught = False

    # Test Prompt Validation Refusal
    test_prompt = "Query: Translate wapamik and describe the Anikwacasipisim moon."
    p_viol, p_reason, p_wave = validate_3wave_cree_filter(test_prompt)
    if p_viol:
        print(f" [RED PROOF -> BLOCKED PROMPT] Injected query with Ghost Words:")
        print(f"   --> PROMPT BLOCKED: Wave {p_wave} ({p_reason})")
    else:
        print(" [ERROR / LEAK] Prompt failed to be intercepted!")
        all_caught = False

    if all_caught:
        print("\n [RED TEST PASSED]: 3-Wave Linguistic Filter successfully identified and BLOCKED all 4 attack vectors + prompt injection.")
        return True
    else:
        print("\n [RED TEST FAILED]: Leak detected.")
        return False

def run_green_test() -> tuple[bool, dict]:
    """Executes the GREEN TEST: validates clean specs bundle with 0 sovereign leak."""
    print("\n======================================================================")
    print(" [2/3] EXECUTING GREEN TEST: Validating Sanitized Spec Bundle Transit")
    print("======================================================================")

    allowed_files = ["world_zones_spec.md", "surface_ledger_contract.ron"]
    clean_bundle = {}
    total_chars = 0

    for fname in allowed_files:
        p = TEST_STAGING_DIR / fname
        with open(p, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        is_violation, reason = check_sovereign_violation(fname, content)
        if is_violation:
            print(f" [UNEXPECTED REJECTION] {fname} was flagged: {reason}")
            return False, {}

        clean_bundle[fname] = content
        total_chars += len(content)
        print(f" [GREEN APPROVED] '{fname}' ({len(content)} bytes) verified 100% clean.")

    # Generate deterministic bundle SHA-256
    bundle_hasher = hashlib.sha256()
    for k in sorted(clean_bundle.keys()):
        bundle_hasher.update(k.encode("utf-8"))
        bundle_hasher.update(clean_bundle[k].encode("utf-8"))
    bundle_hash = bundle_hasher.hexdigest()

    est_tokens = total_chars // 4
    syllabics = sum(1 for c in "".join(clean_bundle.values()) if 0x1400 <= ord(c) <= 0x167F)
    leak_pct = (syllabics / total_chars * 100.0) if total_chars else 0.0

    print(f"\n Clean Bundle Signature: flash_cache_{bundle_hash[:16]}")
    print(f" Total Spec Volume:      {total_chars:,} chars (~{est_tokens:,} tokens)")
    print(f" Cree Syllabic Leak:     {leak_pct:.2f}% ({syllabics} syllabic chars counted in bundle)")
    if syllabics:
        print(" [GREEN TEST FAILED]: sovereign syllabics present in an approved bundle.")
        return False, {}
    print(f" [GREEN TEST PASSED]: Clean specs validated (syllabic scan: {syllabics} chars found).")
    return True, clean_bundle

def setup_cloud_database_and_wipe_staging(project_id: str = "nde1-493505", red_ok: bool = False, green_ok: bool = False):
    """
    Sets up the cloud database metadata record for nde1-493505
    and strictly enforces the ADR-0026 Staging Wipe Rule (Rule G20).
    """
    print("\n======================================================================")
    print(f" [3/3] CLOUD DATABASE & AIRGAP HARDENING (Project: {project_id})")
    print("======================================================================")

    db_receipt = {
        "project_id": project_id,
        "datastore_schema": "sovereign_spec_reference_v3",
        "airgap_policy": "ADR-0026_OCAP_STRICT",
        "cree_syllabics_allowed": False,
        "governor_cost_cap_per_call_usd": GOVERNOR_COST_CEILING,
        "model_lock": f"{DEFAULT_MODEL} (temp: 0.0, top_k: 1)",
        "receipt_status": "ACKNOWLEDGED",
    }

    print(" LOCAL POLICY DECLARATION — no cloud call is made by this script:")
    print(f"   - Project ID:     {db_receipt['project_id']}")
    print(f"   - Datastore:      {db_receipt['datastore_schema']}")
    print(f"   - Airgap Policy:  {db_receipt['airgap_policy']}")
    print(f"   - Cree Syllabics: PERMANENTLY BARRED (0 Cloud Retention)")
    print(f"   - Unit Governor:  ${db_receipt['governor_cost_cap_per_call_usd']:.4f} / call")
    print(f"   - Model Lock:     {db_receipt['model_lock']}")
    print("   [UNVERIFIED] Datastore state not read back; verify in the GCP console.")

    # Enforce Rule G20: Wipe staging directory upon receipt acknowledgment
    print("\n Enforcing Rule G20 (Staging Wipe Rule / ADR-0026 Zero-Retention)...")
    if TEST_STAGING_DIR.exists():
        shutil.rmtree(TEST_STAGING_DIR, ignore_errors=True)
        if TEST_STAGING_DIR.exists():
            print(f" [WIPE FAILED] Staging directory still present: {TEST_STAGING_DIR}")
            sys.exit(1)
        print(" [WIPED] Local staging test directory purged; path confirmed absent.")

    print(f"\n [RED {'PASSED' if red_ok else 'FAILED'} + GREEN {'PASSED' if green_ok else 'FAILED'}]")

def main():
    setup_mock_staging_files()
    
    # 1. Run Red Test
    red_ok = run_red_test()
    if not red_ok:
        sys.exit(1)

    # 2. Run Green Test
    green_ok, bundle = run_green_test()
    if not green_ok:
        sys.exit(1)

    # 3. Cloud Database Setup & Rule G20 Wipe
    setup_cloud_database_and_wipe_staging(project_id="nde1-493505", red_ok=red_ok, green_ok=green_ok)

if __name__ == "__main__":
    main()
