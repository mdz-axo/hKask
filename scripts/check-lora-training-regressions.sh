#!/usr/bin/env bash
# CI gate: enforce LoRA/QLoRA training-config regression library entries.
#
# Each regression in security/regressions/RR-NNNN.yaml with surface: training
# and status: enforced is checked against training config files. If the
# anti-pattern re-appears, the gate fails.
#
# detection.kind: grep regressions are mechanically enforced against
# training config files (LoraConfig, BitsAndBytesConfig, training scripts).
# detection.kind: runtime-assert regressions are acknowledged but not
# mechanically enforced — they require runtime instrumentation
# (gradient flow check, dtype check) that runs during training, not in CI.
# They are counted in the summary as "runtime-assert (deferred)" for
# visibility.
#
# RATCHETED: regressions with status: pending are warnings only (the
# anti-pattern is known but not yet fixed). Once the fix lands, flip
# status to enforced.
#
# Mirrors the ratcheted pattern of check-kali-regressions.sh.
#
# Exit codes:
#   0 — all enforced grep regressions pass (no anti-patterns found)
#   1 — an enforced grep regression's anti-pattern was found (re-introduction)
#
# Usage: bash scripts/check-lora-training-regressions.sh

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
runtime_assert_deferred=0

# Training config file patterns — where LoRA/QLoRA configs live.
# Matches: *.py (training scripts), *.yaml/*.yml (axolotl/TRL configs),
# *.json (LoraConfig JSON), LoraConfig, BitsAndBytesConfig.
TRAINING_CONFIG_INCLUDE=(
  --include='*.py'
  --include='*.yaml'
  --include='*.yml'
  --include='*.json'
  --include='*.toml'
)

for rr_file in "$REGRESSIONS_DIR"/RR-*.yaml; do
  [ -f "$rr_file" ] || continue

  # Extract surface — skip non-training regressions.
  rr_surface=$(grep -m1 '^surface:' "$rr_file" | sed 's/^surface:\s*//')
  if [ "$rr_surface" != "training" ]; then
    continue
  fi

  # Extract fields from YAML (lightweight grep-based parsing — no yq dependency).
  rr_id=$(grep -m1 '^id:' "$rr_file" | sed 's/^id:\s*//')
  rr_status=$(grep -m1 '^status:' "$rr_file" | sed 's/^status:\s*//')
  rr_kind=$(grep -m1 'kind:' "$rr_file" | sed 's/.*kind:\s*//')
  rr_pattern=$(grep -m1 'pattern:' "$rr_file" | sed 's/.*pattern:\s*//' | sed 's/^"\(.*\)"$/\1/')
  rr_title=$(grep -m1 '^title:' "$rr_file" | sed 's/^title:\s*//' | sed 's/^"\(.*\)"$/\1/')

  # Skip non-grep, non-runtime-assert kinds.
  if [ "$rr_kind" != "grep" ] && [ "$rr_kind" != "runtime-assert" ]; then
    continue
  fi

  # runtime-assert regressions require runtime instrumentation during
  # training (gradient flow check, dtype check, merge equivalence test).
  # Acknowledge for visibility but don't enforce in CI.
  if [ "$rr_kind" = "runtime-assert" ]; then
    if [ "$rr_status" = "enforced" ]; then
      runtime_assert_deferred=$((runtime_assert_deferred + 1))
      echo "deferred: $rr_id is runtime-assert (enforced but runtime check runs during training, not CI) — $rr_title"
    elif [ "$rr_status" = "pending" ]; then
      pending=$((pending + 1))
      echo "ratchet: $rr_id is pending (known anti-pattern, not yet enforced) — $rr_title"
    fi
    continue
  fi

  # grep-kind regressions are mechanically enforced against training config files.
  if [ "$rr_status" = "enforced" ]; then
    enforced=$((enforced + 1))
    # Check if the anti-pattern is present in training config files.
    matches=$(grep -rPn "$rr_pattern" . "${TRAINING_CONFIG_INCLUDE[@]}" \
      --exclude-dir=target --exclude-dir=.git --exclude-dir=node_modules \
      2>/dev/null || true)
    if [ -n "$matches" ]; then
      echo "::error::Training-config regression $rr_id violated: $rr_title"
      echo "  pattern: $rr_pattern"
      echo "$matches" | head -5 | sed 's/^/    /'
      violations=$((violations + 1))
    fi
  elif [ "$rr_status" = "pending" ]; then
    pending=$((pending + 1))
    echo "ratchet: $rr_id is pending (known anti-pattern, not yet enforced) — $rr_title"
  fi
done

echo "summary: $violations violation(s), $enforced enforced, $pending pending, $runtime_assert_deferred runtime-assert (deferred)"

[ "$violations" -eq 0 ]
