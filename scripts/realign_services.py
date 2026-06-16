#!/usr/bin/env python3
"""Temporary mechanical realignment of hkask-services contract IDs.

This script is ad-hoc tooling. It must be deleted after use per project policy.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "crates" / "hkask-services" / "src"

# file stem -> (principle, domain short name)
FILE_MAP = {
    "archival": ("P5", "archival"),
    "backup/mod": ("P7", "backup"),
    "backup/config": ("P7", "backup"),
    "backup/serialization": ("P7", "backup"),
    "backup/scope": ("P7", "backup"),
    "backup/loop": ("P7", "backup"),
    "bundle": ("P5", "bundle"),
    "chat": ("P3", "chat"),
    "classify": ("P8", "classify"),
    "compose": ("P3", "compose"),
    "config": ("P7", "config"),
    "consolidation": ("P7", "consolidation"),
    "contacts": ("P1", "contacts"),
    "context": ("P3", "context"),
    "cns": ("P9", "cns"),
    "curator": ("P9", "curator"),
    "daemon_handler": ("P7", "daemon_handler"),
    "deletion_test": ("P5", "deletion_test"),
    "discover": ("P3", "discover"),
    "error": ("P4", "error"),
    "experience": ("P3", "experience"),
    "goal": ("P7", "goal"),
    "inference": ("P9", "inference"),
    "kata": ("P9", "kata"),
    "lifecycle": ("P7", "lifecycle"),
    "onboarding": ("P1", "onboarding"),
    "pods": ("P1", "pods"),
    "scheduler": ("P7", "scheduler"),
    "settings": ("P3", "settings"),
    "skill": ("P5", "skill"),
    "skills": ("P5", "skills"),
    "sovereignty": ("P1", "sovereignty"),
    "spec": ("P8", "spec"),
    "verification": ("P4", "verification"),
    "wallet": ("P9", "wallet"),
}

# P7 context gas calib items must stay P7 even though context is P3
KEEP_P7 = {
    "P5-svc-context-gas-calib-004",
    "P5-svc-context-gas-calib-005",
}


def transform_file(path: Path, principle: str, domain: str) -> str:
    text = path.read_text()
    original = text

    def repl(m: re.Match) -> str:
        full = m.group(1)
        if full in KEEP_P7:
            return f"REQ: {full.replace('P5-svc-context-', 'P7-svc-context-')}"

        # P5-svc-{domain}-svc-{op}-{number}
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-svc-([a-z-]+-\d+)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-svc-{number}[a-z]
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-svc-(\d+[a-z]?)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-svc-{number} -> {principle}-svc-{domain}-{number}
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-svc-(\d+)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-svc-{domain}-{number}
        mm = re.fullmatch(
            rf"P5-svc-{re.escape(domain)}-svc-{re.escape(domain)}-(\d+)", full
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-{domain}-{number}
        mm = re.fullmatch(
            rf"P5-svc-{re.escape(domain)}-{re.escape(domain)}-(\d+)", full
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-mds-{something}-svc-{number}
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-mds-[a-z]+-svc-(\d+)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-mds-{something}-{number}  (chat episodic/gas/memory)
        mm = re.fullmatch(
            rf"P5-svc-{re.escape(domain)}-mds-[a-z]+-([a-z]+-\d+)",
            full,
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-services-settings-{number}
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-services-settings-(\d+)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-p2-{op}
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-p2-(.+)", full)
        if mm:
            return f"REQ: P2-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-p4-{op}
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-p4-(.+)", full)
        if mm:
            return f"REQ: P4-svc-{domain}-{mm.group(1)}"

        # P5-svc-{domain}-must-{number}[-{suffix}]
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-must-(\d+(?:-\d+)?)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-must-{mm.group(1)}"

        # (Already-transformed) P{N}-svc-{domain}-mds-{something}-svc-{number}
        mm = re.fullmatch(
            rf"P\d+-svc-{re.escape(domain)}-mds-[a-z]+-svc-(\d+(?:-\d+)?)", full
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # (Already-transformed) P{N}-svc-{domain}-mds-{something}-{number}
        mm = re.fullmatch(
            rf"P\d+-svc-{re.escape(domain)}-mds-[a-z]+-{re.escape(domain)}-([a-z]+-\d+)",
            full,
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # Generic P5-svc-{domain}-<anything> -> {principle}-svc-{domain}-<anything>
        mm = re.fullmatch(rf"P5-svc-{re.escape(domain)}-(.+)", full)
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        return m.group(0)

    text = re.sub(r"REQ: ([A-Za-z0-9_-]+)", repl, text)

    # Second pass: strip leftover redundant substrings regardless of principle.
    def clean(m: re.Match) -> str:
        full = m.group(1)

        # P{N}-svc-{domain}-mds-{something}-svc-{number}
        mm = re.fullmatch(
            rf"P\d+-svc-{re.escape(domain)}-mds-[a-z]+-svc-(\d+(?:-\d+)?)", full
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        # P{N}-svc-{domain}-mds-{something}-{op}-{number}
        mm = re.fullmatch(
            rf"P\d+-svc-{re.escape(domain)}-mds-[a-z]+-([a-z]+-\d+)",
            full,
        )
        if mm:
            return f"REQ: {principle}-svc-{domain}-{mm.group(1)}"

        return m.group(0)

    text = re.sub(r"REQ: ([A-Za-z0-9_-]+)", clean, text)

    # Only write if changed
    if text == original:
        return ""
    path.write_text(text)
    return path.name


def main():
    changed = []
    for rel, (principle, domain) in FILE_MAP.items():
        path = ROOT / f"{rel}.rs"
        if not path.exists():
            print(f"MISSING: {path}")
            continue
        result = transform_file(path, principle, domain)
        if result:
            changed.append(result)
    print(f"Changed {len(changed)} files: {changed}")


if __name__ == "__main__":
    main()
