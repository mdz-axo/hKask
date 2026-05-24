---
title: "hKask hLexicon — Minimal Composition Vocabulary"
audience: [architects, developers, agents]
last_updated: 2026-05-24
togaf_phase: "C — Application"
version: "0.21.0"
status: "Active"
domain: "Application"
---

# hKask hLexicon — Minimal Composition Vocabulary

## Executive Summary

**hLexicon** is the minimal vocabulary (75 terms allocated across 3 domains) for composing templates in the hKask system. It covers three domains:

1. **WordAct** — Language for prompting/LLM interactions (speech act theory)[^austin]
2. **FlowDef** — Language for process/skill composition (workflow patterns)[^van-der-aalst]
3. **KnowAct** — Language for cognition and metacognition (enactive cognition)[^varela]

**Design Principles:**
- **Minimal:** 75 terms allocated across 3 domains (currently 80)
- **Composable:** Terms combine to express complex patterns
- **Academic Grounding:** Based on established theory (Austin, Searle, van der Aalst, Varela)
- **LLM-Optimized:** Terms selected for LLM comprehension and consistent interpretation
- **Domain-Separated:** Each domain has its own namespace, but terms can cross-reference

**Usage:** Templates declare which hLexicon terms they use in their `[inference]` header:

```jinja2
[inference]
template_type: Prompt
lexicon_terms: [query, assert, contextualize, critique]
contract:
  input: {question: string, context: array}
  output: {answer: string, confidence: float}
```

---

## Domain 1: WordAct — Prompting Language (28 terms)

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

---

## Domain 2: FlowDef — Process Flow Language (27 terms)

**Theoretical Basis:** Workflow Patterns (Wil van der Aalst), Cascade Skill Manifests

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

### 2.6 Git Evolution (8 terms) — Artifact versioning and forking

| Term | Definition | Example Usage |
|------|------------|---------------|
| `fork` | Create divergent copy for modification | "fork: template for customization" |
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

---

## Domain 3: KnowAct — Cognition Language (25 terms)

**Theoretical Basis:** Enactive Cognition (Varela, Thompson), Second-Order Cybernetics (von Foerster), Autopoiesis (Maturana)

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
        type: Prompt
        wordact: [query, contextualize]
    - template:
        type: Prompt
        wordact: [infer, assert]
  post:
    - template:
        type: Cognition
        knowact: [reflect, evaluate]
```

### Pattern 2: Recognize → Sequence → Calibrate

```yaml
name: "forecast-execution"
cascade:
  pre:
    - template:
        type: Cognition
        knowact: [recognize, classify]
  core:
    - template:
        type: Process
        flowdef: [sequence, aggregate]
  post:
    - template:
        type: Cognition
        knowact: [calibrate, monitor]
```

### Pattern 3: Detect → Escalate → Regulate

```yaml
name: "drift-response"
cascade:
  pre:
    - template:
        type: Cognition
        knowact: [detect, discriminate]
  core:
    - template:
        type: Process
        flowdef: [escalate, broadcast]
  post:
    - template:
        type: Cognition
        knowact: [regulate, adapt]
```

---

## hLexicon Grammar

### Template Declaration

```yaml
[inference]
template_type: Prompt | Process | Cognition
lexicon_terms: [term1, term2, ...]  # Must be from hLexicon
contract:
  input: {field: type, ...}
  output: {field: type, ...}
```

### Valid Combinations

| Template Type | Valid Lexicon Domains |
|---------------|----------------------|
| Prompt | WordAct (required), FlowDef (optional) |
| Process | FlowDef (required), WordAct (optional) |
| Cognition | KnowAct (required), WordAct (optional) |

### Validation Rules

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

## hLexicon Term Index (88 terms)

### WordAct (28)
`query`, `request`, `instruct`, `command`, `prompt`, `probe`, `challenge`, `summon`, `pledge`, `propose`, `promise`, `undertake`, `commit`, `assert`, `claim`, `report`, `declare`, `affirm`, `testify`, `acknowledge`, `apologize`, `celebrate`, `create`, `abolish`, `transform`, `specify`, `require`, `constrain`

### FlowDef (27)
`sequence`, `parallel`, `choice`, `iteration`, `fork`, `join`, `sync`, `async`, `transform`, `filter`, `aggregate`, `route`, `broadcast`, `catch`, `fallback`, `compensate`, `escalate`, `abort`, `delay`, `timeout`, `schedule`, `expire`, `compose`, `decompose`, `fork`, `clone`, `branch`, `merge`, `rebase`, `pr`, `upstream`, `downstream`, `curate`, `elicit`, `reconcile`

### KnowAct (25)
`recognize`, `classify`, `detect`, `match`, `discriminate`, `parse`, `infer`, `deduce`, `induce`, `abduct`, `analogy`, `critique`, `acquire`, `integrate`, `crystallize`, `adapt`, `calibrate`, `reflect`, `monitor`, `evaluate`, `regulate`, `orient`, `ground`, `contextualise`, `cultivate`

**Total: 88 terms** (13 over 75 allocation — git evolution terms and spec-curation terms are essential for artifact management and participatory goal elicitation)

---

## Future Expansion

Reserved slots (3 remaining):
- Domain 4: **???** (TBD — possibly social/collective cognition)
- Cross-domain: **???** (TBD — emergent patterns)
- Reserved: **???** (TBD — unknown unknowns)

---

## Usage Examples

### Example 1: Simple Query Template

```jinja2
[inference]
template_type: Prompt
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
        type: Process
        flowdef: [parallel]
        skills: [web-search, scholar-search]
    - template:
        type: Process
        flowdef: [aggregate]
    - template:
        type: Prompt
        wordact: [report]
```

### Example 3: Metacognitive Reflection

```jinja2
[inference]
template_type: Cognition
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

---

*hLexicon v1.1 — 88 terms allocated across 3 domains, 1 unified composition substrate*
