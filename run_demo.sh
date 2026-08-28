#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "================================================================================"
echo "   NISTAM DREAM ENGINE & THE FORGE ENGINE — 180s COMPETITION DEMO"
echo "================================================================================"

python3 scripts/hands_off_demo_driver.py || python scripts/hands_off_demo_driver.py

echo "================================================================================"
echo "   DEMO COMPLETED SUCCESSFULLY WITH 0 FAILURES"
echo "================================================================================"
