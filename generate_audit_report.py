#!/usr/bin/env python3
"""Generate concise markdown report from registry_audit_results.json."""

import json
from collections import Counter
from pathlib import Path

ROOT = Path("/home/mdz-axolotl/Clones/hKask")
DATA = ROOT / "registry_audit_results.json"
OUT = ROOT / "REGISTRY_AUDIT_REPORT.md"

with open(DATA) as f:
    report = json.load(f)

lines = []
lines.append("# hKask Registry Template Layer Audit\n")
lines.append(f"- **Total registry entries:** {len(report)}")
lines.append(f"- **With manifest.yaml:** {sum(1 for r in report if r['has_manifest'])}")
lines.append(f"- **With .j2 templates:** {sum(1 for r in report if r['has_j2'])}")
lines.append(
    f"- **With Zed SKILL.md counterpart:** {sum(1 for r in report if r['has_zed_counterpart'])}"
)
lines.append(f"- **Total .j2 files:** {sum(len(r['j2_templates']) for r in report)}")
lines.append("")

# Distribution by health status
status_counts = Counter()
for r in report:
    s = r["score"]
    if s >= 0.8:
        status_counts["active"] += 1
    elif s >= 0.5:
        status_counts["stale_warning"] += 1
    elif s >= 0.2:
        status_counts["critical"] += 1
    else:
        status_counts["recommend_deprecation"] += 1
lines.append("## Health Distribution")
for k, v in status_counts.most_common():
    lines.append(f"- {k}: {v}")
lines.append("")

# Full inventory table
lines.append("## Registry Inventory\n")
lines.append("| Skill | Manifest | Zed SKILL.md | Score | Status | Notes |")
lines.append("|-------|----------|--------------|-------|--------|-------|")
for r in report:
    skill = r["skill"]
    has_m = "✓" if r["has_manifest"] else "✗"
    has_z = "✓" if r["has_zed_counterpart"] else "✗"
    score = r["score"]
    if score >= 0.8:
        status = "active"
    elif score >= 0.5:
        status = "stale_warning"
    elif score >= 0.2:
        status = "critical"
    else:
        status = "recommend_deprecation"
    notes = []
    if not r["has_j2"]:
        notes.append("no .j2")
    if r["has_manifest"] and r["manifest"]["errors"]:
        notes.append(f"{len(r['manifest']['errors'])} manifest errors")
    j2_errors = sum(len(j["errors"]) for j in r["j2_templates"])
    j2_flags = sum(len(j["flags"]) for j in r["j2_templates"])
    if j2_errors:
        notes.append(f"{j2_errors} J2 errors")
    if j2_flags:
        notes.append(f"{j2_flags} J2 flags")
    note_str = "; ".join(notes[:3]) or "—"
    lines.append(f"| {skill} | {has_m} | {has_z} | {score} | {status} | {note_str} |")
lines.append("")

# Detailed red flags
lines.append("## Red Flags by Registry Entry\n")
for r in report:
    issues = []
    if not r["has_manifest"]:
        issues.append("No manifest.yaml")
    if not r["has_j2"]:
        issues.append("No .j2 templates")
    if not r["has_zed_counterpart"]:
        issues.append("No Zed SKILL.md counterpart")
    if r["has_manifest"]:
        m = r["manifest"]
        for t in m["templates"]:
            if t["flags"]:
                for f in t["flags"]:
                    issues.append(f"manifest template `{t['path']}`: {f}")
        for e in m["errors"]:
            issues.append(f"manifest: {e}")
    for j in r["j2_templates"]:
        name = Path(j["j2_path"]).name
        for e in j["errors"]:
            issues.append(f"`{name}`: {e}")
        for f in j["flags"]:
            issues.append(f"`{name}`: {f}")
    if issues:
        lines.append(f"### {r['skill']} (score {r['score']})")
        for issue in issues[:20]:
            lines.append(f"- {issue}")
        if len(issues) > 20:
            lines.append(f"- ... and {len(issues) - 20} more issues")
        lines.append("")

# Top 5 most broken
broken = sorted(report, key=lambda x: (x["score"], -len(x.get("j2_templates", []))))[:5]
lines.append("## Top 5 Most Broken / Miscalibrated Registry Entries\n")
for i, r in enumerate(broken, 1):
    lines.append(f"### {i}. `{r['skill']}` — score {r['score']}")
    reasons = []
    if not r["has_manifest"]:
        reasons.append("No manifest.yaml")
    if not r["has_j2"]:
        reasons.append("No .j2 templates")
    if not r["has_zed_counterpart"]:
        reasons.append("No Zed SKILL.md counterpart")
    for j in r["j2_templates"]:
        name = Path(j["j2_path"]).name
        for e in j["errors"]:
            reasons.append(f"`{name}`: {e}")
        for f in j["flags"]:
            reasons.append(f"`{name}`: {f}")
    if r["has_manifest"]:
        for e in r["manifest"]["errors"]:
            reasons.append(f"manifest: {e}")
        for t in r["manifest"]["templates"]:
            for f in t["flags"]:
                reasons.append(f"manifest `{t['path']}`: {f}")
    for reason in reasons[:10]:
        lines.append(f"- {reason}")
    lines.append("")

# Summary counts of issue types
issue_types = Counter()
for r in report:
    if r["has_manifest"]:
        for t in r["manifest"]["templates"]:
            for f in t["flags"]:
                if "invalid type" in f or "unknown type" in f:
                    issue_types["invalid/unknown template_type in manifest"] += 1
                if "missing file" in f:
                    issue_types["manifest references missing .j2"] += 1
        for e in r["manifest"]["errors"]:
            if "hlexicon" in e:
                issue_types["manifest hlexicon term not in workspace"] += 1
    for j in r["j2_templates"]:
        for e in j["errors"]:
            if "Jinja2 syntax" in e:
                issue_types["Jinja2 syntax error"] += 1
            elif "frontmatter" in e.lower() or "inference" in e.lower():
                issue_types["missing/invalid frontmatter"] += 1
            else:
                issue_types["other J2 error"] += 1
        for f in j["flags"]:
            if "template_type invalid" in f:
                issue_types["invalid template_type in .j2 frontmatter"] += 1
            elif "FlowDef declared" in f:
                issue_types["FlowDef declared on .j2"] += 1
            elif "energy_cap" in f and "outside" in f:
                issue_types["energy_cap out of range"] += 1
            elif "visibility" in f and "not in" in f:
                issue_types["invalid visibility value"] += 1
            elif "hlexicon term" in f:
                issue_types[".j2 hlexicon term not in workspace"] += 1
            elif "Liquid" in f:
                issue_types["Liquid-style filter syntax"] += 1
            elif "shell" in f:
                issue_types["template asks LLM to execute shell commands"] += 1
            elif "contract input" in f:
                issue_types["contract input not used in body"] += 1
            else:
                issue_types["other flag"] += 1

lines.append("## Issue Type Counts\n")
for k, v in issue_types.most_common():
    lines.append(f"- {k}: {v}")
lines.append("")

with open(OUT, "w") as f:
    f.write("\n".join(lines))
print(f"Report written to {OUT}")
print(f"\nSummary:")
print(f"  Total entries: {len(report)}")
print(f"  Active: {status_counts['active']}")
print(f"  Stale warning: {status_counts['stale_warning']}")
print(f"  Critical: {status_counts['critical']}")
print(f"  Recommend deprecation: {status_counts['recommend_deprecation']}")
print(f"\nTop issue types:")
for k, v in issue_types.most_common(10):
    print(f"  {v}x {k}")
