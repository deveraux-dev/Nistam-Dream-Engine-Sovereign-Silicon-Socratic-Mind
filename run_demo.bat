@echo off
setlocal
cd /d "%~dp0"
title NISTAM & The Forge Engine — Devpost Live Demo
echo ================================================================================
echo    NISTAM DREAM ENGINE & THE FORGE ENGINE — 180s COMPETITION DEMO
echo ================================================================================
python scripts\hands_off_demo_driver.py
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Demo exited with code %ERRORLEVEL%.
    pause
    exit /b %ERRORLEVEL%
)
echo.
echo ================================================================================
echo    Demo exited 0. Read the banner above for which half was proven.
echo ================================================================================
pause
