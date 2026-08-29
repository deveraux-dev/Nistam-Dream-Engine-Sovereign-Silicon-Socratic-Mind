#!/usr/bin/env python3
"""
Surface Ledger & Forge-Envelope — GCP/Vertex AI Billing Credit Validator

This script is designed specifically to verify that your Google Cloud Platform (GCP)
Vertex AI billing is drawing properly from your promotional/competition credits.

To generate a fast, visible, yet safe charge (e.g., $1.00 to $25.00) on your GCP 
billing console, this script can:
1. Read the codebase files as a larger payload context.
2. Call the flagship Gemini 1.5 Pro model (which carries higher billing weights than Flash).
3. Run a configured number of parallel/sequential structured audit evaluations.
4. Keep a running tally of estimated USD cost based on real-time token usage.
"""

import os
import sys
import json
import time
from pathlib import Path
from pydantic import BaseModel, Field
from google import genai
from google.genai import types
from vertex_schema_client import get_gemini_client, PhysicalInspectionAudit, dispatch_structured_audit

# Pricing for Gemini 1.5 Pro (Vertex AI Standard Tiers)
PRO_INPUT_PER_1K = 0.00125    # $1.25 per Million
PRO_OUTPUT_PER_1K = 0.00500   # $5.00 per Million

# Pricing for Gemini 2.5 Flash
FLASH_INPUT_PER_1K = 0.000075 # $0.075 per Million
FLASH_OUTPUT_PER_1K = 0.000300# $0.300 per Million

REPO_ROOT = Path(__file__).parent.parent.resolve()

def bundle_codebase_payload() -> str:
    """Combines key source files to create a substantial text payload (~10k-20k tokens)."""
    payload_parts = []
    files_to_read = [
        "Cargo.toml",
        "GEMINI.md",
        "src/lib.rs",
        "src/weaver.rs",
        "src/somatic_tokenizer.rs",
        "src/cognitive_heal.rs",
        "src/mom.rs",
        "src/safety_router.rs",
        "surfaceledger/index.html",
    ]
    
    for rel_path in files_to_read:
        p = REPO_ROOT / rel_path
        if p.exists():
            payload_parts.append(f"\n--- FILE CONTEXT: {rel_path} ---")
            with open(p, "r", encoding="utf-8", errors="replace") as f:
                payload_parts.append(f.read())
    return "\n".join(payload_parts)

def run_billing_test(target_model: str, max_cost_target_usd: float, queries_count: int, no_confirm: bool = False):
    """Generates real structured queries to Vertex AI and calculates actual cost draw."""
    print("----------------------------------------------------------------------")
    print(f"[START] Commencing GCP Credit Validation run...")
    print(f"        Model:        '{target_model}'")
    print(f"        Max Cost Cap: ${max_cost_target_usd:.2f} USD")
    print(f"        Max Queries:  {queries_count}")
    print("----------------------------------------------------------------------")

    # 1. Connect Vertex Client
    try:
        client = get_gemini_client()
    except Exception as e:
        print(f"\n[ERROR] Connection failed: {e}")
        return

    # 2. Assemble Payload
    print("[LOAD] Reading codebase source files to package a heavy-weight query context...")
    payload_context = bundle_codebase_payload()
    char_count = len(payload_context)
    est_prompt_tokens = char_count // 4
    print(f"[LOAD] Payload Size: {char_count:,} characters (~{est_prompt_tokens:,} prompt tokens).")

    # Determine rates based on model
    is_pro = "pro" in target_model.lower()
    in_rate = PRO_INPUT_PER_1K if is_pro else FLASH_INPUT_PER_1K
    out_rate = PRO_OUTPUT_PER_1K if is_pro else FLASH_OUTPUT_PER_1K

    estimated_cost_per_query = (est_prompt_tokens / 1000.0) * in_rate + (250 / 1000.0) * out_rate
    print(f"[INFO] Estimated cost per single query: ${estimated_cost_per_query:.4f} USD")
    print(f"[INFO] Projected query capacity before hitting ${max_cost_target_usd:.2f} cap: {int(max_cost_target_usd / estimated_cost_per_query)} queries.")
    
    # Check if stdin is tty or if we are in an automated non-interactive terminal or if no_confirm is passed
    if no_confirm:
        print("\n[CONFIRM] --no-confirm flag passed. Proceeding automatically...")
    elif sys.stdin.isatty():
        input(f"\nProceed to dispatch real Vertex AI transactions? [Press Enter to confirm, Ctrl+C to abort]")
    else:
        print("\n[AUTO-CONFIRM] Non-interactive environment detected. Bypassing confirmation prompt and proceeding to dispatch...")

    total_accumulated_cost = 0.0
    successful_audits = 0

    for i in range(queries_count):
        print(f"\n--- DISPATCHING VERIFICATION QUERY #{i+1} of {queries_count} ---")
        
        # We craft a schema prompt containing the heavy payload context
        prompt = f"""
        === REGULATORY VERIFICATION AUDIT ===
        Using the provided physical codebase context as a reference standard:
        {payload_context[:10000]}  # Send a subset to keep it controlled and fast

        Generate a validated audit verdict for our Edmonton Walterdale Bridge physical inspection.
        Verify that Sector A3 curvature of 0.72mm causes a NACE compliance level failure of 3.
        """

        # Enforce Pydantic schema
        config = types.GenerateContentConfig(
            temperature=0.0,
            top_k=1,
            top_p=0.0,
            response_mime_type="application/json",
            response_schema=PhysicalInspectionAudit,
            system_instruction=(
                "You are the Lead Systems Engineer and Physical Coating Auditor. "
                "Output deterministic compliance audits verified against the codebase VARS rules."
            )
        )

        t_start = time.time()
        try:
            response = client.models.generate_content(
                model=target_model,
                contents=prompt,
                config=config,
            )
            t_duration = time.time() - t_start

            # Calculate actual token usage if returned by Vertex AI, otherwise use estimates
            usage = response.usage_metadata
            if usage:
                in_tok = usage.prompt_token_count
                out_tok = usage.candidates_token_count
            else:
                in_tok = est_prompt_tokens
                out_tok = 250

            query_cost = (in_tok / 1000.0) * in_rate + (out_tok / 1000.0) * out_rate
            total_accumulated_cost += query_cost
            successful_audits += 1

            print(f"[SUCCESS] Latency: {t_duration:.2f}s | Tokens: In={in_tok}, Out={out_tok}")
            print(f"[COST]    This Query: ${query_cost:.4f} USD | Total Run Cost: ${total_accumulated_cost:.4f} USD")
            
            # Print a snippet of the validated JSON to prove no smoke and mirrors
            raw_text = response.text.strip()
            print(f"[JSON RECORD RECEIVED]:\n{raw_text[:200]}...")

        except Exception as e:
            print(f"[ERROR] Transaction failed: {e}")
            break

        # Safety Check: Did we exceed the user's spending target?
        if total_accumulated_cost >= max_cost_target_usd:
            print(f"\n[ALERT] Reached user spending target of ${max_cost_target_usd:.2f} USD. Halting execution.")
            break

    print("\n" + "=" * 80)
    print("                 VERTEX AI BILLING TEST COMPLETED                     ")
    print("=" * 80)
    print(f"Total Successful Queries: {successful_audits}")
    print(f"Total Billing Cost Accumulation: ${total_accumulated_cost:.4f} USD")
    print(f"Estimated Draw from GCP Credits: ${total_accumulated_cost:.4f} USD")
    print("-" * 80)
    print("  Go to your Google Cloud Console -> Billing to see this charge draw")
    print("  on your promotional/hackathon credit ledger (typically updates within 1-2 hours).")
    print("=" * 80)


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="GCP/Vertex AI Billing Validator")
    parser.add_argument(
        "--model", 
        type=str, 
        default="gemini-2.5-flash", 
        help="Model to use: 'gemini-2.5-flash' (default, cheaper) or 'gemini-2.5-pro' (faster billing draw)"
    )
    parser.add_argument(
        "--target-usd", 
        type=float, 
        default=2.00, 
        help="Safety budget spending cap in USD (default is $2.00 to verify, can go up to $25.00)"
    )
    parser.add_argument(
        "--queries", 
        type=int, 
        default=5, 
        help="Maximum number of structured queries to run sequentially"
    )
    parser.add_argument(
        "--no-confirm",
        action="store_true",
        help="Bypass confirmation prompt automatically"
    )

    args = parser.parse_args()
    
    # Verify model choice
    m = args.model
    if m not in ["gemini-2.5-flash", "gemini-2.5-flash-lite", "gemini-2.5-pro", "gemini-1.5-pro", "gemini-1.5-flash"]:
        print(f"[WARN] Custom model '{m}' requested. Proceeding...")

    run_billing_test(
        target_model=m, 
        max_cost_target_usd=args.target_usd, 
        queries_count=args.queries,
        no_confirm=args.no_confirm
    )
