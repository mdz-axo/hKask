#!/usr/bin/env python3
"""Temporary helper: inventory hkask-templates contracts. Delete after use."""

import re
from pathlib import Path

src_files = sorted(Path("crates/hkask-templates/src").rglob("*.rs"))


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


contracts = []
for path in src_files:
    lines = path.read_text().splitlines()
    for i, line in enumerate(lines):
        m = re.search(r"REQ:\s*([A-Za-z0-9_.-]+)", line)
        if not m:
            continue
        old_id = m.group(1)
        # get rest of line after REQ: id for description
        rest = line.split(f"REQ: {old_id}", 1)[-1].strip()
        # Look ahead for function/struct name
        target = None
        for k in range(i + 1, min(i + 15, len(lines))):
            tline = lines[k].strip()
            fn_m = re.match(r"(pub\s+)?(async\s+)?fn\s+([A-Za-z0-9_]+)", tline)
            if fn_m:
                target = fn_m.group(3) + "()"
                break
            if tline.startswith("struct ") or tline.startswith("pub struct "):
                target = tline.split()[1].split("{")[0].split("(")[0]
                break
        if not target:
            target = "(inline)"
        contracts.append(
            {
                "file": str(path.relative_to("crates/hkask-templates")),
                "line": i + 1,
                "old_id": old_id,
                "target": target,
                "rest": rest,
                "is_test": is_inside_tests_block(lines, i),
            }
        )

print(f"Total REQ occurrences: {len(contracts)}")
print(f"Unique old IDs: {len(set(c['old_id'] for c in contracts))}")
print()
for c in contracts:
    kind = "TEST" if c["is_test"] else "PROD"
    print(f"{c['file']}:{c['line']}\t{kind}\t{c['old_id']}\t{c['target']}\t{c['rest']}")
