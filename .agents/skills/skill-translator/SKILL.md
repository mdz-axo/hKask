---
name: skill-translator
visibility: public
description: "Translate external skills into hKask registry crates (manifest.yaml + .j2 templates) as the canonical source of truth. Generate SKILL.md companions from registry crates. Convert between skill systems. Use when adapting skills from other projects, converting between formats, or when the user says 'translate this skill' or 'reverse-generate SKILL.md'."
---

# Skill Translator

Translate external skills into hKask's **registry-first** architecture per P5.1. The canonical target is a **registry crate** (`manifest.yaml` + `*.j2` templates) — the primary runtime artifact. The SKILL.md is a generated companion, not a co-equal target.

## Registry-First Target Architecture

| Artifact | Audience | Role |
|----------|----------|------|
| Registry crate (`manifest.yaml` + `*.j2`) | hKask inference engine (`ManifestExecutor`, `SqliteRegistry`) | Canonical source of truth — primary translation target |
| SKILL.md (`.agents/skills/<name>/SKILL.md`) | Zed coding agent | Generated companion — derived from registry crate |

A complete translation always produces a registry crate. The SKILL.md is optional for runtime correctness but should be generated when the Zed coding agent needs the skill during development.

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
  version: "0.28.0"
  description: <one-line>
templates:
  - id: <skill-name>/<template-id>
    path: <template-id>.j2
    type: KnowAct | WordAct | FlowDef
    lexicon_terms: [term, ...]
    description: <one-line>
vocabulary_terms: [term, ...]
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

## Step Classification → Registry Mapping

Every source step maps to a registry template type:

| Step Type | Signal | Registry Target |
|-----------|--------|-----------------|
| **Cognitive** (reasoning, decision, synthesis) | Chooses between alternatives, synthesizes multiple sources | `.j2` KnowAct template |
| **Workflow/process** (dispatch, route, recall) | Deterministic routing, data retrieval | `.j2` WordAct or FlowDef template |
| **Guardrail/constraint** | Safety rule, consent gate, resource limit | manifest constraints + template `visibility`/`energy_cap` |
| **Observability** | Logging, tracing, alerting | CNS span references in `.j2` body |

### Type selection decision tree

```
Step involves reasoning/judgment?
  YES → KnowAct
  NO → Step is mechanical dispatch/routing?
    YES → FlowDef
    NO → Step is deterministic transformation (compress, format, extract)?
      YES → WordAct
      NO → Default to KnowAct
```

### Valid template_type values

**ONLY** these three: `WordAct`, `KnowAct`, `FlowDef`. Never use Cognition, Prompt, Process.

| Type | When | Reasoning Effort | Typical energy_cap |
|------|------|-----------------|-------------------|
| `KnowAct` | Cognitive steps requiring judgment | high/full | 4096–8192 |
| `WordAct` | Deterministic text transformation | low/minimal | 2048–4096 |
| `FlowDef` | Dispatch, routing, recall, process orchestration | low/medium | 2000–4096 |

## Format Analysis Framework

Before translating, characterize the source on these dimensions:

| Dimension | What to Identify | Registry Impact |
|-----------|-----------------|-----------------|
| **Manifest type** | YAML, JSON, markdown frontmatter, etc. | Structural mapping to `manifest.yaml` |
| **Execution model** | REPL, on-demand, pipeline, FSM | Cognitive → KnowAct; dispatch → FlowDef; transform → WordAct |
| **State model** | Explicit vars, implicit context, external, stateless | State → `.j2` contract input/output |
| **Consent model** | User confirmation, OCAP, guardrails | Constraints → manifest + contract visibility |
| **Observability** | Spans, events, logs, silent | Map to CNS span references |
| **Domain references** | External systems, tools, APIs | Substitute via domain table |

## Translation Workflows

### Forward Translation (External → Registry Crate)

1. **Analyze** — Read the source skill. Characterize its format on the six dimensions.
2. **Classify** — Walk each section/step. Classify as cognitive, workflow, or guardrail. Determine template_type.
3. **Map** — For each element, identify the registry target (manifest field or .j2 template).
4. **Draft** — Produce:
   - (a) `manifest.yaml` — crate metadata, template entries, vocabulary terms
   - (b) `.j2` template files — one per classified step, with valid `[inference]` frontmatter
5. **Validate** — Check ALL of the following:
   - `template_type` is WordAct, KnowAct, or FlowDef (never Cognition/Prompt/Process)
   - `visibility` is Private, Public, or Shared
   - `energy_cap` is in 2048–8192 range
   - `lexicon_terms` exist in known vocabulary or are explicitly proposed as new terms
   - `contract.input` and `contract.output` are structured JSON types
   - Every source step appears in at least one registry element
6. **Generate SKILL.md** — From the completed registry crate:
   - Frontmatter `name` from manifest `crate.name`
   - Frontmatter `description` from manifest `crate.description`
   - Body `## When to Use` from template descriptions
   - Body `## Instructions` from `.j2` system prompt bodies
7. **Review** — Present translation summary: preserved, adapted, dropped, unresolved.

### Reverse Translation (Registry Crate → SKILL.md)

When only a registry crate exists and a SKILL.md companion is needed for the Zed coding agent:

1. **Read** `manifest.yaml` → extract `crate.name`, `crate.description`, template entries
2. **Read** each `.j2` → extract `template_type`, `lexicon_terms`, system prompt body
3. **Generate** SKILL.md:
   ```markdown
   ---
   name: <crate.name>
   visibility: public
   description: "<crate.description>"
   ---

   # <Name Title>

   <crate.description>

   ## When to Use
   - <derived from template descriptions and trigger conditions in .j2 body>

   ## Instructions
   1. <derived from .j2 system prompt instructions>
   ...

   ## Registry Templates

   | Template | Type | Purpose |
   |----------|------|--------|
   | `<template.path>` | `<template.type>` | `<template.description>` |
   ...
   ```

4. **Validate**: SKILL.md frontmatter valid, body non-empty, no drift from registry

## Domain Substitution Table

When the source references system-specific concepts, substitute:

| Source Domain | Registry Equivalent |
|--------------|--------------------|
| Journal / log store | `hkask-storage` (SQLite + SQLCipher) |
| Sentinel / sensor | MCP tool dispatch (`cns.tool.<subsystem>`) |
| Baselines / EWMA | CNS variety counters |
| Nurse / regulator | Curator Agent |
| Proprioception | CNS algedonic signals (`cns.cybernetics.backpressure`) |
| IDRS / consent gate | OCAP capability delegation |
| REST API calls | `web` MCP server |
| File system operations | `read_file`/`write_file`/`edit_file` tools |
| LLM inference | `inference` MCP server (hkask-inference router) |
| Database queries | `hkask-storage` |
| Event bus / pubsub | `hkask-cns` algedonic alerts |
| Git operations | `git` MCP server |
| Custom tool dispatch | `hkask-mcp` dynamic discovery |

If no hKask equivalent exists: mark `[unresolved: no hKask equivalent for <source_ref>]`.

## What Gets Lost in Translation

Some source concepts have no direct registry equivalent. Document every asymmetry:

| Concept | Registry | Notes |
|---------|----------|-------|
| Energy/token budgets per step | `energy_cap` in `[inference]` frontmatter | CNS gas budgets are runtime-level |
| Step ordinals / FSM transitions | FlowDef template chain | Ordinal flow → template chain in registry |
| Source persona voice ("I do X") | Dropped (system prompt is imperative) | Rewrite as "Do X" |
| Source consent gates | OCAP in contract | Mechanism differs |
| Source-specific observability | CNS span references in `.j2` | Map to `cns.<domain>.<operation>` |
| Script-based probes | `.j2` contract input/output | Agent uses terminal/grep/read_file |
| Source manifest `symptoms` | Dropped (hKask matches by description) | Encoded in `crate.description` |

## Safety

Refuse to translate:
- Skills with arbitrary code execution steps (no `action: execute` equivalents)
- Skills with unresolvable references (missing templates, broken paths)
- Skills that require network access hKask doesn't have
- Skills that violate Magna Carta principles

## When to Use This Skill

- "Translate this skill" / "Convert this skill": Run forward translation workflow.
- "Generate SKILL.md for X" / "Reverse-generate SKILL.md": Run reverse translation from registry crate.
- "Can I use this [external] skill in hKask?": Analyze format, classify steps, assess feasibility.
- Adapting skills from other projects: Apply domain substitution, produce registry crate.
