---
title: "Platform Engineering Perspective — Systematic Integration Plan"
audience: [architects, platform engineers, project maintainers]
last_updated: 2026-06-30
version: "0.31.0"
status: "Proposal"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
anchored_on: [PRINCIPLES.md §P5, P7, P9, P12]
---

# Platform Engineering Perspective — Systematic Integration Plan

## 1. Problem Statement

hKask is a platform that builds agents. It has sophisticated internal regulation (CNS algedonic pathway, energy budgeting, capability membranes) and strong architectural discipline (hexagonal ports, deep-module surface constraints, property-based testing). But it lacks the **platform engineering lens** — the discipline of treating the platform as a product with measurable health, explicit contracts with users, and continuous improvement driven by data.

Concretely:

- CNS knows something is wrong but not what the user contract says (no SLOs)
- No developer experience metrics (time-to-first-agent, skill adoption, satisfaction)
- No continuous platform auditing agent — skills exist but are human-activated
- Identified gaps acknowledged but not systematically closed by user impact

A world-class platform engineer would see hKask's architecture as **80% of the way to a self-maintaining platform** — the sensors, regulatory loops, and skills are all there. Missing: the contract layer (SLOs), the measurement layer (PaaP metrics), and the automation layer (Platform Engineer replicant).

---

## 2. Current Condition — Platform Engineering Audit

### 2.1 What hKask Already Excels At

| Pattern | Where | PE Significance |
|---------|-------|-----------------|
| **Hexagonal Architecture** | `hkask-ports` — trait abstractions for CNS, inference, embedding, tool dispatch, registry, git-cas, federation | Infrastructure swappable without touching domain logic |
| **Cybernetic Self-Regulation** | 28 CNS span namespaces, VarietyTracker, AlgedonicManager, BackpressureSignal, CircuitBreaker | Observability as architecture. Ashby's Law enforced at type level |
| **Energy-Based Cost Governance** | EnergyBudget, rJoule (1 rJ = 250,000 gas), triple-entry ledger, ProviderIntelligence | FinOps built into type system. Rate limiting subsumed by energy tracking |
| **Capability Membranes (OCAP)** | Read/Write/Signal/Never boundaries between four loops, typed crossings only | Zero-trust architecture. No ambient authority |
| **Self-Healing** | SelfHealer on every fallible operation, 6 built-in strategies, full CNS audit trail | Autonomous recovery as first resort |
| **Deep Module Discipline** | ≤7 public items per crate, deletion test justification for all crates | API surface minimalism |
| **Property-Based + Fuzz + Mutation Testing** | cargo-bolero, cargo-mutants, state-machine roundtrip, CNS span contract fuzzing, LLM QA triage | Testing as verification, not coverage-counting |
| **Skills as Self-Service** | WordAct/FlowDef/KnowAct, ManifestExecutor cascade, PDCA convergence | P3 Generative Space — users extend without permission |
| **Kata Improvement Loop** | PDCA cycles, coaching 5-question dialogue, CNS span trace per experiment | Continuous improvement as first-class process |

### 2.2 Identified Gaps

| Gap | PE Concern | Severity | User Impact |
|-----|-----------|----------|-------------|
| **No explicit SLOs** | Reliability Engineering | High | CNS detects anomalies but has no user-facing contracts |
| **No PaaP metrics** | Platform-as-Product | High | No time-to-first-agent, skill adoption rate, or developer NPS |
| **No continuous platform auditing agent** | Automation / Toil Reduction | High | Skills exist but require human activation |
| **30-method AgentService** | Architectural Debt | Medium | God Object targeted for strangler-fig (archived ADR-040) |
| **No cost attribution to users** | FinOps | Medium | Ledger tracks consumption but not "who spent what" |
| **Kata documentation narrative** | Documentation / DX | Low | No narrative companion for coaching |
| **Skill-MCP doc boundary** | Developer Portal | Low | No unified capability map |
| **utoipa annotation gaps** | API Discoverability | Medium | Unannotated endpoints invisible to auto-generation |
| **Versioned documentation** | Knowledge Management | Low | Docs drift without versioned snapshots |
| **LoRA store security model** | Security Posture | Medium | Adapter tampering threat model undocumented |

### 2.3 CNCF Maturity Assessment

| Level | hKask Status |
|-------|-------------|
| L1 Provisional | ❌ Not here |
| L2 Operational | ⚠️ Partial — CNS automates regulation, gap docs acknowledge gaps |
| L3 Scalable | ✅ Skills are self-service, FlowDef templates, CNS tracks variety |
| L4 Optimizing | ✅ Kata PDCA, SelfHealer, mutation testing — but missing platform-level KPIs |

**Current:** L3→L4 transition. The three investments below complete the L4 transition.

---

## 3. Target Condition — The Three Investments

```
INVESTMENT 1 ── SLOs wired to CNS
                 (User contracts, error budgets, algedonic escalation on SLO breach)

INVESTMENT 2 ── Platform-as-Product Metrics
                 (Time-to-first-agent, skill adoption, developer NPS, adoption funnel)

INVESTMENT 3 ── Platform Engineer Replicant
                 (Continuous audit, recommendation, consent-gated improvement via skills)
```

Each builds on the one before: SLOs define *what* the platform promises. PaaP metrics define *how well* it serves. The replicant automates *continuous improvement* against both.

---

## 4. Investment 1 — SLOs Wired to CNS

hKask already has the full cybernetic feedback loop (Sensor → Model → Comparator → Regulator → Actuator). SLOs enrich the Comparator with user-facing contract thresholds.

### 4.1 Proposed SLOs

| SLO ID | Name | CNS Span | Target | Window | Severity |
|--------|------|----------|--------|--------|----------|
| SLO-INF-001 | Inference availability | cns.inference.* | 99.9% success | 30d | Critical |
| SLO-INF-002 | Inference p95 latency | cns.inference.duration_ms | < 5,000ms | 7d | High |
| SLO-SKL-001 | Skill dispatch success | cns.tool.skill_dispatch | 99.5% | 30d | Critical |
| SLO-SKL-002 | Skill dispatch p95 latency | cns.tool.skill_dispatch.duration_ms | < 2,000ms | 7d | High |
| SLO-CNS-001 | CNS algedonic delivery | cns.algedonic.* | 99.9% within 30s | 30d | Critical |
| SLO-MEM-001 | Memory consolidation | cns.memory.consolidation | 99.0% | 7d | High |
| SLO-CUR-001 | Curator escalation response | cns.curation.escalation | < 60s p95 | 7d | Medium |
| SLO-API-001 | API endpoint availability | cns.api.* | 99.9% | 30d | Critical |
| SLO-WLT-001 | Wallet operation success | cns.wallet.* | 99.99% | 30d | Critical |

### 4.2 Error Budget Model

```
Error Budget = (1 - Target) × Total Operations in Window
```

| SLO ID | Monthly Error Budget | Burn Rate Alert (>2% in 1h) |
|--------|---------------------|---------------------------|
| SLO-INF-001 | ~43 min downtime | Yes |
| SLO-SKL-001 | ~216 failures (1k/day) | Yes |
| SLO-API-001 | ~43 min downtime | Yes |

### 4.3 CNS Integration

New types: `SloDefinition`, `SloSeverity` (Critical/High/Medium), `SloEvaluation`.

New CNS span: `cns.slo.evaluated` — emitted per evaluation cycle with `slo_id`, `current_compliance`, `error_budget_remaining`, `burn_rate`.

Algedonic integration: `AlgedonicManager` gains `SloBreach` trigger type. Error budget burn rate exceeding threshold escalates identically to variety deficits.

### 4.4 API Surface

| Endpoint | Purpose |
|----------|---------|
| GET /api/v1/slos | List all SLOs with current compliance |
| GET /api/v1/slos/:id | Detailed status: compliance, error budget, burn rate, history |
| POST /api/v1/slos | Define new SLO (Admin only) |
| DELETE /api/v1/slos/:id | Remove SLO (Admin only) |

### 4.5 Skills to Activate

- **goal-analysis**: Extract structured SLOs from platform intent
- **mcda**: Rank SLO candidates by user impact vs. implementation cost
- **pragmatic-semantics**: Classify each SLO by constraint force
- **qa-script-builder**: Build SLO compliance verification pipeline

---

## 5. Investment 2 — Platform-as-Product Metrics

### 5.1 Proposed Metrics

| Metric | Definition | CNS Span | Cadence |
|--------|-----------|----------|---------|
| **Time to First Agent** | Wall-clock time from sign-in to first successful agent creation | cns.onboarding.complete → cns.agent.created | Per user |
| **Time to 10th Skill** | Wall-clock time from first skill creation to 10th | cns.skill.created | Per user |
| **Skill Adoption Rate** | % of created skills used in ≥3 sessions within 30 days | cns.tool.skill_dispatch | Monthly |
| **Platform NPS** | Prompt-based survey in REPL after 10th session | N/A (survey) | Quarterly |
| **Active User Retention** | % of users active in both current and previous 30-day windows | cns.session.* | Monthly |
| **Error Resolution Time** | Time from CNS alert to SelfHealer resolution or human intervention | cns.algedonic.* → cns.heal.* | Per incident |

### 5.2 CNS Integration

New CNS span: `cns.platform.metric` — emitted per metric evaluation with `metric_name`, `value`, `window`, `trend`.

### 5.3 API Surface

| Endpoint | Purpose |
|----------|---------|
| GET /api/v1/platform/metrics | Get all platform metrics with current values |
| GET /api/v1/platform/metrics/:name | Get detailed history for one metric |

### 5.4 Skills to Activate

- **scenario-builder**: What happens to adoption if SLO-INF-001 breaches for 24h?
- **superforecasting**: Calibrated probability: "NPS > 50 by Q4 2026"
- **structured-extraction**: Extract DX signals from session transcripts

---

## 6. Investment 3 — Platform Engineer Replicant

The ultimate move: create a hKask agent that continuously audits and improves the platform — using hKask's own skills.

### 6.1 Replicant Definition

```yaml
agent:
  name: Platform Engineer
  type: replicant
charter:
  description: >
    Maintains platform health through continuous SLO monitoring,
    architectural audit, and actionable recommendations.
    Never modifies code or configuration without human approval (P2).
capabilities:
  - semantic-graph-audit      # Crate dependency health
  - deep-module               # Public surface audit
  - pragmatic-cybernetics     # Feedback loop health
  - bug-hunt                  # Platform reliability expedition
  - diagnose                  # SLO breach root cause analysis
  - improve-codebase-architecture  # Deepening opportunities
  - mcda                      # Prioritize interventions
  - superforecasting          # Risk forecasting
  - handoff                   # Continuity between cycles
```

### 6.2 Operating Cadence

| Frequency | Activity | Skills | Output |
|-----------|----------|--------|--------|
| **Daily** | CNS SLO check — are any error budgets burning? | pragmatic-cybernetics | SLO health dashboard update |
| **Weekly** | Dependency graph audit — any new cycles, orphans, drift? | semantic-graph-audit | Dependency health report |
| **Monthly** | Full platform audit — deep-module review, bug hunt expedition | deep-module + bug-hunt | Platform health score + prioritized recommendations |
| **On Alert** | SLO breach diagnosis | diagnose | Root cause analysis + proposed remediation |
| **On Demand** | User-requested review ("audit crate X") | improve-codebase-architecture | Targeted refactoring proposal |

### 6.3 OCAP Boundaries

| Access | Scope | Mechanism |
|--------|-------|-----------|
| **Read** | CNS spans, SLO evaluations, dependency graph, crate public surfaces, test results | Direct via service layer |
| **Signal** | Recommendations to Curator, SLO breach alerts, health score changes | CNS spans + CuratorDirective |
| **Write** | Platform health reports (read-only triple), metric evaluations | EpisodicMemory via OCAP |
| **Never** | Source code, configuration files, deployment, agent definitions, wallet operations | Enforced by capability membrane |

### 6.4 CNS Integration

New CNS spans:
- `cns.platform.audit.started` — Platform audit cycle begins
- `cns.platform.audit.completed` — Audit cycle complete with findings
- `cns.platform.recommendation` — Replicant proposes an intervention
- `cns.platform.recommendation.accepted` — Human curator accepts
- `cns.platform.recommendation.rejected` — Human curator rejects (with reason)

---

## 7. Integration with Existing Systems

### 7.1 How This Composes with the Four Patterns

| Pattern | Enhancement |
|---------|------------|
| **A: Skills Model** | SLOs, PaaP metrics, and platform audit are FlowDef skills — no new types needed |
| **B: CNS Feedback Loop** | SLO breach is a new algedonic trigger; PaaP metrics are new CNS spans; Platform Engineer replicant is a new observer |
| **C: Curator + 7R7** | Platform Engineer replicant is a new agent in the Curator's charge. Curator metacognition now includes platform health as a dimension |
| **D: AgentPod** | Platform Engineer replicant gets its own pod with read-only access to platform state |

### 7.2 How This Composes with the Four Loops

| Loop | Enhancement |
|------|------------|
| **Inference** | SLO-INF-001/002 monitor inference health. Platform Engineer replicant uses inference for audit runs |
| **Memory** | SLO-MEM-001 monitors consolidation. PaaP metrics stored as episodic memories |
| **Curation** | Platform Engineer replicant reports to Curator. New CNS spans for audit/recommendation lifecycle |
| **Cybernetics** | SLO breach triggers enrich algedonic pathway. PaaP metric spans feed VarietyTracker |

### 7.3 Implementation Sequence

| Phase | What | Duration Est. | Prerequisites |
|-------|------|--------------|---------------|
| **Phase 1** | SloDefinition type + CNS integration + 3 seed SLOs (INF-001, SKL-001, API-001) | 2-3 PDCA cycles | None |
| **Phase 2** | Error budget tracking + algedonic SLO breach escalation | 2 PDCA cycles | Phase 1 |
| **Phase 3** | PaaP metric definitions + CNS spans + API | 2 PDCA cycles | Phase 1 |
| **Phase 4** | Platform Engineer replicant definition + OCAP boundaries + basic audit skills | 3 PDCA cycles | Phase 1+2+3 |
| **Phase 5** | Full replicant operating cadence (daily/weekly/monthly) | 2 PDCA cycles | Phase 4 |

---

## 8. Success Criteria

| Criterion | Measurement | Target |
|-----------|------------|--------|
| SLOs are defined and tracked | SloEvaluation counts in CNS | ≥9 SLOs active within 30 days of Phase 1 start |
| Error budgets inform decisions | % of SLO breaches that trigger an intervention | >80% within 60 days |
| Platform metrics are measurable | PaaP metric CNS spans emitted | All 6 metrics emitting within 30 days of Phase 3 start |
| Platform Engineer replicant is active | cns.platform.audit.* spans | Weekly audits running within 30 days of Phase 4 start |
| Replicant recommendations are actionable | Acceptance rate of recommendations | >60% acceptance within 90 days |

---

## 9. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| SLO alert fatigue | Medium | High | Start with 3 SLOs, expand only when signal-to-noise proven |
| Replicant recommendations too frequent | Medium | Medium | Monthly audit cadence; batch recommendations |
| Platform Engineer replicant scope creep | Low | Medium | OCAP boundaries prevent write access; charter is narrow |
| SLO targets too aggressive | Medium | Low | Start with loose targets (99.0%), tighten based on actual performance |
| PaaP metrics gamed | Low | Medium | Metrics anchored in CNS spans — hard to fake without system compromise |

---

## 10. References

- hKask Architecture Master: `docs/architecture/hKask-architecture-master.md`
- hKask Principles: `docs/architecture/core/PRINCIPLES.md`
- MDS Specification: `docs/architecture/core/MDS.md`
- Testing Discipline: `docs/architecture/core/TESTING_DISCIPLINE.md`
- Google SRE Book: Service Level Objectives (§4), Monitoring Distributed Systems (§6)
- Team Topologies: Skelton & Pais (2019) — Platform as a Product, Interaction Modes
- CNCF Platform Engineering Maturity Model: `tag-app-delivery.cncf.io`
- Wardley Mapping: Simon Wardley — situational awareness for platform strategy
