---
title: "hKask hLexicon — Minimal Composition Vocabulary"
audience: [architects, developers, agents]
last_updated: 2026-06-13
version: "0.27.0"
status: "Active"
domain: "Application"
mds_categories: [domain]
---

# hKask hLexicon — Minimal Composition Vocabulary

## Contents

| Section | Description |
|---------|-------------|
| [Executive Summary](#executive-summary) | Overview of the 155-term minimal composition vocabulary |
| [Domain 1: WordAct — Prompting Language](#domain-1-wordact--prompting-language-37-terms) | Speech act theory terms for LLM interactions (37 terms) |
| [Domain 2: FlowDef — Process Flow Language](#domain-2-flowdef--process-flow-language-42-terms) | Workflow pattern terms for skill composition (42 terms) |
| [Domain 3: KnowAct — Cognition Language](#domain-3-knowact--cognition-language-76-terms) | Enactive cognition terms for metacognition (76 terms) |
| [Cross-Domain Composition Patterns](#cross-domain-composition-patterns) | How terms from different domains compose together |
| [hLexicon Grammar](#hlexicon-grammar) | Formal grammar for term usage and validation |
| [Academic Grounding References](#academic-grounding-references) | Academic sources for each domain |
| [hLexicon Term Index](#hlexicon-term-index-147-terms) | Alphabetical index of all 147 terms |
| [Expansion Capacity](#expansion-capacity) | Reserved term slots for domain extensions |
| [Usage Examples](#usage-examples) | Practical template integration examples |
| [References](#references) | Citations and references |

## Executive Summary

**hLexicon** is the minimal composition vocabulary (155 term-slots across 3 domains) for composing templates in the hKask system.

1. **WordAct** — Language for prompting/LLM interactions (speech act theory)[^austin]
2. **FlowDef** — Language for process/skill composition (workflow patterns)[^van-der-aalst]
3. **KnowAct** — Language for cognition and metacognition (enactive cognition)[^varela]

**Design Principles:**
- **Minimal:** 155 term-slots across 3 domains (37 WordAct + 42 FlowDef + 76 KnowAct)
- **Composable:** Terms combine to express complex patterns
- **Academic Grounding:** Based on established theory (Austin, Searle, van der Aalst, Varela)
- **LLM-Optimized:** Terms selected for LLM comprehension and consistent interpretation
- **Domain-Separated:** Each domain has its own namespace, but terms can cross-reference

**Usage:** Templates declare which hLexicon terms they use in their `[inference]` header:

```jinja2
[inference]
template_type: WordAct
lexicon_terms: [query, assert, contextualize, critique]
contract:
  input: {question: string, context: array}
  output: {answer: string, confidence: float}
```

---

## Canonicality & Derivation

**This document is the single source of truth for the hLexicon vocabulary.**
The term tables below are canonical. Code does not hand-maintain a parallel
list; it derives one from this file.

The three representations have deliberately different lifecycles:

| Artifact | Lifecycle | Editable by |
|----------|-----------|-------------|
| `hKask-hLexicon.md` (this file) | **Canonical** — human-authored | Maintainers (prose + tables) |
| `registry/hlexicon/hlexicon-workspace.yaml` | **Derived data** — committed; can be customized/extended | Tooling (regen) + subsystem registries |
| `hkask-types::lexicon` types | **Compiled** | Not user-editable |

```mermaid
flowchart LR
    MD["hKask-hLexicon.md<br/>(CANONICAL)"] -->|load_hlexicon_from_yaml<br/>(active)| YAML["hlexicon-workspace.yaml<br/>(derived, committed)"]
    MD -->|parse_markdown_catalog → render_workspace_yaml<br/>(implemented)| YAML
    YAML -->|hlexicon_yaml_matches_markdown<br/>(active test)| GATE{YAML ==<br/>markdown?}
    YAML --> GATE
    GATE -->|drift| ASK["test fails:<br/>regenerate OR<br/>restore markdown from git"]
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-LEX-001
verified_date: 2026-06-07
verified_against: crates/hkask-templates/src/lexicon.rs (load_hlexicon_from_yaml — active); parse_markdown_catalog, render_workspace_yaml, regenerate_workspace_yaml — implemented (see FocusingAssumption FA-Co1 in MDS.md); registry/hlexicon/hlexicon-workspace.yaml
status: VERIFIED
-->

**Process when the vocabulary changes:**

1. Edit the term tables in this file (add/remove/recategorize a term).
2. Regenerate the derived YAML explicitly (never automatic):
   `cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored`
   **Currently:** The `parse_markdown_catalog` → `render_workspace_yaml` → `regenerate_workspace_yaml` pipeline is implemented and tested (see `crates/hkask-templates/src/lexicon.rs` tests).
3. Commit both this file and `registry/hlexicon/hlexicon-workspace.yaml`.
4. The `hlexicon_yaml_matches_markdown` test (runs under `cargo test --workspace`)
   fails if the YAML and markdown disagree. On failure the maintainer decides:
   the markdown was corrupted → restore from git; or it evolved intentionally →
   run step 2 and commit. The derived YAML is **never** silently overwritten.

**Counting note:** the tables define **87 term-slots** (28 WordAct + 34 FlowDef
+ 25 KnowAct). One term, `transform`, is intentionally shared between WordAct
(Declarative) and FlowDef (Data Flow), so there are **86 globally-unique term
strings**. The lexicon is keyed by term string and therefore holds 86 entries;
`transform` takes its first/primary domain (WordAct), leaving 33 unique FlowDef
keys. `load_workspace_lexicon()` reflects this 86-term functional set.
Requirement: [`REQ-DOM-004`](../../specifications/specs/REQUIREMENTS.md).

---

## Domain 1: WordAct — Prompting Language (37 terms)

**Theoretical Basis:** Speech Act Theory (J.L. Austin, John Searle)[^austin][^searle]

Speech acts distinguish between:
- **Locution:** The words spoken
- **Illocution:** The intent (request, command, promise, declaration)
- **Perlocution:** The effect on the listener

**hKask WordAct Categories:**

### 1.1 Directive Acts (8 terms) — Requesting information or action

| Term | Definition | Example Usage |
|------|------------|---------------|
| `query` | Ask for information | "Query: What is the capital of France?" |
| `request` | Ask for action | "Request: Summarize this document" |
| `instruct` | Give step-by-step direction | "Instruct: Follow these steps..." |
| `command` | Authoritative directive | "Command: Execute the migration" |
| `prompt` | Elicit a response | "Prompt: Continue the story" |
| `probe` | Deep inquiry | "Probe: What assumptions underlie this?" |
| `challenge` | Question validity | "Challenge: Prove this claim" |
| `summon` | Call forth knowledge | "Summon: Recall relevant precedents" |

### 1.2 Commissive Acts (5 terms) — Committing to future action

| Term | Definition | Example Usage |
|------|------------|---------------|
| `pledge` | Commit to action | "Pledge: I will verify this" |
| `propose` | Offer a plan | "Propose: Let's try approach X" |
| `promise` | Guarantee outcome | "Promise: This will complete in 5m" |
| `undertake` | Accept task | "Undertake: I'll handle the analysis" |
| `commit` | Bind to course | "Commit: Proceeding with deployment" |

### 1.3 Assertive Acts (6 terms) — Stating facts or beliefs

| Term | Definition | Example Usage |
|------|------------|---------------|
| `assert` | State as fact | "Assert: The system is operational" |
| `claim` | State with uncertainty | "Claim: This should improve performance" |
| `report` | Present findings | "Report: Tests passed 95%" |
| `declare` | Formal statement | "Declare: Migration complete" |
| `affirm` | Confirm truth | "Affirm: Yes, this is correct" |
| `testify` | Witness-based statement | "Testify: I observed the failure" |

### 1.4 Expressive Acts (3 terms) — Expressing psychological state

| Term | Definition | Example Usage |
|------|------------|---------------|
| `acknowledge` | Recognize receipt | "Acknowledge: Received your request" |
| `apologize` | Express regret | "Apologize: The error was my fault" |
| `celebrate` | Express joy | "Celebrate: All tests passing!" |

### 1.5 Declarative Acts (3 terms) — Changing reality by speaking

| Term | Definition | Example Usage |
|------|------------|---------------|
| `create` | Bring into existence | "Create: New agent pod initialized" |
| `abolish` | End existence | "Abolish: Deprecated endpoint removed" |
| `transform` | Change state | "Transform: Converting to new format" |

### 1.6 Specification Acts (3 terms) — Defining binding requirements

| Term | Definition | Example Usage |
|------|------------|---------------|
| `specify` | Define a binding constraint or intent | "Specify: User must authenticate before access" |
| `require` | State a non-negotiable condition | "Require: All outputs must be validated" |
| `constrain` | Limit the solution space | "Constrain: Response time < 200ms" |

### 1.7 Diagnostic Acts (6 terms) — Verifying and reproducing behavior

| Term | Definition | Example Usage |
|------|------------|---------------|
| `extract` | Pull specific structured data from a source | "Extract: Pull field values from the response" |
| `gap` | Identify an uncovered requirement or missing capability | "Gap: No test for the error path" |
| `reproduce` | Re-create a bug or behavior from a known procedure | "Reproduce: Run the failing input through the parser" |
| `substitute` | Replace a term or reference with an equivalent | "Substitute: Replace deprecated API call" |
| `validate` | Confirm an artifact meets defined criteria | "Validate: Check manifest against schema" |
| `write` | Produce or persist content | "Write: Persist the compiled output" |

### 1.8 Generative Acts (3 terms) — Creating stylistic replicas

| Term | Definition | Example Usage |
|------|------------|---------------|
| `replicate` | Generate prose in a stylistic replica of an author | "Replicate: Hemingway style synthesis" |
| `embed_corpus` | Convert an author's works into vector embeddings | "embed_corpus: Build Woolf embedding corpus" |
| `mashup` | Blend two authorial styles via centroid interpolation | "Mashup: Hemingway at 0.3, Woolf at 0.7" |

---

## Domain 2: FlowDef — Process Flow Language (42 terms)

**Theoretical Basis:** Workflow Patterns (Wil van der Aalst), Cascade Skill Manifests[^van-der-aalst]

**hKask FlowDef Categories:**

### 2.1 Control Flow (8 terms) — Ordering of activities

| Term | Definition | Example Usage |
|------|------------|---------------|
| `sequence` | Linear ordering | "sequence: [fetch, parse, store]" |
| `parallel` | Concurrent execution | "parallel: [validate, index]" |
| `choice` | Exclusive branching | "choice: if error then alert else proceed" |
| `iteration` | Repetition | "iteration: for each document" |
| `fork` | Split into branches | "fork: analyze in parallel" |
| `join` | Merge branches | "join: aggregate results" |
| `sync` | Synchronization point | "sync: wait for all" |
| `async` | Non-blocking | "async: dispatch without waiting" |

### 2.2 Data Flow (5 terms) — Movement of information

| Term | Definition | Example Usage |
|------|------------|---------------|
| `transform` | Convert data | "transform: JSON → triples" |
| `filter` | Select subset | "filter: confidence > 0.8" |
| `aggregate` | Combine results | "aggregate: swarm predictions" |
| `route` | Direct to destination | "route: to memory store" |
| `broadcast` | Send to multiple | "broadcast: all agent pods" |

### 2.3 Exception Handling (5 terms) — Error management

| Term | Definition | Example Usage |
|------|------------|---------------|
| `catch` | Handle error | "catch: timeout → retry" |
| `fallback` | Alternative path | "fallback: single-agent prediction" |
| `compensate` | Undo action | "compensate: rollback transaction" |
| `escalate` | Raise to higher level | "escalate: to System 5" |
| `abort` | Terminate | "abort: critical failure" |

### 2.4 Temporal (4 terms) — Time-based constraints

| Term | Definition | Example Usage |
|------|------------|---------------|
| `delay` | Wait duration | "delay: 100ms" |
| `timeout` | Maximum wait | "timeout: 30s" |
| `schedule` | Plan for time | "schedule: daily at 03:00" |
| `expire` | Become invalid | "expire: after 24h" |

### 2.5 Composition (2 terms) — Building complex flows

| Term | Definition | Example Usage |
|------|------------|---------------|
| `compose` | Combine flows | "compose: [search, extract, summarize]" |
| `decompose` | Break into parts | "decompose: complex question" |

### 2.6 Git Evolution (7 terms) — Artifact versioning and forking

| Term | Definition | Example Usage |
|------|------------|---------------|
| `clone` | Copy without modification intent | "clone: template for local use" |
| `branch` | Divergent development line | "branch: experimental-feature" |
| `merge` | Combine divergent versions | "merge: fork back into main" |
| `rebase` | Reapply changes on new base | "rebase: fork on updated template" |
| `pr` | Pull request to merge fork | "pr: submit improved template" |
| `upstream` | Original source of fork | "upstream: track canonical template" |
| `downstream` | Forks of this artifact | "downstream: 5 forks of this template" |

### 2.7 Curation Process (3 terms) — Composition and integration workflows

| Term | Definition | Example Usage |
|------|------------|---------------|
| `curate` | Select, contextualise, and integrate artifacts | "curate: Build coherent template collection" |
| `elicit` | Draw out latent goals or requirements | "elicit: Discover user's underlying intent" |
| `reconcile` | Resolve conflicts between goals or requirements | "reconcile: Balance speed vs accuracy tradeoffs" |

### 2.8 Skill Lifecycle (8 terms) — Artifact management and lifecycle operations

| Term | Definition | Example Usage |
|------|------------|---------------|
| `defer` | Postpone action pending future evidence | "defer: Skip non-critical fix until next release" |
| `deprecate` | Mark for future removal while remaining functional | "deprecate: Old template format still parsed" |
| `enforce` | Ensure a constraint or rule is obeyed | "enforce: Reject manifest with invalid template_type" |
| `install` | Place an artifact into its operational location | "install: Copy skill templates to registry" |
| `list` | Enumerate items in a collection | "list: Show all installed skills" |
| `prune` | Remove an artifact from the corpus | "prune: Delete stale skill from registry" |
| `retire` | Permanently remove a deprecated artifact | "retire: Remove deprecated template after migration" |
| `search` | Look for candidates across available sources | "search: Find skills matching the capability gap" |

---

## Domain 3: KnowAct — Cognition Language (76 terms)

**Theoretical Basis:** Enactive Cognition (Varela, Thompson),[^varela] Second-Order Cybernetics (von Foerster),[^von-foerster] Autopoiesis (Maturana)[^maturana]

**hKask KnowAct Categories:**

### 3.1 Recognition (6 terms) — Pattern identification

| Term | Definition | Example Usage |
|------|------------|---------------|
| `recognize` | Identify pattern | "recognize: forecastable question" |
| `classify` | Categorize | "classify: binary vs categorical" |
| `detect` | Notice anomaly | "detect: drift in predictions" |
| `match` | Find similarity | "match: prior cases" |
| `discriminate` | Distinguish | "discriminate: signal from noise" |
| `parse` | Analyze structure | "parse: question components" |

### 3.2 Reasoning (6 terms) — Logical operations

| Term | Definition | Example Usage |
|------|------------|---------------|
| `infer` | Draw conclusion | "infer: from premises" |
| `deduce` | Logical derivation | "deduce: necessary consequence" |
| `induce` | Generalize | "induce: from examples" |
| `abduct` | Best explanation | "abduct: most likely cause" |
| `analogy` | Map similarity | "analogy: like previous case" |
| `critique` | Evaluate reasoning | "critique: identify fallacies" |

### 3.3 Learning (5 terms) — Knowledge acquisition

| Term | Definition | Example Usage |
|------|------------|---------------|
| `acquire` | Gain knowledge | "acquire: from document" |
| `integrate` | Merge with existing | "integrate: into knowledge graph" |
| `crystallize` | Stabilize memory | "crystallize: frequent recall" |
| `adapt` | Adjust to context | "adapt: template parameters" |
| `calibrate` | Tune accuracy | "calibrate: confidence scores" |

### 3.4 Metacognition (6 terms) — Thinking about thinking

| Term | Definition | Example Usage |
|------|------------|---------------|
| `reflect` | Observe own process | "reflect: on decision" |
| `monitor` | Track performance | "monitor: pass rate" |
| `evaluate` | Assess quality | "evaluate: template effectiveness" |
| `regulate` | Adjust behavior | "regulate: reduce complexity" |
| `orient` | Direct attention | "orient: to salient features" |
| `ground` | Anchor in reality | "ground: in observed data" |

### 3.5 Curation Cognition (2 terms) — Nurturing coherent collections

| Term | Definition | Example Usage |
|------|------------|---------------|
| `contextualise` | Situate an artifact within its meaningful environment | "contextualise: Place template in workflow context" |
| `cultivate` | Nurture growth and coherence over time | "cultivate: Evolve template collection toward coherence" |

### 3.6 Diagnostic & Regulatory Cognition (12 terms) — Analysis, observation, and variety regulation

| Term | Definition | Example Usage |
|------|------------|---------------|
| `amplify` | Increase regulatory or response variety | "amplify: Add more handlers for edge cases" |
| `analyze` | Decompose into components for understanding | "analyze: Break down the error distribution" |
| `attenuate` | Reduce system or disturbance variety | "attenuate: Filter noise from the signal" |
| `compress` | Distill and reduce context volume | "compress: Summarize session into handoff" |
| `instrument` | Add targeted observation points to a code path | "instrument: Add diagnostic log at branch point" |
| `isolate` | Separate a concern or variable from its context | "isolate: Reproduce with single variable changed" |
| `observe` | Watch and record system behavior without intervention | "observe: Monitor CNS span before hypothesising" |
| `predict` | Forecast an outcome from current evidence | "predict: Hypothesis states a predicted outcome" |
| `resolve` | Determine a winner or course from competing alternatives | "resolve: Pick highest-ranked hypothesis for testing" |
| `score` | Assign a numeric assessment to an artifact | "score: Rate skill health on 0-1 scale" |
| `synthesize` | Combine disparate elements into a coherent whole | "synthesize: Merge findings into root-cause narrative" |
| `trace` | Follow a causal or provenance chain | "trace: Follow spec requirement back to test invariant" |
| `compare` | Measure stylistic distance between author centroids | "compare: Calculate cosine distance between Hemingway and Woolf" |
| `blend` | Interpolate between style vectors for mashup generation | "blend: Mix centroids at 0.7 Woolf / 0.3 Hemingway" |

### 3.7 Skill Management Cognition (8 terms) — Design, planning, and format translation

| Term | Definition | Example Usage |
|------|------------|---------------|
| `deepen` | Extract a smaller interface from a shallow module | "deepen: Collapse shallow module into deep seam" |
| `design` | Define structure and interfaces of a component | "design: Specify the deepened module shape" |
| `explore` | Search systematically for patterns or friction | "explore: Walk the codebase for coupling hotspots" |
| `fix` | Apply a corrective change to resolve a defect | "fix: Patch the root cause, not the symptom" |
| `map` | Produce a structured representation of component relationships | "map: Generate caller graph for the crate" |
| `plan` | Define ordered steps toward a goal before execution | "plan: Extract requirements and prioritize by risk" |
| `rank` | Order items by a comparative criterion | "rank: Sort hypotheses by falsifiability" |
| `translate` | Convert from one representation or format to another | "translate: Adapt skill into dual-layer architecture" |

### 3.8 Session Continuity Cognition (6 terms) — Compaction, cataloging, and redaction

| Term | Definition | Example Usage |
|------|------------|---------------|
| `compact` | Distill and compress context into essential facts | "compact: Reduce session to handoff-ready summary" |
| `distill` | Extract the essential from the voluminous | "distill: Pull key decisions from long conversation" |
| `summarize` | Produce a concise representation of content | "summarize: Condense spec into one-paragraph abstract" |
| `catalog` | Enumerate and classify artifacts by reference | "catalog: List all files changed in this session" |
| `reference` | Point to an artifact by path or URL without duplication | "reference: Link to spec doc instead of inlining" |
| `redact` | Remove or mask sensitive data before transfer | "redact: Strip API keys from handoff document" |

### 3.9 Advisory Cognition (3 terms) — Suggestion, prioritization, and recommendation

| Term | Definition | Example Usage |
|------|------------|---------------|
| `suggest` | Recommend a skill or action based on context | "suggest: Offer TDD skill for new feature" |
| `prioritize` | Rank by importance or urgency | "prioritize: Security fixes before ergonomics" |
| `recommend` | Advise a specific course of action | "recommend: Deepen this module before adding tests" |

### 3.10 Document Composition Cognition (4 terms) — Assembly, structuring, and persistence

| Term | Definition | Example Usage |
|------|------------|---------------|
| `compose` | Assemble parts into a structured whole | "compose: Build handoff document from sections" |
| `structure` | Organize into a defined arrangement | "structure: Arrange findings by MDS category" |
| `document` | Create a persistent, transferable record | "document: Write the architecture decision record" |
| `handoff` | Determine what context to transfer to the next agent | "handoff: Select session facts for continuity" |

### 3.11 Coding Assessment Cognition (5 terms) — Evaluation, verification, and enforcement

| Term | Definition | Example Usage |
|------|------------|---------------|
| `assess` | Evaluate a task against principles before action | "assess: Check coding task for over-engineering risk" |
| `simplify` | Reduce to the minimum code that solves the problem | "simplify: Remove speculative abstraction layer" |
| `verify` | Confirm compliance with defined criteria | "verify: Check diff against surgical-changes rule" |
| `audit` | Systematic examination for violations and compliance | "audit: Scan manifest for invalid template_type" |
| `apply` | Enforce constraints and guardrails on implementation | "apply: Generate constrained implementation directives" |

### 3.12 Least Action & Pragmatic Cognition (8 terms) — Physical grounding and communication efficiency

**Theoretical Basis:** Least Action Principle (Hamilton, Coopersmith),[^coopersmith] Pragmatics (Grice, Sperber & Wilson)[^grice]

| Term | Definition | Example Usage |
|------|------------|---------------|
| `minimize` | Reduce to the lowest possible state or value | "minimize: Compress to stationary action representation" |
| `equilibrium` | State of balance where opposing forces or tendencies cancel | "equilibrium: System at homeostatic set-point" |
| `homeostasis` | Self-regulating stability maintained through feedback mechanisms | "homeostasis: CNS maintains variety within threshold" |
| `converge` | Approach a common point, state, or value through iterative refinement | "converge: Evolutionary architecture settles toward deep modules" |
| `stationary_action` | Path where small variations produce no first-order change in the action integral | "stationary_action: Module at optimal depth — deletion increases energy" |
| `variational_principle` | Principle that physical systems evolve along paths extremizing an integral quantity | "variational_principle: δS = 0 selects which path reality takes" |
| `gradient_descent` | Iterative optimization following the direction of steepest decrease | "gradient_descent: System evolves toward lower-action configurations" |
| `energy_landscape` | Configuration space representation where energy or cost is encoded as elevation | "energy_landscape: Map module depth as elevation in architecture space" |

---

## Cross-Domain Composition Patterns

### Pattern 1: Query → Infer → Reflect

```yaml
name: "question-answer-reflect"
cascade:
  pre:
    - skill: decompose-question
  core:
    - template:
        type: WordAct
        wordact: [query, contextualize]
    - template:
        type: WordAct
        wordact: [infer, assert]
  post:
    - template:
        type: KnowAct
        knowact: [reflect, evaluate]
```

### Pattern 2: Recognize → Sequence → Calibrate

```yaml
name: "forecast-execution"
cascade:
  pre:
    - template:
        type: KnowAct
        knowact: [recognize, classify]
  core:
    - template:
        type: FlowDef
        flowdef: [sequence, aggregate]
  post:
    - template:
        type: KnowAct
        knowact: [calibrate, monitor]
```

### Pattern 3: Detect → Escalate → Regulate

```yaml
name: "drift-response"
cascade:
  pre:
    - template:
        type: KnowAct
        knowact: [detect, discriminate]
  core:
    - template:
        type: FlowDef
        flowdef: [escalate, broadcast]
  post:
    - template:
        type: KnowAct
        knowact: [regulate, adapt]
```

---

## hLexicon Grammar

### Template Declaration

```yaml
[inference]
template_type: WordAct | KnowAct | FlowDef
lexicon_terms: [term1, term2, ...]  # Must be from hLexicon
contract:
  input: {field: type, ...}
  output: {field: type, ...}
```

### Valid Combinations

| Template Type | Valid Lexicon Domains |
|---------------|----------------------|
| WordAct | WordAct (required), FlowDef (optional) |
| FlowDef | FlowDef (required), WordAct (optional) |
| KnowAct | KnowAct (required), WordAct (optional) |

### Validation Rules

Terminology validation follows ISO principles for vocabulary management.[^iso704] Domain-driven vocabulary design ensures that each template type maps to a primary lexicon domain.[^evans-ddd]

1. **Minimum Terms:** Each template must declare ≥1 lexicon term
2. **Maximum Terms:** Each template declares ≤10 lexicon terms (prevent over-specification)
3. **Domain Match:** Template type must match primary lexicon domain
4. **Cross-Reference:** Terms can reference terms from other domains (e.g., Prompt can use `sequence` from FlowDef)

---

## Academic Grounding References

### WordAct — Speech Act Theory

| Theorist | Work | Key Contribution |
|----------|------|------------------|
| J.L. Austin | "How to Do Things with Words" (1962) | Locution/illocution/perlocution distinction |
| John Searle | "Speech Acts" (1969) | Five categories: assertive, directive, commissive, expressive, declarative |
| Jürgen Habermas | "Theory of Communicative Action" (1981) | Validity claims in communication |

### FlowDef — Workflow Patterns

| Theorist | Work | Key Contribution |
|----------|------|------------------|
| Wil van der Aalst | "Workflow Patterns" (2003) | 43 workflow patterns for process modeling |
| BPMN 2.0 | OMG Standard | Business process modeling notation |
| Stafford Beer | "Brain of the Firm" (1972) | VSM coordination (System 2) patterns |

### KnowAct — Enactive Cognition

| Theorist | Work | Key Contribution |
|----------|------|------------------|
| Humberto Maturana | "Autopoiesis and Cognition" (1980) | Cognition as effective action |
| Francisco Varela | "The Tree of Knowledge" (1987) | Enaction: bringing forth a world |
| Evan Thompson | "Mind in Life" (2007) | Autonomy, sense-making, experience |
| Heinz von Foerster | "Cybernetics of Cybernetics" (1974) | Second-order observation |

---

## hLexicon Term Index (155 terms)

### WordAct (37)
`query`, `request`, `instruct`, `command`, `prompt`, `probe`, `challenge`, `summon`, `pledge`, `propose`, `promise`, `undertake`, `commit`, `assert`, `claim`, `report`, `declare`, `affirm`, `testify`, `acknowledge`, `apologize`, `celebrate`, `create`, `abolish`, `transform`, `specify`, `require`, `constrain`, `extract`, `gap`, `reproduce`, `substitute`, `validate`, `write`, `replicate`, `embed_corpus`, `mashup`

### FlowDef (42)
`sequence`, `parallel`, `choice`, `iteration`, `fork`, `join`, `sync`, `async`, `transform`, `filter`, `aggregate`, `route`, `broadcast`, `catch`, `fallback`, `compensate`, `escalate`, `abort`, `delay`, `timeout`, `schedule`, `expire`, `compose`, `decompose`, `clone`, `branch`, `merge`, `rebase`, `pr`, `upstream`, `downstream`, `curate`, `elicit`, `reconcile`, `defer`, `deprecate`, `enforce`, `install`, `list`, `prune`, `retire`, `search`

### KnowAct (76)
`recognize`, `classify`, `detect`, `match`, `discriminate`, `parse`, `infer`, `deduce`, `induce`, `abduct`, `analogy`, `critique`, `acquire`, `integrate`, `crystallize`, `adapt`, `calibrate`, `reflect`, `monitor`, `evaluate`, `regulate`, `orient`, `ground`, `contextualise`, `cultivate`, `amplify`, `analyze`, `attenuate`, `compress`, `instrument`, `isolate`, `observe`, `predict`, `resolve`, `score`, `synthesize`, `trace`, `compare`, `blend`, `deepen`, `design`, `explore`, `fix`, `map`, `plan`, `rank`, `translate`, `compact`, `distill`, `summarize`, `catalog`, `reference`, `redact`, `suggest`, `prioritize`, `recommend`, `compose`, `structure`, `document`, `handoff`, `assess`, `simplify`, `verify`, `audit`, `apply`, `minimize`, `equilibrium`, `homeostasis`, `converge`, `stationary_action`, `variational_principle`, `gradient_descent`, `energy_landscape`

**Total: 155 term-slots** (37 WordAct + 42 FlowDef + 76 KnowAct; `transform` and `compose` appear in both WordAct/FlowDef and KnowAct, so 153 globally-unique strings).

---

## Expansion Capacity

The hLexicon has 3 reserved slots for future domain extension. These are **not planned additions** — they represent headroom in the allocation table should new domains emerge through actual usage:
- Domain 4: Reserved (currently unallocated)
- Cross-domain: Reserved (emergent patterns from composition)
- Reserved: Unallocated (unknown unknowns)

---

## Usage Examples

### Example 1: Simple Query Template

```jinja2
[inference]
template_type: WordAct
lexicon_terms: [query, assert]
contract:
  input: {question: string}
  output: {answer: string, confidence: float}

---
{{ query(question) }}

Provide a direct answer. {{ assert(confidence) }}
```

### Example 2: Cascade Skill

```yaml
name: "research-and-summarize"
version: "1.0.0"
lexicon_terms: [sequence, parallel, aggregate, report]

cascade:
  core:
    - template:
        type: FlowDef
        flowdef: [parallel]
        skills: [web-search]
    - template:
        type: FlowDef
        flowdef: [aggregate]
    - template:
        type: WordAct
        wordact: [report]
```

### Example 3: Metacognitive Reflection

```jinja2
[inference]
template_type: KnowAct
lexicon_terms: [reflect, evaluate, regulate]
contract:
  input: {template_name: string, outcomes: array}
  output: {pass_rate: float, recommendations: array}

---
{{ reflect(on=template_name) }}

Analyze the outcomes: {{ evaluate(outcomes) }}

Recommend adjustments: {{ regulate(pass_rate) }}
```

---

## References

[^austin]: Austin, J. L. (1962). *How to Do Things with Words*. Harvard University Press. Speech act theory foundation.
[^searle]: Searle, J. R. (1969). *Speech Acts: An Essay in the Philosophy of Language*. Cambridge University Press.
[^van-der-aalst]: van der Aalst, W. M. P., ter Hofstede, A. H. M., & Weske, M. (2003). Workflow Patterns. In *Business Process Management* (pp. 1-20). Springer.
[^varela]: Varela, F. J., Thompson, E., & Rosch, E. (1991). *The Embodied Mind: Cognitive Science and Human Experience*. MIT Press. Enactive cognition.
[^russell]: Russell, S., & Norvig, P. (2020). *Artificial Intelligence: A Modern Approach* (4th ed.). Pearson. AI agent design patterns.
[^von-foerster]: von Foerster, H. (2003). *Understanding Understanding: Essays on Cybernetics and Cognition*. Springer. https://doi.org/10.1007/978-1-4419-8972-3
[^maturana]: Maturana, H. R., & Varela, F. J. (1980). *Autopoiesis and Cognition: The Realization of the Living*. D. Reidel.
[^iso704]: International Organization for Standardization. (2022). *ISO 704:2022 — Terminology work: Principles and methods*. ISO. https://www.iso.org/standard/79887.html
[^evans-ddd]: Evans, E. (2003). *Domain-Driven Design: Tackling Complexity in the Heart of Software*. Addison-Wesley. Ubiquitous language and bounded contexts.
[^coopersmith]: Coopersmith, J. (2017). *The Lazy Universe: An Introduction to the Principle of Least Action*. Oxford University Press. Least action as the selection mechanism governing physical systems.
[^grice]: Grice, H. P. (1975). "Logic and Conversation." In *Syntax and Semantics 3: Speech Acts*. Academic Press. Pragmatic implicature — communication seeks the path of least effort.

---

*hLexicon v1.3 — 155 term-slots defined across 3 domains (37 WordAct + 42 FlowDef + 76 KnowAct), 1 unified composition substrate*
