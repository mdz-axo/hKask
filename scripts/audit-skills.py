#!/usr/bin/env python3
"""
Ad-hoc dual-layer skill audit script.
Produces JSON report and markdown summary for the hKask skill corpus.
This is a stop-gap until the Rust harness in crates/hkask-services/src/skills.rs is complete.
"""

import json
import os
import re
import sys
from pathlib import Path
from typing import Any

import yaml

PROJECT_ROOT = Path(__file__).resolve().parent.parent
ZED_DIR = PROJECT_ROOT / ".agents" / "skills"
REGISTRY_DIR = PROJECT_ROOT / "registry" / "templates"
HLEXICON_PATH = PROJECT_ROOT / "registry" / "hlexicon" / "hlexicon-workspace.yaml"
WORKSPACE_VERSION = "0.27.0"

VALID_TEMPLATE_TYPES = {"WordAct", "KnowAct", "FlowDef"}
DDMVSS_ALIASES = {"Cognition", "Prompt", "Process", "cognition", "prompt", "process"}
VALID_VISIBILITY = {"Private", "Public", "Shared"}
ENERGY_CAP_RANGE = (
    1024,
    16384,
)  # per skill-manager R8; skill-maintenance uses 2048-8192


def load_hlexicon() -> set[str]:
    with open(HLEXICON_PATH, "r", encoding="utf-8") as f:
        data = yaml.safe_load(f)
    terms: set[str] = set()
    for domain in ("wordact", "knowact", "flowdef"):
        for entry in data.get("hlexicon", {}).get(domain, []):
            terms.add(entry["term"])
    return terms


def extract_frontmatter(content: str) -> dict[str, Any] | None:
    content = content.lstrip()
    if not content.startswith("[inference]"):
        return None
    # Strip the [inference] header line; the rest is YAML frontmatter ending at '---'
    after_header = content[len("[inference]") :].lstrip("\n")
    idx = after_header.find("\n---")
    if idx == -1:
        return None
    fm_text = after_header[:idx]
    if not fm_text.strip():
        return None
    try:
        return yaml.safe_load(fm_text)
    except yaml.YAMLError:
        return None


def parse_skill_md(path: Path) -> dict[str, Any]:
    content = path.read_text(encoding="utf-8")
    front: dict[str, Any] = {}
    if content.startswith("---"):
        m = re.search(r"^---\s*\n(.*?)\n---\s*\n", content, re.DOTALL)
        if m:
            try:
                front = yaml.safe_load(m.group(1)) or {}
            except yaml.YAMLError:
                front = {"_parse_error": True}
    return {
        "path": str(path.relative_to(PROJECT_ROOT)),
        "name": front.get("name", ""),
        "description": front.get("description", ""),
        "visibility": front.get("visibility", ""),
        "namespace": front.get("namespace", ""),
        "frontmatter": front,
        "has_frontmatter": content.startswith("---"),
    }


def parse_manifest_yaml(path: Path) -> dict[str, Any] | None:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f) or {}
    except yaml.YAMLError as e:
        return {"_parse_error": str(e)}


def audit_j2(path: Path, hlexicon: set[str]) -> dict[str, Any]:
    content = path.read_text(encoding="utf-8")
    fm = extract_frontmatter(content)
    result: dict[str, Any] = {
        "path": str(path.relative_to(PROJECT_ROOT)),
        "filename": path.name,
        "frontmatter_present": fm is not None,
        "template_type": None,
        "template_type_valid": False,
        "ddmvss_alias": False,
        "visibility": None,
        "visibility_valid": False,
        "visibility_misplaced": False,
        "energy_cap": None,
        "energy_cap_valid": False,
        "energy_cap_misplaced": False,
        "contract_input": None,
        "contract_output": None,
        "contract_present": False,
        "lexicon_terms": [],
        "unknown_lexicon_terms": [],
        "body_present": False,
        "version_mentions": re.findall(r"v0\.\d+\.\d+", content),
        "version_stale": False,
        "defects": [],
    }
    if fm is None:
        result["defects"].append("missing [inference] frontmatter")
        return result

    result["template_type"] = fm.get("template_type")
    result["visibility"] = fm.get("visibility")
    result["visibility_misplaced"] = False
    result["energy_cap"] = fm.get("energy_cap")
    result["energy_cap_misplaced"] = False
    result["lexicon_terms"] = fm.get("lexicon_terms", []) or []

    contract = fm.get("contract", {}) or {}
    result["contract_input"] = contract.get("input")
    result["contract_output"] = contract.get("output")
    result["contract_present"] = bool(
        result["contract_input"] is not None and result["contract_output"] is not None
    )

    # Existing templates nest energy_cap/visibility under contract; skill-manager spec says top-level.
    # Accept both, but note misplacement relative to spec.
    if result["energy_cap"] is None and isinstance(
        contract.get("energy_cap"), (int, str)
    ):
        result["energy_cap"] = contract.get("energy_cap")
        result["energy_cap_misplaced"] = True
    else:
        result["energy_cap_misplaced"] = False
    if result["visibility"] is None and contract.get("visibility"):
        result["visibility"] = contract.get("visibility")
        result["visibility_misplaced"] = True
    else:
        result["visibility_misplaced"] = False

    tt = result["template_type"]
    if tt in DDMVSS_ALIASES:
        result["ddmvss_alias"] = True
        result["template_type_valid"] = False
    elif tt in VALID_TEMPLATE_TYPES:
        result["template_type_valid"] = True

    vis = result["visibility"]
    result["visibility_valid"] = vis in VALID_VISIBILITY

    ec = result["energy_cap"]
    if isinstance(ec, int):
        result["energy_cap_valid"] = ENERGY_CAP_RANGE[0] <= ec <= ENERGY_CAP_RANGE[1]
    elif isinstance(ec, str) and ec.isdigit():
        result["energy_cap_valid"] = (
            ENERGY_CAP_RANGE[0] <= int(ec) <= ENERGY_CAP_RANGE[1]
        )

    result["unknown_lexicon_terms"] = [
        t for t in result["lexicon_terms"] if t not in hlexicon
    ]

    # body is everything after the first '---'
    sep = content.find("\n---")
    if sep != -1:
        result["body_present"] = len(content[sep + 4 :].strip()) > 0

    result["version_stale"] = any(
        v != f"v{WORKSPACE_VERSION}" for v in result["version_mentions"]
    )

    return result


def compute_health_score(skill_report: dict[str, Any]) -> float:
    zed = skill_report["zed_layer"]
    reg = skill_report["registry_layer"]

    # Base score: complete = 1.0, single-layer = 0.75, neither = 0.0
    if zed["present"] and reg["present"]:
        score = 1.0
    elif zed["present"] or reg["present"]:
        score = 0.75
    else:
        return 0.0

    if zed["present"]:
        if not zed["has_frontmatter"]:
            score -= 0.10
        if not zed["name_matches_dir"]:
            score -= 0.10
        if zed["description_length"] < 20:
            score -= 0.05
        if zed["description_length"] == 0:
            score -= 0.10

    if reg["present"]:
        if reg.get("manifest_parse_error"):
            score -= 0.20
        if not reg.get("manifest_present"):
            score -= 0.15
        elif not reg.get("templates_list_non_empty"):
            score -= 0.10
        for j2 in reg.get("j2_files", []):
            if not j2["frontmatter_present"]:
                score -= 0.10
            if j2["ddmvss_alias"]:
                score -= 0.15
            elif j2["template_type"] and not j2["template_type_valid"]:
                score -= 0.10
            if not j2["visibility_valid"]:
                score -= 0.10
            if not j2["contract_present"]:
                score -= 0.10
            if j2["unknown_lexicon_terms"]:
                score -= 0.03 * len(j2["unknown_lexicon_terms"])
            if not j2["energy_cap_valid"]:
                score -= 0.05
            if j2["version_stale"]:
                score -= 0.02

    # cross-layer
    if zed["present"] and reg["present"]:
        if zed["name"] and reg["crate_name"] and zed["name"] != reg["crate_name"]:
            score -= 0.10
        if not reg.get("manifest_present"):
            score -= 0.10  # extra penalty for complete skill missing manifest

    return max(0.0, min(1.0, score))


def status_from_score(score: float) -> str:
    if score >= 0.8:
        return "active"
    elif score >= 0.5:
        return "stale_warning"
    elif score >= 0.2:
        return "critical"
    else:
        return "recommend_deprecation"


def main() -> int:
    hlexicon = load_hlexicon()

    zed_skills = {
        p.name: parse_skill_md(p / "SKILL.md") for p in ZED_DIR.iterdir() if p.is_dir()
    }
    reg_skills_dirs = {p.name: p for p in REGISTRY_DIR.iterdir() if p.is_dir()}

    all_names = sorted(set(zed_skills.keys()) | set(reg_skills_dirs.keys()))

    reports = []
    for name in all_names:
        zed_info = zed_skills.get(name)
        reg_dir = reg_skills_dirs.get(name)

        zed_layer = {
            "present": zed_info is not None,
            "path": zed_info["path"] if zed_info else None,
            "name": zed_info["name"] if zed_info else None,
            "description": zed_info["description"] if zed_info else None,
            "description_length": len(zed_info["description"]) if zed_info else 0,
            "has_frontmatter": zed_info["has_frontmatter"] if zed_info else False,
            "name_matches_dir": (zed_info["name"] == name)
            if zed_info and zed_info["name"]
            else False,
        }

        reg_layer: dict[str, Any] = {"present": reg_dir is not None}
        if reg_dir is not None:
            manifest_path = reg_dir / "manifest.yaml"
            manifest = (
                parse_manifest_yaml(manifest_path) if manifest_path.exists() else None
            )
            reg_layer["manifest_present"] = manifest_path.exists()
            reg_layer["manifest_parse_error"] = bool(
                manifest and "_parse_error" in manifest
            )
            reg_layer["crate_name"] = (
                manifest.get("crate", {}).get("name")
                if manifest and not manifest.get("_parse_error")
                else None
            )
            reg_layer["crate_version"] = (
                manifest.get("crate", {}).get("version")
                if manifest and not manifest.get("_parse_error")
                else None
            )
            reg_layer["templates"] = (
                manifest.get("templates", [])
                if manifest and not manifest.get("_parse_error")
                else []
            )
            reg_layer["templates_list_non_empty"] = bool(reg_layer["templates"])
            reg_layer["manifest_hlexicon_terms"] = (
                manifest.get("hlexicon_terms", [])
                if manifest and not manifest.get("_parse_error")
                else []
            )
            reg_layer["unknown_manifest_hlexicon_terms"] = [
                t for t in reg_layer["manifest_hlexicon_terms"] if t not in hlexicon
            ]

            j2_files = []
            for j2_path in sorted(reg_dir.glob("*.j2")):
                j2_files.append(audit_j2(j2_path, hlexicon))
            reg_layer["j2_files"] = j2_files
            reg_layer["j2_count"] = len(j2_files)
            reg_layer["has_flowdef"] = any(
                j["template_type"] == "FlowDef" for j in j2_files
            )
            reg_layer["has_wordact"] = any(
                j["template_type"] == "WordAct" for j in j2_files
            )
            reg_layer["has_knowact"] = any(
                j["template_type"] == "KnowAct" for j in j2_files
            )

        report = {
            "skill_name": name,
            "zed_layer": zed_layer,
            "registry_layer": reg_layer,
        }
        report["health_score"] = compute_health_score(report)
        report["status"] = status_from_score(report["health_score"])

        # top defects
        defects = []
        if not zed_layer["present"]:
            defects.append("missing Zed layer (SKILL.md)")
        if not reg_layer["present"]:
            defects.append("missing registry layer")
        if reg_layer.get("present"):
            if not reg_layer["manifest_present"]:
                defects.append("missing manifest.yaml")
            if reg_layer["manifest_parse_error"]:
                defects.append("manifest.yaml parse error")
            for j2 in reg_layer.get("j2_files", []):
                if j2["ddmvss_alias"]:
                    defects.append(
                        f"{j2['filename']}: invalid DDMVSS alias template_type {j2['template_type']}"
                    )
                elif j2["template_type"] and not j2["template_type_valid"]:
                    defects.append(
                        f"{j2['filename']}: invalid template_type {j2['template_type']}"
                    )
                if not j2["frontmatter_present"]:
                    defects.append(f"{j2['filename']}: missing [inference] frontmatter")
                if not j2["visibility_valid"]:
                    defects.append(
                        f"{j2['filename']}: invalid visibility {j2['visibility']}"
                    )
                if not j2["contract_present"]:
                    defects.append(f"{j2['filename']}: missing/empty contract")
                if j2["unknown_lexicon_terms"]:
                    defects.append(
                        f"{j2['filename']}: unknown hlexicon terms {j2['unknown_lexicon_terms']}"
                    )
                if j2["energy_cap_misplaced"]:
                    defects.append(
                        f"{j2['filename']}: energy_cap nested under contract (spec says top-level)"
                    )
                if j2["visibility_misplaced"]:
                    defects.append(
                        f"{j2['filename']}: visibility nested under contract (spec says top-level)"
                    )
                if not j2["energy_cap_valid"]:
                    defects.append(
                        f"{j2['filename']}: energy_cap {j2['energy_cap']} out of range {ENERGY_CAP_RANGE}"
                    )
        if zed_layer["present"] and reg_layer.get("present"):
            if (
                zed_layer["name"]
                and reg_layer["crate_name"]
                and zed_layer["name"] != reg_layer["crate_name"]
            ):
                defects.append(
                    f"name mismatch: SKILL.md={zed_layer['name']} vs manifest={reg_layer['crate_name']}"
                )
        report["top_defects"] = defects[:3]

        reports.append(report)

    summary = {
        "workspace_version": WORKSPACE_VERSION,
        "total_skills": len(all_names),
        "complete_both_layers": sum(
            1
            for r in reports
            if r["zed_layer"]["present"] and r["registry_layer"]["present"]
        ),
        "zed_only": sum(
            1
            for r in reports
            if r["zed_layer"]["present"] and not r["registry_layer"]["present"]
        ),
        "registry_only": sum(
            1
            for r in reports
            if not r["zed_layer"]["present"] and r["registry_layer"]["present"]
        ),
        "active": sum(1 for r in reports if r["status"] == "active"),
        "stale_warning": sum(1 for r in reports if r["status"] == "stale_warning"),
        "critical": sum(1 for r in reports if r["status"] == "critical"),
        "recommend_deprecation": sum(
            1 for r in reports if r["status"] == "recommend_deprecation"
        ),
    }

    output = {
        "summary": summary,
        "skills": reports,
    }

    json_path = PROJECT_ROOT / "tmp" / "skill-audit.json"
    md_path = PROJECT_ROOT / "tmp" / "skill-audit.md"
    json_path.parent.mkdir(exist_ok=True)

    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2)

    with open(md_path, "w", encoding="utf-8") as f:
        f.write("# Dual-Layer Skill Audit Summary\n\n")
        f.write(f"Workspace version: {WORKSPACE_VERSION}\n\n")
        f.write("| Metric | Count |\n|--------|-------|\n")
        for k, v in summary.items():
            f.write(f"| {k} | {v} |\n")
        f.write("\n## Skill Details\n\n")
        for r in reports:
            zed = "✓" if r["zed_layer"]["present"] else "✗"
            reg = "✓" if r["registry_layer"]["present"] else "✗"
            f.write(
                f"### {r['skill_name']} — {r['status']} ({r['health_score']:.2f})\n"
            )
            f.write(f"- Zed: {zed}, Registry: {reg}\n")
            if r["registry_layer"].get("present"):
                f.write(
                    f"- Templates: {r['registry_layer'].get('j2_count', 0)} .j2 | WordAct={r['registry_layer'].get('has_wordact')} KnowAct={r['registry_layer'].get('has_knowact')} FlowDef={r['registry_layer'].get('has_flowdef')}\n"
                )
            for d in r["top_defects"]:
                f.write(f"- ⚠ {d}\n")
            f.write("\n")

    print(f"Wrote {json_path} and {md_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
