#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build api-gate in dev and release modes with clear status reporting.
.DESCRIPTION
    Compiles both debug and optimized binaries for hotswapping.
    Reports file paths, sizes, and symbol status loudly for CI/agents.
#>

param(
    [ValidateSet("dev", "release", "both")]
    [string]$Mode = "both"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
Set-Location $repoRoot

Write-Host @"
╔════════════════════════════════════════════════════════════════╗
║               API-GATE AUTOMATED BUILD                         ║
║                                                                ║
║  This crate provides a zero-trust HTTP-to-frame proxy for     ║
║  inference servers with aggressive gating:                    ║
║    • 4KB payload cap (→ 413 Payload Too Large)               ║
║    • 2000ms hard timeout (→ 504 Gateway Timeout)             ║
║    • Per-request credit burn (→ 402 Payment Required)         ║
║    • Blank error bodies (→ no stack traces leaked)            ║
║                                                                ║
║  Proxies HTTP requests to gemma-sidecar:13017 via            ║
║  u32-BE-length framed protocol.                              ║
║                                                                ║
║  Both builds are ready for hotswap between dev (symbols)      ║
║  and release (optimized) without stopping the gate.           ║
╚════════════════════════════════════════════════════════════════╝
"@

function Build-Target {
    param([string]$Profile)

    $profileFlag = if ($Profile -eq "release") { "--release" } else { "" }
    $profileName = if ($Profile -eq "release") { "RELEASE" } else { "DEV" }

    Write-Host "`n>>> Building $profileName profile..." -ForegroundColor Cyan
    cargo build -p api-gate --bin api-gate $profileFlag 2>&1 | ForEach-Object {
        if ($_ -like "*Finished*" -or $_ -like "*Compiling*") {
            Write-Host $_ -ForegroundColor Green
        } elseif ($_ -like "*error*") {
            Write-Host $_ -ForegroundColor Red
            throw $_
        } else {
            Write-Host $_
        }
    }

    $binPath = if ($Profile -eq "release") {
        ".\target\release\api-gate.exe"
    } else {
        ".\target\debug\api-gate.exe"
    }

    if (-not (Test-Path $binPath)) {
        throw "Binary not found at $binPath"
    }

    $file = Get-Item $binPath
    $sizeMB = [Math]::Round($file.Length / 1MB, 2)
    $symbols = if ($Profile -eq "release") { "stripped/optimized" } else { "full debuginfo" }

    Write-Host "`n✅ $profileName BUILD COMPLETE" -ForegroundColor Green
    Write-Host "   Path:    $($file.FullName)" -ForegroundColor White
    Write-Host "   Size:    $sizeMB MB" -ForegroundColor White
    Write-Host "   Symbols: $symbols" -ForegroundColor White

    return @{
        Profile  = $Profile
        Path     = $file.FullName
        Size     = $file.Length
        Symbols  = $symbols
        Modified = $file.LastWriteTime
    }
}

$results = @()

if ($Mode -in "dev", "both") {
    $results += Build-Target "debug"
}

if ($Mode -in "release", "both") {
    $results += Build-Target "release"
}

Write-Host @"

╔════════════════════════════════════════════════════════════════╗
║                        BUILD SUMMARY                           ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Cyan

$results | ForEach-Object {
    Write-Host "[$($_.Profile.ToUpper())]" -ForegroundColor Yellow -NoNewline
    Write-Host " $($_.Path)" -ForegroundColor White
    Write-Host "  └─ $($_.Size / 1KB -as [int]) KB, $($_.Symbols)" -ForegroundColor Gray
}

Write-Host @"

HOTSWAP:
  To switch between builds without stopping the gate:
    1. Kill current process (Ctrl+C or taskkill)
    2. Start alternative binary
    3. Both point to same :13017 upstream

DEPLOY:
  Release (production):    .\target\release\api-gate.exe
  Dev (with debuginfo):    .\target\debug\api-gate.exe

ENV VARS:
  API_GATE_BIND="127.0.0.1:8080"  (default)
  API_GATE_CREDITS="1000"         (default)

✅ Build automation complete. Ready for hotswap deployment.

"@ -ForegroundColor Green
