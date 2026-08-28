# Hands-Off 180-Second Live Competition Demo Driver Wrapper
$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..")

Write-Host "Launching NISTAM & The Forge Engine Hands-Off Demo Driver..." -ForegroundColor Cyan
python (Join-Path $ScriptDir "hands_off_demo_driver.py")
