$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path; powershell -ExecutionPolicy Bypass -File (Join-Path $ScriptDir "scripts\run_competition_tests_3min.ps1")
