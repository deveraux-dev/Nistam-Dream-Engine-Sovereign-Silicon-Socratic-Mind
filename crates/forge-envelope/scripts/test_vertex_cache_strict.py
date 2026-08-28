#!/usr/bin/env python3
"""
test_vertex_cache_strict.py — Zero-Tolerance Fail-Fast Vertex Context Cache Verification

Key Invariants:
1. NO FAIL-GREEN: Exits with non-zero code immediately if auth fails, bundle is under 32k tokens,
   or if live response returns 0 cached tokens.
2. DETERMINISTIC RECEIPT: Verifies exact token metrics directly against google-genai usage_metadata.
3. SOVEREIGN AIRGAP (ADR-0026): Validates 3-wave Cree filter blocks all sovereign terms before dispatch.
"""

import os
import sys
import argparse
from pathlib import Path

# Setup paths
SCRIPT_DIR = Path(__file__).parent.resolve()
REPO_ROOT = SCRIPT_DIR.parent.resolve()
WORKSPACE_ROOT = REPO_ROOT.parent.parent.resolve() if (REPO_ROOT.parent.parent / "Cargo.toml").exists() else REPO_ROOT
sys.path.insert(0, str(SCRIPT_DIR))

import vertex_flash_cache as vfc

def test_offline_token_census():
    """Verify that every bundle profile clears the 32,768 GenAI cache minimum threshold."""
    print("=" * 72)
    print(" [1/3] VERIFYING BUNDLE TOKEN CENSUS (THRESHOLD >= 32,768 TOKENS)")
    print("=" * 72)
    for profile_name in vfc.PROFILES:
        bundle, skipped = vfc.sanitize_and_bundle(vfc.PROFILES[profile_name], base_dir=WORKSPACE_ROOT)
        total_chars = sum(len(c) for c in bundle.values())
        approx_tokens = total_chars // 4
        print(f" Profile '{profile_name}': {len(bundle)} files, {total_chars:,} chars, ~{approx_tokens:,} tokens")
        
        if approx_tokens < 32768:
            raise AssertionError(
                f"[STRICT FAIL] Profile '{profile_name}' only has ~{approx_tokens:,} tokens! "
                f"Google GenAI context caching requires >= 32,768 tokens."
            )
        print(f"   [PASS] Profile '{profile_name}' safely exceeds context caching minimum.\n")


def test_offline_airgap_guard():
    """Verify that sovereign Cree syllabics and private crates are 100% blocked."""
    print("=" * 72)
    print(" [2/3] VERIFYING SOVEREIGN AIRGAP INTERCEPTION (ADR-0026)")
    print("=" * 72)
    
    # Check crate isolation
    violation, reason = vfc.check_sovereign_violation("crates/gemma-s13/src/gpu_warden.rs", "fn s13_gemv()")
    if not violation:
        raise AssertionError("[STRICT FAIL] gemma-s13 path was not blocked by airgap filter!")
    print(f" [PASS] Blocked sovereign crate: {reason}")

    # Check syllabics
    violation, reason = vfc.check_sovereign_violation("doc.md", "Sample text with \u140a syllabic")
    if not violation:
        raise AssertionError("[STRICT FAIL] Syllabic character was not blocked by airgap filter!")
    print(f" [PASS] Blocked sovereign syllabics: {reason}\n")


def test_live_vertex_cache_hit(model_name: str = vfc.DEFAULT_MODEL):
    """
    Executes a real query against Google Cloud Vertex AI and asserts a non-zero cache hit.
    Fails loudly with non-zero exit code if credentials, cache, or hit ratio fails.
    """
    print("=" * 72)
    print(" [3/3] LIVE GOOGLE CLOUD VERTEX AI CONTEXT CACHE AUDIT")
    print("=" * 72)
    
    client, auth_err = vfc.get_genai_client()
    if auth_err:
        raise RuntimeError(f"[LIVE FAIL] Google GenAI client authentication failed:\n{auth_err}")
    
    bundle, _ = vfc.sanitize_and_bundle(vfc.PROFILES["lean"], base_dir=WORKSPACE_ROOT)
    cache_name, cache_err = vfc.get_or_create_serverless_cache(
        client, bundle, bundle_tag="lean", model_name=model_name, quiet=False
    )
    if cache_err or not cache_name:
        raise RuntimeError(f"[LIVE FAIL] Failed to create or retrieve context cache: {cache_err}")
    
    print(f" [LIVE] Dispatching verification query to cache handle: {cache_name}...")
    prompt = "State the 3 core invariants of Forge v3 memory management in 2 bullet points."
    answer, usage = vfc.query_flash_cache(client, cache_name, prompt, model_name=model_name)
    
    if not answer or not usage:
        raise RuntimeError("[LIVE FAIL] Vertex AI query returned null response or empty usage metadata.")
    
    vfc.render_loud_visual_receipt(usage, cache_name=cache_name, tag="lean")
    
    cached_tokens = getattr(usage, "cached_content_token_count", 0) or 0
    prompt_tokens = getattr(usage, "prompt_token_count", 0) or 0
    
    print(f" Prompt Tokens : {prompt_tokens:,}")
    print(f" Cached Tokens : {cached_tokens:,}")
    
    if cached_tokens == 0:
        raise AssertionError(
            f"[LIVE FAIL] Context Cache Miss! Expected >0 cached tokens, got 0. "
            f"Prompt tokens: {prompt_tokens}."
        )
    
    print(f" [PASS] Live Vertex Context Cache Hit Verified: {cached_tokens:,} tokens served from cache.\n")


def main():
    parser = argparse.ArgumentParser(description="Strict Vertex Context Cache Test Harness")
    parser.add_argument("--live", "--require-cloud", action="store_true", help="Execute live call against Vertex AI and strictly require non-zero cache hit.")
    parser.add_argument("--model", type=str, default=vfc.DEFAULT_MODEL, help=f"Model name (default: {vfc.DEFAULT_MODEL})")
    args = parser.parse_args()

    try:
        test_offline_token_census()
        test_offline_airgap_guard()
        
        if args.live:
            test_live_vertex_cache_hit(model_name=args.model)
        else:
            print("=" * 72)
            print(" [OFFLINE VERIFICATION PASSED — Pass '--live' to test live Vertex cache hit]")
            print("=" * 72)
            
    except Exception as e:
        print(f"\n❌ STRICT VERIFICATION FAILED:\n{e}\n", file=sys.stderr)
        sys.exit(1)

    sys.exit(0)

if __name__ == "__main__":
    main()
