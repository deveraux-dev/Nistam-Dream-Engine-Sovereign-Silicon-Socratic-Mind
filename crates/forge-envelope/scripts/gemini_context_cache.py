#!/usr/bin/env python3
"""
Surface Ledger & Forge-Envelope — Gemini 3.7 Flash SDK Coding Assistant & Context Caching Hub

This script implements:
1. GCP ADC / Gemini API client initialization (Vertex AI or Gemini Developer API).
2. Automated repository bundling into a persistent Vertex AI Context Cache (75% token cost reduction).
3. Deterministic Zero-Point Cognitive Tuning (temperature: 0.0, top_k: 1, top_p: 0.0) under Rule G19.
4. Immutable Sovereign Airgap (ADR-0026) protecting Cree language and sovereign crates.
5. Interactive multi-file code reasoning REPL with full repository awareness.
"""

import os
import sys
import re
import json
import hashlib
from pathlib import Path
from typing import List, Dict, Tuple, Optional

# Attempt import of official google-genai SDK
try:
    from google import genai
    from google.genai import types
except ImportError:
    print("[WARN] google-genai package not found. Installing via pip...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "google-genai"])
    from google import genai
    from google.genai import types


# Target repository files to bundle into the persistent Context Cache
REPO_ROOT = Path(__file__).parent.parent.resolve()
CORE_FILES = [
    "Cargo.toml",
    "GEMINI.md",
    "AGENTS.md",
    "src/lib.rs",
    "src/weaver.rs",
    "src/somatic_tokenizer.rs",
    "src/cognitive_heal.rs",
    "src/mom.rs",
    "src/safety_router.rs",
    "src/bin/chaos_monkey.rs",
    "surfaceledger/ARCHITECTURE.md",
]

# Rule G19 Mandate: gemini-3.7-flash at deterministic temperature 0.0
MODEL_FLASH = os.environ.get("VERTEX_FLASH_MODEL", "gemini-3.7-flash")

CREE_SYLLABIC_REGEX = re.compile(r"[\u1400-\u167F]")
SOVEREIGN_BLOCKED_PATTERNS = [
    "cree_grammar.rs",
    "gemma-s13",
    "m5_geodesic.rs",
    "dirge-of-ironroot",
    "ironroot-edict",
    "sovereign_cree",
]


def check_sovereign_violation(rel_path: str, content: str) -> Tuple[bool, str]:
    """Inspects relative path and content for sovereign cultural or private data."""
    rel_lower = rel_path.lower().replace("\\", "/")
    for pattern in SOVEREIGN_BLOCKED_PATTERNS:
        if pattern.lower() in rel_lower:
            return True, f"Blocked path pattern '{pattern}' matching sovereign asset."

    if CREE_SYLLABIC_REGEX.search(content):
        return True, "Detected Unicode Cree Syllabics (\\u1400-\\u167F). Cree language is sovereign."

    return False, ""


def compute_manifest_hash(file_contents: Dict[str, str]) -> str:
    """Computes a deterministic SHA-256 signature across all bundled codebase files."""
    hasher = hashlib.sha256()
    for path in sorted(file_contents.keys()):
        hasher.update(path.encode("utf-8"))
        hasher.update(b"\x00")
        hasher.update(file_contents[path].encode("utf-8"))
        hasher.update(b"\xff")
    return hasher.hexdigest()


def bundle_repository() -> Dict[str, str]:
    """Reads all target repository files into a clean dictionary with airgap protection."""
    bundle = {}
    for rel_path in CORE_FILES:
        full_path = REPO_ROOT / rel_path
        if full_path.exists():
            with open(full_path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
            is_violation, reason = check_sovereign_violation(rel_path, content)
            if is_violation:
                print(f"[SOVEREIGN AIRGAP] Skipped '{rel_path}' ({reason})")
                continue
            bundle[rel_path] = content
        else:
            print(f"[WARN] File {rel_path} not found in workspace.")
    return bundle


def get_gemini_client() -> genai.Client:
    """
    Initializes the Gemini client using either:
    1. Vertex AI via Application Default Credentials (ADC) or GCP Project.
    2. Google AI Studio via GEMINI_API_KEY environment variable.
    """
    project_id = os.environ.get("GOOGLE_CLOUD_PROJECT") or os.environ.get("GCP_PROJECT")
    location = os.environ.get("GOOGLE_CLOUD_LOCATION", "us-central1")
    api_key = os.environ.get("GEMINI_API_KEY")

    if project_id:
        print(f"[AUTH] Initializing Vertex AI Client (Project: {project_id}, Location: {location})...")
        return genai.Client(vertexai=True, project=project_id, location=location)
    elif api_key:
        print("[AUTH] Initializing Gemini Developer Client via GEMINI_API_KEY...")
        return genai.Client(api_key=api_key)
    else:
        print("[AUTH] Discovering GCP Application Default Credentials (ADC)...")
        try:
            return genai.Client()
        except Exception as e:
            print(f"[ERROR] Could not authenticate with Google Cloud. Run `gcloud auth application-default login` or set `GEMINI_API_KEY`.")
            raise e


def create_or_reuse_context_cache(client: genai.Client, bundle: Dict[str, str]) -> str:
    """
    Creates or reuses a stable Context Cache on Vertex AI / Gemini API.
    Reduces input token processing by up to 75% on recurring queries.
    """
    manifest_hash = compute_manifest_hash(bundle)
    cache_display_name = f"forge_envelope_codebase_{manifest_hash[:12]}"

    print(f"\n[CACHE] Repository Manifest SHA-256: {manifest_hash}")
    print(f"[CACHE] Checking for existing context cache: '{cache_display_name}'...")

    try:
        for existing in client.cached_contents.list():
            if existing.display_name == cache_display_name:
                print(f"[CACHE HIT] Found active Context Cache: {existing.name} (Expires: {existing.expire_time})")
                return existing.name
    except Exception as e:
        print(f"[CACHE] Note: Could not query existing caches ({e}). Proceeding to create fresh cache...")

    formatted_docs = [
        "=== SURFACE LEDGER & FORGE-ENVELOPE CANONICAL CODEBASE ===\n"
        "You are the Lead Systems Engineer and Cloud-Scale Architect for this deterministic, "
        "tick-bounded, `#![no_std]` Rust library and high-throughput physical state attestation ledger.\n"
        "All code must strictly adhere to zero-allocation hotpaths, balanced-ternary Pararity laws, "
        "and SHA-256 rolling evidence chain integrity.\n"
    ]

    for rel_path, content in bundle.items():
        formatted_docs.append(f"\n--- FILE: {rel_path} ---\n{content}\n")

    full_context_text = "\n".join(formatted_docs)
    total_chars = len(full_context_text)
    estimated_tokens = total_chars // 4

    print(f"[CACHE] Uploading codebase context ({len(bundle)} files, ~{estimated_tokens:,} tokens)...")

    cache = client.cached_contents.create(
        model=MODEL_FLASH,
        config=types.CreateCachedContentConfig(
            contents=[types.Content(parts=[types.Part.from_text(text=full_context_text)])],
            display_name=cache_display_name,
            ttl="3600s",
        ),
    )

    print(f"[CACHE CREATED] Handle: {cache.name} (TTL: 3600s, Est. Recurring Discount: 75%)")
    return cache.name


def get_deterministic_generation_config(cache_name: Optional[str] = None) -> types.GenerateContentConfig:
    """
    Locks the Gemini reasoning engine into the Pararity 'Zero-Point Tuning State' (Rule G19):
    - temperature: 0.0 (unique fixed-point residue; collapses non-deterministic variance)
    - top_k: 1 (deterministic greedy selection)
    - top_p: 0.0 (sharpest probability distribution)
    """
    config_args = {
        "temperature": 0.0,
        "top_k": 1,
        "top_p": 0.0,
        "system_instruction": (
            "You are the Lead Systems Engineer for Surface Ledger & `forge-envelope`. "
            "You write deterministic, `#![no_std]`-compatible Rust, verify state lineage, "
            "and enforce tamper-evident cryptographic evidence chains with zero dynamic heap allocations on replay."
        ),
    }

    if cache_name:
        config_args["cached_content"] = cache_name

    return types.GenerateContentConfig(**config_args)


def query_assistant(client: genai.Client, prompt: str, cache_name: Optional[str] = None) -> str:
    """Dispatches a deterministic query to the Flash model with codebase context."""
    config = get_deterministic_generation_config(cache_name)
    response = client.models.generate_content(
        model=MODEL_FLASH,
        contents=prompt,
        config=config,
    )
    return response.text


def main():
    print("======================================================================")
    print("  SURFACE LEDGER / FORGE-ENVELOPE — GEMINI 3.7 FLASH CODING ASSISTANT HUB  ")
    print("  [Rule G19: gemini-3.7-flash @ temp=0.0, top_k=1]  [ADR-0026: AIRGAP ACTIVE]")
    print("======================================================================")

    bundle = bundle_repository()
    print(f"[OK] Bundled {len(bundle)} core repository files.")

    try:
        client = get_gemini_client()
    except Exception as e:
        print(f"\n[ERROR] Setup failed: {e}")
        print("\nPlease run the following commands to authenticate with Google Cloud:")
        print("  1. gcloud auth application-default login")
        print("  2. gcloud config set project YOUR_PROJECT_ID")
        print("  3. export GOOGLE_CLOUD_PROJECT=YOUR_PROJECT_ID")
        print("  OR export GEMINI_API_KEY=YOUR_GEMINI_API_KEY\n")
        return

    cache_name = None
    try:
        cache_name = create_or_reuse_context_cache(client, bundle)
    except Exception as e:
        print(f"[WARN] Context Caching skipped ({e}). Falling back to uncached queries.")

    if len(sys.argv) > 1:
        user_query = " ".join(sys.argv[1:])
        print(f"\n[QUERY] {user_query}\n")
        answer = query_assistant(client, user_query, cache_name)
        print("------------------- GEMINI 3.7 FLASH RESPONSE -------------------")
        print(answer)
        print("-----------------------------------------------------------------")
    else:
        print("\n[READY] Gemini 3.7 Flash Coding Assistant initialized with Full Repository Context.")
        print("Type your query below (e.g. 'How does WeaverArbiter verify S13 tokens against the EvidenceChain?')")
        print("Type 'exit' or 'quit' to end.\n")

        while True:
            try:
                prompt = input("flash-engineer> ").strip()
                if not prompt:
                    continue
                if prompt.lower() in ["exit", "quit", "q"]:
                    break
                print("\n[REASONING with Gemini 3.7 Flash at Temp=0.0, Top-K=1]...")
                answer = query_assistant(client, prompt, cache_name)
                print(f"\n{answer}\n")
            except KeyboardInterrupt:
                print("\nExiting.")
                break
            except Exception as e:
                print(f"\n[ERROR] {e}\n")


if __name__ == "__main__":
    main()
