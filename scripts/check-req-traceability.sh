#!/usr/bin/env bash
# REQ traceability check — strict per-test presence gate + quality linter.
#
# Enforces Testing Discipline T4:
#   Every test function carries a nearby `// REQ:` tag traceable to a requirement.
#
# Also rejects placeholder/non-contract anchors:
#   - `REQ: pre:`
#   - `REQ: autogen-*`
#
# Quality mode:
#   Flags REQ lines that look like prose summaries rather than stable IDs/principle anchors.
#   Set STRICT_REQ_QUALITY=1 to make quality violations fail the gate.

set -euo pipefail

echo "=== REQ Traceability Check (strict) ==="
echo ""

STRICT_REQ_QUALITY="${STRICT_REQ_QUALITY:-1}"

python3 - <<'PY'
import os
import re
import sys
import collections

ROOTS = ["crates", "mcp-servers"]
TEST_ATTR = re.compile(r"^\s*#\[(?:tokio::)?test(?:\s*\(.*\))?\]")
REQ_LINE = re.compile(r"REQ:")
PLACEHOLDER_REQ = re.compile(r"REQ:\s*(pre:|autogen-)")
PRINCIPLE_REF = re.compile(r"\bP(?:1[0-2]|[1-9])\b")
STRICT_QUALITY = os.environ.get("STRICT_REQ_QUALITY", "0") == "1"

per_crate = collections.defaultdict(lambda: {"tests": 0, "with_req": 0, "missing": 0, "quality": 0})
missing = []
placeholder = []
quality = []

def req_quality_ok(req_line: str) -> bool:
    if PRINCIPLE_REF.search(req_line):
        return True

    # Extract token immediately after REQ: up to em dash/space/end-comment.
    m = re.search(r"REQ:\s*([^—\s]+)", req_line)
    if not m:
        return False
    token = m.group(1).strip()

    # Stable ID heuristics: either has separators typical of IDs, or digits.
    has_sep = any(ch in token for ch in "-_.:")
    has_digit = any(ch.isdigit() for ch in token)
    return has_sep or has_digit

for root in ROOTS:
    if not os.path.isdir(root):
        continue
    for dp, _, fns in os.walk(root):
        for fn in fns:
            if not fn.endswith(".rs"):
                continue
            path = os.path.join(dp, fn).replace("\\", "/")
            parts = path.split("/")
            crate = parts[1] if parts[0] == "crates" else f"mcp-servers/{parts[1]}"

            with open(path, "r", encoding="utf-8", errors="ignore") as f:
                lines = f.read().splitlines()

            for i, line in enumerate(lines):
                if not TEST_ATTR.search(line):
                    continue

                per_crate[crate]["tests"] += 1
                prior = lines[max(0, i - 6): i + 1]
                req_lines = [l for l in prior if REQ_LINE.search(l)]

                if req_lines:
                    per_crate[crate]["with_req"] += 1
                    nearest_req = req_lines[-1]

                    if PLACEHOLDER_REQ.search(nearest_req):
                        placeholder.append((path, i + 1, nearest_req.strip()))

                    if not req_quality_ok(nearest_req):
                        per_crate[crate]["quality"] += 1
                        quality.append((path, i + 1, nearest_req.strip()))
                else:
                    per_crate[crate]["missing"] += 1
                    missing.append((path, i + 1))

print("crate,tests,with_req,missing,quality_flags,coverage")
for crate in sorted(per_crate):
    t = per_crate[crate]["tests"]
    w = per_crate[crate]["with_req"]
    m = per_crate[crate]["missing"]
    q = per_crate[crate]["quality"]
    pct = (w * 100.0 / t) if t else 0.0
    print(f"{crate},{t},{w},{m},{q},{pct:.1f}%")

print()
print(f"Total tests: {sum(v['tests'] for v in per_crate.values())}")
print(f"Missing REQ tags: {len(missing)}")
print(f"Placeholder REQ tags: {len(placeholder)}")
print(f"REQ quality flags: {len(quality)}")

if missing:
    print("\nERROR: tests missing nearby REQ tag:")
    for p, l in missing[:100]:
        print(f"  - {p}:{l}")

if placeholder:
    print("\nERROR: placeholder REQ tags found (replace with real requirement IDs):")
    for p, l, txt in placeholder[:100]:
        print(f"  - {p}:{l} :: {txt}")

if quality:
    level = "ERROR" if STRICT_QUALITY else "WARN"
    print(f"\n{level}: REQ tags lacking stable ID/principle anchor:")
    for p, l, txt in quality[:120]:
        print(f"  - {p}:{l} :: {txt}")

if missing or placeholder:
    sys.exit(1)

if STRICT_QUALITY and quality:
    sys.exit(1)

print("\nPASS: every test has a nearby non-placeholder REQ anchor.")
if quality and not STRICT_QUALITY:
    print("PASS (with warnings): enable STRICT_REQ_QUALITY=1 to enforce quality flags as hard failures.")
PY
