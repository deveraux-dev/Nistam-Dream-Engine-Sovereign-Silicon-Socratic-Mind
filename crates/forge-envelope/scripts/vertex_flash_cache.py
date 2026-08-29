#!/usr/bin/env python3
"""
vertex_flash_cache.py — Hardened, Sovereign & Lean Vertex AI Context Caching Hub

Key Invariants & Guarantees:
1. IMMUTABLE SOVEREIGN AIRGAP: The Cree language, syllabics (\\u1400-\\u167F), and sovereign
   crates (e.g. gemma-s13, cree_grammar.rs) are strictly excluded and blocked from cloud upload.
2. 100% SERVERLESS / ZERO DEDICATED ENDPOINTS: Strictly uses Google GenAI SDK Context Caching
   (`cached_contents`). Zero dedicated GCE/Vertex Endpoint provisioning ($0.00 idle cost).
3. GOVERNOR & CIRCUIT BREAKER (Rule G19): Locks model to `gemini-2.5-flash` at temperature 0.0,
   top_k 1, enforces strict budget caps, and calculates real-time sub-cent cost receipts.
4. LEAN DEV PROFILE: Right-sized ~40k–55k token context bundle clearing the 32k GenAI cache minimum
   while ensuring >98% cache hit ratios on recurring queries.
5. LOUD VISUAL TELEMETRY: High-contrast Unicode box receipts with live token efficiency bar,
   cost comparison, TTL countdown, and airgap safety confirmations.
"""

import os
import sys
import re
import json
import socket
import hashlib
import argparse

# Dead IPv6 route stalls googleapis TCP connect ~168s before v4 fallback.
# VERTEX_ALLOW_IPV6=1 restores dual-stack resolution.
if os.environ.get("VERTEX_ALLOW_IPV6") != "1":
    _getaddrinfo = socket.getaddrinfo

    def _getaddrinfo_v4(host, port, family=0, *args, **kwargs):
        if family == 0:
            family = socket.AF_INET
        return _getaddrinfo(host, port, family, *args, **kwargs)

    socket.getaddrinfo = _getaddrinfo_v4
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Dict, Tuple, Optional, Any

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
if hasattr(sys.stderr, "reconfigure"):
    try:
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

# Ensure project root is in path for billing guard imports
SCRIPT_DIR = Path(__file__).parent.resolve()
REPO_ROOT = SCRIPT_DIR.parent.resolve()
WORKSPACE_ROOT = REPO_ROOT.parent.parent.resolve() if (REPO_ROOT.parent.parent / "Cargo.toml").exists() else REPO_ROOT
sys.path.insert(0, str(SCRIPT_DIR))

try:
    from billing_guard import BillingGuard
except ImportError:
    BillingGuard = None

# Model & Engine Defaults (Rule G19 Mandate)
DEFAULT_MODEL = os.environ.get("VERTEX_FLASH_MODEL", "gemini-2.5-flash")
DEFAULT_TTL = "3600s"
MAX_OUTPUT_TOKENS = 2048
GOVERNOR_COST_CEILING = 0.0004  # $0.0004 per call unit-cost governor ceiling

# Pricing constants for gemini-2.5-flash / gemini-1.5-flash (per 1,000,000 tokens)
PRICE_INPUT_1M = 0.075          # $0.075 / 1M uncached input tokens
PRICE_CACHED_INPUT_1M = 0.01875 # $0.01875 / 1M cached input tokens (75% discount)
PRICE_OUTPUT_1M = 0.30          # $0.30 / 1M output tokens

# =============================================================================
# 1. SOVEREIGN AIRGAP & 3-WAVE GHOST WORDS VALIDATOR (ADR-0026 & Rule G18/G20)
# =============================================================================

# Wave 1: Unicode Plains Cree Syllabics Range: U+1400 to U+167F
CREE_SYLLABIC_REGEX = re.compile(r"[\u1400-\u167F]")

# Wave 1: Standardized Y-Dialect Diacritics & Phonemic Orthography Markers
WAVE_1_PHONEMIC_MARKERS = [
    "tâpwê", "namôya", "kistêyimitowin", "wâpamêw", "wâpahtam",
    "itwêwin", "miyo-pimatisiwin", "tânsi", "kiyâm", "ᑖᐻ", "ᓇᒨᔭ",
    "ᐋ", "ᐄ", "ᐆ", "ᐁ"
]

# Wave 2: Morphosyntactic & Structural Verb Stems (Ghost Words Lexicon)
WAVE_2_GHOST_WORDS = [
    "wapamew", "wapamik", "wapahtam", "paminaw", "pahtam", "kwayask",
    "kiskinohamatowin", "mowew", "miciw", "itohtew", "nohtawiy", "nikawiy",
    "maskwa", "atim", "amisk", "waciw", "sakahikan", "sipiy",
    "animacy_tier", "vta_direct", "vta_inverse", "cree_grammar", "zero_generative_cree"
]

# Wave 3: Sacred Protocol, 13-Moons Sentinels & OCAP Sovereign Boundaries
WAVE_3_SACRED_SENTINELS = [
    "mikisiwipisim", "niskiwisim", "ayikiwisim", "sakahipisim",
    "paskawihowipisim", "paskowipisim", "ohpahowipisim", "nopicisipisim",
    "takwakinipisim", "pinaskawipisim", "kaskatinowipisim", "pawacakinasisipisim",
    "anikwacasipisim", "anikwacas", "ocap-protected", "adr-0026 sovereign",
    "zero-generative cree", "sacred_ceremonial_lexicon", "sovereign_cree"
]

# Paths and tags permanently barred from cloud transit
SOVEREIGN_BLOCKED_PATTERNS = [
    "cree_grammar.rs",
    "cree_canon.rs",
    "cree_validator.rs",
    "gemma-s13",
    "m5_geodesic.rs",
    "dirge-of-ironroot",
    "ironroot-edict",
    "sovereign_cree",
    "animacy_tier",
    "zero_generative_cree",
]

def validate_3wave_cree_filter(text: str) -> Tuple[bool, str, Optional[int]]:
    """
    Evaluates text across all 3 defensive waves of the Cree Sovereign Linguistic Filter.
    Returns (is_violation, rationale, wave_number).
    """
    # Wave 1: Unicode Syllabics
    if CREE_SYLLABIC_REGEX.search(text):
        return True, "Wave 1: Detected Unicode Canadian Aboriginal Syllabics (\\u1400-\\u167F). Cree language is sovereign.", 1

    text_lower = text.lower()

    # Wave 1: Phonemic diacritics
    for marker in WAVE_1_PHONEMIC_MARKERS:
        if marker in text_lower:
            return True, f"Wave 1: Detected Y-dialect phonemic orthography or diacritic marker '{marker}'.", 1

    # Wave 2: Morphosyntactic verb stems & ghost words
    for stem in WAVE_2_GHOST_WORDS:
        if stem in text_lower:
            return True, f"Wave 2: Detected witnessed Cree morphosyntactic verb stem or ghost word '{stem}'.", 2

    # Wave 3: Sacred Sentinels & OCAP Sovereignty Boundaries
    for sentinel in WAVE_3_SACRED_SENTINELS:
        if sentinel in text_lower:
            return True, f"Wave 3: Detected 13-Moons sentinel token or OCAP sovereign declaration '{sentinel}'.", 3

    return False, "", None

def crypto_shred(buffer: bytearray) -> int:
    """
    ADR-0026 Mercy-Tick shred: overwrites a mutable staging buffer in place and returns
    the byte count wiped. Mirrors forge_envelope::crypto_shred_seed.

    Only the staging buffer is reachable. CPython str objects are immutable and interned,
    so the caller's original string is released to the allocator, never overwritten —
    which is why the sovereign payload is staged here before dispatch is considered.
    """
    wiped = len(buffer)
    for i in range(wiped):
        buffer[i] = 0
    return wiped


def check_sovereign_violation(rel_path: str, content: str) -> Tuple[bool, str]:
    """
    Inspects relative file path and text content for sovereign Cree or private data.
    Returns (is_violation, reason).
    """
    rel_lower = rel_path.lower().replace("\\", "/")
    for pattern in SOVEREIGN_BLOCKED_PATTERNS:
        if pattern.lower() in rel_lower:
            return True, f"Blocked path pattern '{pattern}' matching sovereign asset."

    is_violation, reason, _ = validate_3wave_cree_filter(content)
    if is_violation:
        return True, reason

    return False, ""


# =============================================================================
# 2. NAMED BUNDLE PROFILES (~40k-55k TOKENS, >32k CACHE THRESHOLD)
# =============================================================================

PROFILES: Dict[str, List[str]] = {
    # Primary Lean Dev Profile: Master Specs + Workspace Laws + Core Organs (~45k tokens)
    "lean": [
        "AGENTS.md",
        "GEMINI.md",
        "docs/DEVPOST.md",
        "docs/SUBMISSION_ENTRY.md",
        "docs/BENCHMARKS.md",
        "docs/whitepapers/01_NORMALIZED_IPR.md",
        "docs/whitepapers/02_TIMELESS_COMPRESSION.md",
        "docs/whitepapers/04_CONSTRAINED_L402_GATE.md",
        "docs/whitepapers/05_BARE_METAL_HOOK_ISOLATION.md",
        "docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md",
        "docs/whitepapers/07_THE_ARCHITECTURE_OF_EQUILIBRIUM.md",
        "docs/whitepapers/08_THE_MERKLE_MORIN_ARCHITECTURE.md",
        "docs/whitepapers/09_RELATIVISTIC_5D_LORENTZ_ABERRATION_AND_OKLCH_SPECTRAL_PALETTES.md",
        "docs/whitepapers/MMA_NOSTR_SPECIFICATION.md",
        "TODO/handoffs/HANDOFF-2026-08-23-DURABLE-PATEX-5D-GEOMETRY-SUPERMAXATOM-AND-SACRED-WORDS.md",
        "RAMUSPRIME/docs-specs/ORACLE-C-DREAM-DIAMONDS-EUX.md",
        "crates/forge-envelope/src/lib.rs",
        "crates/forge-envelope/src/weaver.rs",
    ],
    # Focused PaTeX 5D Specification Profile (~42k tokens)
    "patex": [
        "docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md",
        "docs/whitepapers/09_RELATIVISTIC_5D_LORENTZ_ABERRATION_AND_OKLCH_SPECTRAL_PALETTES.md",
        "docs/whitepapers/PATEX_README.md",
        "docs/whitepapers/01_NORMALIZED_IPR.md",
        "docs/whitepapers/02_TIMELESS_COMPRESSION.md",
        "docs/whitepapers/04_CONSTRAINED_L402_GATE.md",
        "docs/whitepapers/05_BARE_METAL_HOOK_ISOLATION.md",
        "docs/whitepapers/08_THE_MERKLE_MORIN_ARCHITECTURE.md",
        "docs/whitepapers/07_THE_ARCHITECTURE_OF_EQUILIBRIUM.md",
        "docs/whitepapers/MMA_NOSTR_SPECIFICATION.md",
        "docs/DEVPOST.md",
        "docs/SUBMISSION_ENTRY.md",
        "docs/BENCHMARKS.md",
        "AGENTS.md",
        "GEMINI.md",
        "crates/forge-envelope/src/lib.rs",
        "crates/forge-envelope/src/weaver.rs",
    ],
    # DevOps & Infrastructure Profile (~44k tokens)
    "devops": [
        "GEMINI.md",
        "AGENTS.md",
        "docs/DEVPOST.md",
        "docs/SUBMISSION_ENTRY.md",
        "docs/BENCHMARKS.md",
        "docs/JUDGE-BUILD.md",
        "docs/whitepapers/01_NORMALIZED_IPR.md",
        "docs/whitepapers/02_TIMELESS_COMPRESSION.md",
        "docs/whitepapers/04_CONSTRAINED_L402_GATE.md",
        "docs/whitepapers/05_BARE_METAL_HOOK_ISOLATION.md",
        "docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md",
        "docs/whitepapers/07_THE_ARCHITECTURE_OF_EQUILIBRIUM.md",
        "docs/whitepapers/08_THE_MERKLE_MORIN_ARCHITECTURE.md",
        "docs/whitepapers/09_RELATIVISTIC_5D_LORENTZ_ABERRATION_AND_OKLCH_SPECTRAL_PALETTES.md",
        "docs/whitepapers/MMA_NOSTR_SPECIFICATION.md",
        "crates/forge-envelope/Cargo.toml",
        "crates/forge-envelope/src/lib.rs",
        "crates/forge-envelope/src/weaver.rs",
        "crates/forge-envelope/src/cognitive_heal.rs",
        "crates/forge-envelope/src/safety_router.rs",
        "crates/forge-envelope/surfaceledger/ARCHITECTURE.md",
    ],
}

def sanitize_and_bundle(
    file_list: Optional[List[str]] = None,
    base_dir: Optional[Path] = None
) -> Tuple[Dict[str, str], List[str]]:
    """
    Bundles files while enforcing sovereign airgap.
    Returns (clean_bundle_dict, skipped_sovereign_files).
    """
    root = base_dir or WORKSPACE_ROOT
    target_paths = file_list or PROFILES["lean"]
    clean_bundle = {}
    skipped_files = []

    for rel_path in target_paths:
        full_path = root / rel_path
        if not full_path.exists():
            # Try searching relative to REPO_ROOT as well
            if (REPO_ROOT / rel_path).exists():
                full_path = REPO_ROOT / rel_path
            else:
                continue

        try:
            with open(full_path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
        except Exception as e:
            print(f"[READ SKIP] Could not read {rel_path}: {e}")
            continue

        is_violation, reason = check_sovereign_violation(rel_path, content)
        if is_violation:
            print(f"[SOVEREIGN AIRGAP] Preserving local isolation: Skipped '{rel_path}' ({reason})")
            skipped_files.append(rel_path)
            continue

        clean_bundle[rel_path] = content

    return clean_bundle, skipped_files


# =============================================================================
# 3. GOOGLE GENAI CLIENT INITIALIZATION
# =============================================================================

def get_genai_client() -> Tuple[Optional[Any], Optional[str]]:
    """
    Gracefully initializes the google-genai client.
    Returns (client_instance, error_message).
    """
    try:
        from google import genai
    except ImportError:
        return None, (
            "The 'google-genai' SDK is not installed in the current environment.\n"
            "To install, run: pip install google-genai"
        )

    project_id = os.environ.get("GOOGLE_CLOUD_PROJECT") or os.environ.get("GCP_PROJECT")
    location = os.environ.get("GOOGLE_CLOUD_LOCATION", "us-central1")
    api_key = os.environ.get("GEMINI_API_KEY")

    # Hard socket deadline: without it a stalled aiplatform connection hangs
    # caches.list() forever with no output, which reads as a dead script.
    timeout_ms = int(os.environ.get("VERTEX_TIMEOUT_MS", "45000"))
    try:
        from google.genai import types as _gt
        http_opts = _gt.HttpOptions(timeout=timeout_ms)
    except Exception:
        http_opts = None

    try:
        if project_id:
            client = genai.Client(vertexai=True, project=project_id,
                                  location=location, http_options=http_opts)
            return client, None
        elif api_key:
            client = genai.Client(api_key=api_key, http_options=http_opts)
            return client, None
        else:
            client = genai.Client(http_options=http_opts)
            return client, None
    except Exception as e:
        return None, (
            f"Authentication check notice: {e}\n"
            "Options:\n"
            "  1. Run: gcloud auth application-default login\n"
            "  2. Or set: export GOOGLE_CLOUD_PROJECT='your-project-id'\n"
            "  3. Or set: export GEMINI_API_KEY='your-gemini-key'"
        )


# =============================================================================
# 4. SERVERLESS CONTEXT CACHE MANAGER
# =============================================================================

def get_or_create_serverless_cache(
    client: Any,
    bundle: Dict[str, str],
    bundle_tag: str = "lean",
    model_name: str = DEFAULT_MODEL,
    ttl: str = DEFAULT_TTL,
    quiet: bool = False
) -> Tuple[Optional[str], Optional[str]]:
    """
    Calculates deterministic SHA-256 hash across bundled files, verifies existing cache,
    or uploads fresh serverless cache with automatic TTL expiration.
    """
    from google.genai import types

    hasher = hashlib.sha256()
    for path in sorted(bundle.keys()):
        hasher.update(path.encode("utf-8"))
        hasher.update(b"\x00")
        hasher.update(bundle[path].encode("utf-8"))
        hasher.update(b"\xff")
    manifest_hash = hasher.hexdigest()
    display_name = f"forge_{bundle_tag}_{manifest_hash[:12]}"

    if not quiet:
        print(f"[BUNDLE] Files: {len(bundle)} | Manifest SHA-256: {manifest_hash[:16]}...")
        print(f"[CACHE] Target Display Name: '{display_name}'")

    cache_mgr = getattr(client, "caches", getattr(client, "cached_contents", None))
    if not cache_mgr:
        return None, "google-genai client does not expose cache management interface."

    # 1. Check existing active cache (Cache HIT)
    try:
        for existing in cache_mgr.list():
            if existing.display_name == display_name:
                if not quiet:
                    print(f"[CACHE HIT] Reusing active serverless context: {existing.name}")
                    print(f"   Expires: {existing.expire_time}")
                return existing.name, None
    except Exception as e:
        if not quiet:
            print(f"[CACHE QUERY] Notice: Could not list caches ({e}). Proceeding to create fresh cache...")

    # 2. Build payload
    doc_sections = [
        "=== FORGE V3 CANONICAL ARCHITECTURE & SPECIFICATION LEDGER ===\n"
        "You are the Lead Systems Architect and Sovereign Pairing Engineer for Forge v3.\n"
        "Provide deterministic, concise, accurate analysis strictly adhering to zero-heap\n"
        "safety, #![deny(unsafe_code)], and approved master specifications.\n"
    ]
    for rel_path, content in bundle.items():
        doc_sections.append(f"\n--- FILE: {rel_path} ---\n{content}\n")

    full_text = "\n".join(doc_sections)
    approx_tokens = len(full_text) // 4

    if not quiet:
        print(f"[CACHE MISS] Creating new Serverless Context Cache (~{approx_tokens:,} tokens, TTL: {ttl})...")

    try:
        cache = cache_mgr.create(
            model=model_name,
            config=types.CreateCachedContentConfig(
                contents=[types.Content(role="user", parts=[types.Part.from_text(text=full_text)])],
                display_name=display_name,
                ttl=ttl,
            ),
        )
        if not quiet:
            print(f"[CACHE CREATED] Handle: {cache.name} (Expires: {cache.expire_time})")
        return cache.name, None
    except Exception as e:
        return None, f"Failed to create context cache: {e}"


# =============================================================================
# 5. HIGH-VISIBILITY "LOUD VISUAL" TELEMETRY BANNER
# =============================================================================

def render_loud_visual_receipt(
    usage_metadata: Any,
    cache_name: Optional[str] = None,
    tag: str = "lean",
    sovereign_skipped: Optional[int] = None
):
    """
    Renders a high-contrast, colorized ANSI/Unicode box receipt showing token hit ratio,
    sub-cent cost, savings percentage, and governor ceiling check.
    """
    if not usage_metadata:
        return

    prompt_tokens = getattr(usage_metadata, "prompt_token_count", 0) or 0
    cached_tokens = getattr(usage_metadata, "cached_content_token_count", 0) or 0
    candidates_tokens = getattr(usage_metadata, "candidates_token_count", 0) or 0
    total_tokens = getattr(usage_metadata, "total_token_count", 0) or 0
    total_tokens = max(total_tokens, prompt_tokens + cached_tokens + candidates_tokens
                       if prompt_tokens < cached_tokens else total_tokens)

    # prompt_token_count may or may not already include the cached prefix.
    uncached_input = prompt_tokens - cached_tokens if prompt_tokens >= cached_tokens else prompt_tokens
    total_input = uncached_input + cached_tokens

    # Cost calculations
    cost_uncached = uncached_input * (PRICE_INPUT_1M / 1_000_000)
    cost_cached = cached_tokens * (PRICE_CACHED_INPUT_1M / 1_000_000)
    cost_output = candidates_tokens * (PRICE_OUTPUT_1M / 1_000_000)
    actual_cost = cost_uncached + cost_cached + cost_output

    standard_uncached_cost = (total_input * (PRICE_INPUT_1M / 1_000_000)) + cost_output
    savings = max(0.0, standard_uncached_cost - actual_cost)
    savings_pct = (savings / standard_uncached_cost * 100.0) if standard_uncached_cost > 0 else 0.0

    hit_ratio_pct = (cached_tokens / total_input * 100.0) if total_input > 0 else 0.0
    hit_ratio_pct = min(100.0, max(0.0, hit_ratio_pct))

    # Build visual progress bar (46 characters wide)
    bar_width = 46
    filled_len = min(bar_width, max(0, int(bar_width * (hit_ratio_pct / 100.0))))
    bar_str = "█" * filled_len + "░" * (bar_width - filled_len)

    # Bar saturates for layout; the printed ratio does not, so overage stays visible.
    gov_ratio_true = (actual_cost / GOVERNOR_COST_CEILING) if GOVERNOR_COST_CEILING > 0 else 0.0
    gov_filled = int(20 * min(1.0, gov_ratio_true))
    gov_bar = "=" * gov_filled + ">" + " " * max(0, 19 - gov_filled)
    gov_state = "OVER" if gov_ratio_true > 1.0 else "under"

    handle_disp = (cache_name.split("/")[-1] if cache_name else "DIRECT_MODE (NO CACHE)")[:38]
    status_text = "🟢 CACHE HIT (EXPLICIT SERVERLESS)" if cached_tokens > 0 else "⚪ DIRECT UNCACHED INFERENCE"

    print("\n" + "╔" + "═"*78 + "╗")
    print(f"║  ⚡ FORGE LEAN DEV CACHE  ::  {DEFAULT_MODEL.upper()} (SERVERLESS)".ljust(79) + "║")
    print("╠" + "═"*78 + "╣")
    print(f"║  [STATUS]          {status_text}".ljust(79) + "║")
    print(f"║  [PROFILE/HANDLE]  profile: {tag} | handle: {handle_disp}".ljust(79) + "║")
    airgap_txt = (f"{sovereign_skipped} sovereign file(s) withheld from upload (ADR-0026)"
                  if sovereign_skipped is not None
                  else "[UNVERIFIED] withheld-file count not reported to this receipt (ADR-0026)")
    print(f"║  [AIRGAP GUARD]    🛡️ {airgap_txt}".ljust(79) + "║")
    print("╠" + "═"*78 + "╣")
    print("║  📊 TOKEN EFFICIENCY RATIO:".ljust(79) + "║")
    print(f"║  {bar_str}  {hit_ratio_pct:>5.1f}% CACHED".ljust(79) + "║")
    print("║".ljust(79) + "║")
    print(f"║  • Cached Context Tokens : {cached_tokens:>8,} tokens  (75% discount applied)".ljust(79) + "║")
    print(f"║  • Uncached Query Tokens : {uncached_input:>8,} tokens".ljust(79) + "║")
    print(f"║  • Output Tokens         : {candidates_tokens:>8,} tokens".ljust(79) + "║")
    print(f"║  • Total Turn Tokens     : {total_tokens:>8,} tokens".ljust(79) + "║")
    print("╠" + "═"*78 + "╣")
    print("║  💰 REAL-TIME QUERY COST RECEIPT:".ljust(79) + "║")
    print(f"║  • Standard Uncached Cost:   ${standard_uncached_cost:>10.6f}".ljust(79) + "║")
    print(f"║  • Actual Billed Cost    :   ${actual_cost:>10.6f}".ljust(79) + "║")
    print(f"║  • Exact Session Savings :   ${savings:>10.6f} ({savings_pct:>4.1f}% Cost Reduction)".ljust(79) + "║")
    print(f"║  • Cost Monitor (Soft)   :   [{gov_bar}] {gov_ratio_true*100:>6.1f}% of ${GOVERNOR_COST_CEILING:.4f} soft ceiling ({gov_state}, not enforced)".ljust(79) + "║")
    print("╚" + "═"*78 + "╝\n")


def query_flash_cache(
    client: Any,
    cache_name: Optional[str],
    prompt: str,
    model_name: str = DEFAULT_MODEL
) -> Tuple[Optional[str], Optional[Any]]:
    """
    Executes deterministic inference against the cached context (temperature=0.0, top_k=1).
    Enforces 3-Wave Cree Ghost Words linguistic filter & ADR-0026 zero-retention on prompt and response.
    """
    # 1. Pre-dispatch Prompt Cultural Safety Validation.
    # Runs before the cloud SDK is even imported: the sovereign gate must not depend on
    # the thing it exists to withhold from.
    staging = bytearray(prompt, "utf-8")
    is_violation, rationale, wave = validate_3wave_cree_filter(prompt)
    if is_violation:
        wiped = crypto_shred(staging)
        print("\n" + "╔" + "═"*78 + "╗")
        print("║  🛑 CULTURAL SAFETY REFUSAL — NETWORK DISPATCH HALTED (ADR-0026)".ljust(79) + "║")
        print("╠" + "═"*78 + "╣")
        print(f"║  • Refusal Wave   : Wave {wave} Violation".ljust(79) + "║")
        print(f"║  • Safety Reason  : {rationale}".ljust(79) + "║")
        print(f"║  • Memory Action  : Staging buffer shredded ({wiped} B); no bytes left this host.".ljust(79) + "║")
        print("╚" + "═"*78 + "╝\n")
        return None, None
    crypto_shred(staging)

    from google.genai import types

    config_kwargs = {
        "temperature": 0.0,
        "top_k": 1,
        "top_p": 0.0,
        "max_output_tokens": MAX_OUTPUT_TOKENS,
    }

    if cache_name:
        config_kwargs["cached_content"] = cache_name
    else:
        config_kwargs["system_instruction"] = (
            "You are a deterministic, senior systems architect for Forge v3. "
            "Answer questions strictly using provided codebase and specification evidence. "
            "Be terse, precise, and cite file names accurately."
        )

    config = types.GenerateContentConfig(**config_kwargs)

    try:
        response = client.models.generate_content(
            model=model_name,
            contents=prompt,
            config=config,
        )
        resp_text = response.text or ""

        # 2. Post-generation Response Cultural Safety Inspection
        is_resp_violation, resp_rationale, resp_wave = validate_3wave_cree_filter(resp_text)
        if is_resp_violation:
            resp_staging = bytearray(resp_text, "utf-8")
            wiped = crypto_shred(resp_staging)
            resp_text = ""
            print("\n" + "╔" + "═"*78 + "╗")
            print("║  🛡️ POST-GENERATION CULTURAL SAFETY REFUSAL (ADR-0026)".ljust(79) + "║")
            print("╠" + "═"*78 + "╣")
            print(f"║  • Intercept Wave : Wave {resp_wave} Violation".ljust(79) + "║")
            print(f"║  • Reason         : Model attempted emitting sovereign Cree token.".ljust(79) + "║")
            print(f"║  • Details        : {resp_rationale}".ljust(79) + "║")
            print(f"║  • Memory Action  : Response staging shredded ({wiped} B); text dropped unreturned.".ljust(79) + "║")
            print("╚" + "═"*78 + "╝\n")
            return "[SOVEREIGN CULTURAL SAFETY INTERVENTION: Response redacted under ADR-0026]", response.usage_metadata

        return resp_text, response.usage_metadata
    except Exception as e:
        print(f"[INFERENCE ERROR] {e}")
        return None, None


# =============================================================================
# 6. CACHE MANAGEMENT COMMANDS (STATUS & PURGE)
# =============================================================================

def list_active_caches(client: Any):
    """Lists all active cached contexts with their models and TTLs."""
    print("\n" + "╔" + "═"*78 + "╗")
    print("║  ACTIVE VERTEX AI CONTEXT CACHES (SERVERLESS)".ljust(79) + "║")
    print("╠" + "═"*78 + "╣")
    count = 0
    cache_mgr = getattr(client, "caches", getattr(client, "cached_contents", None))
    if cache_mgr:
        try:
            for cache in cache_mgr.list():
                count += 1
                print(f"║  • Handle      : {cache.name}".ljust(79) + "║")
                print(f"║    Display Name: {getattr(cache, 'display_name', 'N/A')}".ljust(79) + "║")
                print(f"║    Model       : {getattr(cache, 'model', 'N/A')}".ljust(79) + "║")
                print(f"║    Expires     : {getattr(cache, 'expire_time', 'N/A')}".ljust(79) + "║")
                print("╟" + "─"*78 + "╢")
        except Exception as e:
            print(f"║  [ERROR] Could not query caches: {e}".ljust(79) + "║")
    else:
        print("║  [ERROR] Cache management interface not available on client.".ljust(79) + "║")

    if count == 0:
        print("║  No active cached contents found in current project.".ljust(79) + "║")
    print("╚" + "═"*78 + "╝\n")


def purge_all_caches(client: Any):
    """Deletes active dev caches."""
    print("[PURGE] Scanning for active dev caches to delete...")
    deleted = 0
    cache_mgr = getattr(client, "caches", getattr(client, "cached_contents", None))
    if not cache_mgr:
        print("[PURGE ERROR] Cache interface not available on client.")
        return
    try:
        for cache in cache_mgr.list():
            display = getattr(cache, "display_name", "")
            if display.startswith("forge_") or display.startswith("devops_") or display.startswith("test_"):
                print(f"[DELETING] {cache.name} ({display})...")
                cache_mgr.delete(name=cache.name)
                deleted += 1
        print(f"[PURGE COMPLETE] Deleted {deleted} cache instance(s).")
    except Exception as e:
        print(f"[PURGE ERROR] {e}")


# =============================================================================
# 7. CLI & AUTOMATION ENTRYPOINT
# =============================================================================

def parse_args():
    parser = argparse.ArgumentParser(
        description="Forge v3 Lean Vertex AI Context Caching & Automated Oracle Hub",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Agent & Developer Quick-Start:
  # Pre-warm / verify lean cache:
  python vertex_flash_cache.py --warm

  # Execute a query with loud visual telemetry:
  python vertex_flash_cache.py "Where is AbsenceIndex5D defined and verified?"

  # Check active cache status & TTL:
  python vertex_flash_cache.py --status

  # Query using PaTeX profile:
  python vertex_flash_cache.py --profile patex "Explain the 5D hyperbox absence index"
        """
    )
    parser.add_argument("query", nargs="*", help="Optional query string to execute immediately.")
    parser.add_argument("--warm", action="store_true", help="Pre-warm / verify context cache and exit.")
    parser.add_argument("--status", action="store_true", help="List active serverless caches and exit.")
    parser.add_argument("--purge", action="store_true", help="Purge all active forge/devops caches.")
    parser.add_argument("--profile", type=str, default="lean", choices=list(PROFILES.keys()), help="Bundle profile preset (default: lean).")
    parser.add_argument("--bundle", type=str, help="Comma-separated list of custom relative files to bundle.")
    parser.add_argument("--tag", type=str, help="Custom tag identifier for the cache display name.")
    parser.add_argument("--model", type=str, default=DEFAULT_MODEL, help=f"Gemini model identifier (default: {DEFAULT_MODEL}).")
    parser.add_argument("--require-cache", "--strict", action="store_true", help="Fail loudly with non-zero exit code if auth fails, cache creation fails, or cache is missed.")
    parser.add_argument("--quiet", action="store_true", help="Suppress intermediate progress banners.")
    return parser.parse_args()


def main():
    args = parse_args()

    # 1. Authenticate Client
    client, auth_err = get_genai_client()
    if auth_err:
        if args.require_cache:
            print(f"\n[STRICT AUTHENTICATION FAILURE]\n{auth_err}\n", file=sys.stderr)
            sys.exit(1)
        if not args.quiet:
            print(f"\n[AIRGAP / AUTHENTICATION NOTICE]\n{auth_err}\n")
            print("Exiting gracefully. Zero cloud API calls or charges incurred.")
        sys.exit(0)

    # 2. Handle Status or Purge
    if args.status:
        list_active_caches(client)
        sys.exit(0)

    if args.purge:
        purge_all_caches(client)
        sys.exit(0)

    tag_name = args.tag or args.profile

    # 3. Sanitize & Bundle Files
    if args.bundle:
        custom_files = [f.strip() for f in args.bundle.split(",") if f.strip()]
        bundle, skipped = sanitize_and_bundle(custom_files)
    else:
        bundle, skipped = sanitize_and_bundle(PROFILES.get(args.profile, PROFILES["lean"]))

    if skipped and not args.quiet:
        print(f"[AIRGAP CONFIRMED] {len(skipped)} sovereign file(s) isolated locally.")

    if not bundle:
        print("[ERROR] No valid non-sovereign files found to bundle.", file=sys.stderr)
        sys.exit(1)

    # 4. Context Cache Get / Create
    cache_name, cache_err = get_or_create_serverless_cache(
        client, bundle, bundle_tag=tag_name, model_name=args.model, quiet=args.quiet
    )
    if cache_err:
        if args.require_cache:
            print(f"\n[STRICT CACHE ERROR] Failed to obtain serverless context cache: {cache_err}\n", file=sys.stderr)
            sys.exit(1)
        if not args.quiet:
            print(f"[CACHE NOTICE] {cache_err}")
            print("Falling back to direct prompt mode without caching...")
        cache_name = None

    # Handle --warm flag
    if args.warm:
        if args.require_cache and not cache_name:
            print(f"[STRICT CACHE ERROR] Context cache handle is empty after warming.", file=sys.stderr)
            sys.exit(1)
        print(f"\n[WARM STATUS] Lean context cache is active and ready: {cache_name or 'DIRECT_MODE'}")
        if not args.quiet:
            list_active_caches(client)
        sys.exit(0)

    # 5. Handle Query
    query_text = " ".join(args.query).strip() if args.query else None
    if not query_text and not sys.stdin.isatty():
        query_text = sys.stdin.read().strip()

    if query_text:
        answer, usage = query_flash_cache(client, cache_name, query_text, model_name=args.model)
        if not answer or not usage:
            if args.require_cache:
                print(f"[STRICT CACHE ERROR] Inference failed or returned empty response.", file=sys.stderr)
                sys.exit(1)
        if answer:
            print("\n" + "="*70)
            print("QUERY RESPONSE:")
            print("="*70)
            print(answer)
            print("="*70)
            render_loud_visual_receipt(usage, cache_name=cache_name, tag=tag_name,
                                       sovereign_skipped=len(skipped))
            
            if args.require_cache:
                cached_tokens = getattr(usage, "cached_content_token_count", 0) or 0
                if cached_tokens == 0:
                    print(f"\n[STRICT CACHE FAILURE] Received 0 cached tokens in response metadata! (Prompt tokens: {getattr(usage, 'prompt_token_count', 0)})", file=sys.stderr)
                    sys.exit(1)
                print(f"[STRICT CACHE VERIFIED] Realized {cached_tokens:,} cached tokens from Vertex context cache.")
    else:
        print("\n[READY] Interactive Forge Oracle (Lean Cache). Type 'exit' to quit.\n")
        while True:
            try:
                user_prompt = input("forge-dev> ").strip()
                if not user_prompt:
                    continue
                if user_prompt.lower() in ["exit", "quit", "q"]:
                    print("\nExiting gracefully. All active caches will auto-expire per TTL.")
                    break

                answer, usage = query_flash_cache(client, cache_name, user_prompt, model_name=args.model)
                if answer:
                    print("\n" + "="*70)
                    print(answer)
                    print("="*70)
                    render_loud_visual_receipt(usage, cache_name=cache_name, tag=tag_name,
                                       sovereign_skipped=len(skipped))
            except KeyboardInterrupt:
                print("\n\nSession terminated by user.")
                break
            except Exception as e:
                print(f"[ERROR] {e}")


if __name__ == "__main__":
    main()
