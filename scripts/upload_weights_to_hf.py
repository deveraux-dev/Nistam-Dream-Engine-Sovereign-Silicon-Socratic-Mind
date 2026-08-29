#!/usr/bin/env python3
"""Upload S13 quantized weights to Hugging Face Hub."""

import os
import sys
from pathlib import Path
from huggingface_hub import HfApi, login

def upload_weights():
    """Upload s13_gemma_2b_m3 and s13_gemma_9b_m3 to HF Hub."""

    # Authenticate
    hf_token = os.environ.get("HF_TOKEN")
    if not hf_token:
        print("[upload] ERROR: HF_TOKEN environment variable not set")
        print("[upload] Set it: $env:HF_TOKEN = 'hf_...'")
        return False

    print(f"[upload] Authenticating with token (length: {len(hf_token)})")
    login(token=hf_token)

    # Initialize API
    api = HfApi()
    repo_id = "deveraux-dev/s13-gemma-quantized"

    # Create repo if needed
    try:
        print(f"[upload] Checking/creating repo: {repo_id}")
        api.create_repo(repo_id=repo_id, repo_type="model", exist_ok=True, private=False)
    except Exception as e:
        print(f"[upload] Repo check/create: {e}")

    repo_root = Path(__file__).parent.parent
    weights = [
        ("s13_gemma_2b_m3", "s13_gemma_2b_m3"),
        ("s13_gemma_9b_m3", "s13_gemma_9b_m3"),
    ]

    for local_dir, remote_path in weights:
        local_path = repo_root / local_dir
        if not local_path.exists():
            print(f"[upload] ✗ {local_dir} not found at {local_path}")
            continue

        try:
            print(f"[upload] Uploading {local_dir} ({local_path.stat().st_size / 1e9:.2f} GB)...")
            api.upload_folder(
                folder_path=str(local_path),
                repo_id=repo_id,
                path_in_repo=remote_path,
                repo_type="model",
            )
            print(f"[upload] ✓ {local_dir} uploaded")
        except Exception as e:
            print(f"[upload] ERROR uploading {local_dir}: {e}")
            return False

    print(f"\n[upload] SUCCESS: Weights uploaded to https://huggingface.co/{repo_id}")
    return True

if __name__ == "__main__":
    if not upload_weights():
        sys.exit(1)
