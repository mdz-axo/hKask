---
name: attack-taxonomy-mapper
visibility: public
description: >
  Attack taxonomy mapping skill for hKask (v0.31.0). Consumes findings from
  supply-chain-sentinel (CWE-mapped manifest evidence) and kali-audit
  (OWASP/ATLAS-mapped findings) as input — does NOT generate new findings.
  Maps each finding to the OSC&R attack taxonomy (dependency confusion,
  typosquatting, malicious commit injection, build pipeline compromise,
  unmaintained component, unverified registry) plus the OWASP Supply Chain
  dual taxonomy. Adds an optional, backward-compatible `taxonomy_mapping`
  field to existing `surface: supply-chain` regression YAML entries. Emits
  cns.taxonomy.* spans (P9 — all registered in CANONICAL_NAMESPACES).
  P8 evidence-backed: every mapping includes concrete evidence (finding
  reference, CWE category, OSC&R category, OWASP SC category). P12
  replicant_host mandatory. Decomposed into 4 phases matching bug-hunt and
  supply-chain-sentinel pipeline. Minimal (P5): answers all 5W1H; single
  skill, no bundle; complements supply-chain-sentinel (adds taxonomy layer)
  and adversarial-red-team (parallel taxonomy discipline: ATLAS for LLM,
  OSC&R for supply chain — zero overlap).
---

# Attack Taxonomy Mapper

{# goal: Consume findings from supply-chain-sentinel and kali-audit (P4 — consumes workspace findings, no external download). Map each finding to OSC&R attack taxonomy + OWASP Supply Chain dual taxonomy. Propose backward-compatible taxonomy_mapping field additions to existing surface: supply-chain regression entries (status: pending, concrete OSC&R + OWASP SC category). Emit cns.taxonomy.* spans (P9). Compute convergence metric from real mapping evidence only. No synthetic findings; no invented OSC&R categories; replicant_host mandatory (P12). OSC&R IDs are PROPOSED mappings — verify against oscar.io. #}

Attack taxonomy mapping. Consumes findings from `supply-chain-sentinel`
(CWE-mapped manifest evidence) and `kali-audit` (OWASP/ATLAS-mapped
findings) as concrete evidence. Maps each finding to the OSC&R attack
taxonomy plus the OWASP Supply Chain dual taxonomy. Proposes backward-
compatible `taxonomy_mapping` field additions to existing
`surface: supply-chain` regression entries. Tracks OSC&R category coverage
and computes a taxonomy mapping convergence metric.

**Important:** This skill does NOT generate new findings. It consumes
existing findings from `supply-chain-sentinel` and `kali-audit` and adds
a taxonomy mapping layer. The OSC&R category names and IDs referenced are
PROPOSED mappings based on public documentation (oscar.io) — they MUST be
verified against the live oscar.io framework before use in production
audit cycles.

## When to Use

- Mapping supply chain findings to structured attack taxonomy (OSC&R).
- Adding taxonomy context to existing `surface: supply-chain` regressions.
- Investigating incidents with structured attack-pattern taxonomy.
- Producing pattern signatures for detecting similar supply chain attacks.
- Computing taxonomy coverage convergence across audit cycles.

## Design Constraints (Grounded in Project Principles)

- **P5 Essentialism (5W1H gate):** Who = supply chain attacker / threat
  actor taxonomy (the subject — the agent is the replicant host, P12);
  What = finding to map / OSC&R taxonomy entry / OWASP SC category;
  Where = workspace / regression library / CI logs; When = audit cycle /
  incident investigation; Why = P3.1 requires structured taxonomy for
  supply chain threats (like MITRE ATLAS for LLM); P8 semantic grounding
  (findings need taxonomy context for severity and remediation priority);
  How = discover findings → read evidence → map to OSC&R → map to OWASP SC
  → taxonomize → propose taxonomy_mapping field → emit CNS span → compute
  convergence. All 6 present — passes gate.
- **P5.1 Registry canonical:** Registry (`manifest.yaml` + `.j2`) is
  source of truth. SKILL.md derived from it.
- **P5.3 Minimalist test:** No new findings generated; no external
  taxonomy download; consumes existing workspace findings only (P4
  boundary — consumes findings, does not generate them).
- **P5.4 Dual-axis:** Each mapping has state identity (finding reference +
  CWE category) and process identity (`map-taxonomy` flow).
- **P7 Evolutionary:** Pattern signatures compound value over time —
  future audits can detect similar attack patterns automatically.
- **P8 Semantic grounding:** Every mapping: finding reference, CWE
  category, OSC&R category, OWASP SC category, evidence snippet, source
  citation (oscar.io, OWASP, MITRE CWE, supply-chain-sentinel SKILL.md,
  kali-audit SKILL.md). No fabricated findings or invented OSC&R categories.
- **P9 CNS regulation:** Emits `cns.taxonomy.select`, `cns.taxonomy.map`,
  `cns.taxonomy.report`, `cns.taxonomy.convergence` spans. All four are
  registered in `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`)
  and emitted unconditionally.
- **P10 Bot/replicant taxonomy:** `visibility: public` — transparent
  taxonomy mapping.
- **P11 Visibility:** `taxonomy_mapping` field proposals default `status:
  pending` (human-curated ratchet, per `security/regressions/README.md`).
- **P12 Replicant host mandate:** Every action includes `replicant_host`.
- **P3.1 Safety floor:** Structured supply chain taxonomy protects the
  Generative Space container — unstructured findings leave attack patterns
  ambiguous.
- **P4 OCAP boundaries:** Consumes existing workspace findings only; no
  external taxonomy download; no external package download.

## Instructions

### attack-taxonomy-mapper/select-evidence

1. Discover evidence sources in the workspace: `supply-chain-sentinel`
   findings (CWE-mapped manifest evidence), `kali-audit` findings with
   `surface: supply-chain` (OWASP/ATLAS-mapped), CI logs, manifest
   evidence.
2. If zero findings to map, return empty `findings_to_map` (do NOT invent
   findings) and recommend running `supply-chain-sentinel` first.
3. Read `security/regressions/` for entries with `surface: supply-chain`
   and any existing `taxonomy_mapping` field. List
   `existing_taxonomy_mappings` (skipping entries that already have
   mappings when proposing new ones).
4. Return JSON: `{source, findings_to_map: [...], existing_taxonomy_mappings:
   [...], evidence_sources: [...], replicant_host}`.
5. Emit `cns.taxonomy.select` CNS span (P9) with discovered evidence
   sources, findings to map, existing mapping count, host identity,
   latency metric.

### attack-taxonomy-mapper/map-taxonomy

1. Read each finding in `findings_to_map`. Extract: finding reference
   (regression ID or finding ID), CWE category, manifest path, evidence
   snippet, severity.
2. For each finding, determine the OSC&R taxonomy category via CWE
   disambiguation:
   - CWE-829 + git/path dependency → dependency confusion / typosquatting /
     unverified registry (disambiguate by evidence pattern).
   - CWE-1104 + no recent registry updates → unmaintained component.
   - CWE-1357 + CI workflow untrusted action → build pipeline compromise.
   - CWE-829 + git dependency unverified commit SHA → malicious commit
     injection.
3. For each OSC&R category, determine the OWASP Supply Chain category
   (dual taxonomy mapping). Note: OWASP SC codes (SC04–SC09) are PROPOSED
   mappings — verify against live OWASP documentation.
4. Apply pragmatic-cybernetics (embedded in instructions — like `bug-hunt`
   `oracle` phase):
   - IS vs OUGHT: describe what the finding IS (CWE + evidence) vs what it
     OUGHT to map to (OSC&R + OWASP SC).
   - Epistemic mode: `Declarative` (finding observed), `Probabilistic`
     (OSC&R inference from CWE + pattern), `Subjunctive` (alternative
     mappings — labeled clearly).
   - Provenance: `Direct measurement` (read finding), `Inference` (CWE →
     OSC&R disambiguation), `Assessment` (dual taxonomy mapping).
5. Apply grill-me self-challenge: Could this finding map to a different
   OSC&R category? Is the CWE → OSC&R mapping ambiguous? Would a reviewer
   dispute? If yes, note alternative mappings and downgrade confidence.
6. Apply pragmatic-cybernetics analysis (feedback loops):
   - Feedback loop: does the OSC&R attack pattern have a corresponding
     defense layer in `supply-chain-sentinel`?
   - Variety: alternative attack vectors in the same OSC&R category?
   - Good Regulator: is the defense layer mapped to the correct OSC&R
     category?
7. For each mapping, produce structured result:
   `finding_reference`, `cwe_category`, `evidence_snippet`,
   `osc_r_category` (name + proposed ID), `owasp_sc_category` (name +
   proposed code), `mapping_confidence` (confirmed/probable/possible),
   `alternative_mappings`, `defense_layer_mapped`, `provenance`,
   `epistemic_mode`, `replicant_host`.
8. Emit `cns.taxonomy.map` CNS span per mapping (`target:
   "cns.taxonomy.map"`, message: `"CNS"`, operation: `"map_taxonomy"`,
   finding_reference, osc_r_category, owasp_sc_category,
   mapping_confidence, replicant_host, latency_ms).

CONSTRAINT — Evidence integrity (P8):
- No synthetic findings. Every `evidence_snippet` must be verifiable by
  reading the cited finding from `supply-chain-sentinel` or `kali-audit`
  output or `security/regressions/` YAML.
- No invented OSC&R categories. Only map to existing entries in the
  oscar.io framework. The OSC&R category names and IDs are PROPOSED
  mappings — verify against live oscar.io before use.
- Source citations must reference concrete URLs or documents actually
  consulted: OSC&R framework (oscar.io), OWASP Supply Chain reference,
  MITRE CWE definitions, supply-chain-sentinel SKILL.md, kali-audit
  SKILL.md.
- Every mapping must include `replicant_host` identity (P12) — no
  anonymous taxonomy mapping.
- This skill complements `supply-chain-sentinel` (adds taxonomy layer to
  its findings) and `kali-audit` (parallel taxonomy discipline). State
  relationship explicitly in reports.
- Minimal (P5): 4 templates, no bundle, no sub-agent delegation. Each
  template answers specific 5W1H: select (Where), map (What + How),
  taxonomize (Why + What), convergence (When + Why).

### attack-taxonomy-mapper/taxonomize

1. Synthesize `mappings` array from `map-taxonomy` phase. Group by OSC&R
   category: dependency_confusion, typosquatting, malicious_commit_injection,
   build_pipeline_compromise, unmaintained_component, unverified_registry.
2. For each mapping with confidence >= probable and concrete evidence,
   propose `taxonomy_mapping` field addition to the corresponding regression
   entry. Format (backward-compatible optional field):
   ```yaml
   taxonomy_mapping:
     osc_r: "T1.2.3"  # PROPOSED — verify against oscar.io
     owasp_sc: "SC04" # PROPOSED — verify against OWASP
   ```
3. Produce pattern signatures for each OSC&R category — concrete grep
   patterns for detecting similar attack patterns in future manifests.
4. Identify top 3 taxonomy coverage gaps (OSC&R categories with no mapped
   findings).
5. Produce verdict:
   - Pass: all findings mapped, >= 4 OSC&R categories covered.
   - Conditional: some findings unmapped, 2-3 OSC&R categories covered.
   - Fail: majority unmapped, < 2 OSC&R categories covered.
6. Emit `cns.taxonomy.report` CNS span with mappings count by OSC&R
   category, OWASP SC coverage, proposed taxonomy_mapping count, pattern
   signatures count, verdict, replicant_host, latency.

### attack-taxonomy-mapper/convergence-check

1. Compute normalized convergence metric [0, 1] where 0 = fully converged.
2. Score dimensions (weighted):
   - Unmapped findings (critical/high) (0.40): 0 = +0.00; 1+ = +0.40.
   - OSC&R category coverage (0.25): all 6 = +0.00; partial = +0.04 per
     missing; 0 = +0.25.
   - OWASP Supply Chain coverage (0.15): all = +0.00; partial = +0.08;
     none = +0.15.
   - Regression `taxonomy_mapping` field growth (0.10): new field proposed
     = +0.00; stagnation = +0.10.
   - Residual unmapped risk (0.10): zero unmapped = +0.00; any = +0.10.
3. Start at 0.00, add contributions, clamp to [0, 1].
4. Converged: metric ≤ 0.10 AND relative improvement ≥ 5% from previous
   cycle.
5. Return JSON: `{convergence_metric, dimensions, rationale, blockers,
   oscr_categories_covered, oscr_categories_missing, existing_taxonomy_mappings,
   proposed_taxonomy_mappings, cns_span_emitted: true}`.
6. Emit `cns.taxonomy.convergence` CNS span (registered in
   `CANONICAL_NAMESPACES` — emitted unconditionally).

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `select-evidence.j2` | KnowAct | Discover evidence sources; read regression library for existing taxonomy_mappings; emit `cns.taxonomy.select` span. |
| `map-taxonomy.j2` | KnowAct | Map findings to OSC&R + OWASP SC dual taxonomy via CWE disambiguation; apply pragmatic-cybernetics; emit `cns.taxonomy.map` spans. |
| `taxonomize.j2` | KnowAct | Classify mappings by OSC&R category; produce pattern signatures; propose backward-compatible `taxonomy_mapping` field; emit `cns.taxonomy.report` span. |
| `convergence-check.j2` | KnowAct | Compute taxonomy coverage convergence metric (OSC&R coverage + OWASP SC coverage + mapping growth). Emit `cns.taxonomy.convergence` span. |

## OSC&R Taxonomy Reference

**IMPORTANT:** The mappings below are PROPOSED based on public OSC&R
documentation (oscar.io). The OSC&R category names and IDs MUST be
verified against the live oscar.io framework before use in production
audit cycles. Do NOT present proposed mappings as verified.

| OSC&R Category | CWE Mapping | OWASP SC (proposed) | Detection Pattern |
|----------------|-------------|---------------------|-------------------|
| Dependency confusion | CWE-829 | SC04 | Package name exists in both private and public registry |
| Typosquatting | CWE-829 | SC05 | Dependency name differs from canonical by ≤2 chars (Levenshtein) |
| Malicious commit injection | CWE-829 | SC06 | Git dependency references unverified commit SHA |
| Build pipeline compromise | CWE-1357 | SC07 | CI workflow references untrusted action without SHA pinning |
| Unmaintained component | CWE-1104 | SC08 | Dependency with no registry updates in manifest metadata |
| Unverified registry | CWE-829 | SC09 | Dependency source is `git = ...` or `path = ...` without registry reference |

New OSC&R categories can be added as real findings justify them (P7) —
not speculatively. This taxonomy reference is distinct from `kali-audit`'s
OWASP LLM / ATLAS catalog and `supply-chain-sentinel`'s CWE catalog.

## Relationship to Existing Skills

- **`supply-chain-sentinel`:** `supply-chain-sentinel` performs manifest-
  level audit and proposes regressions with CWE mappings.
  `attack-taxonomy-mapper` consumes those findings and adds OSC&R + OWASP
  SC taxonomy layer. Complementary — finding generation vs taxonomy
  mapping. Zero overlap.
- **`kali-audit`:** `kali-audit` maps to OWASP LLM / ATLAS for LLM/code.
  `attack-taxonomy-mapper` maps to OSC&R for supply chain. Parallel
  taxonomy discipline, distinct taxonomies. Zero overlap.
- **`adversarial-red-team`:** Uses MITRE ATLAS for LLM adversarial.
  `attack-taxonomy-mapper` uses OSC&R for supply chain adversarial. Same
  taxonomy discipline, different domain. Zero overlap.
- **`bug-hunt`:** Provides decomposed pipeline structure (`Charter` →
  `Probe` → `Oracle` → `Taxonomize` → `Report`). This skill replicates
  that structure (`select-evidence` ≈ charter; `map-taxonomy` ≈ probe +
  oracle; `taxonomize` ≈ taxonomize + report; `convergence-check` ≈
  convergence). Uses same pragmatic-cybernetics and pragmatic-semantics
  reasoning embedded in instructions.
- **`attack-taxonomy-mapper` does NOT replace any of these:** It fills
  the taxonomy mapping gap. No existing skill maps supply chain findings
  to the OSC&R attack taxonomy.

## Constraints (Concrete — Not Aspirational)

- `select-evidence.j2`: `visibility: public`.
- `map-taxonomy.j2`: `visibility: public`.
- `taxonomize.j2`: `visibility: public`.
- `convergence-check.j2`: `visibility: public`.
- Every mapping includes concrete finding reference, CWE category, OSC&R
  category, OWASP SC category, evidence snippet, source citation — not
  summary description.
- Every proposed `taxonomy_mapping` field uses backward-compatible
  optional YAML format with `osc_r` and `owasp_sc` keys.
- No synthetic findings — only map findings from `supply-chain-sentinel`
  or `kali-audit`.
- No invented OSC&R categories — only map to existing entries in oscar.io.
- No fabricated mappings; read finding evidence before mapping.
- Registry (`manifest.yaml` + `.j2`) is authoritative over this SKILL.md
  (P5.1).
- Do NOT invent OSC&R category IDs not in the oscar.io framework.
- Do NOT claim taxonomy coverage that hasn't been verified through actual
  finding mapping.
- Every mapping action includes `replicant_host` identity (P12).
- Every taxonomy mapping operation emits `cns.taxonomy.*` span. All four
  namespaces are registered in `CANONICAL_NAMESPACES`
  (`crates/hkask-types/src/event.rs`) and emitted unconditionally.
- Apply pragmatic-cybernetics feedback loop analysis: mapping polarity,
  variety of OSC&R categories, Good Regulator (defense layer mapped
  correctly?).
- Apply `grill-me` self-challenge before proposing mappings.
- Apply `IS/OUGHT` classification and label `epistemic_mode` and
  `provenance` for every mapping.
- Convergence metric computed from real evidence: unmapped findings (0.40),
  OSC&R coverage (0.25), OWASP SC coverage (0.15), mapping growth (0.10),
  residual unmapped risk (0.10).
- Do NOT fabricate mappings — only report what was discovered through
  actual finding analysis.
- Source citations must reference concrete sources: OSC&R framework
  (oscar.io), OWASP Supply Chain reference (owasp.org), MITRE CWE
  definitions (mitre.org), `security/regressions/README.md`,
  `supply-chain-sentinel` SKILL.md, `kali-audit` SKILL.md.
- If evidence discovery finds zero findings to map, return empty
  `findings_to_map` and recommend running `supply-chain-sentinel` first —
  do NOT invent findings.
- Before proposing any `taxonomy_mapping` field, verify the finding
  reference exists and evidence snippet can be quoted from the actual
  finding.
- This skill does NOT generate new findings or download external packages.
  It maps existing findings to taxonomy entries only (P4 boundary —
  consumes workspace findings, no external taxonomy download).
- The `taxonomy_mapping` field is backward-compatible (optional) — existing
  regressions without it remain valid. `scripts/check-kali-regressions.sh`
  should not break (verify before first `status: enforced` flip).
- OSC&R category names and IDs are PROPOSED mappings — verify against live
  oscar.io before use in production audit cycles. Do NOT present proposed
  mappings as verified.

## Source References and Taxonomy Anchors

This skill is anchored to concrete, verifiable taxonomy sources (P8):

- **OSC&R Framework:** Open Software Supply Chain Attack Reference —
  ATT&CK-like taxonomy for supply chain threats. Co-created by security
  experts from Google, Microsoft, GitLab. Source: `oscar.io` (verify live
  taxonomy before use — proposed mappings must be confirmed).
- **OWASP Supply Chain:** OWASP Software Supply Chain Security reference.
  Source: `owasp.org/www-project-software-supply-chain-security/`.
- **MITRE CWE:** CWE-1104 (Unmaintained Third-Party Components), CWE-829
  (Inclusion from Untrusted Control Sphere), CWE-1357 (Reliance on
  Component Not Updateable). Source: `mitre.org/data/definitions/`.
- **`supply-chain-sentinel` SKILL.md:** Source of findings to map
  (CWE-mapped manifest evidence).
- **`kali-audit` SKILL.md:** Source of OWASP/ATLAS-mapped findings;
  parallel taxonomy discipline reference.
- **`adversarial-red-team` SKILL.md:** Parallel taxonomy discipline
  reference (ATLAS for LLM, OSC&R for supply chain).
- **`bug-hunt` SKILL.md:** `taxonomize` phase pattern (classify findings
  into taxonomy, assign severity, produce pattern signatures).
- **`security/regressions/README.md`:** Regression YAML format — the
  `taxonomy_mapping` field extends this format (backward-compatible).
