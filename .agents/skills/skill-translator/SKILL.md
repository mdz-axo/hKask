---
name: skill-translator
visibility: public
description: "Translate agent skills into hKask's dual-layer architecture: registry crate (manifest.yaml + .j2 templates) as primary runtime artifact, plus SKILL.md companion guide for the Zed coding agent. Analyze source format, classify steps, map to both layers. Use when adapting skills from other projects, converting between skill systems, or when the user says 'translate this skill' or 'convert this skill'."
---

# Skill Translator

Translate external skills into hKask's **dual-layer** architecture. The target is never just a SKILL.md — it is a **registry crate** (primary runtime artifact) plus a **SKILL.md** (companion guide for the Zed coding agent).

## Dual-Layer Target Architecture

| Layer | Artifact | Audience | Purpose |
|-------|----------|----------|---------|
| **Registry (primary)** | `manifest.yaml` + `*.j2` templates | hKask inference engine | Executable process steps |
| **SKILL.md (companion)** | `.agents/skills/<name>/SKILL.md` | Zed coding agent | How to reason about the domain |

A complete translated skill has BOTH layers. They are not interchangeable — each serves a different consumer.

### Registry crate structure

```
registry/templates/<name>/
  manifest.yaml          # Crate metadata + template list
  <step-name>.j2         # Jinja2 templates (one per step)
```

### manifest.yaml shape

```yaml
crate:
  name: <skill-name>
  version: "<version>"
  description: <one-line>
templates:
  - id: <skill-name>/<template-id>
    path: <template-id>.j2
    type: KnowAct | WordAct | FlowDef
    lexicon_terms: [term, ...]
    description: <one-line>
hlexicon_terms: [term, ...]
```

### .j2 template shape

```
[inference]
template_type: KnowAct | WordAct | FlowDef
lexicon_terms: [term, ...]
contract:
  input:
    <field>: <type>
  output:
    <field>: <type>
  energy_cap: <2048-8192>
  visibility: Private | Public | Shared
---
{# Template header #}
[Jinja2 body with system prompt, {{ variables }}, JSON output schema]
```

## Step Classification → Dual-Layer Mapping

Every source step maps to **both** layers:

| Step Type | Signal | Registry Target | SKILL.md Target |
|-----------|--------|-----------------|-----------------|
| **Cognitive** (reasoning, decision, synthesis) | Chooses between alternatives, synthesizes multiple sources, output depends on context | `.j2` KnowAct template | `## Instructions` section |
| **Workflow/process** (dispatch, route, recall, transform) | Deterministic routing, data retrieval, mechanical transformation | `.j2` WordAct or FlowDef template | `## Procedures` section |
| **Guardrail/constraint** | Safety rule, consent gate, resource limit | `manifest.yaml` constraints + template `visibility`/`energy_cap` | `## Constraints` section |
| **Observability** | Logging, tracing, alerting | CNS span references in `.j2` body | `## Debug` section |

### Type selection decision tree

```
Step involves reasoning/judgment?
  YES → KnowAct
  NO → Step is mechanical dispatch/routing?
    YES → FlowDef
    NO → Step is deterministic transformation (compress, format, extract)?
      YES → WordAct
      NO → Default to KnowAct (safer for ambiguous steps)
```

### Valid template_type values

**ONLY** these three. Never use Cognition, Prompt, Process, or any other name.

| Type | When | Reasoning Effort | Typical energy_cap |
|------|------|-----------------|-------------------|
| `KnowAct` | Cognitive steps requiring judgment | high/full | 4096–8192 |
| `WordAct` | Deterministic text transformation | low/minimal | 2048–4096 |
| `FlowDef` | Dispatch, routing, recall, process orchestration | low/medium | 2000–4096 |

## Format Analysis Framework

Before translating, characterize the source on these dimensions:

| Dimension | What to Identify | Dual-Layer Impact |
|-----------|-----------------|-------------------|
| **Manifest type** | YAML, JSON, markdown frontmatter, etc. | Structural mapping to `manifest.yaml` |
| **Execution model** | REPL, on-demand, pipeline, FSM | Cognitive → KnowAct; dispatch → FlowDef; transform → WordAct |
| **State model** | Explicit vars, implicit context, external, stateless | State → `.j2` contract input/output + SKILL.md context variables |
| **Consent model** | User confirmation, OCAP, guardrails | Constraints → manifest + SKILL.md Constraints section |
| **Observability** | Spans, events, logs, silent | Map to CNS span references in both layers |
| **Domain references** | External systems, tools, APIs | Substitute via domain table → both layers |

## Translation Workflow

1. **Analyze** — Read the source skill. Characterize its format on the six dimensions.
2. **Classify** — Walk each section/step. Classify as cognitive, workflow, or guardrail. Determine template_type for each.
3. **Map** — For each element, identify targets in BOTH layers. Note any drops.
4. **Draft** — Produce three outputs:
   - (a) `manifest.yaml` — crate metadata, template entries, hlexicon terms
   - (b) `.j2` template files — one per classified step, with valid `[inference]` frontmatter
   - (c) `SKILL.md` — companion guide with frontmatter, instructions, procedures, constraints
5. **Validate** — Check ALL of the following:
   - `template_type` is WordAct, KnowAct, or FlowDef (never Cognition/Prompt/Process)
   - `visibility` is Private, Public, or Shared
   - `energy_cap` is in 2048–8192 range
   - `lexicon_terms` exist in hLexicon or are explicitly proposed as new terms
   - `contract.input` and `contract.output` are structured JSON types (string, integer, float, array, object)
   - SKILL.md frontmatter `name` is lowercase-hyphenated, matches directory name
   - SKILL.md frontmatter `description` is 1–1024 chars
   - Every source step appears in at least one layer (or is documented as dropped)
6. **Review** — Present translation summary: preserved, adapted, dropped, unresolved.

## Domain Substitution Table

When the source references system-specific concepts, substitute for BOTH layers:

| Source Domain | Registry Equivalent | SKILL.md Equivalent |
|--------------|--------------------|--------------------|
| Journal / log store | `hkask-storage` (SQLite + SQLCipher) | `hkask-storage` |
| Sentinel / sensor | MCP tool dispatch (`cns.tool.*`) | `cns.tool.*` spans |
| Baselines / EWMA | CNS variety counters | CNS variety counters |
| Nurse / regulator | Curator Agent | Curator Agent |
| Proprioception | CNS algedonic signals (`cns.cybernetics.*`) | `cns.cybernetics.*` spans |
| IDRS / consent gate | OCAP capability delegation | OCAP-gated instructions |
| REST API calls | `web` MCP server | `web` MCP server |
| File system operations | `read_file`/`write_file`/`edit_file` tools | Same tools |
| LLM inference | `inference` MCP server (hkask-inference router) | `inference` MCP server |
| Database queries | `hkask-storage` | `hkask-storage` |
| Event bus / pubsub | `hkask-cns` algedonic alerts | CNS alerts |
| Git operations | `git` MCP server | `git` MCP server |
| Custom tool dispatch | `hkask-mcp` dynamic discovery | `hkask-mcp` |

If no hKask equivalent exists: mark `[unresolved: no hKask equivalent for <source_ref>]`.

## What Gets Lost in Translation

Some source concepts map to one layer but not the other. Document every asymmetry:

| Concept | Registry Layer | SKILL.md Layer | Notes |
|---------|---------------|----------------|-------|
| Energy/token budgets per step | `energy_cap` in `[inference]` frontmatter | Dropped | CNS gas budgets are runtime-level, not agent-facing |
| Step ordinals / FSM transitions | FlowDef template chain | Section structure | Ordinal flow → section order in SKILL.md, template chain in registry |
| Source persona voice ("I do X") | Dropped (system prompt is imperative) | Rewrite as imperative ("Do X") | Strip personality, keep methodology |
| Source consent gates | OCAP in contract | `> Confirm before proceeding` markers | Mechanism differs between layers |
| Source-specific observability | CNS span references in `.j2` | `## Debug` section | Map to `cns.*` namespace |
| Script-based probes | `.j2` contract input/output | Built-in tool procedures | Agent uses terminal/grep/read_file |
| Source manifest `symptoms` | Dropped (hKask matches by description) | Encoded in `description` frontmatter | No symptom catalog in hKask |

## Safety

Refuse to translate:
- Skills with arbitrary code execution steps (no `action: execute` equivalents)
- Skills with unresolvable references (missing templates, broken paths)
- Skills that require network access hKask doesn't have
- Skills that violate Magna Carta principles (e.g., exposing episodic memory without consent)

## When to Use This Skill

- **"Translate this skill" / "Convert this skill":** Run the full workflow.
- **"Can I use this [external] skill in hKask?":** Analyze format, classify steps, assess dual-layer feasibility.
- **Adapting skills from other projects:** Apply domain substitution, produce both layers.
- **Evaluating a skill from a different format system:** Use the format analysis framework to identify what maps where.