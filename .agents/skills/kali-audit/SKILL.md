---
name: kali-audit
visibility: public
description: "Security review skill for hKask. Audits Rust code, Jinja2 templates, YAML manifests, and supply chain for security vulnerabilities. Anchored to OWASP LLM Top 10, CWE, STRIDE, and MITRE ATLAS. Consumes the security regression library (security/regressions/) as input and proposes new regression entries for confirmed findings. Single skill with a surface parameter — not a bundle.
"
---

# Kali Audit

Security review skill for hKask. Audits Rust code, Jinja2 templates, YAML manifests, and supply chain for security vulnerabilities. Anchored to OWASP LLM Top 10, CWE, STRIDE, and MITRE ATLAS. Consumes the security regression library (`security/regressions/`) as input and proposes new regression entries for confirmed findings. Single skill with a surface parameter — not a bundle.

## When to Use

- When you need to audit a crate, template directory, or the supply chain for security vulnerabilities.
- When you need to check for specific CWE classes (path traversal, secret leakage, timing attacks, SSTI, deserialization).
- When you need to propose regression entries for confirmed findings so CI catches re-introductions.
- When you need to compute a security coverage metric (which CWE classes were checked, which remain).
- When you need to consume the existing regression library to avoid re-finding known issues.

## Instructions

### kali-audit/select-surface

1. Identify the target surface from the `target_surface` parameter (code, template, supply-chain, mcp, config).
2. Map the surface to its applicable check catalog (see the template's Surface Check Catalog section).
3. Read the regression library (`security/regressions/RR-*.yaml`) to identify already-enforced checks — skip them.
4. Return the selected surface, the checks to run, and the known regressions.

### kali-audit/audit

1. For each check in `checks_to_run`, use available MCP tools (`file:read`, `code:search`, `terminal`) to probe the target.
2. Collect concrete evidence — file paths, line numbers, code snippets. Do NOT fabricate findings.
3. Classify each finding by CWE, OWASP LLM, severity, confidence, and constraint force (per pragmatic-semantics).
4. For each finding with severity >= medium, propose a regression entry with a concrete, testable detection pattern.
5. Track coverage: which checks ran, passed, failed, and which CWE classes were covered.

### kali-audit/report

1. Synthesize findings into a structured report grouped by severity.
2. Produce a verdict: Pass (no critical/high), Conditional (medium), or Fail (critical/high).
3. For each finding, provide a concrete remediation recommendation.
4. Produce proposed regression entries in YAML format for human review.
5. Identify the top 3 highest-priority fixes.
6. Note coverage gaps (CWE classes not checked) for future audits.

### kali-audit/convergence-check

1. Compute the convergence metric on [0, 1] where 0 = converged (no residual risk).
2. Score four dimensions: critical/high findings (0.50), medium findings (0.20), CWE coverage (0.15), regression library growth (0.15).
3. Converged when metric ≤ 0.05 with minimum 5% relative improvement from previous cycle.
4. Identify specific blockers preventing convergence.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `select-surface.j2` | KnowAct | Select the security audit target surface and map it to the applicable check catalog. Reads the regression library to skip already-enforced checks. |
| `audit.j2` | KnowAct | Run security checks for the selected surface using MCP tools. Collect findings with CWE/OWASP classification and propose regression entries. |
| `report.j2` | KnowAct | Synthesize findings into a structured report with verdict, severity grouping, top-3 fixes, and proposed regression YAML entries. |
| `convergence-check.j2` | KnowAct | Compute normalized convergence metric for kali-audit PDCA cycles. Measures CWE coverage, regression growth, and residual risk. |

## Relationship to the Regression Library

The `security/regressions/` directory is the **deep artifact** — it compounds value over time. The skill consumes it as input (to avoid re-finding known issues) and proposes new entries as output (for human review). The "evolving" property comes from the library growing, not from the skill mutating its own prompts.

**Honest framing:** this is a human-curated checklist with CI enforcement, not autonomous learning. The skill proposes entries; humans curate them; CI enforces them. This is how OWASP ASVS and similar frameworks work.

## Relationship to adversarial-red-team

`kali-audit` covers **code and infrastructure** security (Rust, templates, manifests, supply chain). `adversarial-red-team` covers **LLM I/O robustness** (prompt injection, exfiltration). They are complementary — `kali-audit` checks the static surface; `adversarial-red-team` probes the dynamic LLM boundary.

## Constraints

- `select-surface.j2`: Public.
- `audit.j2`: Public.
- `report.j2`: Public.
- `convergence-check.j2`: Public.
- Do NOT fabricate findings — only report what was actually discovered through tool usage.
- Every finding must include concrete evidence (file path, line number, code snippet).
- Every proposed regression must have a concrete, testable detection pattern.
- Classify constraint force honestly — not everything is a Prohibition.
- Registry is authoritative — when this SKILL.md disagrees with registry templates, the registry wins.
