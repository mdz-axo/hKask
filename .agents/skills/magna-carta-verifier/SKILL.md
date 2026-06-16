---
name: magna-carta-verifier
visibility: public
description: "Verifies that hKask's four Magna Carta principles (User Sovereignty, Affirmative Consent, Generative Space, Clear Boundaries) are correctly implemented and enforced. Uses YAML manifests to declare assertions per principle and Jinja2 templates to render verification procedures, reports, and test cases. Use when auditing sovereignty compliance, onboarding new resources, or verifying consent structures."
---

# Magna Carta Verifier

Verifies that hKask's four Magna Carta principles are correctly implemented and enforced. Uses YAML manifests to declare assertions per principle and Jinja2 templates to render verification procedures, reports, and test cases.

## Registry Templates

This skill's runtime templates live in `registry/templates/magna-carta-verifier/`:

| Template | Type | Purpose |
|----------|------|--------|
| `mc-verify-procedure.j2` | KnowAct | Render step-by-step verification procedure for each assertion |
| `mc-verify-report.j2` | KnowAct | Synthesize verification results into a compliance report |
| `mc-verify-testcase.j2` | KnowAct | Render Rust test case skeletons from assertion definitions |

The SKILL.md (this file) teaches the Zed coding agent the sovereignty verification methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## Principles

| # | Principle | Core Assertion |
|---|-----------|---------------|
| P1 | User Sovereignty (SOLID-grounded) | Data is owned by the user, correctly categorized, portable, and consent is atomic |
| P2 | Affirmative Consent | Default deny, scoped/versioned/expiring consent, hierarchical structures, fail-closed |
| P3 | Generative Space | Settings equally exposed, no privileged engineer access, open-source only, user-curated |
| P4 | Clear Boundaries (OCAP) | P1–P3 enforced through OCAP gates, no bypasses, tokens unforgeable |

## When to Use

- When verifying that the Magna Carta principles are upheld after code changes
- When onboarding new resources, MCP servers, or inference providers
- When consent grants expire or change
- When the user or Curator requests a sovereignty audit
- At start-up, per the trigger model

## Triggers

| Trigger | When |
|---------|------|
| Start-up | Verification runs when hKask starts |
| Expiration | Consent grants expire → re-verification scheduled |
| User change | New consent, settings change, new API key → re-verify affected assertions |
| Resource/service change | New version of MCP server, inference provider, or model → re-verify affected assertions |

## Verification Methods

| Method | Description |
|--------|-------------|
| `structural_audit` | Enumerate access paths and verify gates exist |
| `behavioral_probe` | Generate access attempts and verify denial |
| `resource_verification` | Verify resource categorization at onboarding; re-check on change |
| `absence_check` | Verify that prohibited constructs (hidden gates, admin overrides) do not exist |

## Manifest Structure

Each manifest is a YAML file declaring assertions for one principle:

```yaml
principle: <principle_name>
version: "0.1.0"
description: "..."

assertions:
  - id: <principle_id><letter>
    name: <short_name>
    claim: "Human-readable assertion"
    method: <structural_audit|behavioral_probe|resource_verification|absence_check>
    targets:
      - crate: <crate_name>
        module: <module_path>
        methods: [<method_names>]
        gate: <gate_name>  # optional
```

## Templates

Runtime templates are in `registry/templates/magna-carta-verifier/` (see Registry Templates section above). Legacy template references preserved for documentation:

- `mc-verify-procedure.j2` — Renders the verification procedure for each assertion
- `mc-verify-report.j2` — Renders findings, gaps, and status
- `mc-verify-testcase.j2` — Renders Rust test cases as code blocks in the report

## Resolution Process

When an assertion fails, the verification report is escalated to the Curator. The Curator reviews the finding with the human user or the user's replicant in a chat session. The resolution process is defined by the user in collaboration with the Curator.

## CLI Access

```bash
kask sovereignty verify              # Verify all principles
kask sovereignty verify --principle p1  # Verify P1 only
kask sovereignty verify --json       # JSON output (for MCP/API)
```

There is no dedicated MCP tool for Magna Carta verification. The CLI command is the canonical entry point; it loads assertion manifests from `.agents/skills/magna-carta-verifier/manifests/` and reports pass/fail/gap status.
