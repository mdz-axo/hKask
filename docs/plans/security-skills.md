---
title: Planned Security Skills (hKask v0.31.0+)
audience: [architects, replicants, security auditors]
last_updated: 2026-07-18
status: draft
version: 0.1.0
domain: security / supply-chain / application-security
---

# Planned Security Skills

Status: `draft` (not registry-committed). Created following `skill-logic-audit`
convergence check for `supply-chain-sentinel` (audit cycle converged —
critical/high flaws resolved, `pending` per `P11` ratcheted design).

## Completed / In Audit

| Skill | Registry | Status | Key Design Anchors |
|-------|----------|--------|-------------------|
| `kali-audit` | `registry/templates/kali-audit/` | Enforced (`security/regressions/` active) | P5 minimal, P8 semantic, P9 CNS (`cns.*` spans), 8-layer defense catalog |
| `adversarial-red-team` | `registry/templates/adversarial-red-team/` | Active | P3.1 LLM safety floor, 7 adversarial categories, persistence modes |
| `bug-hunt` | `registry/templates/bug-hunt/` | Active | P5 decomposed pipeline (`Charter` → `Probe` → `Oracle` → `Taxonomize` → `Report`), pragmatic-cybernetics embedded |
| `supply-chain-sentinel` | `registry/templates/supply-chain-sentinel/` (new) | `pending` (audit converged; requires user `accept` per `skill-logic-audit` `user-choice`) | P4 OCAP (manifest-only, no external download), P8 evidence-backed (`CWE-1104/829/1357`, `OWASP Supply Chain`, `OSC&R`), P9 `cns.supply_chain.*` (not yet registered in `CANONICAL_NAMESPACES` — gap noted), P12 `replicant_host` mandatory |

## Audit Revisions Applied (`skill-logic-audit` cycle)

Critical: added `{# goal: ... #}` annotations to all `.j2` files + manifest.
High: fixed manifest description (`manifest-level dependency trees` — no synthetic `vulnerability reachability` claim that violates `P8`); completed truncated `convergence-check.j2`.
Medium: concrete `pattern:` enforcement in regression proposals; `SKILL.md` language clarified (`P4` boundary: manifest-only).
Spurious filtered: bundle/sub-agent requests (`P5` violation); external dependency download (`P4` violation); stylistic preferences.
Convergence metric: ~0 (material flaws resolved). Remains `pending` per `P11` visibility / ratchet.

## Planned Future Skills (Not Registry-Committed)

Design discipline (`P5` Essentialism): each proposed skill must pass the 5W1H gate before registry creation. These are documented here for evolutionary architecture (`P7`) but NOT implemented to avoid speculative abstraction.

### Skill2: `runtime-posture-monitor` (Aikido/Zen-style runtime security)
- **5W1H gate check:** Who = running application / replicant host; What = API endpoint exposure / bot detection / LLM usage; Where = runtime environment / production workload; When = continuous (not audit cycle); Why = `P3.1` safe container requires runtime blocking (`Aikido` `Zen` firewall model: block attacks without code change); How = observe runtime signals (`cns.runtime.*` — proposed namespace, not registered) → classify threat patterns → emit regulation events (`cns.regulation`) → trigger defensive action (`cns.guard.violation`).
- **P5 minimal test:** Does NOT download external packages; does NOT replace endpoint detection (`Huntress` — zero overlap); reads only runtime telemetry (`hkask.*` performative spans) and produces `cns.runtime.*` canonical spans (`P9`).
- **Relationship to `supply-chain-sentinel`:** `supply-chain-sentinel` audits static dependency integrity (`P4` manifest boundary); `runtime-posture-monitor` would observe runtime dependency behavior (`P4` runtime boundary — distinct surface). They are complementary (like `kali-audit` + `adversarial-red-team`).
**Status:** `draft` — requires `CANONICAL_NAMESPACES` proposal (`cns.runtime.*` — direct registration, not subgroup, per research: flat namespace array in `crates/hkask-types/src/event.rs`) and `P9` loop design before registry creation. Not committed.

### Skill3: `attack-taxonomy-mapper` (OX Security OSC&R framework integration)
- **5W1H gate check:** Who = supply chain attacker / threat actor taxonomy; What = software supply chain attack patterns (`OSC&R` taxonomy: dependency confusion, typosquatting, malicious commit injection, build pipeline compromise); Where = dependency registry / CI pipeline / repository; When = audit cycle or incident investigation; Why = `P3.1` requires structured taxonomy for supply chain threats (like `MITRE ATLAS` for LLM, `OWASP LLM Top 10` for LLM); How = map manifest patterns / CI logs to `OSC&R` taxonomy entries → produce taxonomy-aligned findings (`OWASP Supply Chain` + `OSC&R` dual taxonomy).
- **P5 minimal test:** Uses existing `security/regressions/` format; adds `taxonomy_mapping` field to regression YAML (`osc:r` reference); does NOT invent new taxonomy categories (`OSC&R` is open-source framework — `oscar.io`).
- **Relationship to existing skills:** Complements `supply-chain-sentinel` (this skill: manifest-level audit; `attack-taxonomy-mapper`: taxonomy mapping layer). Complements `adversarial-red-team` (`MITRE ATLAS` for LLM adversarial; this: `OSC&R` for supply chain adversarial — parallel taxonomy discipline).
**Status:** `draft` — requires `manifest.yaml` design and registry templates (`taxonomize.j2` mapping `CWE-1104`/`CWE-829`/`CWE-1357` to `OSC&R` taxonomy entries; `map-taxonomy.j2` applying `pragmatic-cybernetics` to taxonomy mapping — like `bug-hunt` `taxonomize` phase). Not committed.

## CNS Namespace Architecture (Point3 Research — `skill-logic-audit` + codebase)

**Finding:** `CANONICAL_NAMESPACES` is a flat array (`crates/hkask-types/src/event.rs`, L111-297). There is NO subgroup mechanism (`cns.skills` does not exist; `cns.skill` is a single namespace for skill lifecycle: activate/load/discover/publish/validate). Security spans are registered directly (`cns.tool.*`, `cns.inference`, `cns.fusion`, etc.).

**Recommendation:** `cns.supply_chain.select/probe/report/convergence` should be registered directly in `CANONICAL_NAMESPACES` (like `cns.inference`, `cns.fusion`). NOT under a subgroup (`cns.skills.supply_chain` violates the flat namespace design; `cns.skill.supply_chain` conflicts with `cns.skill` lifecycle purpose).

**Evidence:** `SpanNamespace::new` validates against `CANONICAL_NAMESPACES`; `is_canonical` checks byte-for-byte match; `scripts/check-cns-canonical.sh` enforces this. The `skill-logic-audit` audit found the gap (`cns.supply_chain` not registered) and the skill reports it honestly (`P9` integrity) rather than inventing registry presence.

## Security Service Mappings (Point7 Research — verified against source docs)

| Service | Category | Native Skill Equivalent | Overlap / Complement |
|---------|----------|--------------------------|---------------------|
| **Snyk** (SCA / Supply Chain / SAST) | Developer security platform (`SCA` = `Cargo.toml` dependency CVE tracking; `SAST` = first-party code analysis; `Container` = image scanning; `IaC` = Terraform scan) | `kali-audit` (SAST/surface audit) + `supply-chain-sentinel` (SCA/dependency audit) | Partial overlap: `kali-audit` covers `surface: supply-chain` at manifest-discovery level (advisory/deny.toml); `supply-chain-sentinel` provides deeper dependency graph audit (version pinning, registry verification, SBOM tracking, defense-layer metric, `cns.supply_chain.*` spans). `Snyk`'s container/IaC scanning has zero overlap (`P4` workspace-boundary enforcement prevents container/IaC audit without separate surface declaration). |
| **Semgrep** (SAST / SSC — Supply Chain / Custom rules) | Pattern-matching engine (`.y2` rules look like source code; registry of 2500+ rules) | `kali-audit` (evidence-backed pattern detection) + proposed `attack-taxonomy-mapper` (taxonomy mapping layer) | Partial overlap: `Semgrep` rules are source-pattern-based; `kali-audit` uses concrete evidence patterns. `supply-chain-sentinel` proposes concrete `grep` regression patterns (like `Semgrep` `pattern:` syntax) but anchored to `security/regressions/` format, not `.yaml` rule registry. `Semgrep`'s `SSC` (dependency reachability) is a deeper dependency analysis than this skill; this skill is manifest-level (no external package download — `P4` boundary). |