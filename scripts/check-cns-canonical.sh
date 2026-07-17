#!/usr/bin/env bash
# CI gate: every `cns.*` tracing target must be a canonical CNS namespace
# (registered in `CANONICAL_NAMESPACES`, directly or via an ancestor).
#
# Rationale: the `cns.*` prefix is reserved for canonical CNS spans — the
# essential, ν-event-eligible, `SpanCategory`-categorized, loop-connected spans.
# Performative telemetry MUST use `hkask.*` targets, not `cns.*` (PRINCIPLES §9.1).
# Without this gate, performative logs borrowing the `cns.` prefix silently
# re-grow, recreating the registry/code drift (the 89 stray `cns.*` targets
# cleaned up in the cns-canonical sweep).
#
# Enabled in CI via `.github/workflows/ci.yml` invariants job.
# Run locally: `bash scripts/check-cns-canonical.sh`
#
# Exit codes:
#   0 — every `cns.*` tracing target in production code is canonical
#   1 — a non-canonical `cns.*` target was found (register it or move it to `hkask.*`)

set -euo pipefail
cd "$(dirname "$0")/.."

REGISTRY="crates/hkask-types/src/event.rs"

if [ ! -f "$REGISTRY" ]; then
  echo "FAIL: registry not found at $REGISTRY"
  exit 1
fi

# is_canonical <namespace>: returns 0 if the namespace (or any ancestor, by
# dot-trimming) is registered in CANONICAL_NAMESPACES, else 1. MIRRORS the
# `is_canonical` ancestor-matching rule in crates/hkask-types/src/event.rs —
# update both together.
# dot-trimming) is registered in CANONICAL_NAMESPACES, else 1. Mirrors the
# `is_canonical` ancestor-matching rule in crates/hkask-types/src/event.rs.
is_canonical() {
  local cur="$1"
  while [ -n "$cur" ]; do
    if grep -qF -- "\"$cur\"" "$REGISTRY" 2>/dev/null; then
      return 0
    fi
    case "$cur" in
      *.*) cur="${cur%.*}" ;;
      *) cur="" ;;
    esac
  done
  return 1
}

FAIL=0
# Collect every distinct `target: "cns.<...>"` in production code
# (exclude tests, examples, build artifacts).
mapfile -t TARGETS < <(
  grep -rhoE 'target: "cns\.[a-z0-9_.]+"' crates/ mcp-servers/ \
    --include='*.rs' \
    --exclude-dir=target \
    --exclude-dir=tests \
    --exclude-dir=examples \
    2>/dev/null \
    | sed -E 's/.*"(cns\.[a-z0-9_.]+)"/\1/' \
    | sort -u
)

for ns in "${TARGETS[@]}"; do
  if ! is_canonical "$ns"; then
    # Best-effort site listing (|| true so pipefail/set -e don't abort before
    # the helpful FAIL message is printed).
    sites=$( { grep -rnF -- "target: \"$ns\"" crates/ mcp-servers/ \
      --include='*.rs' --exclude-dir=target --exclude-dir=tests --exclude-dir=examples \
      2>/dev/null || true; } | head -5 )
    echo "  non-canonical cns.* target: $ns"
    echo "$sites" | sed 's/^/    /'
    FAIL=1
  fi
done

if [ "$FAIL" -eq 0 ]; then
  echo "OK: every cns.* tracing target is canonical (registered in CANONICAL_NAMESPACES)."
  exit 0
else
  echo ""
  echo "FAIL: non-canonical cns.* tracing targets found."
  echo "The cns.* prefix is reserved for canonical CNS spans (PRINCIPLES §9.1)."
  echo "Fix: either register the namespace in $REGISTRY (if it drives a cybernetic"
  echo "loop / becomes a ν-event) or retarget the span to hkask.* (if it is"
  echo "performative telemetry)."
  exit 1
fi