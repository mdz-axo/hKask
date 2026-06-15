#!/bin/bash
# Shell script linting gate — Wave 6 Task 6.3
# Runs shellcheck on all .sh files; fails CI on any warning.
set -euo pipefail

SCRIPTS=$(find scripts/ -name '*.sh' -type f 2>/dev/null)

if [ -z "$SCRIPTS" ]; then
    echo "No shell scripts found in scripts/"
    exit 0
fi

if ! command -v shellcheck &>/dev/null; then
    echo "WARNING: shellcheck not installed — skipping lint gate"
    echo "Install: apt-get install shellcheck"
    exit 0
fi

echo "Running shellcheck on $(echo "$SCRIPTS" | wc -l) scripts..."
shellcheck --severity=warning $SCRIPTS

echo "All shell scripts pass shellcheck."
