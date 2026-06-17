#!/usr/bin/env bash
# scripts/check-docs.sh — Convenience wrapper for the documentation link checker.
#
# Runs docs/ci/check-links.sh from the project root so local sweeps and CI use
# the same validation logic. The checker resolves internal markdown links
# strictly relative to the source file and scans the whole docs/ tree (including
# archive/).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"
exec bash docs/ci/check-links.sh "$@"
