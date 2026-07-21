---
title: Planned Security Skills (hKask v0.31.0+)
audience: [architects, userpods, security auditors]
last_updated: 2026-07-18
status: active
version: 0.2.0
domain: security / supply-chain / application-security
---

# Planned Security Skills

Status: `active`. Created following `skill-logic-audit` convergence check for
`supply-chain-sentinel` (audit cycle converged — critical/high flaws
resolved). `supply-chain-sentinel` accepted by user (2026-07-18) per
`skill-logic-audit` `user-choice` ratchet; promoted from `pending` to
`active`. Stale `cns.supply_chain.*` "gap noted" language cleaned up across
registry templates and SKILL.md — the namespaces were already registered in
`CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs` L295-299).

## Completed / In Audit

| Skill | Registry | Status | Key Design Anchors |
|-------|----------|--------|-------------------|
| `kali-audit` | `registry/templates/kali-audit/` | Enforced (`security/regressions/` active) | P5 minimal, P8 semantic, P9 CNS (`cns.*` spans), 8-layer defense catalog |
| `adversarial-red-team` | `registry/templates/adversarial-red-team/` | Active | P3.1 LLM safety floor, 7 adversarial categories, persistence modes |
| `bug-hunt` | `registry/templates/bug-hunt/` | Active | P5 decomposed pipeline (`Charter` → `Probe` → `Oracle` → `Taxonomize` → `Report`), pragmatic-cybernetics embedded |
| `supply-chain-sentinel` | `registry/templates/supply-chain-sentinel/` (new) | `active` (user-accepted 2026-07-18; was `pending` per `skill-logic-audit` `user-choice`) | P4 OCAP (manifest-only, no external download), P8 evidence-backed (`CWE-1104/829/1357`, `OWASP Supply Chain`, `OSC&R`), P9 `cns.supply_chain.*` registered in `CANONICAL_NAMESPACES` (`event.rs` L295-299), P12 `userpod_host` mandatory |
| `runtime-posture-monitor` | `registry/templates/runtime-posture-monitor/` (new) | `active` (registry-committed 2026-07-18) | P4 OCAP (runtime CNS telemetry only, no external endpoint scanning), P8 evidence-backed (`CWE-1357/829/200`, `OWASP LLM06/07`, `ATLAS AML.TA0010`), P9 `cns.runtime.*` registered in `CANONICAL_NAMESPACES` (`event.rs` L302-308), P12 `userpod_host` mandatory, 6-layer runtime defense catalog (distinct from kali-audit's 8 static layers) |
| `attack-taxonomy-mapper` | `registry/templates/attack-taxonomy-mapper/` (new) | `active` (registry-committed 2026-07-18) | P4 OCAP (consumes existing findings, no external taxonomy download), P8 evidence-backed (`CWE-1104/829/1357`, `OWASP Supply Chain`, `OSC&R`), P9 `cns.taxonomy.*` registered in `CANONICAL_NAMESPACES` (`event.rs` L309-315), P12 `userpod_host` mandatory, backward-compatible `taxonomy_mapping` field |

## Audit Revisions Applied (`skill-logic-audit` cycle)

Critical: added `{# goal: ... #}` annotations to all `.j2` files + manifest.
High: fixed manifest description (`manifest-level dependency trees` — no synthetic `vulnerability reachability` claim that violates `P8`); completed truncated `convergence-check.j2`.
Medium: concrete `pattern:` enforcement in regression proposals; `SKILL.md` language clarified (`P4` boundary: manifest-only).
Spurious filtered: bundle/sub-agent requests (`P5` violation); external dependency download (`P4` violation); stylistic preferences.
Convergence metric: ~0 (material flaws resolved). Promoted to `active` per
user accept (2026-07-18) — `P11` ratchet satisfied.

## Implementation Update (2026-07-18)

User accepted `supply-chain-sentinel` per `skill-logic-audit` `user-choice`.
Cleanup applied:
- `registry/templates/supply-chain-sentinel/convergence-check.j2` — removed
  conditional "if registered; else note gap" language; `cns.supply_chain.convergence`
  is registered, span emitted unconditionally. Removed `cns_span_gap` from
  output (always `cns_span_emitted: true`).
- `registry/templates/supply-chain-sentinel/probe.j2` — removed "If not
  registered, this skill proposes registration" language; namespace IS
  registered, span emitted unconditionally.
- `.agents/skills/supply-chain-sentinel/SKILL.md` — updated P9 CNS regulation
  section, convergence-check step 6, and Constraints to state all four
  `cns.supply_chain.*` namespaces are registered and emitted unconditionally.
- This plan document — fixed stale "not yet registered in
  `CANONICAL_NAMESPACES` — gap noted" claim (the namespaces were already
  registered at `event.rs` L295-299 when this plan was written).

Pre-existing `Result<_, String>` CI gate failure resolved:
- `crates/hkask-cli/src/onboarding.rs` — `register_in_user_store` converted
  from `Result<(), String>` to `Result<(), UserStoreRegistrationError>` (new
  `thiserror` enum with `DbOpen`, `Pool`, `GetUserPod`, `RegisterUserPod`
  variants, mirroring the existing `SessionCreationError` pattern in the
  same file). `scripts/check-string-errors.sh` now passes.

Design specs completed for the two future skills (registry-committed 2026-07-18):
- `docs/plans/runtime-posture-monitor-design.md` — full design spec for
  `runtime-posture-monitor` skill: 5W1H gate, P5 deletion test, CNS namespace
  proposal (`cns.runtime.*` — pre-registered), 4-template decomposition,
  defense-layer catalog, convergence metric, open questions.
- `docs/plans/attack-taxonomy-mapper-design.md` — full design spec for
  `attack-taxonomy-mapper` skill: 5W1H gate, P5 deletion test, CNS namespace
  proposal (`cns.taxonomy.*` — pre-registered), 4-template decomposition,
  OSC&R taxonomy reference, convergence metric, open questions.

Both skills now built and committed to the registry (2026-07-18):
- `registry/templates/runtime-posture-monitor/` — manifest.yaml + 4 .j2
  templates (select-signal, classify-threat, emit-regulation,
  convergence-check) + SKILL.md. All `cns.runtime.*` spans emitted
  unconditionally (namespaces pre-registered).
- `registry/templates/attack-taxonomy-mapper/` — manifest.yaml + 4 .j2
  templates (select-evidence, map-taxonomy, taxonomize, convergence-check)
  + SKILL.md. All `cns.taxonomy.*` spans emitted unconditionally
  (namespaces pre-registered). OSC&R taxonomy verified against
  `github.com/pbom-dev/OSCAR` `matrix.json` (2026-07-18) — OSC&R uses
  tactic + technique names, NOT numeric IDs. OWASP SC codes remain
  PROPOSED (OWASP does not publish a numbered Supply Chain Top 10).

Both skills follow the same P5 discipline as `supply-chain-sentinel`:
4-template decomposition, `{# goal: ... #}` annotations, P12
userpod_host mandatory, P8 evidence-backed, P9 CNS spans emitted
unconditionally.

The two future skills below remain `draft — not committed` per P5
Essentialism. The plan explicitly prohibits implementing them to avoid
speculative abstraction; they are documented here for evolutionary
architecture (P7) only.

## Planned Future Skills (Not Registry-Committed)

Design discipline (`P5` Essentialism): each proposed skill must pass the 5W1H gate before registry creation. These are documented here for evolutionary architecture (`P7`) but NOT implemented to avoid speculative abstraction.

### Skill2: `runtime-posture-monitor` (Aikido/Zen-style runtime security)
- **5W1H gate check:** Who = running application / userpod host; What = API endpoint exposure / bot detection / LLM usage; Where = runtime environment / production workload; When = continuous (not audit cycle); Why = `P3.1` safe container requires runtime blocking (`Aikido` `Zen` firewall model: block attacks without code change); How = observe runtime signals (`cns.runtime.*` — registered in `CANONICAL_NAMESPACES`) → classify threat patterns → emit regulation events (`cns.regulation`) → trigger defensive action (`cns.guard.violation`).
- **P5 minimal test:** Does NOT download external packages; does NOT replace endpoint detection (`Huntress` — zero overlap); reads only runtime telemetry (`hkask.*` performative spans) and produces `cns.runtime.*` canonical spans (`P9`).
- **Relationship to `supply-chain-sentinel`:** `supply-chain-sentinel` audits static dependency integrity (`P4` manifest boundary); `runtime-posture-monitor` observes runtime dependency behavior (`P4` runtime boundary — distinct surface). They are complementary (like `kali-audit` + `adversarial-red-team`).
**Status:** `active` — registry committed (2026-07-18). `cns.runtime.*` namespaces pre-registered in `CANONICAL_NAMESPACES` (`event.rs` L302-308). Registry crate at `registry/templates/runtime-posture-monitor/` (manifest.yaml + 4 .j2 templates: select-signal, classify-threat, emit-regulation, convergence-check). SKILL.md at `.agents/skills/runtime-posture-monitor/SKILL.md`. Design spec at `docs/plans/runtime-posture-monitor-design.md`.

### Skill3: `attack-taxonomy-mapper` (OX Security OSC&R framework integration)
- **5W1H gate check:** Who = supply chain attacker / threat actor taxonomy; What = software supply chain attack patterns (`OSC&R` taxonomy: dependency confusion, typosquatting, malicious commit injection, build pipeline compromise); Where = dependency registry / CI pipeline / repository; When = audit cycle or incident investigation; Why = `P3.1` requires structured taxonomy for supply chain threats (like `MITRE ATLAS` for LLM, `OWASP LLM Top 10` for LLM); How = map manifest patterns / CI logs to `OSC&R` taxonomy entries → produce taxonomy-aligned findings (`OWASP Supply Chain` + `OSC&R` dual taxonomy).
- **P5 minimal test:** Uses existing `security/regressions/` format; adds `taxonomy_mapping` field to regression YAML (`osc:r` reference); does NOT invent new taxonomy categories (`OSC&R` is open-source framework — `github.com/pbom-dev/OSCAR`).
- **Relationship to existing skills:** Complements `supply-chain-sentinel` (this skill: manifest-level audit; `attack-taxonomy-mapper`: taxonomy mapping layer). Complements `adversarial-red-team` (`MITRE ATLAS` for LLM adversarial; this: `OSC&R` for supply chain adversarial — parallel taxonomy discipline).
**Status:** `active` — registry committed (2026-07-18). `cns.taxonomy.*` namespaces pre-registered in `CANONICAL_NAMESPACES` (`event.rs` L309-315). Registry crate at `registry/templates/attack-taxonomy-mapper/` (manifest.yaml + 4 .j2 templates: select-evidence, map-taxonomy, taxonomize, convergence-check). SKILL.md at `.agents/skills/attack-taxonomy-mapper/SKILL.md`. Design spec at `docs/plans/attack-taxonomy-mapper-design.md`. OSC&R taxonomy verified against `github.com/pbom-dev/OSCAR` `matrix.json` (2026-07-18) — uses tactic + technique names, NOT numeric IDs. OWASP SC codes remain PROPOSED.

## CNS Namespace Architecture (Point3 Research — `skill-logic-audit` + codebase)

**Finding:** `CANONICAL_NAMESPACES` is a flat array (`crates/hkask-types/src/event.rs`, L111-297). There is NO subgroup mechanism (`cns.skills` does not exist; `cns.skill` is a single namespace for skill lifecycle: activate/load/discover/publish/validate). Security spans are registered directly (`cns.tool.*`, `cns.inference`, `cns.fusion`, etc.).

**Recommendation (APPLIED):** `cns.supply_chain.select/probe/report/convergence`
are registered directly in `CANONICAL_NAMESPACES` (like `cns.inference`,
`cns.fusion`). NOT under a subgroup (`cns.skills.supply_chain` violates the
flat namespace design; `cns.skill.supply_chain` conflicts with `cns.skill`
lifecycle purpose).

**Evidence:** `SpanNamespace::new` validates against `CANONICAL_NAMESPACES`;
`is_canonical` checks byte-for-byte match; `scripts/check-cns-canonical.sh`
enforces this. The `skill-logic-audit` audit found the gap (`cns.supply_chain`
not registered) — the gap has since been CLOSED: all four namespaces
(`cns.supply_chain`, `cns.supply_chain.select`, `cns.supply_chain.probe`,
`cns.supply_chain.report`, `cns.supply_chain.convergence`) are registered at
`crates/hkask-types/src/event.rs` L295-299. The skill templates and SKILL.md
have been updated to emit spans unconditionally (no more "if registered; else
note gap" conditional language).

## Security Service Mappings (Point7 Research — verified against source docs)

| Service | Category | Native Skill Equivalent | Overlap / Complement |
|---------|----------|--------------------------|---------------------|
| **Snyk** (SCA / Supply Chain / SAST) | Developer security platform (`SCA` = `Cargo.toml` dependency CVE tracking; `SAST` = first-party code analysis; `Container` = image scanning; `IaC` = Terraform scan) | `kali-audit` (SAST/surface audit) + `supply-chain-sentinel` (SCA/dependency audit) | Partial overlap: `kali-audit` covers `surface: supply-chain` at manifest-discovery level (advisory/deny.toml); `supply-chain-sentinel` provides deeper dependency graph audit (version pinning, registry verification, SBOM tracking, defense-layer metric, `cns.supply_chain.*` spans). `Snyk`'s container/IaC scanning has zero overlap (`P4` workspace-boundary enforcement prevents container/IaC audit without separate surface declaration). |
| **Semgrep** (SAST / SSC — Supply Chain / Custom rules) | Pattern-matching engine (`.y2` rules look like source code; registry of 2500+ rules) | `kali-audit` (evidence-backed pattern detection) + `attack-taxonomy-mapper` (active — OSC&R taxonomy mapping layer) | Partial overlap: `Semgrep` rules are source-pattern-based; `kali-audit` uses concrete evidence patterns. `supply-chain-sentinel` proposes concrete `grep` regression patterns (like `Semgrep` `pattern:` syntax) but anchored to `security/regressions/` format, not `.yaml` rule registry. `Semgrep`'s `SSC` (dependency reachability) is a deeper dependency analysis than this skill; this skill is manifest-level (no external package download — `P4` boundary). |