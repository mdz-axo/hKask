---
title: "Skill Design Spec — attack-taxonomy-mapper"
audience: [architects, replicants, security auditors]
last_updated: 2026-07-18
status: as-built
version: 0.2.0
domain: security / supply-chain / taxonomy
skill_status: active — registry-committed (2026-07-18)
---

# Skill Design Spec — `attack-taxonomy-mapper`

**Status:** `active` — registry-committed (2026-07-18). This design spec
is retained as as-built documentation. The skill is implemented at
`registry/templates/attack-taxonomy-mapper/` (manifest.yaml + 4 .j2
templates) + `.agents/skills/attack-taxonomy-mapper/SKILL.md`.

The `cns.taxonomy.*` namespaces are registered in `CANONICAL_NAMESPACES`
(`crates/hkask-types/src/event.rs` L309-315). The OSC&R taxonomy was
verified against `github.com/pbom-dev/OSCAR` `matrix.json` (2026-07-18).
The skill passed `skill-logic-audit` with all material flaws resolved.

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

## 11. Open Questions (Resolved)

1. **`cns.taxonomy.*` namespace registration:** RESOLVED — all 5 namespaces
   registered in `CANONICAL_NAMESPACES` (`event.rs` L309-315). No conflict
   with `cns.classify.*` (distinct purpose).
2. **OSC&R taxonomy verification:** RESOLVED — verified against
   `github.com/pbom-dev/OSCAR` `matrix.json` (2026-07-18). OSC&R uses
   tactic + technique names, NOT numeric IDs. All 12 tactics and all
   referenced techniques verified against the live matrix.
3. **`taxonomy_mapping` field format:** RESOLVED — backward-compatible
   optional field with `osc_r_tactic`, `osc_r_technique`, `osc_r_categories`,
   and `owasp_sc_reference` keys. Compatible with `scripts/check-kali-regressions.sh`
   (which parses specific fields; optional field is safe).
4. **Finding consumption API:** PARTIALLY OPEN — the skill reads
   `security/regressions/` YAML files for already-merged findings. It cannot
   consume fresh findings from a current `supply-chain-sentinel` or
   `kali-audit` audit cycle in real-time. This limits the skill's utility for
   real-time incident investigation. See §13 below.
5. **Dual taxonomy value:** RESOLVED — OSC&R is the primary taxonomy
   (verified). OWASP SC codes are secondary (PROPOSED — OWASP does not
   publish a numbered Supply Chain Top 10). The convergence metric treats
   OWASP SC as a presence/absence signal, not a coverage dimension.

## 12. Path to Registry Commit (COMPLETED)

All steps completed (2026-07-18):
1. ✅ Resolved open questions in §11 (4 resolved, 1 partially open — see §13).
2. ✅ Registered `cns.taxonomy.*` namespaces in `CANONICAL_NAMESPACES`.
3. ✅ Verified OSC&R taxonomy IDs against live `github.com/pbom-dev/OSCAR`.
4. ✅ Created `registry/templates/attack-taxonomy-mapper/manifest.yaml`.
5. ✅ Created 4 `.j2` template files with `{# goal: ... #}` annotations.
6. ✅ Created `.agents/skills/attack-taxonomy-mapper/SKILL.md`.
7. ✅ Ran `skill-logic-audit` convergence check — all material flaws fixed.
8. ✅ User accepted per `skill-logic-audit` `user-choice` ratchet.
9. ✅ Updated `docs/plans/security-skills.md` — skill marked `active`.

## 13. Remaining Infrastructure Work (Post-Commit)

The skill is registry-committed and passes `kask skill audit` (score 1.00,
0 defects). However, one infrastructure gap remains before the skill is
fully invocable for real-time incident investigation:

1. **Finding consumption API:** The skill currently reads
   `security/regressions/` YAML files for already-merged findings. It cannot
   consume fresh findings from a current `supply-chain-sentinel` or
   `kali-audit` audit cycle in real-time. For post-audit taxonomy mapping
   (the primary use case), this is sufficient. For real-time incident
   investigation, a finding-passing mechanism (e.g., CNS span history or an
   inter-skill data flow API) would be needed.

This is an infrastructure task, not a skill design task. The skill itself
is complete and correct for its primary use case (post-audit taxonomy
mapping of merged regression entries).
