---
title: "Dual-Axis Ontology — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Dual-Axis Ontology

## Why Two Axes?

Most systems pick a single source of truth. hKask does not. P5.4 of the architecture principles declares that "no single source of truth" is not a bug — it is the design. Every artifact in hKask has both a state identity and a process identity. It is simultaneously a noun AND a verb.

The two axes are:

| Axis | Master Ontology | Question | Domain |
|---|---|---|---|
| **Process (Flow)** | PKO | How did this come to be? What flow? | Procedures, steps, executions — the *verb* dimension |
| **State (Entity)** | Dublin Core + BIBO | What is this? What type? Who made it? | Entities, resources, types, metadata — the *noun* dimension |

The architectural metaphor is deliberate. P5.4 invokes Heisenberg: the more precisely you sample state (DC typing), the less you can know about process position (PKO flow), and vice versa. You are always sampling, never arriving at truth. The bridges are sampling instruments, not truth claims.

## The 5W1H Core

Before either axis engages, there is a simpler filter. P5.2 defines the 5W1H ontological core — **Who, What, When, Where, Why, How** — as the drop-dead-simple gate every artifact must pass. An artifact that answers none of these six questions is ontological noise. This is not abstract philosophy. It is operational:

- **Who** — agent, replicant, bot (anchored by P12 replicant host mandate)
- **What** — entity, resource, data, state
- **When** — time, sequence, duration, temporal scope
- **Where** — pod boundary, namespace, domain
- **Why** — goal, purpose, constraint motivation (anchored by Magna Carta P1–P4)
- **How** — method, mechanism, procedure, execution path

The 5W1H core is grounded in Ontology Design Pattern methodology (Norouzi et al., 2025): instead of navigating entire complex ontologies, hKask extracts compact, requirement-driven patterns. The six questions are the minimal set that distinguishes "understood" from "not understood."

## Bridge Crates

Two shared crates implement the dual-axis core:

### `hkask-bridge-dublincore` — The State Axis

This crate (`crates/hkask-bridge-dublincore/src/lib.rs`, 128 lines) provides canonical URI constants for Dublin Core, BIBO, and CiTO vocabularies. It is a pure-vocabulary crate: no dependencies, no reasoners, no overhead. It defines the type `DcConcept = &'static str` and exports constants like `TITLE`, `CREATOR`, `DATE`, `ARTICLE`, `BOOK`, `CITES`, `SUPPORTS`, `REFUTES`.

Two mapping helpers earn the bridge its keep. `mime_to_dc_type()` maps MIME types to Dublin Core resource types — `"image/png"` → `STILL_IMAGE`, `"application/json"` → `DATASET`. `kind_to_bibo()` maps informal labels like `"preprint"` or `"conference"` to their BIBO equivalents. These thin functions sit between raw data and ontological precision, answering the *Who* and *What* questions by connecting unstructured metadata to structured vocabularies.

### `hkask-bridge-pko` — The Process Axis

This crate (`crates/hkask-bridge-pko/src/lib.rs`, 174 lines) maps hKask's procedural concepts to the PKO (Procedural Knowledge Ontology) standard. PKO is built on PROV-O (Activity, Agent), P-Plan (Step, Plan), and DCAT (Resource). The crate exports `PkoConcept = &'static str` constants: `PROCEDURE`, `HAS_STEP`, `STEP_EXECUTION`, `ISSUE_OCCURRENCE`, `USER_FEEDBACK_OCCURRENCE`, `AGENT`, `ROLE`, `HAS_VERSION`.

Three mapping functions connect domain workflows to ontological concepts. `kanban_status_to_pko_execution()` maps task statuses (`"in_progress"` → `"pko:ProcedureExecutionStatus/inProgress"`). `docproc_stage_to_pko_step()` classifies document processing stages as PKO Steps, Functions, or Actions. `research_stage_to_pko()` maps research workflow stages (`"hypothesis"` → `USER_QUESTION_OCCURRENCE`, `"evaluate"` → `STEP_VERIFICATION`). These answer the *How* and *Why* questions — connecting concrete procedure fragments to a shared process vocabulary.

## How Bridges Earn Their Keep

P5.3 is explicit: bridges must themselves pass the 5W1H test. A bridge that doesn't connect a 5W1H question to domain-specific depth is a P5 violation. The two bridge crates earn their keep by different routes:

- **Dublin Core bridge** answers *What is this thing?* and *Who made it?* for any artifact in hKask. Every MCP server depends on it because every server produces resources that need typing. A condensed document, a generated image, a research finding — all carry DC identity.

- **PKO bridge** answers *How was this produced?* and *What flow is it part of?* Every server's workflow — training a model, processing a document, searching for papers — is a PKO Procedure composed of Steps and producing Executions.

## Beyond the Core

The dual-axis core (PKO + DC+BIBO) is the minimum viable ontology for any server. But some domains need more specificity. The architecture principles define domain-specific bridges layered on top where DC+BIBO's state axis isn't specific enough:

- **FIBO** (financial concepts) supplements the `companies` MCP server
- **GOLEM** (narrative structure) supplements the `replica` MCP server
- **CogAT** (cognitive concepts) supplements the `memory` MCP server
- **ML-Schema** (ML experiments) supplements the `training` MCP server
- **OMC** (media creation) supplements the `media` MCP server

These follow the same `fibo.rs` pattern: concept URI constants, field-to-concept mapping functions, no dependencies, no reasoners. Each is typically ≤150 lines. They are supplements, not alternatives to the dual-axis core.

## The Architectural Invariant

P8.1 states the invariant clearly: **hKask never requires knowledge of a full domain ontology.** All interaction with domain ontologies flows through thin bridges. The dual-axis core provides the minimum viable ontology for any server; domain bridges are opt-in specificity. You can stand up a new MCP server, and without writing a single ontological constant, your artifacts carry DC identity (the noun) and PKO flow semantics (the verb). That's the dual axis working at the architectural level.
