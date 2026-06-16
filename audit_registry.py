#!/usr/bin/env python3
"""Audit hKask registry template layer."""

import json
import os
import re
import sys
from pathlib import Path

import jinja2
import yaml

ROOT = Path("/home/mdz-axolotl/Clones/hKask")
REGISTRY = ROOT / "registry" / "templates"
SKILLS = ROOT / ".agents" / "skills"
HLEXICON = ROOT / "registry" / "hlexicon" / "hlexicon-workspace.yaml"

VALID_TYPES = {"WordAct", "KnowAct", "FlowDef"}
INVALID_TYPES = {"Cognition", "Prompt", "Process"}
VALID_VISIBILITY = {"Private", "Public", "Shared"}
ENERGY_RANGE = (2048, 8192)


def load_hlexicon():
    with open(HLEXICON) as f:
        data = yaml.safe_load(f)
    terms = set()
    for cat in data.get("hlexicon", {}).values():
        for entry in cat:
            terms.add(entry["term"])
    return terms


HLEXICON_TERMS = load_hlexicon()


def parse_j2_frontmatter(text: str, path: Path):
    """Parse [inference] ... --- frontmatter from a .j2 file."""
    if not text.startswith("[inference]"):
        return None, "No [inference] frontmatter"
    parts = text.split("---", 1)
    if len(parts) != 2:
        return None, "No --- separator after frontmatter"
    fm_text = parts[0].replace("[inference]", "").strip()
    body = parts[1]
    try:
        fm = yaml.safe_load(fm_text)
    except yaml.YAMLError as e:
        return None, f"YAML parse error in frontmatter: {e}"
    return fm, body


def check_jinja2_syntax(body: str, path: Path):
    env = jinja2.Environment()
    try:
        env.parse(body)
        return None
    except jinja2.exceptions.TemplateSyntaxError as e:
        return f"Jinja2 syntax error at line {e.lineno}: {e.message}"


def find_liquid_filters(body: str):
    """Find Liquid-style filters like | join: '...' or | split: '...'"""
    # Match filter name followed by colon before parentheses/arg
    pattern = re.compile(r"\|\s*(\w+)\s*:\s*['\"]")
    matches = pattern.findall(body)
    return matches


def find_shell_command_prompts(body: str):
    """Detect text asking an LLM to execute shell commands as if it could."""
    # Heuristic phrases
    phrases = [
        r"run\s+(?:cargo|npm|yarn|pip|poetry|make|cmake|go|rustc|python|pytest|cargo\s+check|cargo\s+test|cargo\s+build)",
        r"execute\s+(?:the\s+)?(?:command|script|shell)",
        r"run\s+(?:the\s+)?(?:command|script|test|check)",
        r"perform\s+a\s+(?:cargo|shell|command)",
    ]
    found = []
    for p in phrases:
        for m in re.finditer(p, body, re.IGNORECASE):
            # Get surrounding context
            start = max(0, m.start() - 40)
            end = min(len(body), m.end() + 40)
            found.append(body[start:end].replace("\n", " "))
    return found[:5]  # limit


def extract_contract_fields(fm):
    """Return set of input/output field names from frontmatter contract."""
    contract = fm.get("contract", {})
    inputs = set(contract.get("input", {}).keys())
    outputs = set(contract.get("output", {}).keys())
    return inputs, outputs


def extract_used_vars(body: str):
    """Very rough extraction of top-level variable names used in {{ var.field }} or {% if var %}."""
    names = set()
    for m in re.finditer(r"\{\{\s*([A-Za-z_][A-Za-z0-9_]*)\b", body):
        names.add(m.group(1))
    for m in re.finditer(r"\{%\s*if\s+([A-Za-z_][A-Za-z0-9_]*)", body):
        names.add(m.group(1))
    for m in re.finditer(r"\{%\s*for\s+\w+\s+in\s+([A-Za-z_][A-Za-z0-9_]*)\b", body):
        names.add(m.group(1))
    return names


def check_manifest(path: Path):
    rel = path.relative_to(ROOT)
    skill_name = path.parent.name
    result = {
        "skill": skill_name,
        "manifest_path": str(rel),
        "templates": [],
        "errors": [],
    }
    try:
        with open(path) as f:
            data = yaml.safe_load(f)
    except yaml.YAMLError as e:
        result["errors"].append(f"Manifest YAML parse error: {e}")
        return result

    crate = data.get("crate", {})
    result["crate_name"] = crate.get("name")
    result["version"] = crate.get("version")
    result["description"] = crate.get("description", "").strip()

    templates = data.get("templates", [])
    manifest_template_paths = set()
    for t in templates:
        tid = t.get("id", "?")
        tpath = t.get("path", "")
        ttype = t.get("type", "")
        manifest_template_paths.add(tpath)
        entry = {
            "id": tid,
            "path": tpath,
            "type": ttype,
            "lexicon_terms": t.get("lexicon_terms", []),
            "exists": False,
            "flags": [],
        }
        if ttype in INVALID_TYPES:
            entry["flags"].append(
                f"manifest declares invalid type '{ttype}' (must be WordAct/KnowAct/FlowDef)"
            )
        if ttype not in VALID_TYPES and ttype not in INVALID_TYPES:
            entry["flags"].append(f"manifest declares unknown type '{ttype}'")
        full = path.parent / tpath
        if not full.exists():
            entry["flags"].append(f"manifest path references missing file: {tpath}")
        else:
            entry["exists"] = True
        result["templates"].append(entry)

    result["hlexicon_terms"] = data.get("hlexicon_terms", [])
    for term in result["hlexicon_terms"]:
        if term not in HLEXICON_TERMS:
            result["errors"].append(f"manifest hlexicon term not in workspace: {term}")

    return result, manifest_template_paths


def check_j2(path: Path, manifest_entry=None):
    rel = path.relative_to(ROOT)
    skill_name = path.parent.name
    result = {
        "skill": skill_name,
        "j2_path": str(rel),
        "frontmatter": {},
        "errors": [],
        "flags": [],
    }
    try:
        with open(path) as f:
            text = f.read()
    except Exception as e:
        result["errors"].append(f"Read error: {e}")
        return result

    fm, body_or_err = parse_j2_frontmatter(text, path)
    if fm is None:
        result["errors"].append(body_or_err)
        # Still try to parse whole file as Jinja2 to find syntax errors
        body = text
    else:
        body = body_or_err
        result["frontmatter"] = fm
        ttype = fm.get("template_type")
        if ttype in INVALID_TYPES:
            result["flags"].append(
                f"frontmatter template_type invalid: '{ttype}' (must be WordAct/KnowAct/FlowDef)"
            )
        if ttype == "FlowDef":
            result["flags"].append(
                "FlowDef declared on .j2 file (runtime says FlowDef = YAML .yaml)"
            )
        if ttype not in VALID_TYPES and ttype not in INVALID_TYPES:
            result["flags"].append(f"frontmatter template_type unknown: '{ttype}'")

        contract = fm.get("contract", {})
        energy = contract.get("energy_cap")
        if energy is not None:
            if energy < ENERGY_RANGE[0] or energy > ENERGY_RANGE[1]:
                result["flags"].append(
                    f"energy_cap {energy} outside [{ENERGY_RANGE[0]}, {ENERGY_RANGE[1]}]"
                )
        else:
            result["flags"].append("energy_cap missing from contract")

        visibility = contract.get("visibility")
        if visibility is not None:
            if visibility not in VALID_VISIBILITY:
                result["flags"].append(
                    f"visibility '{visibility}' not in {VALID_VISIBILITY}"
                )
        else:
            result["flags"].append("visibility missing from contract")

        lexicon = fm.get("lexicon_terms", [])
        for term in lexicon:
            if term not in HLEXICON_TERMS:
                result["flags"].append(
                    f"frontmatter hlexicon term not in workspace: {term}"
                )

        # Contract fields vs body usage
        if isinstance(contract, dict):
            inputs, outputs = extract_contract_fields(fm)
            used = extract_used_vars(body)
            for inp in inputs:
                if inp not in used and inp != "_":
                    result["flags"].append(
                        f"contract input '{inp}' not obviously used in template body"
                    )
            # outputs can't be directly checked; flag if outputs list names never appear in body as instructions
            # We'll only flag inputs for now

    # Jinja2 syntax check
    syntax_err = check_jinja2_syntax(body, path)
    if syntax_err:
        result["errors"].append(syntax_err)

    # Liquid filters
    liq = find_liquid_filters(body)
    if liq:
        result["flags"].append(f"Liquid-style filter syntax detected: {set(liq)}")

    # Shell command prompts
    shell = find_shell_command_prompts(body)
    if shell:
        result["flags"].append(
            f"Template may ask LLM to execute shell commands: {shell[:3]}"
        )

    return result


def score_skill(skill_name, manifest_result, j2_results, has_zed):
    score = 1.0
    if not has_zed:
        score -= 0.25
    if manifest_result:
        for e in manifest_result["errors"]:
            if "hlexicon" in e:
                score -= 0.05
            else:
                score -= 0.15
        for t in manifest_result["templates"]:
            if t["flags"]:
                score -= 0.10 * len(
                    [
                        f
                        for f in t["flags"]
                        if "invalid type" in f or "unknown type" in f
                    ]
                )
                score -= 0.10 * len([f for f in t["flags"] if "missing file" in f])
    for j in j2_results:
        for e in j["errors"]:
            if "Jinja2 syntax" in e:
                score -= 0.15
            else:
                score -= 0.10
        for f in j["flags"]:
            if "template_type invalid" in f or "FlowDef declared" in f:
                score -= 0.15
            elif "energy_cap" in f:
                score -= 0.05
            elif "visibility" in f:
                score -= 0.10
            elif "hlexicon term" in f:
                score -= 0.05
            elif "Liquid" in f:
                score -= 0.10
            elif "shell" in f:
                score -= 0.05
            else:
                score -= 0.05
    return max(0.0, min(1.0, score))


def main():
    skills_in_registry = set()
    manifest_results = {}
    all_template_paths = {}  # skill -> set of paths declared in manifest

    for manifest in sorted(REGISTRY.rglob("manifest.yaml")):
        rel_dir = manifest.parent.relative_to(REGISTRY)
        skill_name = str(rel_dir)
        skills_in_registry.add(skill_name)
        mres, paths = check_manifest(manifest)
        manifest_results[skill_name] = mres
        all_template_paths[skill_name] = paths

    j2_results_by_skill = {}
    for j2_path in sorted(REGISTRY.rglob("*.j2")):
        rel_dir = j2_path.parent.relative_to(REGISTRY)
        skill_name = str(rel_dir)
        skills_in_registry.add(skill_name)
        j2res = check_j2(j2_path)
        j2_results_by_skill.setdefault(skill_name, []).append(j2res)

    # Also include skills that have only .j2 and no manifest
    all_skills = sorted(skills_in_registry)

    report = []
    for skill in all_skills:
        has_manifest = skill in manifest_results
        has_j2 = skill in j2_results_by_skill
        has_zed = (SKILLS / skill / "SKILL.md").exists()

        mres = manifest_results.get(skill)
        j2res = j2_results_by_skill.get(skill, [])
        score = score_skill(skill, mres, j2res, has_zed)

        report.append(
            {
                "skill": skill,
                "has_manifest": has_manifest,
                "has_j2": has_j2,
                "has_zed_counterpart": has_zed,
                "manifest": mres,
                "j2_templates": j2res,
                "score": round(score, 2),
            }
        )

    # Print summary
    print("# Registry Template Layer Audit\n")
    total_j2 = sum(len(r["j2_templates"]) for r in report)
    total_manifests = sum(1 for r in report if r["has_manifest"])
    print(f"Skills in registry: {len(report)}")
    print(f"Manifests: {total_manifests}")
    print(f".j2 templates: {total_j2}")
    print(
        f"With Zed SKILL.md counterpart: {sum(1 for r in report if r['has_zed_counterpart'])}\n"
    )

    for r in report:
        status = (
            "active"
            if r["score"] >= 0.8
            else (
                "stale"
                if r["score"] >= 0.5
                else ("critical" if r["score"] >= 0.2 else "recommend-deprecation")
            )
        )
        icon = "✓" if status == "active" else ("⚠" if status == "stale" else "✗")
        print(f"{icon} {r['skill']}  {status} ({r['score']})")
        if r["has_manifest"]:
            m = r["manifest"]
            print(
                f"   Manifest: crate={m.get('crate_name')} version={m.get('version')}"
            )
            print(f"   Description: {m.get('description', '')[:100]}")
            for t in m["templates"]:
                flag_str = " | ".join(t["flags"]) if t["flags"] else ""
                print(f"   - {t['path']} [{t['type']}] {flag_str}")
            for e in m["errors"]:
                print(f"   MANIFEST ERROR: {e}")
        if r["has_j2"]:
            for j in r["j2_templates"]:
                err_str = " | ".join(j["errors"]) if j["errors"] else ""
                flag_str = " | ".join(j["flags"]) if j["flags"] else ""
                print(f"   J2 {Path(j['j2_path']).name}: {err_str} {flag_str}")
        if not r["has_zed_counterpart"]:
            print(f"   MISSING ZED COUNTERPART at .agents/skills/{r['skill']}/SKILL.md")
        print()

    # Top 5 most broken
    broken = sorted(report, key=lambda x: x["score"])[:5]
    print("\n## Top 5 Most Broken / Miscalibrated Registry Entries")
    for i, r in enumerate(broken, 1):
        print(f"{i}. **{r['skill']}** — score {r['score']}")
        reasons = []
        if not r["has_manifest"]:
            reasons.append("no manifest.yaml")
        if not r["has_j2"]:
            reasons.append("no .j2 templates")
        if not r["has_zed_counterpart"]:
            reasons.append("no Zed SKILL.md counterpart")
        for j in r["j2_templates"]:
            for e in j["errors"]:
                reasons.append(f"{Path(j['j2_path']).name}: {e}")
            for f in j["flags"]:
                reasons.append(f"{Path(j['j2_path']).name}: {f}")
        if r["has_manifest"]:
            for e in r["manifest"]["errors"]:
                reasons.append(f"manifest: {e}")
            for t in r["manifest"]["templates"]:
                for f in t["flags"]:
                    reasons.append(f"{t['path']}: {f}")
        print("   - " + "\n   - ".join(reasons[:8]))

    # Also write raw JSON for further inspection
    out = ROOT / "registry_audit_results.json"
    with open(out, "w") as f:
        json.dump(report, f, indent=2, default=str)
    print(f"\nFull JSON written to {out}")


if __name__ == "__main__":
    main()
