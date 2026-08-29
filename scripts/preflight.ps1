# preflight - run before any push or re-upload. Fails loudly rather than degrading.
# Usage: .\scripts\preflight.ps1  [-SkipCloud]
param([switch]$SkipCloud)

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$fail = 0

function Step($n, $t) { Write-Host "`n[$n] $t" -ForegroundColor Yellow }
function Ok($m)   { Write-Host "  PASS  $m" -ForegroundColor Green }
function Bad($m)  { Write-Host "  FAIL  $m" -ForegroundColor Red; $script:fail++ }

Write-Host "=== preflight: $RepoRoot ===" -ForegroundColor Cyan

Step 1 "Unreceipted claims (claim_gate)"
python (Join-Path $RepoRoot "scripts\claim_gate.py") $RepoRoot
if ($LASTEXITCODE -eq 0) { Ok "no unreceipted claims" } else { Bad "unreceipted claims above" }

Step 2 "Fabricated verdicts (verdict_gate)"
python (Join-Path $RepoRoot "scripts\verdict_gate_selftest.py") | Out-Null
if ($LASTEXITCODE -eq 0) { Ok "verdict_gate selftest fixtures hold" } else { Bad "verdict_gate selftest FAILED - the gate itself is broken" }
python (Join-Path $RepoRoot "scripts\verdict_gate.py") $RepoRoot
if ($LASTEXITCODE -eq 0) { Ok "no fabricated verdicts" } else { Bad "fabricated verdicts above" }

Step 3 "Model id consistency"
$dead = Get-ChildItem $RepoRoot -Recurse -File -Include *.md,*.rs,*.py,*.ps1,*.json,*.html,*.txt -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notmatch '\\(target|target_env|node_modules|\.git)\\' -and
                       $_.Name -notmatch 'forge_lint\.py|claim_gate\.py|MODEL-STRING-SWEEP' } |
        Select-String -Pattern 'gemini-3\.\d' -ErrorAction SilentlyContinue
if ($dead) { $dead | ForEach-Object { Bad "$($_.Filename):$($_.LineNumber)" } } else { Ok "no dead model ids" }

Step 4 "Cloud config agreement"
$agent = Get-Content (Join-Path $RepoRoot "scripts\demo_cloud_agent.ps1") -Raw
if ($agent -match 'GOOGLE_CLOUD_LOCATION = "([^"]+)"') {
    $loc = $Matches[1]
    if ($loc -eq "us-central1") { Ok "location $loc matches Firestore" }
    else { Bad "location is $loc but Firestore lives in us-central1" }
}

Step 5 "Receipts shipped"
foreach ($r in @("docs\_archive-benchmarks-2026-08-27\RECEIPT-RUN-2026-08-27.txt",
                 "crates\forge-envelope\surfaceledger\mtok_bench_receipt.txt")) {
    if (Test-Path (Join-Path $RepoRoot $r)) { Ok $r } else { Bad "MISSING $r - README cites it" }
}

if (-not $SkipCloud) {
    Step 6 "Google Cloud reachable"
    $proj = (gcloud config get-value project 2>$null).Trim()
    if (-not $proj) { Bad "no gcloud project set" } else { Ok "project $proj" }

    $buckets = gcloud storage buckets list --format="value(name)" 2>$null
    $inbox = "$proj-s13-inbox"
    if ($buckets -contains $inbox) { Ok "inbox bucket gs://$inbox exists" }
    else { Bad "inbox bucket gs://$inbox MISSING - run: gcloud storage buckets create gs://$inbox --location=us-central1" }

    $apis = gcloud services list --enabled --format="value(config.name)" 2>$null
    foreach ($a in @("aiplatform.googleapis.com", "firestore.googleapis.com", "storage.googleapis.com")) {
        if ($apis -contains $a) { Ok "$a enabled" } else { Bad "$a NOT enabled" }
    }
} else {
    Write-Host "`n[6] Google Cloud checks skipped (-SkipCloud)" -ForegroundColor DarkGray
}

Write-Host ""
if ($fail) {
    Write-Host "=== preflight RED: $fail check(s) failed. Do not push. ===" -ForegroundColor Red
    exit 1
}
Write-Host "=== preflight GREEN ===" -ForegroundColor Green
exit 0
