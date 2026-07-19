---
title: "Skill Design Spec — attack-taxonomy-mapper"
audience: [architects, replicants, security auditors]
last_updated: 2026-07-18
status: draft
version: 0.1.0
domain: security / supply-chain / taxonomy
skill_status: draft — not registry-committed
---

# Skill Design Spec — `attack-taxonomy-mapper`

**Status:** `draft` — not registry-committed. Per P5 Essentialism, this skill is
documented for evolutionary architecture (P7) but NOT implemented to avoid
speculative abstraction. A `manifest.yaml` and `.j2` templates will be created
under `registry/templates/attack-taxonomy-mapper/` only after this design
passes the 5W1H gate review and the OSC&R taxonomy mapping is verified
against the actual `oscar.io` framework.

This spec follows the same structure as `docs/plans/security-skills.md` Skill3.
It is the design artifact referenced by that plan.

## 1. Identity

| Field | Value |
|-------|-------|
| Skill name | `attack-taxonomy-mapper` |
| Version | 0.1.0 (proposed) |
| Visibility | `public` (P10, P11) |
| Type | Skill (PDCA FlowDef with convergence threshold + energy budget) |
| Surface parameter | `taxonomy` (single surface — distinct from `supply-chain-sentinel`'s manifest surface) |
| Decomposition | 4 templates: `select-evidence` → `map-taxonomy` → `taxonomize` → `convergence-check` |
| CNS spans | `cns.taxonomy.select`, `cns.taxonomy.map`, `cns.taxonomy.report`, `cns.taxonomy.convergence` (proposed — NOT yet registered in `CANONICAL_NAMESPACES`) |

## 2. 5W1H Gate (P5 Essentialism)

| Question | Answer |
|----------|--------|
| **Who** | Supply chain attacker / threat actor taxonomy (the *subject* of the taxonomy, not the *agent* — the agent is the replicant host, P12) |
| **What** | Software supply chain attack patterns (OSC&R taxonomy: dependency confusion, typosquatting, malicious commit injection, build pipeline compromise) |
| **Where** | Dependency registry / CI pipeline / repository (manifest evidence from `supply-chain-sentinel` findings + CI logs from `kali-audit`) |
| **When** | Audit cycle or incident investigation (consumes findings from `supply-chain-sentinel` and `kali-audit` as input) |
| **Why** | P3.1 requires structured taxonomy for supply chain threats (like MITRE ATLAS for LLM, OWASP LLM Top 10 for LLM). Without taxonomy mapping, findings are unstructured — severity and remediation priority are ambiguous. |
| **How** | Map manifest patterns / CI logs to OSC&R taxonomy entries → produce taxonomy-aligned findings (OWASP Supply Chain + OSC&R dual taxonomy) |

**Gate verdict:** All 6 answered. Passes P5.1 minimalist test (see §3).

## 3. P5 Minimal Test (Deletion Test)

**Does this skill replace an existing capability?** No. `supply-chain-sentinel`
finds manifest-level supply chain issues but maps them only to CWE categories
(CWE-1104, CWE-829, CWE-1357). It does NOT map to the OSC&R attack taxonomy
(which describes *attack patterns* like dependency confusion, typosquatting —
distinct from CWE *weakness categories*). `kali-audit` maps to OWASP LLM /
ATLAS but not to OSC&R. No existing skill provides OSC&R taxonomy mapping.

**Does this skill invent new taxonomy categories?** No. OSC&R is an open-
source framework (oscar.io) co-created by Google/Microsoft/GitLab. This skill
maps findings to existing OSC&R entries — it does not invent categories.

**Does this skill duplicate `bug-hunt`'s `taxonomize` phase?** No.
`bug-hunt`'s taxonomize phase classifies findings into Beizer's bug taxonomy
(requirements, structural, data, coding, interface, integration, timing,
configuration). This skill classifies supply chain findings into OSC&R attack
taxonomy. Distinct taxonomies, distinct domains.

**Deletion test:** If this skill is deleted, supply chain findings from
`supply-chain-sentinel` would have CWE mappings but no OSC&R attack pattern
mapping. Incident investigation would lack structured attack-pattern context.
Passes deletion test — skill earns its existence.

## 4. Relationship to Existing Skills

| Skill | Relationship | Overlap |
|-------|--------------|---------|
| `supply-chain-sentinel` | Complementary — `supply-chain-sentinel` performs manifest-level audit and proposes `surface: supply-chain` regressions with CWE mappings; `attack-taxonomy-mapper` consumes those findings and adds OSC&R taxonomy mapping. This skill adds a `taxonomy_mapping` field to regression YAML. | Zero (audit vs taxonomy mapping layer) |
| `kali-audit` | Complementary — `kali-audit` maps to OWASP LLM / ATLAS for LLM/code; `attack-taxonomy-mapper` maps to OSC&R for supply chain. Parallel taxonomy discipline. | Zero (distinct taxonomies) |
| `adversarial-red-team` | Parallel — `adversarial-red-team` uses MITRE ATLAS for LLM adversarial; `attack-taxonomy-mapper` uses OSC&R for supply chain adversarial. Same taxonomy discipline, different domain. | Zero (LLM vs supply chain) |
| `bug-hunt` | Structural — `bug-hunt`'s `taxonomize` phase provides the pattern (classify findings into taxonomy, assign severity). This skill replicates that pattern for OSC&R. | Pattern reuse, no surface overlap |

## 5. CNS Namespace Proposal

**Proposed namespaces (NOT yet registered):**

```rust
// crates/hkask-types/src/event.rs — CANONICAL_NAMESPACES
// ── Attack taxonomy (security audit — attack-taxonomy-mapper skill) ──
"cns.taxonomy",
"cns.taxonomy.select",
"cns.taxonomy.map",
"cns.taxonomy.report",
"cns.taxonomy.convergence",
```

**Registration discipline (per `docs/plans/security-skills.md` CNS Namespace
Architecture):** Direct registration in the flat `CANONICAL_NAMESPACES` array
(like `cns.supply_chain.*`, `cns.runtime.*`). NOT under a subgroup.

**Note:** `cns.taxonomy` does NOT conflict with `cns.classify.*` (which is
for classification drift / dual-fidelity spans in the inference domain).
`cns.taxonomy.*` is for security taxonomy mapping (OSC&R, OWASP, MITRE).
Distinct namespace, distinct purpose.

**Registration gate:** The `cns.taxonomy.*` namespaces MUST be registered in
`CANONICAL_NAMESPACES` before this skill is committed to the registry.

## 6. Proposed Templates (Design — Not Implemented)

| Template | Type | Purpose |
|----------|------|---------|
| `select-evidence.j2` | KnowAct | Discover evidence sources: `supply-chain-sentinel` findings (CWE-mapped), `kali-audit` findings (OWASP/ATLAS-mapped), CI logs, manifest evidence. Read `security/regressions/` for existing taxonomy mappings. Emit `cns.taxonomy.select` span. |
| `map-taxonomy.j2` | KnowAct | For each finding, map to OSC&R taxonomy entry (dependency confusion, typosquatting, malicious commit injection, build pipeline compromise, etc.). Apply pragmatic-cybernetics (feedback loop: does the attack pattern have a defense layer? variety: are there alternative attack vectors in the same OSC&R category?). Emit `cns.taxonomy.map` span per mapping. |
| `taxonomize.j2` | KnowAct | Classify mapped findings into OSC&R categories + OWASP Supply Chain dual taxonomy. Produce pattern signatures for detecting similar attack patterns. Propose `taxonomy_mapping` field additions to existing `surface: supply-chain` regression entries. Emit `cns.taxonomy.report` span. |
| `convergence-check.j2` | KnowAct | Compute normalized convergence metric: unmapped findings (0.40), OSC&R category coverage (0.25), OWASP Supply Chain coverage (0.15), regression taxonomy_mapping field growth (0.10), residual unmapped risk (0.10). Emit `cns.taxonomy.convergence` span. |

## 7. OSC&R Taxonomy Reference (Proposed Mappings)

| OSC&R Category | CWE Mapping | OWASP Supply Chain | Detection Pattern |
|----------------|-------------|---------------------|-------------------|
| Dependency confusion | CWE-829 | SC04 (Dependency confusion) | Manifest references package name not in private registry but exists in public registry |
| Typosquatting | CWE-829 | SC05 (Typosquatting) | Dependency name differs from canonical by ≤2 chars (Levenshtein distance) |
| Malicious commit injection | CWE-1104 | SC06 (Malicious commit) | Git dependency references unverified commit SHA |
| Build pipeline compromise | CWE-1357 | SC07 (Build compromise) | CI workflow references untrusted action without SHA pinning |
| Unmaintained component | CWE-1104 | SC08 (Unmaintained) | Dependency with no registry updates in manifest metadata |
| Unverified registry | CWE-829 | SC09 (Unverified registry) | Dependency source is `git = ...` or `path = ...` without registry reference |

**Note:** The OSC&R category names above are *proposed mappings* based on
public OSC&R documentation (oscar.io). The actual OSC&R taxonomy IDs must be
verified against the live oscar.io framework before registry commit. This
design spec does NOT invent OSC&R categories — it maps existing CWE/OWASP
findings to existing OSC&R entries.

## 8. Convergence Metric (Design)

Normalized metric [0, 1] where 0 = fully converged:

| Dimension | Weight | Scoring |
|-----------|--------|---------|
| Unmapped findings (critical/high) | 0.40 | 0 = +0.00; 1+ critical/high unmapped = +0.40 |
| OSC&R category coverage | 0.25 | All 6 categories covered by mappings = +0.00; partial = +0.06 per missing; none = +0.25 |
| OWASP Supply Chain coverage | 0.15 | All OWASP SC categories covered = +0.00; partial = +0.08; none = +0.15 |
| Regression `taxonomy_mapping` field growth | 0.10 | New mapping added this cycle = +0.00; stagnation = +0.10 |
| Residual unmapped risk | 0.10 | Zero unmapped findings = +0.00; any remaining = +0.10 |

Converged when metric ≤ 0.10 AND relative improvement ≥ 5% from previous cycle.

## 9. Constraints (Concrete — Not Aspirational)

- Every taxonomy mapping includes concrete evidence (finding reference,
  manifest path, CWE category, OSC&R category ID, OWASP SC category).
- No invented OSC&R categories — only map to existing entries in the
  oscar.io framework.
- No synthetic findings — consumes findings from `supply-chain-sentinel` and
  `kali-audit` as input; does not generate new findings.
- Every output includes `replicant_host` identity (P12).
- Registry (`manifest.yaml` + `.j2`) is authoritative over SKILL.md (P5.1).
- `cns.taxonomy.*` namespaces MUST be registered before skill commit (P9
  integrity — same discipline as `supply-chain-sentinel`).
- The `taxonomy_mapping` field added to regression YAML must be backward-
  compatible (optional field — existing regressions without it remain valid).

## 10. Source References and Taxonomy Anchors

- **OSC&R Framework:** Open Software Supply Chain Attack Reference —
  ATT&CK-like taxonomy for supply chain threats. Co-created by security
  experts from Google, Microsoft, GitLab. Source: `oscar.io` (verify live
  taxonomy before commit).
- **MITRE CWE:** CWE-1104 (Unmaintained Third-Party Components), CWE-829
  (Inclusion from Untrusted Control Sphere), CWE-1357 (Reliance on Component
  Not Updateable). Source: `mitre.org/data/definitions/`.
- **OWASP Supply Chain:** OWASP Software Supply Chain Security reference
  (owasp.org/www-project-software-supply-chain-security/).
- **`supply-chain-sentinel` SKILL.md:** Source of findings to map (CWE-
  mapped manifest evidence).
- **`kali-audit` SKILL.md:** Source of OWASP/ATLAS-mapped findings.
- **`bug-hunt` SKILL.md:** `taxonomize` phase pattern (classify findings
  into taxonomy, assign severity, produce pattern signatures).
- **`security/regressions/README.md`:** Regression YAML format — the
  `taxonomy_mapping` field extends this format.

## 11. Open Questions (Blocking Registry Commit)

1. **`cns.taxonomy.*` namespace registration:** Must be proposed and accepted
   before skill commit. Verify no conflict with `cns.classify.*` (which is
   for inference classification drift — distinct purpose).
2. **OSC&R taxonomy verification:** The OSC&R category names and IDs in §7
   are *proposed mappings* based on public documentation. Must verify against
   the live oscar.io framework before commit. Do NOT invent OSC&R IDs.
3. **`taxonomy_mapping` field format:** What is the exact YAML schema for
   the `taxonomy_mapping` field added to regression entries? Proposed:
   ```yaml
   taxonomy_mapping:
     osc_r: "T1.2.3"  # OSC&R taxonomy ID (verified against oscar.io)
     owasp_sc: "SC04" # OWASP Supply Chain category
   ```
   Need to verify this is compatible with `scripts/check-kali-regressions.sh`
   (which is surface-agnostic and parses specific fields — adding a new
   optional field should not break it, but verify).
4. **Finding consumption API:** How does this skill consume findings from
   `supply-chain-sentinel` and `kali-audit`? Via the CNS span history, via
   the regression library, or via a new finding-passing mechanism? Need to
   verify the inter-skill data flow path.
5. **Dual taxonomy value:** Does mapping to BOTH OSC&R and OWASP Supply Chain
   add value, or is one sufficient? The plan says "dual taxonomy" but P5
   minimalism may require choosing one. Resolve before commit.

## 12. Path to Registry Commit

1. Resolve open questions in §11.
2. Register `cns.taxonomy.*` namespaces in `CANONICAL_NAMESPACES`
   (`crates/hkask-types/src/event.rs`).
3. Verify OSC&R taxonomy IDs against live oscar.io framework.
4. Create `registry/templates/attack-taxonomy-mapper/manifest.yaml` with
   4 template entries (select-evidence, map-taxonomy, taxonomize,
   convergence-check).
5. Create the 4 `.j2` template files with `{# goal: ... #}` annotations
   (per `skill-logic-audit` critical revision).
6. Create `.agents/skills/attack-taxonomy-mapper/SKILL.md` derived from
   the registry manifest (P5.1 — registry authoritative).
7. Run `skill-logic-audit` convergence check on the new skill.
8. User `accept` per `skill-logic-audit` `user-choice` ratchet (P11).
9. Update `docs/plans/security-skills.md` to mark skill as `active`.

Until steps 1-8 complete, this skill remains `draft — not committed`.
