---
name: attack-taxonomy-mapper
visibility: public
description: >
  OSC&R taxonomy mapping skill for hKask (v0.31.0). Consumes findings
  from supply-chain-sentinel (CWE-mapped manifest evidence) and
  kali-audit (OWASP/ATLAS-mapped findings) as input — does NOT
  generate new findings. Maps each finding to the OSC&R attack
  taxonomy (dependency confusion, typosquatting, malicious commit
  injection, build pipeline compromise, unmaintained component,
  unverified registry) plus the OWASP Supply Chain dual taxonomy.
  Adds an optional, backward-compatible `taxonomy_mapping` field to
  existing `surface: supply-chain` regression YAML entries. Emits
  cns.taxonomy.* spans (P9) — all four namespaces (select, map,
  report, convergence) are registered in CANONICAL_NAMESPACES
  (crates/hkask-types/src/event.rs). P8 evidence-backed: every
  mapping includes concrete evidence (finding reference, CWE
  category, OSC&R category, OWASP SC category). P12 replicant_host
  mandatory. Decomposed into 4 phases matching bug-hunt and
  supply-chain-sentinel pipeline. Minimal (P5): answers all 5W1H;
  single skill, no bundle; complements supply-chain-sentinel
  (adds taxonomy layer) and kali-audit (parallel taxonomy discipline).
  Zero overlap with adversarial-red-team (LLM boundary — ATLAS, not
  OSC&R).
---

# Attack Taxonomy Mapper

{# goal: Map supply-chain-sentinel and kali-audit findings to the OSC&R attack taxonomy plus OWASP Supply Chain dual taxonomy. Add an optional, backward-compatible taxonomy_mapping field to existing surface: supply-chain regression YAML entries. Emit cns.taxonomy.* spans (P9) — all four namespaces (select, map, report, convergence) registered in CANONICAL_NAMESPACES (crates/hkask-types/src/event.rs). P8 evidence-backed: every mapping includes concrete evidence (finding reference, CWE category, OSC&R category, OWASP SC category). No invented OSC&R categories — only map to existing entries in the oscar.io framework. No synthetic findings — consumes upstream skill outputs only. Replicant_host mandatory (P12). Compute convergence metric from real mapping evidence only. #}

OSC&R taxonomy mapping for supply chain findings. Consumes findings
from `supply-chain-sentinel` (CWE-mapped manifest evidence) and
`kali-audit` (OWASP/ATLAS-mapped findings — `surface: supply-chain`
subset only) as input. Maps each finding to an OSC&R attack taxonomy
entry plus an OWASP Supply Chain category. Produces pattern signatures
for detecting similar attack patterns. Proposes an optional, backward-
compatible `taxonomy_mapping` field addition to existing
`surface: supply-chain` regression YAML entries. Tracks OSC&R
category coverage, OWASP SC coverage, and `taxonomy_mapping` field
growth across audit cycles via a taxonomy-specific convergence metric.

## When to Use

- After `supply-chain-sentinel` has produced CWE-mapped findings and
  you need OSC&R attack-pattern context for incident response.
- After `kali-audit` has produced `surface: supply-chain` findings
  and you need to back-fill the `taxonomy_mapping` field on the
  corresponding regression entries.
- When investigating a supply chain incident and need structured
  attack-pattern classification (dependency confusion, typosquatting,
  malicious commit injection, build pipeline compromise, unmaintained
  component, unverified registry).
- When proposing `taxonomy_mapping` field additions to existing
  `security/regressions/` entries (backward-compatible — existing
  regressions without the field remain valid).
- When verifying OSC&R category coverage across audit cycles (P9
  observable via `cns.taxonomy.*` spans).
- When computing taxonomy-mapping-specific convergence across audit
  cycles.

## Design Constraints (Grounded in Project Principles)

- **P5 Essentialism (5W1H gate):** Who = replicant host (P12); What =
  OSC&R + OWASP SC taxonomy mapping per finding; Where = upstream
  skill outputs + `security/regressions/` + CI logs; When = audit
  cycle (consumes prior phase outputs); Why = P3.1 safe container /
  P8 semantic grounding (taxonomy mapping gives findings attack-pattern
  context) / P9 observable regulation; How = collect evidence → map
  to OSC&R + OWASP SC → classify + propose `taxonomy_mapping` field →
  compute convergence → emit CNS spans. All 6 present — passes gate.
- **P5.1 Registry canonical:** Registry (`manifest.yaml` + `.j2`) is
  source of truth. SKILL.md derived from it.
- **P5.3 Minimalist test:** No speculative finding generation —
  consumes upstream skill outputs only. No extra abstractions. 4
  templates only, no bundle, no sub-agent delegation.
- **P5.4 Dual-axis:** Each mapping has state identity (finding
  reference + CWE category) and process identity (map-taxonomy flow).
- **P7 Evolutionary:** Pattern signatures and `taxonomy_mapping`
  proposals emerge from real mapping evidence, not speculation. New
  OSC&R categories can be added as the oscar.io framework evolves —
  but only when verified against live oscar.io, not speculatively.
- **P8 Semantic grounding:** Every mapping: finding reference, CWE
  category, OSC&R category, OWASP SC category, evidence snippet
  (quoted from upstream finding — verifiable), source citation (MITRE
  CWE URL, OWASP Supply Chain reference, OSC&R framework URL). No
  fabricated findings. No invented OSC&R categories. The OSC&R
  category names in §OSC&R Taxonomy Reference are PROPOSED mappings —
  they MUST be verified against live oscar.io before use. Do NOT
  present them as verified.
- **P9 CNS regulation:** Emits `cns.taxonomy.select`,
  `cns.taxonomy.map`, `cns.taxonomy.report`,
  `cns.taxonomy.convergence` spans. All four are registered in
  `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`) and
  emitted unconditionally. Do NOT use "if registered; else note gap"
  language — the namespaces ARE registered.
- **P10 Bot/replicant taxonomy:** `visibility: public` — transparent
  taxonomy mapping.
- **P11 Visibility:** `taxonomy_mapping` proposals default
  `status: pending` (human-curated ratchet, per
  `security/regressions/README.md`). Alternative mappings recorded
  for transparency when multiple OSC&R categories match.
- **P12 Replicant host mandate:** Every action includes
  `replicant_host`.
- **P3.1 Safety floor:** Taxonomy mapping gives findings attack-
  pattern context, enabling structured incident response — protects
  the Generative Space container.
- **P4 OCAP boundaries:** Reads only declared evidence sources
  (upstream skill outputs, regression library, CI logs); no ambient
  scanning; no external taxonomy database download without explicit
  consent (P2).

## Instructions

### attack-taxonomy-mapper/select-evidence

1. Collect `supply-chain-sentinel` findings: each must include
   `dependency`, `version_spec`, `manifest_path`, `line`,
   `evidence_snippet`, `cwe_reference` (CWE-1104 / CWE-829 /
   CWE-1357), `severity`, `defense_layers_present`,
   `defense_layers_missing`, `source_citation`. Findings without
   `cwe_reference` are unmappable — flag as
   `unmappable_reason: "missing_cwe_reference"`.
2. Collect `kali-audit` findings with `surface: supply-chain` (the
   subset relevant to OSC&R taxonomy — kali-audit's other surfaces
   map to OWASP LLM / ATLAS, not OSC&R). Each must include `surface`,
   `cwe_reference` (if present), `owasp_llm` (if present),
   `severity`, `evidence_snippet`, `discovered_in`.
3. Read `security/regressions/` for YAML entries with
   `surface: supply-chain`. For each, check whether the optional
   `taxonomy_mapping` field is present. List:
   - `existing_taxonomy_mappings`: entries that already have
     `taxonomy_mapping` (skip re-mapping in map-taxonomy — P5
     minimalism, no duplicate work).
   - `regressions_needing_mapping`: entries with `surface:
     supply-chain` and `cwe` field but no `taxonomy_mapping` yet.
4. Collect CI log paths (concrete file paths only — no synthetic
   logs). CI logs may reveal build-pipeline-compromise patterns
   (untrusted GitHub Actions without SHA pinning, unverified runner
   image references). Each path must exist on disk — do NOT invent
   paths.
5. Build `findings_to_map` array: union of (a) supply-chain-sentinel
   findings, (b) kali-audit `surface: supply-chain` findings, (c)
   `regressions_needing_mapping` entries (treat each as a finding to
   back-fill `taxonomy_mapping` for). Each entry tagged with
   `evidence_source` (`supply-chain-sentinel` | `kali-audit` |
   `regression-library` | `ci-log`).
6. Verify every `findings_to_map` entry has concrete evidence
   (`manifest_path` + line, or regression id + cwe field, or
   `ci_log_path` + line). Drop entries without evidence — do NOT
   fabricate evidence to make an entry mappable. Record dropped
   entries as `unmappable_findings` with `unmappable_reason`.
7. Return JSON: `{evidence_sources, findings_to_map,
   existing_taxonomy_mappings, regressions_needing_mapping,
   unmappable_findings, replicant_host}`.
8. Emit `cns.taxonomy.select` CNS span (P9) with
   `findings_to_map_count`, `existing_taxonomy_mappings_count`,
   `regressions_needing_mapping_count`, `unmappable_findings_count`,
   host identity, latency metric. Namespace is registered — emit
   unconditionally.

### attack-taxonomy-mapper/map-taxonomy

1. For each finding in `findings_to_map` (skip entries in
   `existing_taxonomy_mappings` — already mapped, P5 minimalism):
2. Match the finding's `cwe_reference` against the OSC&R category
   list (see §OSC&R Taxonomy Reference — PROPOSED mappings, verify
   against live oscar.io):
   - CWE-829 → candidates: `dependency_confusion`, `typosquatting`,
     `unverified_registry`. Disambiguate using `evidence_snippet`
     and `defense_layers_missing`.
   - CWE-1104 → candidates: `malicious_commit_injection`,
     `unmaintained_component`. Disambiguate using `evidence_snippet`.
   - CWE-1357 → candidate: `build_pipeline_compromise` (CI workflow
     references untrusted action without SHA pinning — the action is
     not updateable to a verified version).
3. If the finding's `cwe_reference` does not match any OSC&R
   category's CWE mapping, mark `unmapped` with
   `unmapped_reason: "no_osc_r_category_for_cwe"` — do NOT fabricate
   a category. Record in `unmapped_findings` output array.
4. If multiple OSC&R categories match, choose the one whose pattern
   best fits the finding's `evidence_snippet`. Record runner-up
   categories in `alternative_mappings` for transparency (P11).
5. Apply pragmatic-cybernetics (embedded — like bug-hunt oracle and
   supply-chain-sentinel probe):
   - Feedback loop: does the mapped OSC&R attack pattern have a
     corresponding defense layer in the finding's
     `defense_layers_present`? If yes → note whether enforcing. If
     no → record `defense_gap: true`.
   - Variety (Ashby's Law): are there alternative attack vectors
     within the same OSC&R category that existing defense layers do
     not cover? Record `variety_gap: true` if so.
   - Good Regulator check: is the defense layer (if present) actually
     enforcing, or advisory? Record
     `regulator_strength: enforcing | advisory | missing`.
6. For each mapped finding, produce structured mapping:
   `finding_reference`, `cwe_reference`, `osc_r_category`,
   `osc_r_pattern`, `owasp_sc_category`, `evidence_snippet` (quoted
   from upstream finding — verifiable), `defense_layers_present`,
   `defense_layers_missing`, `defense_gap`, `variety_gap`,
   `regulator_strength`, `alternative_mappings`,
   `mapping_confidence` (`high` | `medium` | `low` — justified by
   evidence specificity), `replicant_host`.
7. For each unmapped finding, produce: `finding_reference`,
   `cwe_reference`, `unmapped_reason`, `evidence_snippet`,
   `replicant_host`.
8. Emit `cns.taxonomy.map` CNS span per mapping (`target:
   "cns.taxonomy.map"`, message: `"CNS"`, operation:
   `"map_taxonomy"`, finding_reference, cwe_reference,
   osc_r_category, owasp_sc_category, mapping_confidence,
   defense_gap, replicant_host, latency_ms). Namespace is
   registered — emit unconditionally per mapping.

CONSTRAINT — Evidence integrity (P8):
- No invented OSC&R categories — only map to entries in the OSC&R
  category list (which must be verified against live oscar.io per
  §OSC&R Taxonomy Reference note).
- No synthetic findings — only map findings from `findings_to_map`.
- Every mapping includes concrete evidence: `finding_reference`,
  `cwe_reference`, `osc_r_category`, `owasp_sc_category`,
  `evidence_snippet`.
- Skip entries in `existing_taxonomy_mappings` — P5 minimalism, no
  duplicate mapping work.
- Every mapping includes `replicant_host` identity (P12) — no
  anonymous mapping.
- When multiple OSC&R categories match, record `alternative_mappings`
  for transparency (P11) — do NOT silently pick one.
- When no OSC&R category matches, mark `unmapped` with explicit
  reason — do NOT force a mapping.
- Minimal (P5): 4 templates (`select-evidence`, `map-taxonomy`,
  `taxonomize`, `convergence-check`), no bundle, no sub-agent
  delegation. Each template answers specific 5W1H: select-evidence
  (Where), map-taxonomy (What + How), taxonomize (Why + What),
  convergence-check (When + Why).

### attack-taxonomy-mapper/taxonomize

1. Group `mappings` by `osc_r_category`. For each OSC&R category,
   produce a summary: `findings_count`, `findings` (list of
   `finding_reference`s), `cwe_references` (set), `owasp_sc_categories`
   (set), `defense_gap_count`, `variety_gap_count`,
   `regulator_strength_summary` ({enforcing, advisory, missing}
   counts), `pattern_signature` (concrete — see step 3).
2. Group `mappings` by `owasp_sc_category`. Produce a parallel
   summary keyed by OWASP SC.
3. Produce `pattern_signature` per OSC&R category — a concrete
   grep-able detection pattern (regex or structural predicate) for
   finding similar attack patterns in future audits. Pattern
   signatures must reference concrete manifest/CI content (not vague
   descriptions):
   - `dependency_confusion`: regex matching package names not in
     private registry but available publicly.
   - `typosquatting`: Levenshtein distance ≤2 between dependency
     name and canonical name (concrete algorithm).
   - `malicious_commit_injection`: `git = "..."` dependency without
     `rev = "<full-sha>"` pinning (regex against `Cargo.toml`).
   - `build_pipeline_compromise`: GitHub Actions `uses:` field
     without `@<full-sha>` pinning (regex against
     `.github/workflows/*.yml`).
   - `unmaintained_component`: dependency with no `deny.toml`
     advisory-bypass entry and version spec referencing a stale
     registry entry.
   - `unverified_registry`: `git = ...` or `path = ...` dependency
     without `registry = "..."` reference (regex against
     `Cargo.toml`).
4. For each regression in `regressions_needing_mapping` (from
   select-evidence), propose a backward-compatible `taxonomy_mapping`
   field addition. Format (per `security/regressions/README.md` —
   optional field, existing regressions without it remain valid):
   ```yaml
   taxonomy_mapping:
     osc_r: "<osc_r_category_id>"   # e.g., "dependency_confusion"
     owasp_sc: "<SCxx>"              # e.g., "SC04"
   ```
   Each proposal must include: `regression_id`, `cwe_reference`
   (from the regression's existing `cwe` field),
   `proposed_taxonomy_mapping` (the YAML snippet above),
   `evidence_reference` (which mapping justified this proposal —
   traceable to upstream evidence), `mapping_confidence` (from the
   mapping), `status: pending` (human-curated ratchet — P11).
5. Identify top 3 taxonomy coverage gaps:
   - OSC&R categories with zero mappings (may indicate audit blind
     spot, not absence).
   - OWASP SC categories with zero mappings.
   - Findings with `defense_gap: true` and `regulator_strength:
     missing` (highest-priority remediation targets).
6. Produce verdict:
   - Pass: zero unmapped findings, all OSC&R categories with evidence
     have ≥1 mapping, all proposed `taxonomy_mapping` fields have
     `mapping_confidence: high`.
   - Conditional: 1-5 unmapped findings, OR some mappings with
     `mapping_confidence: medium`, OR some `defense_gap: true`.
   - Fail: >5 unmapped findings, OR any critical/high finding
     unmapped, OR any OSC&R category with evidence but zero
     defense-layer coverage (`regulator_strength: missing` for all
     findings in that category).
7. Emit `cns.taxonomy.report` CNS span with `mappings_count`,
   `unmapped_count`, `osc_r_categories_covered`,
   `owasp_sc_categories_covered`, `proposed_taxonomy_mappings_count`,
   `verdict`, replicant host, latency metric. Namespace is
   registered — emit unconditionally.

### attack-taxonomy-mapper/convergence-check

1. Compute normalized convergence metric [0, 1] where 0 = fully
   converged.
2. Score dimensions (weighted):
   - Unmapped findings (0.40): 0 unmapped = +0.00; any critical/high
     unmapped = +0.40; partial = proportional based on
     `unmapped_findings` count.
   - OSC&R category coverage (0.25): all 6 categories covered by
     ≥1 mapping = +0.00; partial = +0.06 per missing; none = +0.25.
   - OWASP Supply Chain coverage (0.15): all 6 OWASP SC categories
     covered = +0.00; partial = +0.08 per missing; none = +0.15.
   - Regression `taxonomy_mapping` field growth (0.10): new
     `taxonomy_mapping` proposed and accepted in current cycle =
     +0.00; no new proposal despite
     `regressions_needing_mapping` evidence = +0.10 (stagnation).
   - Residual unmapped risk (0.10): zero unmapped findings = +0.00;
     any remaining unmapped finding = +0.10.
3. Start at 0.00, add contributions, clamp to [0, 1].
4. Converged: metric ≤ 0.10 AND relative improvement ≥ 5% from
   previous cycle. If metric has not improved by ≥5%, identify
   blocker (unmapped finding, missing OSC&R category, missing OWASP
   SC category, stagnation, residual risk).
5. Return JSON: `{convergence_metric, previous_metric,
   improvement_pct, converged, blockers, unmapped_findings_count,
   osc_r_categories_covered, osc_r_categories_missing,
   owasp_sc_categories_covered, owasp_sc_categories_missing,
   existing_taxonomy_mappings_count, proposed_taxonomy_mappings_count,
   regressions_needing_mapping_count, residual_unmapped_risk,
   replicant_host, cns_span_emitted: true}`.
6. Emit `cns.taxonomy.convergence` CNS span (registered in
   `CANONICAL_NAMESPACES` — `crates/hkask-types/src/event.rs`).
   Emit unconditionally — namespace IS registered.

## Registry Templates

| Template | Type | Purpose |
|----------|------|----------|
| `select-evidence.j2` | KnowAct | Discover evidence sources (supply-chain-sentinel findings, kali-audit findings, CI logs, regression library); emit `cns.taxonomy.select` span. |
| `map-taxonomy.j2` | KnowAct | Map each finding to OSC&R + OWASP SC taxonomy; apply pragmatic-cybernetics (feedback loop, variety, Good Regulator); emit `cns.taxonomy.map` span per mapping. |
| `taxonomize.j2` | KnowAct | Classify mapped findings into OSC&R + OWASP SC dual taxonomy; produce pattern signatures; propose backward-compatible `taxonomy_mapping` field for existing `surface: supply-chain` regressions; emit `cns.taxonomy.report` span. |
| `convergence-check.j2` | KnowAct | Compute taxonomy-mapping-specific convergence metric (unmapped findings + OSC&R coverage + OWASP SC coverage + regression `taxonomy_mapping` growth + residual risk). Emit `cns.taxonomy.convergence` span. |

## OSC&R Taxonomy Reference

The following OSC&R category mappings are **PROPOSED** — based on
public OSC&R documentation (oscar.io). The actual OSC&R taxonomy IDs
MUST be verified against the live oscar.io framework before registry
commit. This skill does NOT invent OSC&R categories — it maps
existing CWE/OWASP findings to existing OSC&R entries. If a finding
does not fit any listed category, mark it `unmapped` with
`unmapped_reason` — do NOT fabricate a category.

| OSC&R Category (proposed) | CWE Mapping | OWASP Supply Chain (proposed) | Detection Pattern |
|---------------------------|-------------|-------------------------------|-------------------|
| `dependency_confusion` | CWE-829 | SC04 | Manifest references package name not in private registry but exists in public registry |
| `typosquatting` | CWE-829 | SC05 | Dependency name differs from canonical by ≤2 chars (Levenshtein distance) |
| `malicious_commit_injection` | CWE-1104 | SC06 | Git dependency references unverified commit SHA |
| `build_pipeline_compromise` | CWE-1357 | SC07 | CI workflow references untrusted action without SHA pinning |
| `unmaintained_component` | CWE-1104 | SC08 | Dependency with no registry updates in manifest metadata |
| `unverified_registry` | CWE-829 | SC09 | Dependency source is `git = ...` or `path = ...` without registry reference |

**Verification gate:** Before relying on these mappings in production
audit cycles, verify each OSC&R category name and ID against the live
oscar.io framework. The OWASP SC category codes (SC04–SC09) are
similarly proposed and must be verified against the live OWASP
Software Supply Chain Security reference. If verification reveals
different IDs or category names, update `osc_r_categories` in
`map-taxonomy.j2` and `osc_r_categories_all` / `owasp_sc_categories_all`
in `convergence-check.j2` — the registry templates are authoritative
(P5.1).

## Relationship to Existing Skills

- **`supply-chain-sentinel`:** Complementary. `supply-chain-sentinel`
  performs manifest-level audit and proposes `surface: supply-chain`
  regressions with CWE mappings (CWE-1104, CWE-829, CWE-1357). It does
  NOT map to the OSC&R attack taxonomy. This skill consumes those
  findings and adds the OSC&R + OWASP SC taxonomy mapping layer. This
  skill adds an optional, backward-compatible `taxonomy_mapping` field
  to the regression YAML entries `supply-chain-sentinel` produces.
  Zero overlap — audit vs taxonomy mapping layer.
- **`kali-audit`:** Parallel taxonomy discipline. `kali-audit` maps
  findings to OWASP LLM Top 10 / MITRE ATLAS for code/templates/MCP/
  supply-chain surfaces. This skill maps `surface: supply-chain`
  findings to OSC&R + OWASP SC. Distinct taxonomies, distinct domains.
  Zero overlap — OWASP LLM / ATLAS vs OSC&R / OWASP SC.
- **`adversarial-red-team`:** Parallel. `adversarial-red-team` uses
  MITRE ATLAS for LLM adversarial robustness. This skill uses OSC&R
  for supply chain adversarial patterns. Same taxonomy discipline,
  different domain. Zero overlap — LLM boundary vs supply chain.
- **`bug-hunt`:** Structural pattern reuse. `bug-hunt`'s `taxonomize`
  phase classifies findings into Beizer's bug taxonomy (requirements,
  structural, data, coding, interface, integration, timing,
  configuration). This skill classifies supply chain findings into
  OSC&R attack taxonomy. Distinct taxonomies, distinct domains. Pattern
  reuse (classify findings into taxonomy, assign severity, produce
  pattern signatures), no surface overlap.
- **`attack-taxonomy-mapper` does NOT replace any of these:** It fills
  the gap between `supply-chain-sentinel` (CWE-mapped findings) and
  structured attack-pattern context (OSC&R). Without this skill,
  supply chain findings have CWE weakness-category mappings but no
  OSC&R attack-pattern mapping — incident response lacks structured
  attack-pattern context. Passes the P5 deletion test (see design spec
  §3).

## Constraints (Concrete — Not Aspirational)

- `select-evidence.j2`: `visibility: public`.
- `map-taxonomy.j2`: `visibility: public`.
- `taxonomize.j2`: `visibility: public`.
- `convergence-check.j2`: `visibility: public`.
- Every mapping includes concrete evidence: `finding_reference`,
  `cwe_reference`, `osc_r_category`, `owasp_sc_category`,
  `evidence_snippet` (quoted from upstream finding — verifiable).
- Every proposed `taxonomy_mapping` uses the exact YAML format
  (`security/regressions/README.md`) with `osc_r`, `owasp_sc` fields,
  `status: pending`, `evidence_reference` traceable to a mapping.
- No synthetic findings — only consume findings from
  `supply-chain-sentinel` and `kali-audit` as input.
- No invented OSC&R categories — only map to entries in the OSC&R
  category list (which must be verified against live oscar.io per
  §OSC&R Taxonomy Reference note).
- No fabricated unmapped reasons — only `no_osc_r_category_for_cwe`,
  `ambiguous_evidence`, `missing_cwe_reference`.
- The `taxonomy_mapping` field added to regression YAML must be
  backward-compatible (optional field — existing regressions without
  it remain valid).
- Registry (`manifest.yaml` + `.j2`) is authoritative over this
  SKILL.md (P5.1).
- Do NOT propose `taxonomy_mapping` for regressions that already have
  one (skip — P5 minimalism, no duplicate work).
- Do NOT propose `taxonomy_mapping` for regressions without a `cwe`
  field (cannot map without CWE reference — record as
  `unmappable_regression` with reason).
- Every mapping action includes `replicant_host` identity (P12).
- Every mapping emits `cns.taxonomy.map` span. All four namespaces
  (`select`, `map`, `report`, `convergence`) are registered in
  `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`) and
  emitted unconditionally. Do NOT use "if registered; else note gap"
  language — the namespaces ARE registered.
- Apply pragmatic-cybernetics feedback loop analysis: defense layer
  presence/absence for the mapped attack pattern, variety of
  alternative vectors, Good Regulator enforcement strength.
- When multiple OSC&R categories match, record `alternative_mappings`
  for transparency (P11) — do NOT silently pick one.
- Convergence metric computed from real evidence: unmapped findings
  (0.40), OSC&R category coverage (0.25), OWASP SC coverage (0.15),
  regression `taxonomy_mapping` field growth (0.10), residual
  unmapped risk (0.10).
- Do NOT fabricate mappings — only report what was discovered through
  actual upstream finding consumption.
- Source citations must reference concrete sources (not aspirational):
  MITRE CWE definitions (mitre.org), OWASP Supply Chain reference,
  OSC&R framework (oscar.io — verify live taxonomy before mapping),
  `security/regressions/README.md` for regression format.
- This skill does NOT scan manifests directly (that is
  `supply-chain-sentinel`'s responsibility) and does NOT download
  external taxonomy databases (P4 boundary — no ambient authority).
- Propose `taxonomy_mapping` field additions only for
  `surface: supply-chain` regressions; do NOT add the field to other
  surfaces (`code`, `template`, `mcp`, `config`) — OSC&R is a supply
  chain taxonomy, not a code/template/MCP taxonomy.
- Convergence metric must reflect actual coverage, not aspirational:
  OSC&R categories only count as covered when ≥1 evidence-backed
  mapping exists for them; OWASP SC categories likewise;
  `taxonomy_mapping` growth only counts when a new proposal is made
  for a regression that previously lacked the field.

## Source References and Taxonomy Anchors

This skill is anchored to concrete, verifiable taxonomy sources (P8):

- **MITRE CWE:** CWE-1104 (Use of Unmaintained Third-Party Components),
  CWE-829 (Inclusion of Functionality from Untrusted Control Sphere —
  applies to `git`/`path` dependencies without registry verification),
  CWE-1357 (Reliance on Component That is Not Updateable). Source:
  `mitre.org/data/definitions/1104.html` (and related CWE pages).
- **OWASP Supply Chain:** Supply chain security taxonomy for
  dependency management. Source: OWASP Software Supply Chain Security
  reference
  (owasp.org/www-project-software-supply-chain-security/). The SC04–
  SC09 category codes referenced in §OSC&R Taxonomy Reference are
  PROPOSED — verify against the live OWASP reference before registry
  commit.
- **OSC&R Framework:** Open Software Supply Chain Attack Reference —
  ATT&CK-like taxonomy for supply chain threats. Co-created by
  security experts from Google, Microsoft, GitLab. Source: `oscar.io`.
  The OSC&R category names in §OSC&R Taxonomy Reference are PROPOSED
  mappings based on public documentation — verify against the live
  oscar.io framework before registry commit. Do NOT invent OSC&R IDs.
- **`security/regressions/README.md`:** Regression YAML format — the
  `taxonomy_mapping` field extends this format (optional, backward-
  compatible). Source: local project standard — authoritative.
- **`supply-chain-sentinel` SKILL.md:** Source of CWE-mapped findings
  to map (manifest evidence with CWE-1104/CWE-829/CWE-1357 references).
- **`kali-audit` SKILL.md:** Source of OWASP/ATLAS-mapped findings —
  `surface: supply-chain` subset only is relevant to OSC&R mapping.
- **`bug-hunt` SKILL.md:** `taxonomize` phase pattern (classify
  findings into taxonomy, assign severity, produce pattern signatures)
  — adopted for OSC&R classification.
- **`adversarial-red-team` SKILL.md:** Parallel taxonomy discipline —
  ATLAS for LLM adversarial, OSC&R for supply chain adversarial. Same
  discipline, different domain.
- **`pragmatic-cybernetics` SKILL.md:** Feedback loop analysis,
  variety engineering (Ashby's Law), Good Regulator check — applied
  in `map-taxonomy` phase.
- **`principles.md`:** P3.1 (Social Generativity / safety floor),
  P5 (Essentialism / 5W1H gate), P5.1 (Registry canonical), P7
  (Evolutionary), P8 (Semantic Grounding), P9 (Homeostatic
  Self-Regulation), P11 (Visibility), P12 (Replicant Host Mandate) —
  grounding for this skill's design and outputs.
