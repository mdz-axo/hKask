---
name: security-scan-pdca
visibility: public
description: >
  Security-service-aligned scanning skill for hKask replicants. Vertical
  slice: Snyk (SAST + dependency scanning) + Semgrep (SAST + IaC rules).
  PDCA cycle integration per kata-improvement. Safe container (P3.1):
  core content safety controls mandatory at LLM boundary.
---

# Security Scan PDCA

Security-service-aligned scanning skill. Performs static analysis and
dependency vulnerability scanning aligned to external service patterns
(Snyk, Semgrep). Operates within the safe generative container (P3.1).

**Note:** This skill is superseded by the newer, native hKask security
skills: `kali-audit` (code/template/MCP/supply-chain surface audit),
`supply-chain-sentinel` (dependency manifest audit), `runtime-posture-monitor`
(runtime telemetry monitoring), and `attack-taxonomy-mapper` (OSC&R taxonomy
mapping). This skill is retained for backward compatibility but should not
be used for new audit work — prefer the native skills.

## When to Use

- When you need to align hKask security scanning with external service
  patterns (Snyk, Semgrep).
- When you need PDCA cycle integration for security scanning.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `security-scan-flow.j2` | KnowAct | Security scan PDCA flow: scan → dependency-check → IaC-rule-check → result → convergence. |

## Constraints

- Registry is authoritative — when this SKILL.md disagrees with registry
  templates, the registry wins.
- Unverified claims (Snyk dependency mapping, Semgrep IaC domain bridge)
  are documented in `security/unverified/`.
