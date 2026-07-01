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

## 1. Human Exemplars of Platform Engineering

Platform engineering inherits from systems engineering, operations research, distributed systems, and organizational design. The following figures shaped the discipline — each contributed a lens through which a world-class platform engineer would evaluate hKask.

### 1.1 The Pioneers

| Exemplar | Key Contribution | Relevance to hKask |
|----------|-----------------|-------------------|
| **Werner Vogels** (AWS CTO) | Defined the modern cloud platform: "You build it, you run it." API-first design, two-pizza teams, distributed systems at global scale. Author of the distributed systems papers underpinning DynamoDB and S3. | hKask's hexagonal ports and API-first design echo Vogels' conviction that a platform IS its API. The tri-surface pattern (CLI/API/MCP) is Vogels-ian: every capability must be accessible programmatically. |
| **Kelsey Hightower** | The conscience of platform engineering. His Kubernetes advocacy taught the industry that platforms must *disappear* for developers — "the best platform is the one you don't notice." Walked away from Google to emphasize that platform engineering is about people, not technology. | hKask's REPL and self-service skills (P3 Generative Space) embody Hightower's principle. But he would ask: "Does a user need to understand CNS spans to use hKask?" The answer should be no. |
| **Charity Majors** (Honeycomb CTO) | Defined **Observability 2.0**: a single unified storage model for wide structured log events, replacing the "three pillars" (metrics, logs, traces) she called a vendor marketing construct. Her insight: observability is a *data analytics* problem, not an ops problem. Platform engineering teams become data governance teams. | hKask's CNS with 73 typed span variants is an Observability 2.0 architecture born before the term existed. Wide structured events at every membrane crossing. But hKask still separates CNS from the user-facing REPL — are developers observing *their agents* or just the platform's internals? |
| **Dr. Nicole Forsgren** | Created the **DORA metrics** (Deployment Frequency, Lead Time, Change Failure Rate, MTTR) and the **SPACE framework** for developer productivity. Rigorous statistical research proving that delivery performance drives organizational performance — not the other way around. | hKask has no DORA metrics for itself. How frequently do skills get deployed? What's the lead time from FlowDef creation to production use? What's the change failure rate of a YAML manifest edit? These are measurable with existing CNS spans. |
| **Simon Wardley** | Created **Wardley Mapping**: a situational awareness technique that maps components on a value chain against their evolutionary stage (Genesis → Custom → Product → Commodity). Introduced the Pioneer-Settler-Town Planner (now Explorer-Villager-Town Planner) team structure. His doctrine: "Strategy is the art of manipulating an environment to gain a desirable outcome." | hKask needs a Wardley Map. Where are skills on the evolution axis? Where is the CNS? The MCP protocol? A map would reveal what to commoditize (deploy to MCP servers), what to productize (skills), and what to keep in Genesis (the Platform Engineer replicant). |

### 1.2 The Architects of Reliability

| Exemplar | Key Contribution | Relevance to hKask |
|----------|-----------------|-------------------|
| **John Allspaw** (Etsy) | Pioneered **blameless postmortems** and resilience engineering. His insight: complex systems fail in complex ways, and the goal is not to eliminate failure but to learn from it. Introduced the concept of "being an expert at learning from incidents." | hKask's SelfHealer with 6 autonomous strategies is a computational implementation of Allspaw's resilience engineering — but the *learning* loop is missing. When SelfHealer recovers, does the Curator learn? Are healing patterns shared across pods? |
| **Michael Nygard** | Author of *Release It!* — defined stability patterns (Circuit Breaker, Bulkhead, Timeout, Handshaking) that are now industry standard. His insight: stability is a system property, not a feature. You design for it or you get it wrong. | hKask implements CircuitBreaker, BackpressureSignal, and tokio::time::timeout at the type level. Nygard would approve. But Bulkheads — isolating failure domains so one agent's crash doesn't cascade — are implicit in per-pod SQLCipher, not explicit in the architecture. |
| **Brendan Gregg** (Netflix/Intel) | Systems performance engineering. Created the USE method (Utilization, Saturation, Errors), flame graphs, and the Linux performance observability toolkit. His insight: performance is a feature; you must measure everything. | hKask's CNS VarietyTracker is saturation tracking. But where are utilization metrics? Where are flame-graph-equivalent traces of a FlowDef cascade through to inference? |

### 1.3 The Architects of Delivery

| Exemplar | Key Contribution | Relevance to hKask |
|----------|-----------------|-------------------|
| **Gene Kim** | Co-author of *The Phoenix Project*, *The DevOps Handbook*, and *Accelerate*. Codified the Three Ways of DevOps: Flow, Feedback, and Continual Learning. His insight: IT is not a cost center — it's a value stream. | hKask's kata system (PDCA cycles) is the Continual Learning way embodied. The CNS is the Feedback way. But where is Flow? The time from a user's intent ("I need an agent that does X") to a running replicant is unmeasured. |
| **Jez Humble** | Co-author of *Continuous Delivery* and *Accelerate*. Defined the principle that software should always be in a deployable state. Introduced the deployment pipeline as the central metaphor for delivery. | hKask's skill registry + FlowDef cascade is a deployment pipeline for agent behavior. But there's no equivalent of a canary deploy for skills — you can't test a new WordAct in 1% of sessions before rolling it out. |
| **Sam Newman** | Author of *Building Microservices* and *Monolith to Microservices*. Defined the characteristics of evolutionary architecture and the importance of independently deployable units. | hKask's 53 crates with deep-module discipline and hexagonal ports is a microservices architecture at the crate level. Each MCP server is an independently deployable unit. Newman's "evolutionary architecture" maps to hKask's P7. |
| **Matthew Skelton & Manuel Pais** | Authors of *Team Topologies*. Defined the four fundamental team types (Stream-Aligned, Enabling, Complicated Subsystem, Platform) and three interaction modes (Collaboration, X-as-a-Service, Facilitating). Their insight: **platform is a product, not a project.** | hKask's architecture maps cleanly to Team Topologies — the service layer is a Platform, the Curator is an Enabling Team, hkask-inference is a Complicated Subsystem. But hKask has no Team Topology of its own operators. |

### 1.4 Synthesis: What These Exemplars Would Ask of hKask

| Exemplar | The Question |
|----------|-------------|
| Vogels | "Can I access every platform capability through a stable API? What's your API versioning strategy?" |
| Hightower | "Does a new user need to learn CNS, OCAP, FlowDef, and rJoule just to create their first agent?" |
| Majors | "Can a developer ask an arbitrary question about their agent's behavior and get an answer without pre-aggregating metrics?" |
| Forsgren | "What's your deployment frequency for skills? What's your change failure rate? Show me the data." |
| Wardley | "Where's the map? Which components are commodity vs. custom? What are you building that should be buying?" |
| Allspaw | "What did you learn from the last incident? How did that learning change the system?" |
| Nygard | "If one agent pod crashes, what else breaks? Where are your bulkheads?" |
| Kim | "What's the value stream? From user intent to running agent — how long does it take and where does it slow down?" |
| Humble | "Can you deploy a skill change at 4 PM on Friday with confidence?" |
| Skelton & Pais | "Who is the product manager for the hKask platform? Who decides what gets built next?" |

---

## 2. What Is a Platform?

### 2.1 The Academic Definition

Platform research converges on a definition from **Tiwana, Konsynski & Bush (2010)** — the most cited in the Information Systems literature:

> A software platform is **the extensible codebase of a software-based system that provides core functionality shared by apps that interoperate with it, and the interfaces through which they interoperate.**

This definition has three essential properties:

1. **Extensible codebase** — the platform is not closed. External parties (complementors) build on it.
2. **Core functionality shared by apps** — the platform provides capabilities that apps consume, not duplicate.
3. **Interfaces through which they interoperate** — the API is not an afterthought; it IS the platform.

**Cusumano, Gawer & Yoffie (2019)** add the ecosystem dimension:

> Platforms connect individuals and organizations for a common purpose or to share a common resource.

They classify platforms into two types:

| Type | Definition | Example | hKask Mapping |
|------|-----------|---------|---------------|
| **Transaction Platform** | Facilitates exchange between participants | Uber, Airbnb, eBay | hKask's MCP server marketplace (skills discoverable and composable) |
| **Innovation Platform** | Provides a foundation for others to build complementary products | iOS, AWS, Kubernetes | hKask's core platform — agents, skills, templates |

hKask is primarily an **innovation platform** with transaction platform properties (skills can be shared/discovered).

**Tiwana (2014)** in *Platform Ecosystems* identifies three gears of platform orchestration:

| Gear | Definition | hKask Implementation |
|------|-----------|---------------------|
| **Architecture** | Reduces structural complexity through modularity | Deep-module discipline, hexagonal ports, capability membranes |
| **Governance** | Reduces behavioral complexity through decision rights, control mechanisms, pricing | OCAP, P2 Affirmative Consent, energy/gas pricing, delegation tokens |
| **Strategy** | Aligns platform evolution with ecosystem co-evolution | Kata PDCA cycles, Curator metacognition, CNS variety tracking |

### 2.2 hKask as a Platform: The Three-Gear Assessment

| Gear | hKask Strength | hKask Gap |
|------|---------------|----------|
| **Architecture** | 53 crates with deep-module discipline. Every public item survives the deletion test. Hexagonal ports prevent infrastructure lock-in. Capability membranes enforce zero-trust between loops. | The architecture is built for platforms engineers, not platform *users*. A complementor creating a skill doesn't see the architecture — they see YAML. Is the YAML surface as mature as the Rust surface? |
| **Governance** | OCAP with attenuation, energy budgeting with rJoule, P2 Affirmative Consent, DelegationToken for every operation. The Magna Carta (P1–P4) is a constitutional governance document. | Governance is enforced internally but not communicated externally. The Magna Carta exists in `PRINCIPLES.md` — does the average hKask user know their rights? |
| **Strategy** | Kata PDCA drives continuous capability development. Curator metacognition detects drift. CNS variety tracking enforces Ashby's Law. | No market-facing strategy. hKask doesn't know whether it's competing with LangChain, Cursor, or Kubernetes. Wardley Mapping would reveal positioning. |

---

## 3. Loyalty-Anchored Platform Design (vs. Lock-In)

### 3.1 The Lock-In Playbook

Traditional platforms retain users through **structural lock-in**:

| Lock-In Mechanism | How It Works | Example |
|-------------------|-------------|---------|
| **Data gravity** | Your data is in our format, in our database. Export is lossy or impossible. | AWS S3 (data egress fees), Salesforce (proprietary data model) |
| **Proprietary APIs** | Your integrations use our SDK. Switching means rewriting every integration. | iOS (Swift/UIKit), Google Cloud (client libraries) |
| **Network effects** | All your collaborators are here. Leaving means losing the network. | Slack, GitHub, Jira |
| **Behavioral lock-in** | You've built workflows, templates, CI/CD pipelines. Rebuilding them is too expensive. | Kubernetes (Helm charts, operators), Terraform (HCL modules) |
| **Opacity as control** | You can't see how it works, so you can't replicate it. "Just trust us." | Most SaaS platforms |

Lock-in is *effective* but breeds **resentment**. Users stay because they must, not because they choose to. When a viable alternative appears, they flee.

### 3.2 The Loyalty Alternative

A **loyalty-anchored platform** earns retention through value, not barriers:

| Loyalty Mechanism | How It Works | hKask Implementation |
|-------------------|-------------|---------------------|
| **Data sovereignty** | User owns their data. Export is complete and lossless. Portability is a first-class feature. | Per-pod SQLCipher files. Backup as portable archive. P1 User Sovereignty. "Your data is yours." |
| **Open interfaces** | APIs are documented, stable, and standards-based. No proprietary SDK lock-in. | MCP protocol (open standard). OpenAPI spec auto-generated via utoipa. CLI/API/MCP tri-surface equivalence. |
| **Composable by choice** | Users combine capabilities freely. No forced bundling. Pay only for what you use. | Energy/gas/rJoule per-operation pricing. Skills compose via FlowDef — no mandatory skill bundles. |
| **Co-evolutionary governance** | Users participate in platform evolution. Decisions are transparent. Consent is required. | P2 Affirmative Consent. P3 Generative Space. Kata coaching invites users into improvement cycles. |
| **Radical transparency** | Architecture is documented. Failures are visible. The platform teaches you how it works. | CNS spans emit at every membrane crossing. Self-healing is CNS-audited. Deep-module justifications are public. |

### 3.3 hKask's Magna Carta as a Loyalty Constitution

hKask's four foundational principles (P1–P4) are a **platform constitution** that encodes loyalty into the architecture:

| Principle | What It Says | Loyalty Dimension |
|-----------|-------------|------------------|
| **P1 — User Sovereignty** | "Users own their data, agents, and computation. The platform never claims ownership." | Data sovereignty. Portable backups. No data gravity. |
| **P2 — Affirmative Consent** | "No operation happens without explicit user authorization. Consent is granular, revocable, and auditable." | Co-evolutionary governance. DelegationToken for every operation. |
| **P3 — Generative Space** | "Users extend the platform without permission. Anyone can create a skill, template, or agent definition." | Composable by choice. Self-service. No approval gates. |
| **P4 — Clear Boundaries (OCAP)** | "Capability is explicit, not ambient. You can only do what you've been granted permission to do." | Radical transparency. Capability membranes. No hidden parameters. |

**The loyalty contract:** hKask says: "We will never lock you in. Your data is portable. Your agents are yours. You can extend the platform yourself. We only act with your consent. And we show you exactly what we're doing."

This is the opposite of the lock-in playbook. It's a bet that users will stay because they *trust* the platform, not because they *can't leave*.

### 3.4 What Loyalty Demands of Platform Engineering

A loyalty-anchored platform must develop capabilities that lock-in platforms never need:

| Loyalty Demand | Platform Engineering Capability | hKask Status |
|---------------|-------------------------------|-------------|
| **Provable sovereignty** | Cryptographic proof that user data has not been accessed without consent. Audit trail for every data access. | Partial — CNS spans record operations, but there's no cryptographic receipt for the user. |
| **Seamless portability** | One-command backup export. Restore to any hKask instance. No loss. No lock-in. | Planned — backup as portable SQLCipher archive. Not yet implemented. |
| **Consent observability** | User can see every DelegationToken they've issued, to whom, for what, and revoke any of them. | Partial — DelegationToken exists in the type system but there's no user-facing consent dashboard. |
| **Governance transparency** | All platform decisions (Curator directives, spec changes, energy budget adjustments) are visible and explainable. | Strong — CNS spans at every membrane crossing. Curator metacognition is documented. |
| **Exit assistance** | If a user wants to leave, the platform helps them. Export all data, agents, skills. No friction. | Design intent — per-pod SQLCipher, portable backups. Implementation pending. |
| **Co-evolution infrastructure** | Users can propose changes to the platform itself. Platform evolves with its ecosystem, not despite it. | Kata PDCA cycles invite user participation. Contract proposal CNS spans exist (ContractProposed/Accepted/Rejected). |

### 3.5 Counterpoint: The Risk of Loyalty-First Design

Loyalty is harder than lock-in. The risks are real:

| Risk | Why It Matters | Mitigation |
|------|---------------|------------|
| **Loyalty is slow** | Lock-in platforms grow faster because switching costs create inertia. Loyalty takes time to earn trust. | Network effects from quality, not captivity. Each genuinely loyal user attracts more than ten locked-in users. |
| **Portability enables departure** | If it's easy to leave, users might leave. That's the point — but it's scary for adoption metrics. | Track loyalty, not retention. A user who *can* leave but *chooses* to stay is more valuable than one who can't leave. |
| **Transparency exposes weakness** | If you show every failure, users see every failure. Lock-in platforms hide failures. | Transparency builds trust. Users who see failures and see them fixed trust the platform more than users who never see failures. |
| **Co-evolution is messy** | Letting users influence platform direction means saying "no" to good ideas and managing conflicting demands. | Governance clarity. The Magna Carta defines what can and cannot change. Contract proposal/rejection CNS spans make decisions traceable. |

---

## 4. Skills a Loyalty-Anchored, Continuously Improving Platform Must Develop

hKask's existing 43 capabilities (39 skills, 2 templates, 1 bundle, 1 legacy) are comprehensive but not organized for platform engineering. A loyalty-anchored platform engineer replicant needs to *compose* these skills into platform-maintenance workflows and *develop* new capabilities that don't yet exist.

### 4.1 Existing Skills Repurposed for Platform Engineering

These skills already exist in hKask. The Platform Engineer replicant would activate them on a cadence:

| Skill | Platform Engineering Use | Cadence |
|-------|------------------------|---------|
| **semantic-graph-audit** | Audit the 53-crate dependency graph. Detect cycles, orphans, contract drift, and impedance mismatches between crate boundaries. Classify edges by constraint force. | Weekly |
| **deep-module** | Apply the deletion test to every crate's public surface. Report modules where surface > 7 items. Propose consolidation or decomposition. | Monthly |
| **pragmatic-cybernetics** | Analyze the four-loop feedback architecture. Is Ashby's Law satisfied? Are variety deficits accumulating? Is the algedonic pathway firing correctly? | Daily |
| **bug-hunt** | Run an expedition against the platform itself — not against agents, but against the platform's reliability. Find edge cases in FlowDef execution, CNS span emission, OCAP enforcement. | Monthly |
| **diagnose** | When an SLO breaches, run the full diagnosis loop: reproduce → anchor → hypothesise → instrument → fix → regression-test. Produce a root-cause analysis for the Curator. | On SLO breach |
| **improve-codebase-architecture** | Hunt for deepening opportunities. Where can the platform add depth (high benefit/cost ratio) by reducing public surface while increasing internal capability? | Monthly |
| **mcda** | When multiple interventions compete for attention, evaluate them on: user impact, implementation cost, risk, alignment with Magna Carta. Produce ranked recommendations with sensitivity analysis. | On demand |
| **superforecasting** | Calibrated probability forecasts: "What is the probability that CNS alert fatigue becomes a problem by Q4 2026?" "Probability that a skill injection vulnerability is found in the wild within 12 months?" | Quarterly |
| **adversarial-red-team** | Test the platform's defenses. Can a compromised Inference loop read Curator state? Can a malicious skill exhaust the energy budget? Can a forged DelegationToken bypass OCAP? | Monthly |
| **scenario-builder** | Scenario planning: "What if the primary inference provider goes down for 24 hours?" "What if a replicant gains write access to the skill registry?" "What if CNS variety tracking saturates?" | Quarterly |
| **handoff** | Between platform iterations, capture: what was done, what remains, key decisions, open questions. Ensures continuity across PDCA cycles. | Per cycle |

### 4.2 New Skills the Platform Engineer Replicant Needs

These capabilities don't yet exist in hKask's skill registry. They must be developed as FlowDef manifests (steps that orchestrate existing capabilities) or as new KnowAct templates:

| New Skill | Type | Function | Inputs | Outputs |
|-----------|------|----------|--------|---------|
| **platform-slo-evaluator** | FlowDef | Evaluate all registered SLOs against CNS span data. Compute compliance, error budget remaining, burn rate. Emit `cns.slo.evaluated` spans. Classify breaches by severity. | SloDefinition registry, CNS ν-event store | SloEvaluation per SLO, breach alerts |
| **platform-contract-auditor** | FlowDef | Run the CI contract check script. Parse compiler errors. Detect trait ↔ implementation drift across all port boundaries. Map each violation to the affected crate and port trait. | `cargo check` output, port trait registry | Contract violation report, CNS spans |
| **platform-health-scorer** | KnowAct | Aggregate SLO compliance, test pass rate, contract violations, dependency health, CNS variety metrics into a single platform health score (0.0–1.0). Track score over time. Identify degradation trends. | SloEvaluation list, test results, contract report, dependency graph | Health score (0.0–1.0), trend line, degradation alerts |
| **platform-dx-analyzer** | KnowAct | Analyze CNS spans for developer experience signals. Time from sign-in to first agent creation. Skill creation frequency. Error resolution patterns. Identify friction points in the user journey. | CNS spans (SessionOpen, AgentPod, Skill, Tool, SelfHeal) | DX report: time-to-first-agent, adoption rate, friction heatmap |
| **platform-wardley-mapper** | KnowAct | Map hKask's components on a Wardley Map (value chain × evolution axis). Classify each component: Genesis, Custom, Product, Commodity. Identify what should be commoditized, what should stay product, and what is missing entirely. | hKask architecture docs, crate inventory, skill registry | Wardley Map artifact, strategic recommendations |
| **platform-bulkhead-auditor** | FlowDef | Identify failure domains. For each agent pod, skill, and MCP server: if this crashes, what else breaks? Map blast radius. Recommend bulkheads where blast radius > 1 component. | Crate dependency graph, CNS spans for each loop membrane | Bulkhead audit: blast radius per component, recommendations |
| **platform-consent-auditor** | FlowDef | Audit every DelegationToken issued. Report: who issued it, to whom, for what resource, with what action, when does it expire, has it been used. Flag anomalous patterns (e.g., token with Critical severity issued outside working hours). | DelegationToken registry, CNS spans | Consent audit report, anomaly alerts |
| **platform-portability-verifier** | FlowDef | Verify that a user's data can be fully exported. Run backup export. Verify export integrity (checksums, record counts). Verify import on a clean hKask instance. Report any data loss or corruption. | Per-pod SQLCipher file, backup command | Portability verification report |
| **platform-governance-transparency-reporter** | KnowAct | Generate a human-readable report of all platform governance decisions in the last N days. Curator directives, spec changes, energy budget adjustments, contract proposals/acceptions/rejections. Explain each decision in plain language. | CNS spans (Curation, Spec, ContractProposed/Accepted/Rejected, CuratorDirective) | Governance transparency report |
| **platform-loyalty-scorecard** | KnowAct | The ultimate loyalty metric. Combines: data sovereignty score, portability score, consent transparency, governance visibility, exit friction, self-service capability, and user satisfaction. Produces a loyalty score (0.0–1.0) with trend. | All platform health metrics, consent audit, portability verification, governance report, DX analysis | Loyalty score (0.0–1.0), loyalty trend, degradation alerts |

### 4.3 Bundling the Platform Engineer

The `skill-bundler` composes these skills into a Platform Engineer bundle:

```yaml
bundle:
  name: Platform Engineer
  goal: >
    Maintain platform health through continuous SLO monitoring,
    architectural audit, contract verification, and loyalty-scored
    recommendations. Never modify code or config without human
    consent (P2).
  skills:
    # Diagnostic (daily)
    - pragmatic-cybernetics
    - platform-slo-evaluator

    # Audit (weekly)
    - semantic-graph-audit
    - platform-contract-auditor

    # Deep assessment (monthly)
    - deep-module
    - bug-hunt
    - platform-health-scorer
    - platform-bulkhead-auditor
    - platform-dx-analyzer
    - improve-codebase-architecture
    - adversarial-red-team

    # Strategic (quarterly)
    - platform-wardley-mapper
    - scenario-builder
    - superforecasting
    - platform-loyalty-scorecard
    - platform-governance-transparency-reporter

    # Sovereignty (monthly)
    - platform-consent-auditor
    - platform-portability-verifier

    # Decision support (on demand)
    - mcda

    # Continuity
    - handoff

  convergence:
    method: weighted_avg
    threshold: 0.80

  energy_budget:
    gas: 500_000     # 2 rJ per full audit cycle
    rjoule: 5        # inference for KnowAct skills
```

This bundle is the computational embodiment of a world-class platform engineer — continuously auditing, recommending, and (with consent) improving the platform. It doesn't replace the human platform engineer; it amplifies them. The human sets the direction; the replicant maintains the vigilance.

### 4.4 The Loyalty Feedback Loop

The Platform Engineer replicant closes a cybernetic loop unique to loyalty-anchored platforms:

```
User Sovereignty (P1) → Portability Verified → Loyalty Score ↑
Affirmative Consent (P2) → Consent Audited → Trust Accumulates
Generative Space (P3) → Adoption Measured → Platform Evolves
Clear Boundaries (P4) → Bulkheads Audited → Resilience Improves
    ↓
Platform Loyalty Scorecard (monthly)
    ↓
Recommendations to Curator → Human Consent → Platform Improvement
    ↓
(Scores improve) → (Trust deepens) → (Adoption grows) → (loop)
```

This loop is what lock-in platforms cannot replicate. They can measure retention but not loyalty. They can count users but not trust. hKask's architecture — Magna Carta principles, CNS observability, OCAP boundaries, P2 consent — makes loyalty *measurable* for the first time.

---

## 5. Problem Statement

hKask is a platform that builds agents. It has sophisticated internal regulation (CNS algedonic pathway, energy budgeting, capability membranes) and strong architectural discipline (hexagonal ports, deep-module surface constraints, property-based testing). Its Magna Carta (P1–P4) already encodes loyalty-anchored design — user sovereignty, affirmative consent, generative space, clear boundaries. But it lacks the **platform engineering lens** — the discipline of treating the platform as a product with measurable health, explicit contracts with users, and continuous improvement driven by data. As the exemplars above would observe: the architecture is ready; the platform operating model is not.

Concretely:

- CNS knows something is wrong but not what the user contract says (no SLOs — Forsgren would demand DORA metrics; Majors would demand arbitrary queries)
- No developer experience metrics (time-to-first-agent, skill adoption, satisfaction — Hightower would ask: "how long to first agent?")
- No continuous platform auditing agent — skills exist but are human-activated
- The loyalty contract (Magna Carta) is constitutionally encoded but not operationally verified — no consent audit, no portability verification, no loyalty scorecard
- Identified gaps acknowledged but not systematically closed by user impact

A world-class platform engineer — combining Vogels' API discipline, Hightower's user empathy, Majors' observability rigor, Forsgren's measurement science, and Wardley's situational awareness — would see hKask's architecture as **80% of the way to a self-maintaining, loyalty-anchored platform**. The sensors, regulatory loops, and skills are all there. Missing: the contract layer (SLOs), the measurement layer (PaaP metrics), the verification layer (loyalty scorecard), and the automation layer (Platform Engineer replicant).

---

## 6. Current Condition — Platform Engineering Audit

### 6.1 What hKask Already Excels At

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

### 6.2 Identified Gaps

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

### 6.3 CNCF Maturity Assessment

| Level | hKask Status |
|-------|-------------|
| L1 Provisional | ❌ Not here |
| L2 Operational | ⚠️ Partial — CNS automates regulation, gap docs acknowledge gaps |
| L3 Scalable | ✅ Skills are self-service, FlowDef templates, CNS tracks variety |
| L4 Optimizing | ✅ Kata PDCA, SelfHealer, mutation testing — but missing platform-level KPIs |

**Current:** L3→L4 transition. The three investments below complete the L4 transition.

---

## 7. Target Condition — The Three Investments

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

## 8. Investment 1 — SLOs Wired to CNS

hKask already has the full cybernetic feedback loop (Sensor → Model → Comparator → Regulator → Actuator). SLOs enrich the Comparator with user-facing contract thresholds.

### 8.1 Proposed SLOs

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

### 8.2 Error Budget Model

```
Error Budget = (1 - Target) × Total Operations in Window
```

| SLO ID | Monthly Error Budget | Burn Rate Alert (>2% in 1h) |
|--------|---------------------|---------------------------|
| SLO-INF-001 | ~43 min downtime | Yes |
| SLO-SKL-001 | ~216 failures (1k/day) | Yes |
| SLO-API-001 | ~43 min downtime | Yes |

### 8.3 CNS Integration

New types: `SloDefinition`, `SloSeverity` (Critical/High/Medium), `SloEvaluation`.

New CNS span: `cns.slo.evaluated` — emitted per evaluation cycle with `slo_id`, `current_compliance`, `error_budget_remaining`, `burn_rate`.

Algedonic integration: `AlgedonicManager` gains `SloBreach` trigger type. Error budget burn rate exceeding threshold escalates identically to variety deficits.

### 8.4 API Surface

| Endpoint | Purpose |
|----------|---------|
| GET /api/v1/slos | List all SLOs with current compliance |
| GET /api/v1/slos/:id | Detailed status: compliance, error budget, burn rate, history |
| POST /api/v1/slos | Define new SLO (Admin only) |
| DELETE /api/v1/slos/:id | Remove SLO (Admin only) |

### 8.5 Skills to Activate

- **goal-analysis**: Extract structured SLOs from platform intent
- **mcda**: Rank SLO candidates by user impact vs. implementation cost
- **pragmatic-semantics**: Classify each SLO by constraint force
- **qa-script-builder**: Build SLO compliance verification pipeline

---

## 9. Investment 2 — Platform-as-Product Metrics

### 9.1 Proposed Metrics

| Metric | Definition | CNS Span | Cadence |
|--------|-----------|----------|---------|
| **Time to First Agent** | Wall-clock time from sign-in to first successful agent creation | cns.onboarding.complete → cns.agent.created | Per user |
| **Time to 10th Skill** | Wall-clock time from first skill creation to 10th | cns.skill.created | Per user |
| **Skill Adoption Rate** | % of created skills used in ≥3 sessions within 30 days | cns.tool.skill_dispatch | Monthly |
| **Platform NPS** | Prompt-based survey in REPL after 10th session | N/A (survey) | Quarterly |
| **Active User Retention** | % of users active in both current and previous 30-day windows | cns.session.* | Monthly |
| **Error Resolution Time** | Time from CNS alert to SelfHealer resolution or human intervention | cns.algedonic.* → cns.heal.* | Per incident |

### 9.2 CNS Integration

New CNS span: `cns.platform.metric` — emitted per metric evaluation with `metric_name`, `value`, `window`, `trend`.

### 9.3 API Surface

| Endpoint | Purpose |
|----------|---------|
| GET /api/v1/platform/metrics | Get all platform metrics with current values |
| GET /api/v1/platform/metrics/:name | Get detailed history for one metric |

### 9.4 Skills to Activate

- **scenario-builder**: What happens to adoption if SLO-INF-001 breaches for 24h?
- **superforecasting**: Calibrated probability: "NPS > 50 by Q4 2026"
- **structured-extraction**: Extract DX signals from session transcripts

---

## 10. Investment 3 — Platform Engineer Replicant

The ultimate move: create a hKask agent that continuously audits and improves the platform — using hKask's own skills. This is not just automation; it is the platform engineering discipline encoded as an agent. It combines Vogels' API-first thinking (every audit is a programmatic invocation), Hightower's user empathy (recommendations are actionable and human-readable), Majors' observability rigor (every decision is CNS-observable), and Forsgren's measurement science (every recommendation is backed by data).

### 10.1 Replicant Definition

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

### 10.2 Operating Cadence

| Frequency | Activity | Skills | Output |
|-----------|----------|--------|--------|
| **Daily** | CNS SLO check — are any error budgets burning? | pragmatic-cybernetics | SLO health dashboard update |
| **Weekly** | Dependency graph audit — any new cycles, orphans, drift? | semantic-graph-audit | Dependency health report |
| **Monthly** | Full platform audit — deep-module review, bug hunt expedition | deep-module + bug-hunt | Platform health score + prioritized recommendations |
| **On Alert** | SLO breach diagnosis | diagnose | Root cause analysis + proposed remediation |
| **On Demand** | User-requested review ("audit crate X") | improve-codebase-architecture | Targeted refactoring proposal |

### 10.3 OCAP Boundaries

| Access | Scope | Mechanism |
|--------|-------|-----------|
| **Read** | CNS spans, SLO evaluations, dependency graph, crate public surfaces, test results | Direct via service layer |
| **Signal** | Recommendations to Curator, SLO breach alerts, health score changes | CNS spans + CuratorDirective |
| **Write** | Platform health reports (read-only triple), metric evaluations | EpisodicMemory via OCAP |
| **Never** | Source code, configuration files, deployment, agent definitions, wallet operations | Enforced by capability membrane |

### 10.4 CNS Integration

New CNS spans:
- `cns.platform.audit.started` — Platform audit cycle begins
- `cns.platform.audit.completed` — Audit cycle complete with findings
- `cns.platform.recommendation` — Replicant proposes an intervention
- `cns.platform.recommendation.accepted` — Human curator accepts
- `cns.platform.recommendation.rejected` — Human curator rejects (with reason)

---

## 11. Integration with Existing Systems

### 11.1 How This Composes with the Four Patterns

| Pattern | Enhancement |
|---------|------------|
| **A: Skills Model** | SLOs, PaaP metrics, and platform audit are FlowDef skills — no new types needed |
| **B: CNS Feedback Loop** | SLO breach is a new algedonic trigger; PaaP metrics are new CNS spans; Platform Engineer replicant is a new observer |
| **C: Curator + 7R7** | Platform Engineer replicant is a new agent in the Curator's charge. Curator metacognition now includes platform health as a dimension |
| **D: AgentPod** | Platform Engineer replicant gets its own pod with read-only access to platform state |

### 11.2 How This Composes with the Four Loops

| Loop | Enhancement |
|------|------------|
| **Inference** | SLO-INF-001/002 monitor inference health. Platform Engineer replicant uses inference for audit runs |
| **Memory** | SLO-MEM-001 monitors consolidation. PaaP metrics stored as episodic memories |
| **Curation** | Platform Engineer replicant reports to Curator. New CNS spans for audit/recommendation lifecycle |
| **Cybernetics** | SLO breach triggers enrich algedonic pathway. PaaP metric spans feed VarietyTracker |

### 11.3 Implementation Sequence

| Phase | What | Duration Est. | Prerequisites |
|-------|------|--------------|---------------|
| **Phase 1** | SloDefinition type + CNS integration + 3 seed SLOs (INF-001, SKL-001, API-001) | 2-3 PDCA cycles | None |
| **Phase 2** | Error budget tracking + algedonic SLO breach escalation | 2 PDCA cycles | Phase 1 |
| **Phase 3** | PaaP metric definitions + CNS spans + API | 2 PDCA cycles | Phase 1 |
| **Phase 4** | Platform Engineer replicant definition + OCAP boundaries + basic audit skills | 3 PDCA cycles | Phase 1+2+3 |
| **Phase 5** | Full replicant operating cadence (daily/weekly/monthly) | 2 PDCA cycles | Phase 4 |

---

## 12. Success Criteria

| Criterion | Measurement | Target |
|-----------|------------|--------|
| SLOs are defined and tracked | SloEvaluation counts in CNS | ≥9 SLOs active within 30 days of Phase 1 start |
| Error budgets inform decisions | % of SLO breaches that trigger an intervention | >80% within 60 days |
| Platform metrics are measurable | PaaP metric CNS spans emitted | All 6 metrics emitting within 30 days of Phase 3 start |
| Platform Engineer replicant is active | cns.platform.audit.* spans | Weekly audits running within 30 days of Phase 4 start |
| Replicant recommendations are actionable | Acceptance rate of recommendations | >60% acceptance within 90 days |

---

## 13. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| SLO alert fatigue | Medium | High | Start with 3 SLOs, expand only when signal-to-noise proven |
| Replicant recommendations too frequent | Medium | Medium | Monthly audit cadence; batch recommendations |
| Platform Engineer replicant scope creep | Low | Medium | OCAP boundaries prevent write access; charter is narrow |
| SLO targets too aggressive | Medium | Low | Start with loose targets (99.0%), tighten based on actual performance |
| PaaP metrics gamed | Low | Medium | Metrics anchored in CNS spans — hard to fake without system compromise |

---

## 14. References

- hKask Architecture Master: `docs/architecture/hKask-architecture-master.md`
- hKask Principles: `docs/architecture/core/PRINCIPLES.md`
- MDS Specification: `docs/architecture/core/MDS.md`
- Testing Discipline: `docs/architecture/core/TESTING_DISCIPLINE.md`
- Google SRE Book: Service Level Objectives (§4), Monitoring Distributed Systems (§6)
- Team Topologies: Skelton & Pais (2019) — Platform as a Product, Interaction Modes
- CNCF Platform Engineering Maturity Model: `tag-app-delivery.cncf.io`
- Wardley Mapping: Simon Wardley — situational awareness for platform strategy
