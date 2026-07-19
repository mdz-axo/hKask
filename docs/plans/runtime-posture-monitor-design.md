---
title: "Skill Design Spec — runtime-posture-monitor"
audience: [architects, replicants, security auditors]
last_updated: 2026-07-18
status: draft
version: 0.1.0
domain: security / runtime / application-security
skill_status: draft — not registry-committed
---

# Skill Design Spec — `runtime-posture-monitor`

**Status:** `draft` — not registry-committed. Per P5 Essentialism, this skill is
documented for evolutionary architecture (P7) but NOT implemented to avoid
speculative abstraction. A `manifest.yaml` and `.j2` templates will be created
under `registry/templates/runtime-posture-monitor/` only after this design
passes the 5W1H gate review and the `cns.runtime.*` namespace proposal is
accepted.

This spec follows the same structure as `docs/plans/security-skills.md` Skill2.
It is the design artifact referenced by that plan.

## 1. Identity

| Field | Value |
|-------|-------|
| Skill name | `runtime-posture-monitor` |
| Version | 0.1.0 (proposed) |
| Visibility | `public` (P10, P11) |
| Type | Skill (PDCA FlowDef with convergence threshold + energy budget) |
| Surface parameter | `runtime` (single surface — distinct from `supply-chain-sentinel`'s manifest surface and `kali-audit`'s 5 surfaces) |
| Decomposition | 4 templates: `select-signal` → `classify-threat` → `emit-regulation` → `convergence-check` |
| CNS spans | `cns.runtime.select`, `cns.runtime.classify`, `cns.runtime.regulate`, `cns.runtime.convergence` (proposed — NOT yet registered in `CANONICAL_NAMESPACES`) |

## 2. 5W1H Gate (P5 Essentialism)

| Question | Answer |
|----------|--------|
| **Who** | Running application / replicant host (P12 — `replicant_host` mandatory in every output) |
| **What** | API endpoint exposure / bot detection / LLM usage anomalies / runtime dependency behavior |
| **Where** | Runtime environment / production workload (NOT workspace manifest — distinct from `supply-chain-sentinel`) |
| **When** | Continuous (not audit cycle) — observes `hkask.*` performative spans as they are emitted |
| **Why** | P3.1 safe container requires runtime blocking (Aikido/Zen firewall model: block attacks without code change). Static audit (`kali-audit`, `supply-chain-sentinel`) cannot detect runtime anomalies. |
| **How** | Observe runtime signals (`hkask.*` performative spans) → classify threat patterns → emit regulation events (`cns.regulation`) → trigger defensive action (`cns.guard.violation`) |

**Gate verdict:** All 6 answered. Passes P5.1 minimalist test (see §3).

## 3. P5 Minimal Test (Deletion Test)

**Does this skill replace an existing capability?** No. `kali-audit` audits
static code/templates/manifests. `supply-chain-sentinel` audits static
dependency manifests. `adversarial-red-team` probes the LLM I/O boundary
with synthetic attacks. None of them observe **runtime** behavior of a
deployed replicant.

**Does this skill download external packages?** No. Reads only `hkask.*`
performative spans already emitted by the running system (P4 runtime
boundary — distinct surface from `supply-chain-sentinel`'s manifest
boundary).

**Does this skill replace endpoint detection (Huntress)?** No. Zero overlap
with Huntress (EDR/MDR — endpoint threat detection). This skill observes
hKask's own CNS telemetry, not OS-level endpoint signals.

**Deletion test:** If this skill is deleted, runtime anomalies (API endpoint
exposure, bot detection, LLM usage spikes) would not be observed by any
existing hKask skill. Complexity reappears in ad-hoc monitoring scripts.
Passes deletion test — skill earns its existence.

## 4. Relationship to Existing Skills

| Skill | Relationship | Overlap |
|-------|--------------|---------|
| `supply-chain-sentinel` | Complementary — `supply-chain-sentinel` audits static dependency integrity (P4 manifest boundary); `runtime-posture-monitor` observes runtime dependency behavior (P4 runtime boundary — distinct surface). Like `kali-audit` + `adversarial-red-team`. | Zero (distinct P4 boundaries) |
| `kali-audit` | Complementary — `kali-audit` checks static defense-layer presence (8 layers); `runtime-posture-monitor` observes whether those layers fire at runtime. | Zero (static vs runtime) |
| `adversarial-red-team` | Complementary — `adversarial-red-team` probes LLM I/O with synthetic attacks; `runtime-posture-monitor` observes real runtime traffic for anomalies. | Zero (synthetic vs real) |
| `bug-hunt` | Structural — `bug-hunt` provides the decomposed pipeline pattern (`Charter` → `Probe` → `Oracle` → `Taxonomize` → `Report`). This skill replicates that structure. | Pattern reuse, no surface overlap |

## 5. CNS Namespace Proposal

**Proposed namespaces (NOT yet registered):**

```rust
// crates/hkask-types/src/event.rs — CANONICAL_NAMESPACES
// ── Runtime posture (security audit — runtime-posture-monitor skill) ──
"cns.runtime",
"cns.runtime.select",
"cns.runtime.classify",
"cns.runtime.regulate",
"cns.runtime.convergence",
```

**Registration discipline (per `docs/plans/security-skills.md` CNS Namespace
Architecture):** Direct registration in the flat `CANONICAL_NAMESPACES` array
(like `cns.supply_chain.*`, `cns.inference`, `cns.fusion`). NOT under a
subgroup (`cns.skills.runtime` violates the flat namespace design;
`cns.skill.runtime` conflicts with `cns.skill` lifecycle purpose).

**Downstream regulation spans (already registered):**
- `cns.regulation` — emit when a runtime threat triggers a regulation action
- `cns.guard.violation` — emit when a runtime threat triggers a defensive block

The skill emits `cns.runtime.*` for its own PDCA loop and `cns.regulation` /
`cns.guard.violation` for downstream regulation. This mirrors how
`supply-chain-sentinel` emits `cns.supply_chain.*` for its loop and
references `cns.regulation` for downstream action.

**Registration gate:** The `cns.runtime.*` namespaces MUST be registered in
`CANONICAL_NAMESPACES` before this skill is committed to the registry. Until
then, the skill remains `draft`. This is the same discipline
`supply-chain-sentinel` followed (gap noted honestly, then closed).

## 6. Proposed Templates (Design — Not Implemented)

| Template | Type | Purpose |
|----------|------|---------|
| `select-signal.j2` | KnowAct | Discover runtime signal sources (`hkask.*` performative spans, `cns.guard.*` violations, `cns.regulation` events). Map signal types to monitor (API endpoint exposure, bot detection, LLM usage anomalies, runtime dependency behavior). Emit `cns.runtime.select` span. |
| `classify-threat.j2` | KnowAct | Classify observed runtime signals into threat patterns (endpoint abuse, bot traffic, LLM usage spike, dependency behavior anomaly). Apply pragmatic-cybernetics (feedback loop polarity, variety, Good Regulator). Emit `cns.runtime.classify` span per classified threat. |
| `emit-regulation.j2` | KnowAct | For each classified threat, emit a regulation event (`cns.regulation`) and trigger a defensive action (`cns.guard.violation` if blocking is warranted). Propose runtime regression entries (`surface: runtime`) for the security regression library. Emit `cns.runtime.regulate` span. |
| `convergence-check.j2` | KnowAct | Compute normalized convergence metric: unresolved runtime threats (0.40), defense-layer runtime coverage (0.25), threat-pattern taxonomy coverage (0.15), regression library growth (0.10), residual runtime risk (0.10). Emit `cns.runtime.convergence` span. |

## 7. Defense-Layer Catalog (Runtime Specific)

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

## 8. Convergence Metric (Design)

Normalized metric [0, 1] where 0 = fully converged:

| Dimension | Weight | Scoring |
|-----------|--------|---------|
| Unresolved runtime threats (critical/high) | 0.40 | 0 = +0.00; 1+ critical/high = +0.40 |
| Defense-layer runtime coverage | 0.25 | 6 layers firing = +0.00; 5 = +0.04; 4 = +0.08; 3 = +0.13; 2 = +0.19; 1 = +0.25 |
| Threat-pattern taxonomy coverage | 0.15 | All 4 threat types covered by findings/regressions = +0.00; partial = +0.08; none = +0.15 |
| Regression library growth (`surface: runtime`) | 0.10 | New regression proposed this cycle = +0.00; stagnation = +0.10 |
| Residual runtime risk | 0.10 | Zero unresolved runtime anomalies = +0.00; any remaining = +0.10 |

Converged when metric ≤ 0.10 AND relative improvement ≥ 5% from previous cycle.

## 9. Constraints (Concrete — Not Aspirational)

- Every finding includes concrete runtime signal evidence (`hkask.*` span
  target, timestamp, signal value) — not summary description.
- Every proposed regression uses exact YAML format (`security/regressions/`)
  with `surface: runtime`, concrete `pattern` (regex against span target or
  signal value), `status: pending`, `cwe: CWE-XXX` (taxonomy mapping, not
  vulnerability claim).
- No synthetic runtime signals; only observe spans actually emitted by the
  running system.
- No external package download; reads only local CNS telemetry (P4 boundary).
- Every output includes `replicant_host` identity (P12).
- Registry (`manifest.yaml` + `.j2`) is authoritative over SKILL.md (P5.1).
- `cns.runtime.*` namespaces MUST be registered before skill commit (P9
  integrity — same discipline as `supply-chain-sentinel`).

## 10. Source References and Taxonomy Anchors

- **MITRE CWE:** CWE-1357 (Reliance on Component That is Not Updateable —
  runtime dependency behavior), CWE-829 (Inclusion from Untrusted Control
  Sphere — runtime untrusted input), CWE-200 (Information Exposure — runtime
  endpoint exposure).
- **OWASP LLM Top 10 (2025):** LLM06 (Excessive Agency — runtime tool misuse),
  LLM07 (System Prompt Leakage — runtime canary detection).
- **MITRE ATLAS:** AML.TA0010 (Exfiltration — runtime data exfiltration
  detection).
- **Aikido Security** (`aikido.dev`): ASPM, auto-triage, runtime blocking
  model (context reference — not replacement).
- **Huntress** (`huntress.com`): Managed EDR/MDR (context — distinct surface,
  zero overlap per P5 minimal test).
- **`hkask-guard` pipeline:** `cns.guard.*` span sources (runtime evidence).
- **`hkask-cns` cybernetics loop:** `cns.regulation` span sink (downstream
  regulation action).

## 11. Open Questions (Blocking Registry Commit)

1. **`cns.runtime.*` namespace registration:** Must be proposed and accepted
   before skill commit. Follows the same pattern as `cns.supply_chain.*`
   (direct registration in flat `CANONICAL_NAMESPACES` array).
2. **Runtime signal access path:** How does the skill read `hkask.*`
   performative spans? Via the CNS observer/subscriber mechanism, or via a
   new runtime telemetry reader? Need to verify the existing CNS subscriber
   API supports skill-level consumption.
3. **Regulation action triggering:** Can a skill directly emit
   `cns.regulation` events, or must it propose them to the CyberneticsLoop?
   Need to verify the regulation action submission path.
4. **`surface: runtime` regression format:** Does the existing
   `scripts/check-kali-regressions.sh` CI gate handle `surface: runtime`
   entries, or does it need extension? (Currently the gate is surface-
   agnostic — it checks `status: enforced` grep patterns regardless of
   surface. Verify before commit.)

## 12. Path to Registry Commit

1. Resolve open questions in §11.
2. Register `cns.runtime.*` namespaces in `CANONICAL_NAMESPACES`
   (`crates/hkask-types/src/event.rs`).
3. Create `registry/templates/runtime-posture-monitor/manifest.yaml` with
   4 template entries (select-signal, classify-threat, emit-regulation,
   convergence-check).
4. Create the 4 `.j2` template files with `{# goal: ... #}` annotations
   (per `skill-logic-audit` critical revision).
5. Create `.agents/skills/runtime-posture-monitor/SKILL.md` derived from
   the registry manifest (P5.1 — registry authoritative).
6. Run `skill-logic-audit` convergence check on the new skill.
7. User `accept` per `skill-logic-audit` `user-choice` ratchet (P11).
8. Update `docs/plans/security-skills.md` to mark skill as `active`.

Until steps 1-7 complete, this skill remains `draft — not committed`.
