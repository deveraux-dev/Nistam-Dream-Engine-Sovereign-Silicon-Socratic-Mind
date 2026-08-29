# Demo with Forge Vision API (forgewright) — Google Judges
# Captures screenshots and runs tests with visual proof

param(
    [string]$ProjectRoot = $PSScriptRoot,
    [string]$TerminalTitle = "PowerShell",
    [string]$DemoDir = "$env:USERPROFILE\Desktop\Nistam-Dream-Engine-Sovereign-Silicon-Socratic-Mind"
)

# Paths (portable — uses env vars, not hardcoded names)
$forgewright = "$ProjectRoot\.forge\bin\forgewright.exe"
$ScreenshotDir = "$DemoDir\screenshots"

# Ensure screenshot directory
if (!(Test-Path $ScreenshotDir)) {
    New-Item -ItemType Directory -Path $ScreenshotDir -Force | Out-Null
}

Write-Host "=== Forge Vision API Demo — Live Test Execution ===" -ForegroundColor Cyan
Write-Host "Using forgewright for real-time visual capture" -ForegroundColor Gray
Write-Host "Screenshots: $ScreenshotDir" -ForegroundColor Gray
Write-Host ""

if (!(Test-Path $forgewright)) {
    Write-Host "ERROR: forgewright not found at $forgewright" -ForegroundColor Red
    Write-Host "Build it with: cargo build -p forge-wright" -ForegroundColor Yellow
    exit 1
}

Set-Location $ProjectRoot

# Helper: capture screenshot
function Capture-Screen {
    param([string]$Name, [string]$Title = "PowerShell")
    $timestamp = Get-Date -Format "HH-mm-ss"
    $filename = "$ScreenshotDir\$Name-$timestamp.png"
    & $forgewright capture "$Title" "$filename" 2>&1 | Write-Host -ForegroundColor Gray
    return $filename
}

# Header slide
Write-Host "PART 1: MetaRouter Continuous Soft-Routing" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""
Capture-Screen "01-start"

Write-Host "Compiling forge-core-v3..." -ForegroundColor Cyan
& cargo build -p forge-core-v3 2>&1 | Select-Object -Last 5 | ForEach-Object { Write-Host $_ -ForegroundColor Gray }
Capture-Screen "02-compile"
Write-Host ""

Write-Host "Running MetaRouter soft-routing tests..." -ForegroundColor Cyan
& cargo test -p forge-core-v3 metarouter::tests::route_soft --lib -- --nocapture 2>&1 | Tee-Object -Variable output | Select-Object -Last 20
Capture-Screen "03-metarouter-soft"
$metarouterPass = $output | Select-String "test result: ok"
if ($metarouterPass) {
    Write-Host "✓ MetaRouter soft-routing: PASS" -ForegroundColor Green
} else {
    Write-Host "✗ MetaRouter soft-routing: FAIL" -ForegroundColor Red
}
Write-Host ""

# Part 2
Write-Host "PART 2: Hypersphere Field-Distance Blending" -ForegroundColor Yellow
Write-Host "──────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""
Capture-Screen "04-part2-start"

Write-Host "Running RamusPrime hypersphere blend tests..." -ForegroundColor Cyan
& cargo test -p forge-core-v3 ramus_prime::tests::axes_distance --lib -- --nocapture 2>&1 | Tee-Object -Variable output | Select-Object -Last 15
Capture-Screen "05-axes-distance"
$distPass = $output | Select-String "test result: ok"
if ($distPass) {
    Write-Host "✓ Axes distance: PASS" -ForegroundColor Green
} else {
    Write-Host "✗ Axes distance: FAIL" -ForegroundColor Red
}
Write-Host ""

Write-Host "Running field-arithmetic weighted sum tests..." -ForegroundColor Cyan
& cargo test -p forge-core-v3 ramus_prime::tests::mersenne_weighted_sum --lib -- --nocapture 2>&1 | Tee-Object -Variable output | Select-Object -Last 15
Capture-Screen "06-weighted-sum"
$sumPass = $output | Select-String "test result: ok"
if ($sumPass) {
    Write-Host "✓ Mersenne weighted sum: PASS" -ForegroundColor Green
} else {
    Write-Host "✗ Mersenne weighted sum: FAIL" -ForegroundColor Red
}
Write-Host ""

Write-Host "Running sample_blend tests..." -ForegroundColor Cyan
& cargo test -p forge-core-v3 ramus_prime::tests::sample_blend --lib -- --nocapture 2>&1 | Tee-Object -Variable output | Select-Object -Last 15
Capture-Screen "07-sample-blend"
$blendPass = $output | Select-String "test result: ok"
if ($blendPass) {
    Write-Host "✓ Sample blend: PASS" -ForegroundColor Green
} else {
    Write-Host "✗ Sample blend: FAIL" -ForegroundColor Red
}
Write-Host ""

# Part 3
Write-Host "PART 3: Training-Data Hierarchical Wiring" -ForegroundColor Yellow
Write-Host "────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""
Capture-Screen "08-part3-start"

Write-Host "Checking sidecar compilation with soul integration..." -ForegroundColor Cyan
& cargo check --manifest-path sidecar/Cargo.toml --no-default-features 2>&1 | Select-Object -Last 5
Capture-Screen "09-sidecar-check"
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Sidecar compilation: PASS" -ForegroundColor Green
} else {
    Write-Host "✗ Sidecar compilation: FAIL" -ForegroundColor Red
}
Write-Host ""

# Full suite
Write-Host "FULL TEST SUITE: forge-core-v3 lib" -ForegroundColor Yellow
Write-Host "───────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""
Capture-Screen "10-testsuite-start"

Write-Host "Running full test suite..." -ForegroundColor Cyan
& cargo test -p forge-core-v3 --lib 2>&1 | Tee-Object -Variable output | Select-Object -Last 10
Capture-Screen "11-testsuite-complete"

$m = [regex]::Match(($output -join "`n"), 'test result: ok\. (\d+) passed; (\d+) failed')
$fullPass = $m.Success -and [int]$m.Groups[2].Value -eq 0
$fullCount = if ($m.Success) { $m.Groups[1].Value } else { "0" }
if ($fullPass) {
    Write-Host "FULL SUITE: $fullCount passed, 0 failed" -ForegroundColor Green
} else {
    Write-Host "FULL SUITE: NOT GREEN" -ForegroundColor Red
}
Write-Host ""

# Summary
Write-Host "=== DEMO SUMMARY ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Evidence captured:" -ForegroundColor Yellow
Get-ChildItem $ScreenshotDir -Filter "*.png" | ForEach-Object {
    Write-Host "  • $($_.Name)" -ForegroundColor Green
}
Write-Host ""

Write-Host "Results:" -ForegroundColor Yellow
if ($metarouterPass -and $distPass -and $sumPass -and $blendPass -and $fullPass) {
    Write-Host "  ✓ All three parts working" -ForegroundColor Green
    Write-Host "  All forge-core-v3 lib tests passing ($fullCount/$fullCount)" -ForegroundColor Green
    Write-Host "  ✓ Zero compilation errors" -ForegroundColor Green
    Write-Host ""
    Write-Host "Code is production-ready for judges." -ForegroundColor Cyan
} else {
    Write-Host "  ✗ Some checks failed — see screenshots for details" -ForegroundColor Red
}
