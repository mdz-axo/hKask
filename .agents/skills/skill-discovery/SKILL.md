---
name: skill-discovery
visibility: public
description: "Find, evaluate, and install skills for hKask. Registry crate (manifest.yaml + *.j2) is the canonical source of truth. Detect capability gaps, search for candidates, validate format and quality, and guide installation. Use when the user says 'find a skill for X', 'I need a skill that does Y', or when a task pattern has no matching skill."
---

# Skill Discovery

Manage the full lifecycle of finding, evaluating, and installing skills. Per P5.1, the registry crate is the canonical source of truth. The SKILL.md is a generated companion.

## Registry-First Architecture

| Artifact | Path | Audience | Role |
|----------|------|----------|------|
| Registry crate | `registry/templates/<name>/manifest.yaml` + `*.j2` | hKask inference engine | Canonical source of truth |
| SKILL.md | `.agents/skills/<name>/SKILL.md` | Zed coding agent | Generated companion |

A skill is complete when its registry crate exists. The SKILL.md is optional for runtime.

## Detecting Capability Gaps

| Trigger | Layer | Gap Type | Detection Method |
|---------|-------|----------|-------------------|
| Task pattern has no matching registry crate | Registry | Coverage gap | Scan `registry/templates/` manifest `type` fields |
| No WordAct/KnowAct/FlowDef template for a task pattern | Registry | Template coverage gap | Scan manifests for template_type coverage |
| Template uses vocabulary terms not in known vocabulary | Registry | Vocabulary gap | Cross-reference `lexicon_terms` against vocabulary |
| Registry crate exists but no SKILL.md companion | Companion | Documentation gap | Pair-check: `registry/templates/<name>/` exists but `.agents/skills/<name>/` absent |
| Too few .j2 templates for cascade to select from | Registry | Cascade gap | Count `.j2` files per skill; flag <2 templates |
| User says "I wish the agent could do X" | Either | Feature gap | Is X within hKask scope? |
| Repeated manual steps across sessions | Either | Automation gap | Can steps be codified into a registry crate? |
| A constraint is repeatedly violated | Either | Governance gap | A skill enforcing that constraint may help |
| SKILL.md exists without registry crate | Registry | Critical gap | SKILL.md is not executable — needs registry crate |

When you spot a gap, say so explicitly: "No registry crate covers X. Want me to search for one, or should I help build one?"

### Template-Type Coverage Audit

| Type | Purpose | Typical Count |
|------|---------|---------------|
| WordAct | Produce output text | >=1 per skill |
| KnowAct | Analyze, classify, evaluate | >=1 per skill |
| FlowDef | Orchestrate multi-step process | If skill has procedural flow |

Flag if a skill's manifest declares zero templates of a type that its description implies.

## Searching for Skills

### Where skills live

| Source | What to Look For |
|--------|-----------------|
| GitHub repos | `registry/templates/` directories, `skills/` directories |
| Direct URLs | manifest.yaml files, .j2 template collections |
| Local filesystem | Registry crate paths in the workspace |
| hKask registry | Already-registered templates |

### Search strategies

```
"hKask skill [domain] site:github.com"
"manifest.yaml template_type [domain] registry/templates"
```

If no hKask-specific skill exists, search for the underlying capability, then adapt using `skill-manager-translate`.

## Evaluating a Candidate Skill

**The registry crate is the only required artifact.** Evaluate it first.

### Registry Crate Validation

```
- manifest.yaml exists: registry/templates/<name>/manifest.yaml
- crate.name matches skill name, lowercase-hyphenated
- crate.version is semver string
- templates list is non-empty
- Each template entry has: id, path, type, lexicon_terms, description
- template_type in {WordAct, KnowAct, FlowDef} — reject anything else
- Each referenced .j2 file exists in the directory
- .j2 frontmatter: template_type in {WordAct, KnowAct, FlowDef}
- .j2 frontmatter: contract has structured input and output
- .j2 frontmatter: energy_cap in [2048, 8192]
- .j2 frontmatter: visibility in {Private, Public, Shared}
- lexicon_terms in manifest all exist in known vocabulary
- lexicon_terms in each .j2 frontmatter exist in known vocabulary
```

### SKILL.md Companion Validation (optional)

```
- File exists: .agents/skills/<name>/SKILL.md
- Frontmatter has name field matching directory name
- name is lowercase-hyphenated
- description field present, 1–1024 chars, specific
- Body is non-empty, contains concrete imperative instructions
- No drift from registry — does not claim behaviors registry templates do not support
```

### Quality Evaluation

```
- Instructions are imperative — "Do X" not "Consider doing X"
- Instructions are self-contained — no assumed context not provided
- No placeholder content ("TODO", "Fill in later")
- No contradictions within the registry crate or between crate and SKILL.md
- Skill scope is clear — one thing well
- No unnecessary complexity
```

### Safety Evaluation

```
- No Magna Carta violations (P1–P4)
- No headless constraint violations (no visual UI, no Grafana, no Prometheus)
- No todo! / unimplemented! / #[deprecated] references
- If skill references CNS spans, they exist in the canonical namespace list
- If skill references crate paths, they exist in the workspace
- No secrets or API keys embedded
```

### Red Flags

| Flag | Action |
|------|--------|
| Vague descriptions ("Helps with code") | Reject, request specificity |
| Invalid template_type | Reject, require WordAct/KnowAct/FlowDef |
| Vocabulary terms not in known vocabulary | Reject until terms are registered or replaced |
| External dependencies not in workspace | Flag for review |
| Network calls without documentation | Flag |
| Instructions that assume a specific model or persona | Needs adaptation |
| Secrets or API keys | Reject immediately |
| SKILL.md with no registry crate | Flag as incomplete — needs registry crate |

## Installation Guide

### Full install (registry crate)

1. Copy `manifest.yaml` + `*.j2` to `registry/templates/<name>/`
2. Verify registry: manifest structure, .j2 frontmatter, vocabulary terms exist
3. Generate SKILL.md companion (if needed for Zed agent)
4. Verify SKILL.md: frontmatter validates, matches registry

### Registry-only install (no SKILL.md needed)

1. Copy `manifest.yaml` + `*.j2` to `registry/templates/<name>/`
2. Validate registry layer checks
3. Done — skill is complete and executable in cascade

### SKILL.md-only source (needs registry)

1. Use `skill-manager-translate` forward translation to create registry crate
2. Copy registry crate to `registry/templates/<name>/`
3. Validate both

### Install with translation (non-hKask format)

Use `skill-manager-translate` to convert, then install registry crate.

### Verify after installation

1. **Registry format check**: manifest.yaml structure, .j2 frontmatter, template_type validity
2. **Vocabulary check**: All declared terms exist in known vocabulary
3. **Content check**: Instructions concrete and actionable
4. **Safety check**: No Magna Carta violations, no headless constraint violations
5. **CNS check**: Any `cns.*` span references are valid

Failure modes:

| Failure | Fix |
|---------|-----|
| Registry format fails | Fix manifest structure or .j2 frontmatter |
| Vocabulary check fails | Register missing terms or replace with existing terms |
| Content fails | Rewrite vague instructions |
| Safety fails | Report violation, refuse installation |
| CNS fails | Update span references |

## Building a Registry Crate from Scratch

### Step 1: Decide scope

Project-local (`registry/templates/`) vs global (no global equivalent for registry).

### Step 2: Choose name

Descriptive, lowercase-hyphenated. Must be valid as directory name and crate name.

### Step 3: Write manifest.yaml

```yaml
crate:
  name: skill-name
  version: "0.28.0"
  description: >
    [What this skill does at runtime]

templates:
  - id: skill-name/template-name
    path: template-name.j2
    type: KnowAct
    lexicon_terms: [term1, term2, term3]
    description: >
      [What this template produces at inference time]

vocabulary_terms:
  - term1
  - term2
  - term3
```

### Step 4: Write at least one .j2 template

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

### Step 5: Validate vocabulary terms

Cross-reference every `lexicon_terms` entry against the known vocabulary (`crates/hkask-templates/src/vocabulary.rs` `KNOWN_TERMS`). Unknown terms are logged as warnings at registration.

### Step 6: Validate registry crate

Run the evaluation checks from the "Evaluating a Candidate Skill" section.

### Step 7: Generate SKILL.md companion (optional)

Use `skill-manager-reverse` reverse translation to generate a SKILL.md for the Zed coding agent.

## Skill Hygiene

### When to update a skill

- Architecture changes make references stale
- New Magna Carta principles or constraints added
- .j2 contract no longer matches actual inputs/outputs
- Vocabulary terms deprecated or renamed

### When to retire a skill

- Domain no longer relevant
- Better skill supersedes it
- User says they don't need it

Retirement: delete `registry/templates/<name>/` (canonical). Also delete `./agents/skills/<name>/` if a companion exists.

## When to Use This Skill

| Trigger | Action |
|---------|--------|
| "Find a skill for X" | Detect gap → search → evaluate → install registry crate |
| "I need the agent to do Y" | Check registry; if absent, search or build |
| "Can I use this skill from [other project]?" | Evaluate format compatibility, translate if needed |
| Recurring manual patterns | Suggest codifying into registry crate |
| After installing a skill | Verify registry crate loaded and valid |
| "Audit skill coverage" | Run template-type and vocabulary coverage audit |

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `skill-discovery-detect-gap.j2` | KnowAct | Detect capability gaps in the registry corpus |
| `skill-discovery-evaluate.j2` | KnowAct | Evaluate a candidate registry crate against criteria |


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/skill-discovery.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = the identified capability gap is sufficiently resolved

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 16000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
