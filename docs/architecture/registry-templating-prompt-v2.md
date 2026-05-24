---
title: "Registry & Templating System — Composite Design Prompt v2"
audience: [architects, developers, agents]
last_updated: 2026-05-24
togaf_phase: "C — Application"
version: "2.0.0"
status: "Active"
domain: "Application"
---

# hKask Registry & Templating System — Composite Design Prompt v2

## Contents

| Section | Description |
|---------|-------------|
| [Architectural Invariant: Code vs. Content Separation](#architectural-invariant-code-vs-content-separation) | Rust is the loom; YAML/Jinja2 is the thread |
| [Part 1: Semantic Root Cause Analysis](#part-1-semantic-root-cause-analysis) | Conceptual distinctions and RDF triples |
| [Part 2: Entity Relationship Model](#part-2-entity-relationship-model) | Core entities and their relationships |
| [Part 3: Hexagonal Architecture Mapping](#part-3-hexagonal-architecture-mapping) | Ports, adapters, and boundary mapping |
| [Part 4: Core Manifest Specification](#part-4-core-manifest-specification) | Manifest schema, steps, and cascade rules |
| [Part 5: Rust Execution Loop](#part-5-rust-execution-loop-minimal-hard-logic) | Minimal hard logic in Rust for manifest execution |
| [Part 6: Implementation Tasks](#part-6-implementation-tasks) | Ordered implementation task list |
| [Part 7: Future — Open Questions](#part-7-future--open-questions) | Unresolved design questions |
| [Completion Criterion](#completion-criterion) | Definition of done for the design prompt |
| [References](#references) | Citations and references |

## Architectural Invariant: Code vs. Content Separation

**Root Principle:** Rust is the loom. YAML/Jinja2 is the thread. The loom doesn't change when you weave a different pattern.[^evans-ddd]

```
| Layer          | Technology      | Mutability           |
|----------------|-----------------|----------------------|
| Hard (Kernel)  | Rust            | Fixed, stable        |
| Soft (Material)| YAML, Jinja2, MD| Mutable, evolving    |
| Testing        | Rust (tests)    | Verification edge    |
```

**Rust owns:** Parsing YAML steps, rendering Jinja2[^jinja2] via minijinja,[^minijinja] enforcing matroshka depth, validating hLexicon terms, routing MCP/LLM calls.

**Rust does NOT own:** Which templates exist, what they say, how selection logic is phrased, what steps a manifest contains, what a prompt looks like.

---

## Part 1: Semantic Root Cause Analysis

### 1.1 Conceptual Distinctions (RDF Triples)

```turtle
# Core Entities
:RustKernel    a :FixedMachine ;
               :property :HardLogic ;
               :budget "≤30k LOC" ;
               :changes :Rarely .

:YAMLFile      a :MutableProgram ;
               :interpretedBy :RustKernel ;
               :budget :Unbounded ;
               :changes :Frequently .

:Jinja2File    a :MutableProgram ;
               :renderedBy :RustKernel ;
               :budget :Unbounded ;
               :changes :Frequently .

# Domain Entities
:Manifest      a :YAMLFile ;
               :defines :StepSequence ;
               :varies false "per invocation" ;
               :varies true "over time via editing" .

:Template      a :Jinja2File ;
               :composesInto :RenderedDocument ;
               :varies true "per invocation AND over time" .

:Registry      :dispatchedBy :Manifest ;
               :contains :Template ;
               :indexedBy :RustKernel .

# Relationships
:Bot           :executes :Manifest ;
               :discovers :Template .

:hLexicon      :grounds :Template ;
               :vocabulary [:WordAct | :FlowDef | :KnowAct] .

:CNS           :observes :TemplateOutcome ;
               :namespace "cns.prompt.*" .
```

### 1.2 Driver Analysis

| Entity | Problem Solved | What Breaks If Removed |
|--------|----------------|------------------------|
| **Manifest** | Executable process definition without recompilation | All logic hard-coded in Rust; no evolution without redeploy |
| **Template** | Dynamic document composition from variable input | Static prompts; no contextual adaptation |
| **Registry** | Context-aware template discovery | Manual template selection; no intelligent routing |
| **hLexicon** | Consistent vocabulary for LLM interpretation | Term drift; unpredictable template behavior |
| **CNS** | Cybernetic feedback on template outcomes | No learning; no drift detection |

### 1.3 The Three-Way Split: Kernel or Template Concern?

**Question:** Should Prompts (WordAct/say), Processes (FlowDef/do), and Cognition (KnowAct/think) be three registries or one?

**Analysis via hKask Constraints:**
- **P1 (No trait without two consumers):** Does each registry have distinct consumers?
- **C4 (Repetition is missing primitive):** Is three registries repetition or genuine distinction?
- **P4 (No builder without complexity):** Does split justify added complexity?

**Decision:** **One unified registry** with `template_type` discriminator.[^fowler-poeaa] The three-way distinction is preserved as metadata tags on entries, not separate Rust data structures or code paths. Selection intelligence lives in Jinja2/LLM, not Rust branching.[^lewis-rag]

---

## Part 2: Entity Relationship Model

The registry provides context-aware template discovery through type-discriminated indexing.[^fowler-poeaa]

### 2.1 Mermaid ERD

```mermaid
erDiagram
    BOT ||--o{ MANIFEST : executes
    MANIFEST ||--|{ STEP : defines
    MANIFEST ||--o{ TEMPLATE : discovers
    TEMPLATE ||--o{ hLEXICON : grounded_by
    TEMPLATE ||--|{ FIELD : declares
    REGISTRY ||--o{ TEMPLATE : contains
    CNS ||--o{ OUTCOME : observes
    
    BOT {
        string name
        string type "Bot|Replicant"
        string manifest_ref
        array capabilities
    }
    
    MANIFEST {
        string id
        array steps
        string template_ref
        string model_tier
    }
    
    STEP {
        int ordinal
        string action "select|populate|execute"
        string template_ref
        object output_schema
    }
    
    TEMPLATE {
        string id
        string template_type "Prompt|Process|Cognition"
        array lexicon_terms
        object contract
        string source_path
    }
    
    hLEXICON {
        string term
        string domain "WordAct|FlowDef|KnowAct"
        string definition
    }
    
    REGISTRY {
        string root_path
        string index_method "filesystem|sqlite"
    }
    
    CNS {
        string span "cns.prompt.*|cns.tool.*"
        object outcome
        float confidence
    }
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-RTL-001
verified_date: 2026-05-24
verified_against: crates/hkask-templates/src/cascade.rs; crates/hkask-templates/src/engine.rs
status: VERIFIED
-->

### 2.2 Data Flow

```
input → [Registry Lookup] → [Template Discovery] → [Jinja2 Render] → [LLM/Tool Call] → output
                              ↓
                        [hLexicon Validation]
                              ↓
                        [CNS ν-event]
```

---

## Part 3: Hexagonal Architecture Mapping

The system follows the hexagonal (ports and adapters) architecture pattern, isolating the domain core from infrastructure concerns.[^cockburn-hex]

### 3.1 Architecture Diagram

```mermaid
graph TB
    subgraph "Inbound Adapters (Driven)"
        CLI[CLI Commands]
        API[HTTP API]
        ACP[ACP Agent Calls]
    end
    
    subgraph "Core Domain (Rust — hkask-templates)"
        REG[RegistryPort]
        MAN[ManifestExecutor]
        TPL[TemplateRenderer]
        VAL[hLexicon Validator]
    end
    
    subgraph "Outbound Adapters (Driving)"
        STO[Storage Adapter]
        INF[Inference Adapter]
        MCP[MCP Adapter]
        CNS[CNS Adapter]
    end
    
    subgraph "Soft Layer (YAML/Jinja2 — Mutable)"
        YAML[Manifest Files]
        J2[Template Files]
        MD[Cascade Files]
    end
    
    CLI --> REG
    API --> REG
    ACP --> REG
    
    REG --> STO
    MAN --> INF
    MAN --> MCP
    TPL --> CNS
    
    STO -. loads .- YAML
    STO -. loads .- J2
    STO -. loads .- MD
    
    YAML -. interpreted by .- MAN
    J2 -. rendered by .- TPL
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-RTL-002
verified_date: 2026-05-24
verified_against: crates/hkask-templates/src/cascade.rs; crates/hkask-templates/src/engine.rs
status: VERIFIED
-->

### 3.2 Port Definitions

**Inbound Ports (Driven):**
- `kask template list [--type Prompt|Process|Cognition]`
- `kask template render --id <id> --input <json>`
- `kask bot manifest pull/push <bot-name>`
- `GET /api/v1/templates`, `POST /api/v1/templates/:id/render`
- ACP: `template:dispatch` message

**Outbound Ports (Driving):**
- `StoragePort`: `load_template(id)`, `save_template(template)`, `list_templates(hint)`
- `InferencePort`: `call(model_tier, prompt) → result`
- `McpPort`: `discover_tools()`, `invoke_tool(name, input)`
- `CnsPort`: `emit_event(span, outcome, confidence)`

**Core Domain (Rust — ≤5,000 LOC for `hkask-templates`):**
- Registry lookup API (flat index with type discriminator)
- Template resolution interface (load, validate, render)
- Manifest execution loop (generic step interpreter)
- hLexicon validation API (term existence check)

**Soft Layer (YAML/Jinja2):**
- Template definitions (`.j2` files with `[inference]` header)
- Manifest workflows (`.yaml` files with `steps[]`)
- Cascade compositions (`.yaml` with `pre/core/post` stages)

---

## Part 4: Core Manifest Specification

Manifests define executable workflow steps following established process patterns.[^vander-aalst-wf]

### 4.1 Dispatch Manifest (YAML)

```yaml
# File: registry/manifests/dispatch.yaml
manifest:
  name: registry-dispatch
  description: >
    Bootstrap process for all registry resolution.
    Selects, populates, and executes a template from the unified registry.

steps:
  - ordinal: 1
    action: select
    description: "Render selector template with raw input + registry index; call fast model to choose best-fit template"
    renderer: minijinja
    template_ref: registry/templates/selector.j2
    model_tier: fast_local
    mcp: hkask-mcp-inference
    output_schema:
      selected_template_id: string
      rationale: string
      confidence: float

  - ordinal: 2
    action: populate
    description: "Bind raw prompt into selected template's Jinja2 fields"
    renderer: minijinja
    template_ref: "{{ selected_template_id }}"
    output_schema:
      rendered_document: string

  - ordinal: 3
    action: execute
    description: "Submit rendered document to model/tool per template contract"
    target: from_template_contract
    mcp: from_template_contract
    output_schema:
      result: any
```

### 4.2 Selector Template (Jinja2)

```jinja2
# File: registry/templates/selector.j2
[inference]
template_type: Cognition
lexicon_terms: [recognize, classify, match, discriminate]
contract:
  input: {raw_prompt: string, registry_index: array, domain_hint: string|null}
  output: {selected_template_id: string, rationale: string, confidence: float}

---
Given the following input:
{{ raw_prompt }}

Available templates in registry:
{% for entry in registry_index %}
- id: {{ entry.id }}
  type: {{ entry.template_type }}
  lexicon: {{ entry.lexicon_terms | join(", ") }}
  description: {{ entry.description }}
{% endfor %}

{% if domain_hint %}
Context indicates domain: {{ domain_hint }}
{% endif %}

Select the single best-fit template.
Respond with:
1. selected_template_id — the id of the best match
2. rationale — why this template fits (discriminate between top candidates)
3. confidence — score 0.0 to 1.0
```

### 4.3 Bot Manifest (YAML)

```yaml
# File: registry/bots/registry-dispatch-bot.yaml
bot:
  name: registry-dispatch-bot
  type: Bot
  binding_contract: true
  editor: curator-or-human-admin

capabilities:
  - tool:inference:call
  - tool:template:render
  - tool:registry:index

responsibilities:
  - respond_to: template_dispatch_requests
  - emit: cns.prompt.select
  - emit: cns.prompt.render
  - emit: cns.prompt.outcome

process_manifest: registry/manifests/dispatch.yaml
```

---

## Part 5: Rust Execution Loop (Minimal Hard Logic)

The executor interprets manifest steps through a fixed dispatch loop, applying the Interpreter pattern where the step grammar is stable and step content is variable.[^gamma-interpreter]

```rust
/// Core loop — fixed logic, applies to ANY manifest
fn execute_manifest(
    manifest: &ProcessManifest,
    input: Value,
    registry: &dyn RegistryIndex,
    renderer: &dyn TemplateRenderer,
    inference: &dyn InferencePort,
    depth: u8,
) -> Result<Value> {
    if depth > MAX_MATROSHKA_DEPTH {
        return Err(Error::RecursionLimit);
    }

    let mut state = input;
    for step in &manifest.steps {
        state = match step.action {
            Action::Select => {
                let template = renderer.load(&step.template_ref)?;
                let prompt = renderer.render(&template, state.clone())?;
                inference.call(&step.model_tier, &prompt)?
            }
            Action::Populate => {
                let template_id = state.get("selected_template_id")?;
                let template = registry.get(&template_id)?;
                renderer.render(&template.template, state.clone())?
                    .into()
            }
            Action::Execute => {
                let target = resolve_target(&step, &state)?;
                target.invoke(state, depth + 1)?
            }
        };
        emit_cns_event(&step, &state);
    }
    Ok(state)
}
```

**~50 lines of logic.** Never changes when templates are added, edited, or removed. Only changes if the *grammar of steps themselves* changes.

---

## Part 6: Implementation Tasks

Port traits and adapters are defined per hKask constraint principles.[^hKask-AGENTS]

### Task 1: Define Port Traits (`hkask-templates/src/ports.rs`)

```rust
trait ManifestExecutor {
    fn load(&self, path: &Path) -> Result<ProcessManifest>;
    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value>;
}

trait TemplateRenderer {
    fn load(&self, path: &Path) -> Result<CompositionTemplate>;
    fn render(&self, template: &CompositionTemplate, bindings: Value) -> Result<String>;
}

trait RegistryIndex {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry>;
    fn get(&self, id: &TemplateId) -> Option<RegistryEntry>;
    fn bootstrap_manifest(&self) -> &ProcessManifest;
}
```

### Task 2: Implement Flat Registry Adapter (`hkask-templates/src/registry.rs`)

Single adapter scanning filesystem or SQLite, filtering by `template_type` if hint provided.

### Task 3: Port from Clones/kask

- `stack-prompts/src/prompt_registry.rs` → `hkask-templates/src/registry.rs`
- `stack-prompts/src/contracts.rs` → `hkask-templates/src/contracts.rs`
- `stack-prompts/src/outcomes.rs` → `hkask-cns/src/outcomes.rs`

**Simplify:** Remove OKH → CNS spans, remove UCAN → OCAP-only, remove feedback crate.

### Task 4: Author Soft Layer Files

- `registry/manifests/dispatch.yaml`
- `registry/templates/selector.j2`
- `registry/bots/registry-dispatch-bot.yaml`

### Task 5: Wire CNS Integration

Emit `cns.prompt.select`, `cns.prompt.render`, `cns.prompt.outcome` spans at each step.

---

## Part 7: Future — Open Questions

Open questions are tracked against the architecture master specification.[^hKask-master]

1. **Enrichment Port:** What injects `domain_hint` and context before dispatch? Pre-step in manifest, or caller responsibility?

2. **Bootstrap Loading Order:** Are dispatch manifest and selector template loaded by convention from fixed paths, or is there a Rust bootstrap sequence?

3. **Selector Failure:** If fast model returns confidence below threshold, does manifest have conditional step (`choice` in FlowDef), or is fallback handled outside manifest by Rust executor?

4. **Template Hot-Reload:** When YAML/Jinja2 changes on disk, does Rust detect (fswatch), or must it be signaled (API/CLI)? Affects caching strategy.

5. **Manifest Step Grammar Extensibility:** Current actions: `select|populate|execute`. If new actions needed (`validate`, `transform`, `branch`), is this Rust code change or pure YAML? Define extension point.

6. **Git Versioning Interaction:** Does `RegistryIndex::get` resolve by filename (HEAD), or can callers request specific SHA? If latter, port needs `revision` parameter.

7. **Matroshka Limits:** Default recursion depth? Should CNS track depth and emit algedonic alerts on near-limit calls?

8. **hLexicon Validation:** Load-time or render-time? Failure mode if unknown term referenced?

9. **Cross-Registry Composition:** Can Process template invoke Prompt template? Can Cognition invoke Process? What are composition rules?

10. **Bot Manifest vs Template Manifest:** Same thing, or does bot's charter differ from template's process definition?

**Resolution Path:** Implement Tasks 1-5 with sensible defaults. Revisit these questions when operational data informs the decision.

---

## Completion Criterion

The registry/templating system is complete when:

1. **Dispatch manifest** loads and executes without Rust changes for new templates
2. **Selector template** correctly routes inputs to best-fit templates (validated by CNS outcomes)
3. **Flat registry** indexes all templates with type discrimination
4. **CNS spans** emitted at select/render/execute stages
5. **Matroshka depth** enforced (default: 7, configurable)
6. **hLexicon terms** validated against canonical set
7. **YAML/Jinja2 files** editable without recompilation
8. **Rust LOC** for `hkask-templates` ≤5,000

---

## References

[^jinja2]: Ronacher, A. (2024). *Jinja2 documentation*. Pallets Projects. https://jinja.palletsprojects.com/
[^minijinja]: Ronacher, A. (2024). *minijinja: A Jinja2-compatible template engine for Rust*. https://github.com/mitsuhiko/minijinja
[^hKask-AGENTS]: hKask Project. (2026). *AGENTS.md*. `/home/mdz-axolotl/Clones/hKask/AGENTS.md`.
[^hKask-master]: hKask Project. (2026). *hKask Architecture Master v0.21.0*. `docs/architecture/hKask-architecture-master.md`.
[^evans-ddd]: Evans, E. (2003). *Domain-Driven Design: Tackling Complexity in the Heart of Software*. Addison-Wesley. Strategic design and bounded context separation.
[^cockburn-hex]: Cockburn, A. (2005). *Hexagonal Architecture* (Ports and Adapters). https://alistair.cockburn.us/hexagonal-architecture/
[^fowler-poeaa]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture*. Addison-Wesley. Registry pattern (pp. 490–494).
[^lewis-rag]: Lewis, P., Perez, E., Piktus, A., Petroni, F., Karpukhin, V., Goyal, N., Küttler, H., Lewis, M., Yih, W., Rocktäschel, T., Kiela, D., & Petroni, F. (2020). Retrieval-augmented generation for knowledge-intensive NLP tasks. *Advances in Neural Information Processing Systems*, 33, 9459–9474. https://arxiv.org/abs/2005.11401
[^vander-aalst-wf]: van der Aalst, W. M. P., ter Hofstede, A. H. M., Kiepuszewski, B., & Barros, A. P. (2003). Workflow patterns. *Distributed and Parallel Databases*, 14(1), 5–51. https://doi.org/10.1023/A:1022883727209
[^gamma-interpreter]: Gamma, E., Helm, R., Johnson, R., & Vlissides, J. (1994). *Design Patterns: Elements of Reusable Object-Oriented Software*. Addison-Wesley. Interpreter pattern (pp. 243–255).

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*The Rust is the loom. The YAML/Jinja2 is the thread.*
*MVP in progress.*
