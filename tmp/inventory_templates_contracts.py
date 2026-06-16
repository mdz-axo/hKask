#!/usr/bin/env python3
"""Temporary helper: inventory hkask-templates contracts. Delete after use."""

import re
from pathlib import Path

src_files = sorted(Path("crates/hkask-templates/src").rglob("*.rs"))
test_files = sorted(Path("crates/hkask-templates/tests").rglob("*.rs"))


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


production = set()
tests = set()

for path in src_files:
    lines = path.read_text().splitlines()
    for i, line in enumerate(lines):
        m = re.search(r"REQ:\s*(P\d-[a-z0-9-]+)", line)
        if m:
            cid = m.group(1)
            if is_inside_tests_block(lines, i):
                tests.add(cid)
            else:
                production.add(cid)

for path in test_files:
    for line in path.read_text().splitlines():
        m = re.search(r"REQ:\s*(P\d-[a-z0-9-]+)", line)
        if m:
            tests.add(m.group(1))

print(f"Production unique IDs: {len(production)}")
print(f"Test unique IDs: {len(tests)}")
print(f"Total unique IDs: {len(production | tests)}")
print(
    f"Total occurrences: {sum(1 for p in src_files for line in p.read_text().splitlines() if re.search(r'REQ:\s*P\d-', line)) + sum(1 for p in test_files for line in p.read_text().splitlines() if re.search(r'REQ:\s*P\d-', line))}"
)
