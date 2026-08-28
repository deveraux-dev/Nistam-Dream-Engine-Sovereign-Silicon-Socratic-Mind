#!/usr/bin/env python3
"""
scripts/cloud_run_client.py
Surface Ledger — Authenticated Client for Cloud Run Services:
1. archive-arbiter: https://archive-arbiter-362227725307.us-central1.run.app
2. archive-indexer: https://archive-indexer-362227725307.us-central1.run.app
"""

import os
import sys
import json
import urllib.request
import urllib.error
from typing import Dict, Any, Optional

try:
    import google.auth
    import google.auth.transport.requests
    from google.oauth2 import id_token
except ImportError:
    google = None

ARBITER_URL = os.environ.get(
    "ARCHIVE_ARBITER_URL", 
    "https://archive-arbiter-362227725307.us-central1.run.app"
)
INDEXER_URL = os.environ.get(
    "ARCHIVE_INDEXER_URL", 
    "https://archive-indexer-362227725307.us-central1.run.app"
)

def get_id_token(target_audience: str) -> Optional[str]:
    """Retrieves an identity token for authenticating to private Cloud Run services."""
    try:
        auth_req = google.auth.transport.requests.Request()
        token = id_token.fetch_id_token(auth_req, target_audience)
        return token
    except Exception as e:
        # Fallback to gcloud CLI if running locally
        import subprocess
        try:
            res = subprocess.run(
                ["gcloud", "auth", "print-identity-token", f"--audiences={target_audience}"],
                capture_output=True, text=True, check=True
            )
            return res.stdout.strip()
        except Exception:
            return None

def call_cloud_run_service(
    service_url: str, 
    endpoint: str = "", 
    method: str = "GET", 
    payload: Optional[Dict[str, Any]] = None
) -> Dict[str, Any]:
    """Invokes a Cloud Run service endpoint with IAM ID token authentication."""
    url = f"{service_url.rstrip('/')}/{endpoint.lstrip('/')}".rstrip('/')
    headers = {"Content-Type": "application/json"}
    
    token = get_id_token(service_url)
    if token:
        headers["Authorization"] = f"Bearer {token}"
        
    data = json.dumps(payload).encode("utf-8") if payload else None
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    
    try:
        with urllib.request.urlopen(req) as resp:
            body = resp.read().decode("utf-8")
            try:
                return json.loads(body)
            except json.JSONDecodeError:
                return {"status_code": resp.status, "response": body}
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8")
        return {
            "error": True,
            "status_code": e.code,
            "message": error_body or e.reason
        }
    except Exception as e:
        return {"error": True, "message": str(e)}

if __name__ == "__main__":
    print("=== Testing Cloud Run Live Endpoints ===")
    print(f"Arbiter: {ARBITER_URL}")
    print(f"Indexer: {INDEXER_URL}\n")
    
    print("[1/2] Probing archive-arbiter health/root...")
    arbiter_resp = call_cloud_run_service(ARBITER_URL, endpoint="")
    print(f"Response: {json.dumps(arbiter_resp, indent=2)}\n")
    
    print("[2/2] Probing archive-indexer health/root...")
    indexer_resp = call_cloud_run_service(INDEXER_URL, endpoint="")
    print(f"Response: {json.dumps(indexer_resp, indent=2)}")
