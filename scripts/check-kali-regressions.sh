#!/usr/bin/env bash
# CI gate: enforce security regression library entries.
#
# Each regression in security/regressions/RR-NNNN.yaml with status: enforced
# and detection.kind: grep is checked against the codebase. If the bug pattern
# re-appears, the gate fails.
#
# detection.kind: cns-span regressions (surface: runtime) are acknowledged but
# not mechanically enforced — they require runtime CNS span history infrastructure
# that is not yet implemented. They are counted in the summary as "cns-span
# (deferred)" for visibility.
#
# RATCHETED: regressions with status: pending are warnings only (the bug is
# known but not yet fixed). Once the fix lands, flip status to enforced.
#
# This mirrors the ratcheted pattern of check-mcp-tool-tests.sh.
#
# Exit codes:
#   0 — all enforced grep regressions pass (no bug patterns found)
#   1 — an enforced grep regression's bug pattern was found (re-introduction)
#
# Usage: bash scripts/check-kali-regressions.sh

set -euo pipefail
cd "$(dirname "$0")/.."

REGRESSIONS_DIR="security/regressions"

if [ ! -d "$REGRESSIONS_DIR" ]; then
  echo "OK: no regressions directory — nothing to check."
  exit 0
fi

violations=0
pending=0
enforced=0
cns_span_deferred=0

for rr_file in "$REGRESSIONS_DIR"/RR-*.yaml; do
  [ -f "$rr_file" ] || continue

  # Extract fields from YAML (lightweight grep-based parsing — no yq dependency)
  rr_id=$(grep -m1 '^id:' "$rr_file" | sed 's/^id:\s*//')
  rr_status=$(grep -m1 '^status:' "$rr_file" | sed 's/^status:\s*//')
  rr_kind=$(grep -m1 'kind:' "$rr_file" | sed 's/.*kind:\s*//')
  rr_pattern=$(grep -m1 'pattern:' "$rr_file" | sed 's/.*pattern:\s*//' | sed 's/^"\(.*\)"$/\1/')
  rr_include=$(grep -m1 'include:' "$rr_file" | sed 's/.*include:\s*//' | sed 's/^"\(.*\)"$/\1/')
  rr_title=$(grep -m1 '^title:' "$rr_file" | sed 's/^title:\s*//' | sed 's/^"\(.*\)"$/\1/')

  # Skip non-grep, non-cns-span kinds (e.g., cargo-test, skill-probe).
  if [ "$rr_kind" != "grep" ] && [ "$rr_kind" != "cns-span" ]; then
    continue
  fi

  # cns-span regressions require runtime CNS span history infrastructure
  # (not yet implemented). Acknowledge for visibility but don't enforce.
  if [ "$rr_kind" = "cns-span" ]; then
    if [ "$rr_status" = "enforced" ]; then
      cns_span_deferred=$((cns_span_deferred + 1))
      echo "deferred: $rr_id is cns-span (enforced but runtime check not yet implemented) — $rr_title"
    elif [ "$rr_status" = "pending" ]; then
      pending=$((pending + 1))
      echo "ratchet: $rr_id is pending (known bug, not yet enforced) — $rr_title"
    fi
    continue
  fi

  # grep-kind regressions are mechanically enforced against source files.
  if [ "$rr_status" = "enforced" ]; then
    enforced=$((enforced + 1))
    # Check if the bug pattern is present in the codebase.
    matches=$(grep -rPn "$rr_pattern" $rr_include 2>/dev/null || true)
    if [ -n "$matches" ]; then
      echo "::error::Security regression $rr_id violated: $rr_title"
      echo "  pattern: $rr_pattern"
      echo "$matches" | head -5 | sed 's/^/    /'
      violations=$((violations + 1))
    fi
  elif [ "$rr_status" = "pending" ]; then
    pending=$((pending + 1))
    echo "ratchet: $rr_id is pending (known bug, not yet enforced) — $rr_title"
  fi
done

echo "summary: $violations violation(s), $enforced enforced, $pending pending, $cns_span_deferred cns-span (deferred)"

[ "$violations" -eq 0 ]
