#!/usr/bin/env python3
"""Temporary helper: generate FUNCTIONAL_SPECIFICATION.md section 3.4. Delete after use."""

import re
from pathlib import Path

src_files = sorted(Path("crates/hkask-inference/src").rglob("*.rs"))
test_files = sorted(Path("crates/hkask-inference/tests").rglob("*.rs"))


def is_inside_tests_block(lines, line_idx):
    depth = 0
    in_tests = False
    for i, line in enumerate(lines):
        if i == line_idx:
            return in_tests
        stripped = line.strip()
        if re.match(r"mod\s+tests\s*\{", stripped):
            in_tests = True
        if in_tests:
            if "{" in line:
                depth += line.count("{")
            if "}" in line:
                depth -= line.count("}")
            if depth == 0 and "}" in line:
                in_tests = False
    return in_tests


def extract_contracts(files, src_only=False):
    contracts = []
    for path in files:
        lines = path.read_text().splitlines()
        for i, line in enumerate(lines):
            m = re.search(r"REQ:\s*(P\d-[a-z0-9-]+)", line)
            if m:
                cid = m.group(1)
                # Skip doc comments that describe the format
                if "REQ: P{N}" in line or "REQ: pre/post" in line:
                    continue
                # Collect annotations immediately following
                annotations = []
                j = i + 1
                while j < len(lines):
                    ann_line = lines[j]
                    stripped = ann_line.strip()
                    if stripped.startswith("/// [") or stripped.startswith("// ["):
                        ann = stripped.lstrip("/").strip()
                        annotations.append(ann)
                        j += 1
                    else:
                        break
                # Determine target name
                target = None
                # Look ahead for pub fn / fn / pub async fn / async fn
                k = j
                while k < len(lines) and k < i + 15:
                    tline = lines[k].strip()
                    fn_m = re.match(r"(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)", tline)
                    if fn_m:
                        target = fn_m.group(3) + "()"
                        break
                    if tline.startswith("struct ") or tline.startswith("pub struct "):
                        target = tline.split()[1].split("{")[0].split("(")[0]
                        break
                    k += 1
                if not target:
                    target = "(inline)"
                contracts.append(
                    {
                        "id": cid,
                        "file": str(path.relative_to("crates/hkask-inference")),
                        "target": target,
                        "annotations": annotations,
                        "line": i + 1,
                    }
                )
    return contracts


# Extract production (outside mod tests in src) and tests (inside mod tests + tests/)
all_src_contracts = []
for path in src_files:
    lines = path.read_text().splitlines()
    for i, line in enumerate(lines):
        m = re.search(r"REQ:\s*(P\d-[a-z0-9-]+)", line)
        if m:
            cid = m.group(1)
            if "REQ: P{N}" in line or "REQ: pre/post" in line:
                continue
            is_test = is_inside_tests_block(lines, i)
            annotations = []
            j = i + 1
            while j < len(lines):
                ann_line = lines[j]
                stripped = ann_line.strip()
                if stripped.startswith("/// [") or stripped.startswith("// ["):
                    ann = stripped.lstrip("/").strip()
                    annotations.append(ann)
                    j += 1
                else:
                    break
            target = None
            k = j
            while k < len(lines) and k < i + 15:
                tline = lines[k].strip()
                fn_m = re.match(r"(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)", tline)
                if fn_m:
                    target = fn_m.group(3) + "()"
                    break
                if tline.startswith("struct ") or tline.startswith("pub struct "):
                    target = tline.split()[1].split("{")[0].split("(")[0]
                    break
                k += 1
            if not target:
                target = "(inline)"
            all_src_contracts.append(
                {
                    "id": cid,
                    "file": str(path.relative_to("crates/hkask-inference")),
                    "target": target,
                    "annotations": annotations,
                    "is_test": is_test,
                    "line": i + 1,
                }
            )

production = [c for c in all_src_contracts if not c["is_test"]]
tests = [c for c in all_src_contracts if c["is_test"]]

for path in test_files:
    lines = path.read_text().splitlines()
    for i, line in enumerate(lines):
        m = re.search(r"REQ:\s*(P\d-[a-z0-9-]+)", line)
        if m:
            cid = m.group(1)
            if "REQ: P{N}" in line or "REQ: pre/post" in line:
                continue
            annotations = []
            j = i + 1
            while j < len(lines):
                ann_line = lines[j]
                stripped = ann_line.strip()
                if stripped.startswith("/// [") or stripped.startswith("// ["):
                    ann = stripped.lstrip("/").strip()
                    annotations.append(ann)
                    j += 1
                else:
                    break
            target = None
            k = j
            while k < len(lines) and k < i + 15:
                tline = lines[k].strip()
                fn_m = re.match(r"(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)", tline)
                if fn_m:
                    target = fn_m.group(3) + "()"
                    break
                k += 1
            if not target:
                target = "(inline)"
            tests.append(
                {
                    "id": cid,
                    "file": str(path.relative_to("crates/hkask-inference")),
                    "target": target,
                    "annotations": annotations,
                    "is_test": True,
                    "line": i + 1,
                }
            )

# Deduplicate by ID, keeping first occurrence
seen_prod = {}
for c in production:
    if c["id"] not in seen_prod:
        seen_prod[c["id"]] = c
production = list(seen_prod.values())

seen_test = {}
for c in tests:
    if c["id"] not in seen_test:
        seen_test[c["id"]] = c
tests = list(seen_test.values())


def format_ann(anns):
    return "; ".join(anns) if anns else ""


# Generate markdown section
lines = []
lines.append("### 3.4 Inference (`hkask-inference`)")
lines.append("")
lines.append(
    "**Motivating Principles:** P9 (Homeostatic Self-Regulation) + P4 (Clear Boundaries — provider membrane)"
)
lines.append("**Crate:** `hkask-inference` | **Sources:** `src/*.rs`, `tests/*.rs`")
lines.append("")
lines.append(
    f"**{len(production)} production contracts** + **{len(tests)} test contracts**."
)
lines.append("")
lines.append("#### Production Contracts")
lines.append("")
lines.append("| FR# | Contract ID | Function | Principle Annotations |")
lines.append("|-----|------------|----------|---------------------|")
for idx, c in enumerate(production, 1):
    lines.append(
        f"| FR-I{idx:03d} | `{c['id']}` | `{c['target']}` | {format_ann(c['annotations'])} |"
    )
lines.append("")
lines.append("#### Test Contracts")
lines.append("")
lines.append("| FR# | Contract ID | Test Name |")
lines.append("|-----|------------|-----------|")
for idx, c in enumerate(tests, 1):
    lines.append(f"| FR-IT{idx:03d} | `{c['id']}` | `{c['target']}` |")
lines.append("")

output = "\n".join(lines)
print(output)

# Write to temp file for easy copying
Path("tmp/section_3_4_inference.md").write_text(output)
print("\nWrote tmp/section_3_4_inference.md")
print(f"Production: {len(production)}, Tests: {len(tests)}")
