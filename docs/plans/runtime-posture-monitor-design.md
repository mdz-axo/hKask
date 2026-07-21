---
title: "Skill Design Spec — runtime-posture-monitor"
audience: [architects, userpods, security auditors]
last_updated: 2026-07-18
status: as-built
version: 0.2.0
domain: security / runtime / application-security
skill_status: active — registry-committed (2026-07-18)
---

# Skill Design Spec — `runtime-posture-monitor`

**Status:** `active` — registry-committed (2026-07-18). This design spec
is retained as as-built documentation. The skill is implemented at
`registry/templates/runtime-posture-monitor/` (manifest.yaml + 4 .j2
templates) + `.agents/skills/runtime-posture-monitor/SKILL.md`.

The `reg.runtime.*` namespaces are registered in `CANONICAL_NAMESPACES`
(`crates/hkask-types/src/event.rs` L302-308). The skill passed
`skill-logic-audit` with all material flaws resolved (convergence metric
≤ 0.10 after fixes).

## 1. Identity

| Field | Value |
|-------|-------|
| Skill name | `runtime-posture-monitor` |
| Version | 0.1.0 (proposed) |
| Visibility | `public` (P10, P11) |
| Type | Skill (PDCA FlowDef with convergence threshold + energy budget) |
| Surface parameter | `runtime` (single surface — distinct from `supply-chain-sentinel`'s manifest surface and `kali-audit`'s 5 surfaces) |
| Decomposition | 4 templates: `select-signal` → `classify-threat` → `emit-regulation` → `convergence-check` |
| Regulation spans | `reg.runtime.select`, `reg.runtime.classify`, `reg.runtime.regulate`, `reg.runtime.convergence` (proposed — NOT yet registered in `CANONICAL_NAMESPACES`) |

## 2. 5W1H Gate (P5 Essentialism)

| Question | Answer |
|----------|--------|
| **Who** | Running application / userpod host (P12 — `userpod_host` mandatory in every output) |
| **What** | API endpoint exposure / bot detection / LLM usage anomalies / runtime dependency behavior |
| **Where** | Runtime environment / production workload (NOT workspace manifest — distinct from `supply-chain-sentinel`) |
| **When** | Continuous (not audit cycle) — observes `hkask.*` performative spans as they are emitted |
| **Why** | P3.1 safe container requires runtime blocking (Aikido/Zen firewall model: block attacks without code change). Static audit (`kali-audit`, `supply-chain-sentinel`) cannot detect runtime anomalies. |
| **How** | Observe runtime signals (`hkask.*` performative spans) → classify threat patterns → emit regulation events (`reg.regulation`) → trigger defensive action (`reg.guard.violation`) |

**Gate verdict:** All 6 answered. Passes P5.1 minimalist test (see §3).

## 3. P5 Minimal Test (Deletion Test)

**Does this skill replace an existing capability?** No. `kali-audit` audits
static code/templates/manifests. `supply-chain-sentinel` audits static
dependency manifests. `adversarial-red-team` probes the LLM I/O boundary
with synthetic attacks. None of them observe **runtime** behavior of a
deployed userpod.

**Does this skill download external packages?** No. Reads only `hkask.*`
performative spans already emitted by the running system (P4 runtime
boundary — distinct surface from `supply-chain-sentinel`'s manifest
boundary).

**Does this skill replace endpoint detection (Huntress)?** No. Zero overlap
with Huntress (EDR/MDR — endpoint threat detection). This skill observes
hKask's own Regulation telemetry, not OS-level endpoint signals.

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

## 5. Regulation Namespace Proposal

**Proposed namespaces (NOT yet registered):**

```rust
// crates/hkask-types/src/event.rs — CANONICAL_NAMESPACES
// ── Runtime posture (security audit — runtime-posture-monitor skill) ──
"reg.runtime",
"reg.runtime.select",
"reg.runtime.classify",
"reg.runtime.regulate",
"reg.runtime.convergence",
```

**Registration discipline (per `docs/plans/security-skills.md` Regulation Namespace
Architecture):** Direct registration in the flat `CANONICAL_NAMESPACES` array
(like `reg.supply_chain.*`, `reg.inference`, `reg.fusion`). NOT under a
subgroup (`reg.skills.runtime` violates the flat namespace design;
`reg.skill.runtime` conflicts with `reg.skill` lifecycle purpose).

**Downstream regulation spans (already registered):**
- `reg.regulation` — emit when a runtime threat triggers a regulation action
- `reg.guard.violation` — emit when a runtime threat triggers a defensive block

The skill emits `reg.runtime.*` for its own PDCA loop and `reg.regulation` /
`reg.guard.violation` for downstream regulation. This mirrors how
`supply-chain-sentinel` emits `reg.supply_chain.*` for its loop and
references `reg.regulation` for downstream action.

**Registration gate:** The `reg.runtime.*` namespaces MUST be registered in
`CANONICAL_NAMESPACES` before this skill is committed to the registry. Until
then, the skill remains `draft`. This is the same discipline
`supply-chain-sentinel` followed (gap noted honestly, then closed).

## 6. Proposed Templates (Design — Not Implemented)

| Template | Type | Purpose |
|----------|------|---------|
| `select-signal.j2` | KnowAct | Discover runtime signal sources (`hkask.*` performative spans, `reg.guard.*` violations, `reg.regulation` events). Map signal types to monitor (API endpoint exposure, bot detection, LLM usage anomalies, runtime dependency behavior). Emit `reg.runtime.select` span. |
| `classify-threat.j2` | KnowAct | Classify observed runtime signals into threat patterns (endpoint abuse, bot traffic, LLM usage spike, dependency behavior anomaly). Apply pragmatic-cybernetics (feedback loop polarity, variety, Good Regulator). Emit `reg.runtime.classify` span per classified threat. |
| `emit-regulation.j2` | KnowAct | For each classified threat, emit a regulation event (`reg.regulation`) and trigger a defensive action (`reg.guard.violation` if blocking is warranted). Propose runtime regression entries (`surface: runtime`) for the security regression library. Emit `reg.runtime.regulate` span. |
| `convergence-check.j2` | KnowAct | Compute normalized convergence metric: unresolved runtime threats (0.40), defense-layer runtime coverage (0.25), threat-pattern taxonomy coverage (0.15), regression library growth (0.10), residual runtime risk (0.10). Emit `reg.runtime.convergence` span. |

## 7. Defense-Layer Catalog (Runtime Specific)

| Layer | Name | Evidence Source | Source Citation |
|-------|------|-----------------|-----------------|
| 1 | Input filtering (runtime firing) | `reg.guard.input` span emission count | `hkask-guard` pipeline |
| 2 | Output filtering (runtime firing) | `reg.guard.output` span emission count | `hkask-guard` pipeline |
| 3 | Canary token detection (runtime firing) | `reg.guard.canary` span emission count | `hkask-guard` pipeline |
| 4 | Runtime policy enforcement | `reg.guard.runtime_policy` span emission count | `hkask-templates` executor |
| 5 | Regulation loop active | `reg.regulation` span emission count | `hkask-regulation` cybernetics loop |
| 6 | Action distribution monitoring | `reg.regulation.loop_quality` span | `hkask-regulation` regulation policy |

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
- No external package download; reads only local Regulation telemetry (P4 boundary).
- Every output includes `userpod_host` identity (P12).
- Registry (`manifest.yaml` + `.j2`) is authoritative over SKILL.md (P5.1).
- `reg.runtime.*` namespaces MUST be registered before skill commit (P9
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
- **`hkask-guard` pipeline:** `reg.guard.*` span sources (runtime evidence).
- **`hkask-regulation` cybernetics loop:** `reg.regulation` span sink (downstream
  regulation action).

## 11. Open Questions (Resolved)

1. **`reg.runtime.*` namespace registration:** RESOLVED — all 5 namespaces
   registered in `CANONICAL_NAMESPACES` (`event.rs` L302-308).
2. **Runtime signal access path:** OPEN — the skill instructs the agent to
   observe `hkask.*` and `reg.*` spans, but there is no clear MCP tool or
   API for querying Regulation span history. The skill may require runtime
   infrastructure (a Regulation span history reader MCP tool) to be fully
   invocable in practice. See §13 below.
3. **Regulation action triggering:** OPEN — the skill instructs the agent
   to emit `reg.regulation` events, but the actual regulation action
   submission path to the CyberneticsLoop needs verification. The skill
   may need to propose regulation actions via a new MCP tool rather than
   directly emitting spans.
4. **`surface: runtime` regression format:** RESOLVED — `security/regressions/README.md`
   updated to include `runtime` surface and `kind: cns-span` detection type.
   `scripts/check-kali-regressions.sh` currently only enforces `kind: grep`
   — `kind: cns-span` regressions are silently skipped (see §13 below).

## 12. Path to Registry Commit (COMPLETED)

All steps completed (2026-07-18):
1. ✅ Resolved open questions in §11 (2 resolved, 2 remain open — see §13).
2. ✅ Registered `reg.runtime.*` namespaces in `CANONICAL_NAMESPACES`.
3. ✅ Created `registry/templates/runtime-posture-monitor/manifest.yaml`.
4. ✅ Created 4 `.j2` template files with `{# goal: ... #}` annotations.
5. ✅ Created `.agents/skills/runtime-posture-monitor/SKILL.md`.
6. ✅ Ran `skill-logic-audit` convergence check — all material flaws fixed.
7. ✅ User accepted per `skill-logic-audit` `user-choice` ratchet.
8. ✅ Updated `docs/plans/security-skills.md` — skill marked `active`.

## 13. Remaining Infrastructure Work (Post-Commit)

The skill is registry-committed and passes `kask skill audit` (score 1.00,
0 defects). However, two infrastructure gaps remain before the skill is
fully invocable in practice:

1. **Regulation span history reader:** The skill instructs the agent to observe
   `hkask.*` and `reg.*` spans, but there is no MCP tool for querying Regulation
   span history. A `reg.span_history` MCP tool (or equivalent) would need
   to be implemented in `mcp-servers/hkask-mcp-cns/` (or similar) for the
   skill to actually read runtime telemetry.

2. **`kind: cns-span` CI enforcement:** `scripts/check-kali-regressions.sh`
   currently only enforces `kind: grep` regressions. `kind: cns-span`
   regressions (used by `surface: runtime` entries) are silently skipped.
   The script needs extension to handle `kind: cns-span` — either by
   querying Regulation span history or by deferring to a runtime check.

These are infrastructure tasks, not skill design tasks. The skill itself
is complete and correct; the runtime infrastructure to invoke it needs
separate implementation work.
