---
name: runtime-posture-monitor
visibility: public
description: >
  Runtime security posture monitoring skill for hKask (v0.31.0). Observes
  runtime telemetry (hkask.* performative spans, cns.guard.* violations,
  cns.regulation events) to detect runtime threats: API endpoint abuse,
  bot traffic, LLM usage anomalies, and runtime dependency behavior
  anomalies. Distinct from supply-chain-sentinel (static manifest audit,
  P4 manifest boundary) — this skill observes runtime behavior (P4
  runtime boundary). Anchored to MITRE CWE (CWE-1357, CWE-829, CWE-200),
  OWASP LLM Top 10 (LLM06, LLM07), MITRE ATLAS (AML.TA0010). Consumes
  security/regressions/; proposes RR-NNNN.yaml entries with surface:
  runtime. Emits cns.runtime.* spans (P9 — all registered in
  CANONICAL_NAMESPACES). Decomposed into 4 phases matching bug-hunt,
  kali-audit, and supply-chain-sentinel pipeline. Minimal (P5): answers
  all 5W1H; single skill, no bundle; complements supply-chain-sentinel
  (distinct P4 boundaries) and adversarial-red-team (synthetic vs real
  traffic — zero overlap).
---

# Runtime Posture Monitor

{# goal: Observe runtime telemetry (hkask.* performative spans, cns.guard.* violations, cns.regulation events) within deployed replicant host (P4 runtime boundary). Classify runtime threats (endpoint abuse, bot traffic, LLM usage spike, dependency behavior anomaly). Map to MITRE CWE-1357/CWE-829/CWE-200, OWASP LLM06/LLM07, ATLAS AML.TA0010. Emit cns.regulation and cns.guard.violation for downstream action. Propose concrete RR-NNNN.yaml entries (surface: runtime, status: pending, concrete grep pattern against span target). Emit cns.runtime.* spans (P9). Compute convergence metric from real runtime evidence only. No synthetic signals; no external endpoint scanning; replicant_host mandatory (P12). #}

Runtime security posture monitoring. Observes hKask's own CNS telemetry
(`hkask.*` performative spans, `cns.guard.*` violations, `cns.regulation`
events) as concrete evidence. Maps runtime threats to MITRE CWE / OWASP
LLM / MITRE ATLAS taxonomy. Proposes CI-enforced regressions
(`surface: runtime`). Tracks defense-layer firing coverage (6 runtime
layers — distinct from `kali-audit`'s 8 static layers and
`supply-chain-sentinel`'s 4 manifest layers) and computes a runtime
posture convergence metric.

## When to Use

- Monitoring a deployed replicant host for runtime security anomalies.
- Investigating runtime defense-layer firing patterns (`cns.guard.*`
  violations increasing? `cns.regulation.action_blocked` not keeping
  pace?).
- Verifying that static defense layers (checked by `kali-audit`) actually
  fire at runtime.
- Proposing `security/regressions/` entries backed by runtime span
  evidence.
- Computing runtime-posture-specific convergence across monitoring cycles.

## Design Constraints (Grounded in Project Principles)

- **P5 Essentialism (5W1H gate):** Who = running application / replicant
  host (P12); What = runtime signal / span target / threat pattern;
  Where = deployed runtime environment / production workload; When =
  observation window / continuous monitoring (not audit cycle); Why =
  P3.1 safe container requires runtime blocking (Aikido/Zen firewall
  model); How = discover signals → observe → classify → regulate →
  report → propose regression → emit CNS span → compute convergence.
  All 6 present — passes gate.
- **P5.1 Registry canonical:** Registry (`manifest.yaml` + `.j2`) is
  source of truth. SKILL.md derived from it.
- **P5.3 Minimalist test:** No external package download; no OS-level
  endpoint scanning; observes hKask's own CNS telemetry only (P4 runtime
  boundary — distinct from `supply-chain-sentinel`'s manifest boundary).
- **P5.4 Dual-axis:** Each finding has state identity (span target +
  timestamp) and process identity (`classify-threat` flow).
- **P7 Evolutionary:** Defense-layer firing patterns emerge from real
  runtime telemetry, not speculation.
- **P8 Semantic grounding:** Every claim: span target, timestamp, signal
  value, baseline reference, evidence snippet, source citation (MITRE
  CWE URL, OWASP LLM reference, ATLAS reference, hkask-guard docs,
  hkask-cns docs). No fabricated CVEs or synthetic span observations.
- **P9 CNS regulation:** Emits `cns.runtime.select`, `cns.runtime.classify`,
  `cns.runtime.regulate`, `cns.runtime.convergence` spans. All four are
  registered in `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`)
  and emitted unconditionally. Also emits `cns.regulation` and
  `cns.guard.violation` for downstream consumption (both registered).
- **P10 Bot/replicant taxonomy:** `visibility: public` — transparent
  runtime monitoring.
- **P11 Visibility:** Regression proposals default `status: pending`
  (human-curated ratchet, per `security/regressions/README.md`).
- **P12 Replicant host mandate:** Every action includes `replicant_host`.
- **P3.1 Safety floor:** Runtime threat detection protects the Generative
  Space container — runtime compromise is destructive to the space itself.
- **P4 OCAP boundaries:** Observes only hKask's own CNS telemetry; no
  external endpoint scanning; no OS-level process inspection without
  explicit consent (P2).

## Instructions

### runtime-posture-monitor/select-signal

1. Discover runtime signal sources in the deployed replicant host:
   `hkask.*` performative spans, `cns.guard.*` violation spans
   (`cns.guard.input`, `cns.guard.output`, `cns.guard.canary`,
   `cns.guard.runtime_policy`), `cns.regulation` events
   (`cns.regulation.action_blocked`, `cns.regulation.action_substituted`,
   `cns.regulation.plateau_detected`), `cns.tool.*` invocations,
   `cns.inference` calls.
2. If zero signal sources found, return empty `signal_sources` (do NOT
   invent signals) and recommend `signal: guard` or `signal: regulation`
   based on deployed components.
3. Read `security/regressions/` for entries with `surface: runtime`.
   List `existing_regressions` (skipping `enforced` duplicates when
   proposing new entries).
4. Verify defense layers to check (runtime firing, not static presence):
   `input_filtering` (`cns.guard.input` firing), `output_filtering`
   (`cns.guard.output` firing), `canary_detection` (`cns.guard.canary`
   firing), `runtime_policy` (`cns.guard.runtime_policy` firing),
   `regulation_loop` (`cns.regulation` events observed),
   `action_distribution_monitoring` (`cns.regulation.loop_quality`
   observed).
5. Return JSON: `{signal, signal_sources: [...], signal_types: [...],
   defense_layers: [...], existing_regressions: [...], replicant_host}`.
6. Emit `cns.runtime.select` CNS span (P9) with discovered signal sources,
   signal selection, regression count, defense layers, host identity,
   latency metric.

### runtime-posture-monitor/classify-threat

1. Observe each signal source in `signal_sources` within the observation
   window. Record span target, timestamp, signal value, baseline reference.
2. For `cns.guard.*` signals: count violations per scanner. Note violation
   frequency vs baseline.
3. For `cns.regulation.*` signals: count regulation actions. Note whether
   actions are increasing (system struggling) or decreasing (stable).
4. For `cns.tool.*` signals: count tool invocations per tool type. Note
   invocation rate, error rate, unusual tool call chains.
5. For `cns.inference` signals: count LLM inference calls. Note inference
   rate, token consumption, usage spikes.
6. Apply pragmatic-cybernetics (embedded in instructions — like `bug-hunt`
   `oracle` phase):
   - IS vs OUGHT: describe observed signal pattern (`IS`) vs expected
     baseline (`OUGHT` — stable, regulated, firing defenses).
   - Epistemic mode: `Declarative` (span observed), `Probabilistic`
     (threat inference from pattern), `Subjunctive` (potential runtime
     risk — labeled clearly, not presented as fact).
   - Provenance: `Direct measurement` (read span), `Inference` (pattern
     analysis), `Assessment` (security taxonomy mapping) — label each
     finding explicitly.
7. Apply grill-me self-challenge: Could this signal pattern be intentional?
   Is the usage spike a deliberate load test? Would a reviewer dismiss?
   If yes, downgrade or omit. Only propose concrete findings with quoted
   span evidence.
8. Apply pragmatic-cybernetics analysis (feedback loops): trace signal
   polarity (increasing/decreasing risk?), check variety (alternative
   signal sources for same threat?), Good Regulator (is defense layer
   regulating the threat?).
9. For each classified threat, produce structured finding:
   `threat_type`, `signal_target`, `timestamp`, `signal_value`,
   `baseline_value`, `deviation_pct`, `severity` (critical/high/medium/low
   — justified by evidence), `provenance`, `epistemic_mode`,
   `defense_layers_firing`, `defense_layers_silent`, `evidence_snippet`
   (quoted span target + timestamp + value), `source_citation` (MITRE
   CWE reference URL, OWASP LLM reference, ATLAS reference, hkask-guard
   docs, hkask-cns docs).
10. Emit `cns.runtime.classify` CNS span per classified threat
    (`target: "cns.runtime.classify"`, message: `"CNS"`, operation:
    `"classify_threat"`, threat_type, signal_target, severity,
    replicant_host, latency_ms).

CONSTRAINT — Evidence integrity (P8):
- No synthetic span observations. Every `evidence_snippet` must be
  verifiable by querying the CNS span history for the cited target and
  timestamp.
- No synthetic CVE numbers. Only reference MITRE CWE taxonomy categories:
  CWE-1357 (Reliance on Component Not Updateable — runtime dependency
  behavior), CWE-829 (Inclusion from Untrusted Control Sphere — runtime
  untrusted input), CWE-200 (Information Exposure — runtime endpoint
  exposure). These are taxonomy mappings, not vulnerability claims.
- Source citations must reference concrete URLs or documents actually
  consulted: MITRE CWE definitions, OWASP LLM Top 10 2025, MITRE ATLAS,
  hkask-guard pipeline docs, hkask-cns cybernetics loop docs.
- Every finding must include `replicant_host` identity (P12) — no
  anonymous runtime scanning.
- When referencing `security/regressions/`, read actual YAML files; do
  not invent regression entries.
- This skill complements `supply-chain-sentinel` (static manifest audit,
  distinct P4 boundary) by providing runtime behavior observation. It
  complements `adversarial-red-team` (synthetic LLM I/O attacks — zero
  overlap). State relationship explicitly in reports.
- Minimal (P5): 4 templates, no bundle, no sub-agent delegation. Each
  template answers specific 5W1H: select (Where), classify (What + How),
  regulate (Why + What), convergence (When + Why).

### runtime-posture-monitor/emit-regulation

1. Synthesize `threats` array from `classify-threat` phase. Group by
   severity: critical (defense-layer bypass + active threat), high
   (defense-layer firing but threat persists), medium (anomaly detected,
   defense regulating), low (minor deviation from baseline).
2. For each finding: include `threat_type`, `signal_target`, `timestamp`,
   `evidence_snippet`, `severity`, `cwe_reference`, `owasp_reference`,
   `atlas_reference`, `taxonomy_mapping`, `defense_layers_firing`,
   `defense_layers_silent`, `remediation_recommendation`,
   `regulation_action_emitted`, `guard_violation_emitted`, `replicant_host`.
3. For each critical/high threat:
   a. Emit `cns.regulation` event (feeds CyberneticsLoop for regulation
      action selection).
   b. If blocking warranted (critical severity, defense-layer bypass),
      emit `cns.guard.violation` (triggers defensive blocking).
4. Propose regression entry for findings with severity >= medium (only
   when evidence is concrete). Use exact YAML format from
   `security/regressions/README.md`: `surface: runtime`, `cwe: CWE-XXX`,
   `owasp_llm_2025: LLMXX`, `atlas_tactic: AML.TAXXXX`,
   `discovered_in: <span_target>`, `status: pending`,
   `detection: kind: grep`, `pattern: "..."` (concrete regex against
   span target or signal value).
5. Identify defense-layer firing gaps (e.g., `input_filtering` firing but
   `regulation_loop` silent). Propose top 3 highest-priority fixes.
6. Produce verdict:
   - Pass: zero critical/high threats, >= 5 defense layers firing.
   - Conditional: medium threats present or 3-4 defense layers firing.
   - Fail: critical/high threats present or < 3 defense layers firing.
7. Emit `cns.runtime.regulate` CNS span with threats count by severity,
   defense layers firing/silent, proposed regression count, regulation
   events emitted, guard violations emitted, verdict, replicant host,
   latency metric.

### runtime-posture-monitor/convergence-check

1. Compute normalized convergence metric [0, 1] where 0 = fully converged.
2. Score dimensions (weighted):
   - Critical + high threats resolved (0.40): 0 critical/high = +0.00;
     1+ critical/high unresolved = +0.40; partial resolution = proportional.
   - Defense-layer firing coverage (0.25): 6 layers firing = +0.00;
     5 = +0.04; 4 = +0.08; 3 = +0.13; 2 = +0.19; 1 = +0.25.
   - Threat-pattern taxonomy coverage (0.15): all 4 threat types
     (endpoint_abuse, bot_traffic, llm_usage_spike,
     dependency_behavior_anomaly) covered = +0.00; partial = +0.04 per
     missing; 0 = +0.15.
   - Regression library growth (0.10): new `surface: runtime` regression
     proposed and accepted = +0.00; no new regression despite evidence =
     +0.10 (stagnation).
   - Residual runtime risk (0.10): unresolved runtime anomalies remaining
     = +0.10; all resolved = +0.00.
3. Start at 0.00, add contributions, clamp to [0, 1].
4. Converged: metric ≤ 0.10 AND relative improvement ≥ 5% from previous
   cycle. If not improved by ≥5%, identify blocker.
5. Return JSON: `{convergence_metric, dimensions, rationale, blockers,
   defense_layers_firing, defense_layers_silent, existing_regressions,
   proposed_regressions, cns_span_emitted: true}`.
6. Emit `cns.runtime.convergence` CNS span (registered in
   `CANONICAL_NAMESPACES` — emitted unconditionally).

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `select-signal.j2` | KnowAct | Discover runtime signal sources; read regression library; emit `cns.runtime.select` span. |
| `classify-threat.j2` | KnowAct | Observe runtime signals; classify threats; apply pragmatic-cybernetics; emit `cns.runtime.classify` spans. |
| `emit-regulation.j2` | KnowAct | Synthesize threats with CWE/OWASP/ATLAS taxonomy; emit `cns.regulation` and `cns.guard.violation`; propose `RR-NNNN.yaml` entries (`surface: runtime`); emit `cns.runtime.regulate` span. |
| `convergence-check.j2` | KnowAct | Compute runtime-posture-specific convergence metric (defense-layer firing coverage + regression growth + residual risk). Emit `cns.runtime.convergence` span. |

## Defense-Layer Catalog (Runtime Specific)

| Layer | Name | Evidence Source | Source Citation |
|-------|------|-----------------|-----------------|
| 1 | Input filtering (runtime firing) | `cns.guard.input` span emission count | `hkask-guard` pipeline |
| 2 | Output filtering (runtime firing) | `cns.guard.output` span emission count | `hkask-guard` pipeline |
| 3 | Canary token detection (runtime firing) | `cns.guard.canary` span emission count | `hkask-guard` pipeline |
| 4 | Runtime policy enforcement | `cns.guard.runtime_policy` span emission count | `hkask-templates` executor |
| 5 | Regulation loop active | `cns.regulation` span emission count | `hkask-cns` cybernetics loop |
| 6 | Action distribution monitoring | `cns.regulation.loop_quality` span | `hkask-cns` regulation policy |

New layers can be added as real runtime patterns justify them (P7) — not
speculatively. This catalog is distinct from `kali-audit`'s 8-layer static
catalog and `supply-chain-sentinel`'s 4-layer manifest catalog.

## Relationship to Existing Skills

- **`supply-chain-sentinel`:** `supply-chain-sentinel` audits static
  dependency manifests (P4 manifest boundary). `runtime-posture-monitor`
  observes runtime behavior (P4 runtime boundary — distinct surface).
  They are complementary (like `kali-audit` + `adversarial-red-team`).
  Zero overlap — distinct P4 boundaries.
- **`kali-audit`:** `kali-audit` checks static defense-layer presence
  (8 layers in code). `runtime-posture-monitor` observes whether those
  layers actually fire at runtime (6 runtime firing layers). Complementary
  — static presence vs runtime firing.
- **`adversarial-red-team`:** Covers synthetic LLM I/O adversarial
  robustness (prompt injection, exfiltration, 7 attack categories).
  `runtime-posture-monitor` observes real runtime traffic. Zero overlap —
  synthetic vs real.
- **`bug-hunt`:** Provides decomposed pipeline structure (`Charter` →
  `Probe` → `Oracle` → `Taxonomize` → `Report`). This skill replicates
  that structure (`select-signal` ≈ charter; `classify-threat` ≈ probe +
  oracle; `emit-regulation` ≈ taxonomize + report; `convergence-check` ≈
  convergence). Uses same pragmatic-cybernetics and pragmatic-semantics
  reasoning embedded in instructions.
- **`runtime-posture-monitor` does NOT replace any of these:** It fills
  the runtime observation gap. No existing skill observes hKask's own CNS
  telemetry for runtime security posture.

## Constraints (Concrete — Not Aspirational)

- `select-signal.j2`: `visibility: public`.
- `classify-threat.j2`: `visibility: public`.
- `emit-regulation.j2`: `visibility: public`.
- `convergence-check.j2`: `visibility: public`.
- Every finding includes concrete span target, timestamp, signal value,
  baseline reference, quoted evidence snippet, source citation — not
  summary description.
- Every proposed regression uses exact YAML format (`security/regressions/`)
  with `surface: runtime`, concrete `pattern` (grep regex against span
  target or signal value), `status: pending`, `cwe: CWE-XXX`,
  `owasp_llm_2025: LLMXX`, `atlas_tactic: AML.TAXXXX`.
- No synthetic span observations; query CNS span history before quoting.
- No synthetic CVE references; only MITRE CWE / OWASP LLM / ATLAS taxonomy
  categories as mappings, not vulnerability claims.
- No fabricated runtime signals; only observe spans actually emitted by
  the running system.
- Registry (`manifest.yaml` + `.j2`) is authoritative over this SKILL.md
  (P5.1).
- Do NOT invent span targets not emitted by the running system.
- Do NOT claim external endpoint scanning or OS-level process inspection
  capability — CNS telemetry observation only (P4 boundary enforcement).
- Every scan action includes `replicant_host` identity (P12).
- Every security-sensitive runtime operation emits `cns.runtime.*` span.
  All four namespaces are registered in `CANONICAL_NAMESPACES`
  (`crates/hkask-types/src/event.rs`) and emitted unconditionally.
- Apply pragmatic-cybernetics feedback loop analysis: signal polarity,
  variety of signal sources, Good Regulator (defense layer regulating?).
- Apply `grill-me` self-challenge before proposing findings.
- Apply `IS/OUGHT` classification and label `epistemic_mode` and
  `provenance` for every finding.
- Convergence metric computed from real evidence: unresolved critical/high
  threats (0.40), defense-layer firing coverage (0.25), threat-pattern
  taxonomy coverage (0.15), regression library growth (0.10), residual
  runtime risk (0.10).
- Do NOT fabricate findings — only report what was discovered through
  actual span observation.
- Source citations must reference concrete sources: MITRE CWE definitions
  (mitre.org), OWASP LLM Top 10 2025 (owasp.org), MITRE ATLAS
  (atlas.mitre.org), `security/regressions/README.md`, hkask-guard docs,
  hkask-cns docs.
- If signal discovery finds zero runtime signals, return empty
  `signal_sources` and recommend `signal: guard` or `signal: regulation`
  based on deployed components — do NOT invent signals.
- Before proposing any regression entry, verify span target was actually
  emitted and evidence snippet can be quoted from CNS span history.
- This skill does NOT scan OS-level endpoints or download external
  packages. It observes hKask's own CNS telemetry within the deployed
  replicant host (P4 OCAP enforcement perimeter — runtime CNS boundary).
- Propose `surface: runtime` regression entries only; do NOT reuse
  `surface: code`, `surface: template`, `surface: mcp`, `surface: config`,
  or `surface: supply-chain` — runtime threats have distinct defense-layer
  catalog (6 firing layers) distinct from `kali-audit`'s 8-layer static
  catalog and `supply-chain-sentinel`'s 4-layer manifest catalog.
- Convergence metric must reflect actual runtime coverage, not aspirational:
  defense layers only count as firing when actual `cns.guard.*` or
  `cns.regulation.*` span emissions confirm them.

## Source References and Taxonomy Anchors

This skill is anchored to concrete, verifiable taxonomy sources (P8):

- **MITRE CWE:** CWE-1357 (Reliance on Component Not Updateable — runtime
  dependency behavior), CWE-829 (Inclusion from Untrusted Control Sphere —
  runtime untrusted input), CWE-200 (Information Exposure — runtime
  endpoint exposure). Source: `mitre.org/data/definitions/`.
- **OWASP LLM Top 10 (2025):** LLM06 (Excessive Agency — runtime tool
  misuse), LLM07 (System Prompt Leakage — runtime canary detection).
  Source: `owasp.org/www-project-top-10-for-large-language-model-applications/`.
- **MITRE ATLAS:** AML.TA0010 (Exfiltration — runtime data exfiltration
  detection). Source: `atlas.mitre.org`.
- **`hkask-guard` pipeline:** `cns.guard.*` span sources (runtime evidence).
  Source: `crates/hkask-guard/src/pipeline.rs`.
- **`hkask-cns` cybernetics loop:** `cns.regulation` span sink (downstream
  regulation action). Source: `crates/hkask-cns/src/cybernetics_loop.rs`.
- **`security/regressions/README.md`:** Regression YAML format and ratchet
  lifecycle. Source: local project standard — authoritative.
- **Aikido Security** (`aikido.dev`): ASPM, auto-triage, runtime blocking
  model (context reference — not replacement).
- **Huntress** (`huntress.com`): Managed EDR/MDR (context — distinct
  surface, zero overlap per P5 minimal test).
