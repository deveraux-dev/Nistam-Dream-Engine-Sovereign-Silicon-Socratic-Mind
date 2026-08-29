# Live Google Cloud demo: one autonomous agent, one pass, no mocks.
# Fails loudly if any GCP dependency is missing rather than degrading silently.
$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$Envelope = Join-Path $RepoRoot "crates\forge-envelope"

if (-not $env:GOOGLE_CLOUD_PROJECT) {
    $env:GOOGLE_CLOUD_PROJECT = (gcloud config get-value project 2>$null).Trim()
}
if (-not $env:GOOGLE_CLOUD_PROJECT) { throw "GOOGLE_CLOUD_PROJECT unset and gcloud has no default project" }
if (-not $env:GOOGLE_CLOUD_LOCATION) { $env:GOOGLE_CLOUD_LOCATION = "us-central1" }
if (-not $env:INBOX_BUCKET) { $env:INBOX_BUCKET = "$($env:GOOGLE_CLOUD_PROJECT)-s13-inbox" }

Write-Host "=== Surface Ledger — live cloud agent ===" -ForegroundColor Cyan
Write-Host "  project  : $($env:GOOGLE_CLOUD_PROJECT)"
Write-Host "  location : $($env:GOOGLE_CLOUD_LOCATION)"
Write-Host "  inbox    : gs://$($env:INBOX_BUCKET)"
Write-Host "  model    : $(if ($env:GEMINI_MODEL) { $env:GEMINI_MODEL } else { 'gemini-2.5-flash' })"

Write-Host "`n[1/3] Building the attestation binary..." -ForegroundColor Yellow
Push-Location $Envelope
try {
    cargo build --release --bin attest --features cli
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    $env:FORGE_ENVELOPE_BIN = Join-Path $Envelope "target\release\attest.exe"
    if (-not (Test-Path $env:FORGE_ENVELOPE_BIN)) {
        $env:FORGE_ENVELOPE_BIN = Join-Path $Envelope "target\release\attest"
    }
    if (-not (Test-Path $env:FORGE_ENVELOPE_BIN)) { throw "attest binary not found after build" }

    Write-Host "[2/3] Checking Python dependencies..." -ForegroundColor Yellow
    python -c "import google.genai, google.cloud.firestore, google.cloud.storage, pydantic"
    if ($LASTEXITCODE -ne 0) { throw "pip install -r crates/forge-envelope/requirements.txt" }

    Write-Host "[3/3] Running one live audit pass against Google Cloud...`n" -ForegroundColor Yellow
    python scripts\agent_loop.py --manual --require-cloud
    if ($LASTEXITCODE -ne 0) { throw "agent aborted (exit $LASTEXITCODE) — see cloud_required_abort above" }
}
finally { Pop-Location }

Write-Host "`n=== Chain head written to Firestore. Zero local retention. ===" -ForegroundColor Green
