#!/usr/bin/env python3
"""
Bulk-calibrate .j2 frontmatter in registry/templates/.

Actions:
1. Move `energy_cap` and `visibility` from `contract:` to top-level under `[inference]`
   if they are misplaced (per skill-manager spec).
2. Update ℏKask version mentions to the current workspace version.
3. Report what changed.
"""

import re
from pathlib import Path

import yaml

PROJECT_ROOT = Path(__file__).resolve().parent.parent
REGISTRY_DIR = PROJECT_ROOT / "registry" / "templates"
WORKSPACE_VERSION = "0.27.0"


def parse_inference_block(content: str) -> tuple[dict, str, str] | None:
    """Return (frontmatter_dict, raw_frontmatter_text, body_after_separator) or None."""
    content = content.lstrip()
    if not content.startswith("[inference]"):
        return None
    after_header = content[len("[inference]") :].lstrip("\n")
    sep = after_header.find("\n---")
    if sep == -1:
        return None
    fm_text = after_header[:sep]
    body = content[content.find("\n---") + 1 :]
    try:
        fm = yaml.safe_load(fm_text)
    except yaml.YAMLError:
        return None
    if not isinstance(fm, dict):
        return None
    return fm, fm_text, body


def render_frontmatter(fm: dict) -> str:
    """Render frontmatter as YAML without the [inference] wrapper."""
    # Use safe_dump and then clean up the default style.
    text = yaml.safe_dump(
        fm, sort_keys=False, allow_unicode=True, default_flow_style=False
    )
    # Remove trailing newline that safe_dump adds
    return text.rstrip("\n")


def calibrate_file(path: Path) -> tuple[bool, list[str]]:
    content = path.read_text(encoding="utf-8")
    parsed = parse_inference_block(content)
    if parsed is None:
        return False, ["no [inference] frontmatter"]
    fm, _raw, body = parsed
    changes: list[str] = []

    contract = fm.get("contract", {})
    if isinstance(contract, dict):
        # Move energy_cap to top-level
        if "energy_cap" in contract and "energy_cap" not in fm:
            fm["energy_cap"] = contract.pop("energy_cap")
            changes.append("moved energy_cap from contract to top-level")
        elif "energy_cap" in contract and "energy_cap" in fm:
            # both present: keep top-level, remove nested
            contract.pop("energy_cap")
            changes.append("removed duplicate energy_cap from contract")

        # Move visibility to top-level
        if "visibility" in contract and "visibility" not in fm:
            fm["visibility"] = contract.pop("visibility")
            changes.append("moved visibility from contract to top-level")
        elif "visibility" in contract and "visibility" in fm:
            contract.pop("visibility")
            changes.append("removed duplicate visibility from contract")

        # Clean empty contract
        if contract == {}:
            fm.pop("contract", None)
            changes.append("removed empty contract block")

    # Update version strings in the body
    new_body = re.sub(r"ℏKask v0\.\d+\.\d+", f"ℏKask v{WORKSPACE_VERSION}", body)
    new_body = re.sub(r"hKask v0\.\d+\.\d+", f"hKask v{WORKSPACE_VERSION}", new_body)
    if new_body != body:
        changes.append(f"updated version to v{WORKSPACE_VERSION}")

    if not changes:
        return False, []

    new_fm_text = render_frontmatter(fm)
    new_content = f"[inference]\n{new_fm_text}\n{new_body}"
    path.write_text(new_content, encoding="utf-8")
    return True, changes


def main() -> int:
    changed = 0
    skipped = 0
    for j2_path in sorted(REGISTRY_DIR.rglob("*.j2")):
        modified, reasons = calibrate_file(j2_path)
        if modified:
            rel = j2_path.relative_to(PROJECT_ROOT)
            print(f"{rel}: {', '.join(reasons)}")
            changed += 1
        elif reasons == ["no [inference] frontmatter"]:
            print(f"{j2_path.relative_to(PROJECT_ROOT)}: skipped ({reasons[0]})")
            skipped += 1
    print(f"\nChanged {changed} files, skipped {skipped} files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
