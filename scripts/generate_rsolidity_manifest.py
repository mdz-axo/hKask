#!/usr/bin/env python3
"""Generate machine-readable rSolidity contract manifest from FUNCTIONAL_SPECIFICATION.md."""

import json
import re
from pathlib import Path

SPEC = Path("docs/architecture/core/FUNCTIONAL_SPECIFICATION.md")
OUT = Path("data/rsolidity_contract_manifest.json")

text = SPEC.read_text()

sections = re.split(r"\n### \d+\.\d+ ", text)
records = []
for sec in sections:
    m = re.match(r"(.+?) \(", sec)
    domain = m.group(1).strip() if m else "unknown"
    for cm in re.finditer(r"`(P[0-9]+-([a-z0-9-]+))`", sec):
        cid = cm.group(1)
        parts = cid.split("-")
        principle = parts[0]
        records.append(
            {
                "contract_id": cid,
                "principle": principle,
                "domain": "-".join(parts[1:-1]) if len(parts) > 2 else parts[1],
                "operation": parts[-1],
            }
        )

seen = set()
unique = []
for r in records:
    if r["contract_id"] not in seen:
        seen.add(r["contract_id"])
        unique.append(r)

manifest = {
    "version": "0.27.0",
    "source": str(SPEC),
    "total_contracts": len(unique),
    "contracts": sorted(unique, key=lambda x: x["contract_id"]),
}

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Wrote {len(unique)} contracts to {OUT}")
