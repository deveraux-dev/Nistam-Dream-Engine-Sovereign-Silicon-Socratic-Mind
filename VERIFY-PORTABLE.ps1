# Updated VERIFY.ps1 — Portable Version

param(
    [string]$ProjectRoot = $PSScriptRoot
)

Write-Host "=== Continuous Soft-Routing + Hypersphere Blend Demo ===" -ForegroundColor Cyan
Write-Host "Project: $ProjectRoot" -ForegroundColor Gray
Write-Host ""

if (!(Test-Path $ProjectRoot)) {
    Write-Host "ERROR: Project root not found: $ProjectRoot" -ForegroundColor Red
    exit 1
}

Set-Location $ProjectRoot

# Test 1: MetaRouter soft-routing
Write-Host "Part 1: MetaRouter Soft-Routing" -ForegroundColor Yellow
$result = & cargo test -p forge-core-v3 metarouter::tests::route_soft --lib 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ PASS" -ForegroundColor Green
} else {
    Write-Host "✗ FAIL" -ForegroundColor Red
    exit 1
}

# Test 2: HierarchicalMoe soft evaluation
Write-Host "Part 1b: HierarchicalMoe Soft Evaluation" -ForegroundColor Yellow
$result = & cargo test -p forge-core-v3 hierarchical_moe::tests::evaluate_soft --lib 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ PASS" -ForegroundColor Green
} else {
    Write-Host "✗ FAIL" -ForegroundColor Red
    exit 1
}

# Test 3: RamusPrime hypersphere blending
Write-Host "Part 2: Hypersphere Blending" -ForegroundColor Yellow
$result = & cargo test -p forge-core-v3 ramus_prime::tests --lib 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ PASS" -ForegroundColor Green
} else {
    Write-Host "✗ FAIL" -ForegroundColor Red
    exit 1
}

# Full test suite
Write-Host "Full Test Suite" -ForegroundColor Yellow
$result = & cargo test -p forge-core-v3 --lib 2>&1 | Select-Object -Last 5
$m = [regex]::Match(($result -join "`n"), 'test result: ok\. (\d+) passed; (\d+) failed')
if ($m.Success -and [int]$m.Groups[2].Value -eq 0) {
    Write-Host "PASS: $($m.Groups[1].Value) forge-core-v3 lib tests passed, 0 failed" -ForegroundColor Green
} else {
    Write-Host "FAIL: forge-core-v3 lib suite not green" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "=== ✓ DEMO COMPLETE ===" -ForegroundColor Green
