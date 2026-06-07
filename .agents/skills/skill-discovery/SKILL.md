---
name: skill-discovery
visibility: public
description: "Find, evaluate, and install dual-layer skills (SKILL.md + registry templates). Detect capability gaps across both layers, search for candidates, validate format and quality for each layer, and guide installation. Use when the user says 'find a skill for X', 'I need a skill that does Y', or when a task pattern has no matching skill."
---

# Skill Discovery

Manage the full lifecycle of finding, evaluating, and installing **dual-layer** skills. A complete skill has two layers; both must be accounted for at every stage.

## Dual-Layer Architecture

| Layer | Path | Audience | Purpose |
|-------|------|----------|---------|
| **Zed Agent Skill** | `.agents/skills/<name>/SKILL.md` | Zed coding agent | Description-based activation, imperative instructions |
| **Registry Template** | `registry/templates/<name>/manifest.yaml` + `*.j2` | hKask inference engine | Runtime template selection, cascade dispatch |

The registry crate is the primary runtime artifact. The SKILL.md is the companion guide. A skill with only one layer is **incomplete** — flag it.

## Detecting Capability Gaps

Check BOTH layers for gaps.

| Trigger | Layer | Gap Type | Detection Method |
|---------|-------|----------|-------------------|
| Task pattern has no matching skill description | Zed | Coverage gap | Search `.agents/skills/` descriptions |
| No WordAct/KnowAct/FlowDef template for a task pattern | Registry | Template coverage gap | Scan `registry/templates/` manifest `type` fields |
| Template uses hLexicon terms not in workspace registry | Registry | hLexicon gap | Cross-reference `lexicon_terms` in manifests against `registry/registries/hlexicon-workspace.yaml` |
| Skill has SKILL.md but no registry templates (or vice versa) | Both | Layer completeness gap | Pair-check: `.agents/skills/<name>/` exists ↔ `registry/templates/<name>/` exists |
| Too few .j2 templates for cascade to select from | Registry | Cascade gap | Count `.j2` files per skill; flag skills with <2 templates or missing a key template_type |
| User says "I wish the agent could do X" | Either | Feature gap | Is X within hKask scope? (No visual UI, no external monitoring stacks) |
| Repeated manual steps across sessions | Either | Automation gap | Can steps be codified into both layers? |
| A constraint is repeatedly violated with no skill to guide it | Either | Governance gap | A skill enforcing that constraint may help |
| Current skill instructions are vague or incomplete | Zed | Quality gap | Improve the existing SKILL.md rather than find a new one |

When you spot a gap, say so explicitly: "No skill covers X [Zed layer / registry layer / both]. Want me to search for one, or should I help build one?"

### Template-Type Coverage Audit

Across the entire corpus, the three valid `template_type` values should all be represented:

| Type | Purpose | Typical Count |
|------|---------|---------------|
| WordAct | Produce output text | ≥1 per skill |
| KnowAct | Analyze, classify, evaluate | ≥1 per skill |
| FlowDef | Orchestrate multi-step process | If skill has procedural flow |

Flag if a skill's manifest declares zero templates of a type that its SKILL.md instructions imply.

## Searching for Skills

### Where skills live

| Source | What to Look For |
|--------|-----------------|
| GitHub repos | `skills/` or `.agents/skills/` dirs, `registry/templates/` dirs |
| Direct URLs | SKILL.md files, manifest.yaml files |
| Local filesystem | Both layer paths in the workspace |
| hKask registry | Already-registered templates |

### Search strategies

```
"hKask skill [domain] site:github.com"
"SKILL.md agent skill [domain] .agents/skills"
"manifest.yaml template_type [domain] registry/templates"
```

If no hKask-specific skill exists, search for the underlying capability, then adapt using `skill-translator`.

## Evaluating a Candidate Skill

Evaluate BOTH layers. A candidate may provide one or both.

### Zed Layer (SKILL.md) Validation

```
☐ File exists: .agents/skills/<name>/SKILL.md
☐ Frontmatter has `name` field matching directory name
☐ `name` is lowercase-hyphenated (^[a-z0-9]+(-[a-z0-9]+)*$)
☐ `description` field present, 1–1024 chars, specific and actionable
☐ Body is non-empty, contains concrete imperative instructions
☐ No references to files or paths that don't exist within the skill directory
```

### Registry Layer Validation

```
☐ manifest.yaml exists: registry/templates/<name>/manifest.yaml
☐ crate.name matches skill name, lowercase-hyphenated
☐ crate.version is semver string
☐ templates list is non-empty
☐ Each template entry has: id, path, type, lexicon_terms, description
☐ template_type ∈ {WordAct, KnowAct, FlowDef} — reject anything else
☐ Each referenced .j2 file exists in the directory
☐ .j2 frontmatter: template_type ∈ {WordAct, KnowAct, FlowDef}
☐ .j2 frontmatter: contract has structured input and output
☐ .j2 frontmatter: energy_cap ∈ [2048, 8192]
☐ .j2 frontmatter: visibility ∈ {Private, Public, Shared}
☐ hlexicon_terms in manifest all exist in registry/registries/hlexicon-workspace.yaml
☐ lexicon_terms in each .j2 frontmatter exist in workspace registry
```

### Quality Evaluation (Both Layers)

```
☐ Instructions are imperative — "Do X" not "Consider doing X"
☐ Instructions are self-contained — no assumed context not provided
☐ No placeholder content ("TODO", "Fill in later")
☐ No contradictions between layers or within a layer
☐ Skill scope is clear — one thing well
☐ No unnecessary complexity
```

### Safety Evaluation (Both Layers)

```
☐ No Magna Carta violations (P1–P4)
☐ No headless constraint violations (no visual UI, no Grafana, no Prometheus)
☐ No `todo!` / `unimplemented!` / `#[deprecated]` references (P6/P7)
☐ If skill references CNS spans, they exist in the canonical namespace list
☐ If skill references crate paths, they exist in the workspace
☐ No secrets or API keys embedded
```

### Red Flags

| Flag | Action |
|------|--------|
| Vague descriptions ("Helps with code") | Reject, request specificity |
| Invalid template_type | Reject, require WordAct/KnowAct/FlowDef |
| hLexicon terms not in workspace | Reject until terms are registered or replaced |
| External dependencies not in workspace | Flag for review |
| Network calls without documentation | Flag |
| Instructions that assume a specific model or persona | Needs adaptation |
| Secrets or API keys | Reject immediately |
| SKILL.md with no registry templates (or vice versa) | Flag as incomplete |

## Installation Guide

Install BOTH layers when the candidate provides them.

### Full install (both layers)

1. Copy SKILL.md to `.agents/skills/<name>/`
2. Copy manifest.yaml + *.j2 to `registry/templates/<name>/`
3. Verify Zed layer: frontmatter validates, instructions are concrete
4. Verify registry layer: manifest structure, .j2 frontmatter, hLexicon terms exist in workspace
5. Verify cross-layer: SKILL.md name matches manifest crate.name

### Zed-only install (no registry templates)

1. Copy SKILL.md to `.agents/skills/<name>/`
2. Validate Zed layer checks
3. Flag as incomplete — note that no registry templates exist

### Registry-only install (no SKILL.md)

1. Copy manifest.yaml + *.j2 to `registry/templates/<name>/`
2. Validate registry layer checks
3. Flag as incomplete — note that no companion SKILL.md exists

### Install with translation (non-hKask format)

Use `skill-translator` to convert, then install both layers.

### Verify after installation

1. **Zed format check**: SKILL.md frontmatter validates
2. **Registry format check**: manifest.yaml structure, .j2 frontmatter, template_type validity
3. **hLexicon check**: All declared terms exist in workspace registry
4. **Content check**: Instructions concrete and actionable
5. **Safety check**: No Magna Carta violations, no headless constraint violations
6. **CNS check**: Any `cns.*` span references are valid
7. **Cross-layer check**: Names match, scope is consistent between layers
8. **Integration check**: Description specific enough to be matched when relevant

Failure modes:

| Failure | Fix |
|---------|-----|
| Zed format fails | Fix frontmatter (name, description) |
| Registry format fails | Fix manifest structure or .j2 frontmatter |
| hLexicon fails | Register missing terms or replace with existing terms |
| Content fails | Rewrite vague instructions |
| Safety fails | Report violation, refuse installation |
| CNS fails | Update span references |
| Cross-layer fails | Align names and scope |
| Integration fails | Improve description specificity |

## Building a Skill from Scratch

Scaffold BOTH layers. Use `create-skill` conventions for the Zed layer.

### Step 1: Decide scope

Project-local (`.agents/skills/` + `registry/templates/`) vs global (`~/.agents/skills/` — registry layer has no global equivalent).

### Step 2: Choose name

Descriptive, lowercase-hyphenated. Must be valid as both directory name and crate name.

### Step 3: Write SKILL.md (Zed layer)

```markdown
---
name: skill-name
visibility: public
description: "Specific, actionable description of what the skill does and when to activate."
---

# Skill Name

[Imperative instructions for the Zed agent]
```

### Step 4: Write manifest.yaml (Registry layer)

```yaml
crate:
  name: skill-name
  version: "0.24.0"
  description: >
    [Same scope as SKILL.md description, runtime-oriented]

templates:
  - id: skill-name/template-name
    path: template-name.j2
    type: KnowAct          # WordAct | KnowAct | FlowDef
    lexicon_terms: [term1, term2, term3]
    description: >
      [What this template produces at inference time]

hlexicon_terms:
  - term1
  - term2
  - term3
```

### Step 5: Write at least one .j2 template

```jinja2
[inference]
template_type: KnowAct
lexicon_terms: [term1, term2, term3]
contract:
  input:
    key_name: type
  output:
    result_name: type
  energy_cap: 4096
  visibility: Shared

---
{# Template: skill-name/template-name.j2 #}
{# KnowAct — [concise purpose] #}

[inference]
temperature = 0.3
reasoning_effort = "high"
verbosity = "detailed"
max_tokens = 4096
thinking_budget = "full"

[Template body with Jinja2 expressions]
{{ input_variable }}
```

### Step 6: Validate hLexicon terms

Cross-reference every `lexicon_terms` and `hlexicon_terms` entry against `registry/registries/hlexicon-workspace.yaml`. If a term is missing, either:
- Register it in the workspace hLexicon (update markdown source, regenerate YAML)
- Replace with an existing term that covers the same semantic space

### Step 7: Validate both layers

Run the evaluation checks from the "Evaluating a Candidate Skill" section against both layers of the new skill.

## Skill Hygiene

### When to update a skill

- Architecture changes make references stale (moved crates, renamed spans)
- New Magna Carta principles or constraints added
- Instructions too vague or too rigid
- Description no longer matches instructions
- .j2 contract no longer matches actual inputs/outputs
- hLexicon terms deprecated or renamed

### When to retire a skill

- Domain no longer relevant
- Better skill supersedes it
- Instructions consistently ignored (wrong abstraction level)
- User says they don't need it

Retirement: delete both `.agents/skills/<name>/` AND `registry/templates/<name>/`. Partial retirement (one layer) leaves an incomplete skill — avoid unless replacing that layer.

### Sharing skills

Share both layers together:
1. Push `.agents/skills/<name>/` and `registry/templates/<name>/` to a git repo
2. Recipient copies both directories into their workspace

## When to Use This Skill

| Trigger | Action |
|---------|--------|
| "Find a skill for X" | Detect gap (both layers) → search → evaluate → install |
| "I need the agent to do Y" | Check both layers; if absent, search or build |
| "Can I use this skill from [other project]?" | Evaluate format compatibility, translate if needed |
| Recurring manual patterns | Suggest codifying into both layers |
| After installing a skill | Verify both layers loaded and valid |
| "Audit skill coverage" | Run template-type and hLexicon coverage audit |