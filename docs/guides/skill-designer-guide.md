---
title: "hKask Skill Designer Guide — Creating and Maintaining Agent Skills"
audience: [developers, architects, agents]
last_updated: 2026-06-16
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, lifecycle, curation]
---

# hKask Skill Designer Guide

**Purpose:** Definitive reference for designing, building, and maintaining skills in the hKask dual-layer architecture. Covers when to create each layer, structural rules, template_type discipline, visibility (P11), CNS span grounding, registration, and testing.

**Companion docs:** [`PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) (P1-P12).

---

## 1. The Dual-Layer Model

Every hKask skill exists in up to two layers:

| Layer | Location | Format | Purpose |
|-------|----------|--------|---------|
| **Zed agent layer** | `.agents/skills/<name>/SKILL.md` | Markdown with YAML frontmatter | Teaches the Zed coding agent a domain and methodology |
| **Registry template layer** | `registry/templates/<name>/` | `manifest.yaml` + `.j2` files | Runtime-executable templates (WordAct, KnowAct, FlowDef) |

**A skill does not need both layers.** The model supports four types:

### 1.1 Skill Type Classification

| Type | Description | Has SKILL.md? | Has manifest.yaml? | Example |
|------|-------------|---------------|-------------------|---------|
| **Type 1** | WordAct/KnowAct — pure cognitive act | Yes | No | `caveman` (single compression act) |
| **Type 2** | Orchestrator — composes other skills | Yes | No | `pragmatic-laziness` (orchestrates 5 sub-skills) |
| **Type 3** | Standalone flow/process | Sometimes | Yes | `superforecasting` (multi-step FlowDef, no Zed companion) |
| **Type 4** | Meta process — both layers | Yes | Yes | `coding-guidelines`, `kata-coaching`, `essentialist` |

**The deletion test (P5):** If a skill is Type 1 and works as SKILL.md alone, do NOT add a manifest. Premature bundling creates maintenance debt — every manifest must be registered, schema-validated, and tested.

### 1.2 When Each Layer Is Needed

| If... | Then create... |
|-------|----------------|
| A Zed agent needs procedural domain knowledge | `SKILL.md` |
| The runtime must execute a template (WordAct/KnowAct/FlowDef) | `manifest.yaml` + `.j2` |
| A template composes other templates by id | `.j2` FlowDef in registry layer |
| A skill is referenced by a bundle | Both layers (Type 4), score ≥ 0.8 |

---

## 2. Creating a SKILL.md (Zed Agent Layer)

### 2.1 Structure

```markdown
---
name: my-skill
visibility: Public
description: One-sentence purpose statement. When to activate. Use when...
---

# My Skill

## Domain

Brief domain context the agent needs.

## Procedure

### Step 1: ...

### Step 2: ...

## Constraints

1. Principle-backed constraints
2. ...

## Key Files

| File | Purpose |
|------|---------|
| `path/to/file.rs` | What it does |
```

### 2.2 Frontmatter Rules

| Field | Required | Values | Notes |
|-------|----------|--------|-------|
| `name` | Yes | kebab-case, matches directory name | Must match `.agents/skills/<name>/` |
| `visibility` | Yes | `Public` or `Private` | P11 — governs who can discover and load the skill |
| `description` | Yes | Single line, ~80 chars | Shown in `kask skill list`; must include activation trigger |

**No other frontmatter fields are required.** The SKILL.md body is freeform Markdown.

### 2.3 CNS Span References

Every SKILL.md must reference only canonical CNS spans from `crates/hkask-types/src/cns.rs`. Never invent span names or use wildcard patterns (`cns.*`, `cns.tool.*`).

**Correct:**
- `cns.tool.condenser` — tool invocation governance
- `cns.inference.router` — inference routing
- `kata.cycle.start` — kata cycle span

**Incorrect (will fail CNS health check):**
- `cns.cybernetics.*` — wildcard, not canonical
- `cns.inference` — underspecified; use `cns.inference.router`
- `kask /status` — hallucinated CLI command; use `kask cns health`

### 2.4 Checklist: SKILL.md Quality Gate

- [ ] `name` matches directory name
- [ ] `visibility` is `Public` or `Private` (not `Shared`)
- [ ] `description` includes activation trigger ("Use when...")
- [ ] CNS spans reference canonical names from `cns.rs`
- [ ] No `kask /status` or other hallucinated CLI commands
- [ ] Constraints are grounded in PRINCIPLES.md (cite P# where applicable)

---

## 3. Creating a Manifest + Templates (Registry Layer)

### 3.1 Directory Structure

```
registry/templates/<skill-name>/
├── manifest.yaml          # Crate metadata + template registry
├── step-one.j2            # WordAct or KnowAct template
├── step-two.j2            # WordAct or KnowAct template
└── flow.j2                # FlowDef (if multi-step orchestration)
```

### 3.2 manifest.yaml Structure

```yaml
# Template crate manifest for <skill-name>
# ℏKask v0.27.0

crate:
  name: <skill-name>
  version: "0.27.0"
  description: >
    Skill purpose. What it does, when it activates, what principles it enforces.

templates:
  - id: <skill-name>/template-id
    path: template-id.j2
    type: WordAct          # WordAct | KnowAct | FlowDef
    lexicon_terms: [term1, term2, term3]
    description: >
      What this specific template does.
```

**Manifest rules:**
- `crate.name` must match the directory name
- `crate.version` must match the hKask version
- `templates[].type` must be one of: `WordAct`, `KnowAct`, `FlowDef`
- `templates[].lexicon_terms` must reference real terms from the known vocabulary (`crates/hkask-templates/src/vocabulary.rs`)
- **`type` in `manifest.yaml` is correct as `FlowDef`** — this is NOT the same as `template_type` in `.j2` frontmatter (see §3.3)

### 3.3 .j2 Template Structure

Each `.j2` file has a TOML-format frontmatter block followed by `---` and Jinja2 template content:

```toml
[inference]
template_type: WordAct    # WordAct | KnowAct (NOT FlowDef in .j2)
lexicon_terms:
- term1
- term2
contract:
  input:
    field_name: string
    field_name: object
  output:
    result_field: string
energy_cap: 5120
visibility: Public
---
{# Template: <skill-name>/<template-id>.j2 #}
{# WordAct — Description of what this template does #}
{# ℏKask v0.27.0 #}

You are...
```

### 3.4 template_type Discipline — CRITICAL

This is the most common skill design error. The rules are:

| File type | Correct `template_type` values | Prohibited values |
|-----------|-------------------------------|-------------------|
| `manifest.yaml` `type:` field | `WordAct`, `KnowAct`, `FlowDef` | `Cognition`, `Prompt`, `Process`, `DDMVSS`, `Compositor` |
| `.j2` frontmatter `template_type:` | `WordAct`, `KnowAct` | `FlowDef`, `DDMVSS`, `Cognition`, `Prompt`, `Process`, `Compositor` |

**The DDMVSS aliases are specification-only.** `Cognition`→`KnowAct`, `Prompt`→`WordAct`, `Process`→`FlowDef` are conceptual mappings from the DDMVSS model. They must **never** appear in `.j2` frontmatter or `manifest.yaml` `type:` fields. This is Constraint #3 of the dual-layer model.

**FlowDef goes in manifests, not `.j2` files.** A `.j2` file with `template_type: FlowDef` is malformed. FlowDef templates live in `manifest.yaml` as orchestration manifests that reference WordAct/KnowAct `.j2` files by id.

**How to classify a template:**

| If the template... | template_type is... |
|--------------------|---------------------|
| Produces text or structured output for an agent action | `WordAct` |
| Reasons, classifies, evaluates, or decides | `KnowAct` |
| Orchestrates other templates (only in `manifest.yaml`) | `FlowDef` |

### 3.5 Visibility (P11 — Digital Public/Private Sphere)

P11 governs what is discoverable and loadable. The **only** canonical values are `Public` and `Private`:

| Value | Meaning | When to use |
|-------|---------|-------------|
| `Public` | Discoverable by all agents and users | Default for shared skills, templates in the registry |
| `Private` | Only discoverable by the owning replicant/namespace | Internal experiments, personal workflows, unreleased skills |

**`Shared` is a deprecated runtime synonym.** All manifests and `.j2` files now use canonical `Public`/`Private`. `Shared` must not appear in any new template.

**Visibility appears in two places per template:**
1. The `manifest.yaml` entry (if the template has one)
2. The `.j2` frontmatter contract

Both must agree on the canonical value.

### 3.6 Lexicon Grounding

All `lexicon_terms` must reference real terms defined in the known vocabulary (`crates/hkask-templates/src/vocabulary.rs`). Template IDs should use the `namespace/action` naming convention (e.g., `coding-guidelines/guidelines-assess`).

---

## 4. Registration in bootstrap-registry.yaml

After creating a registry template directory, add entries to `registry/templates/bootstrap-registry.yaml`:

```yaml
- id: wordact/<skill-name>-<action>
  template_type: WordAct
  name: "Human-readable name"
  lexicon_terms:
    - relevant
    - terms
  description: "What this template does"
  source_path: registry/templates/<skill-name>/<file>.j2
  required_capabilities: []
  cascade_level: 0
  matroshka_limit: 7
```

**Bootstrap registry rules:**
- `id` format: `wordact/` or `knowact/` prefix, then kebab-case name
- `template_type`: `WordAct`, `KnowAct`, or `FlowDef`
- `source_path`: relative from workspace root
- `cascade_level`: always 0 at bootstrap (runtime sets this)
- `required_capabilities`: always `[]` at bootstrap

**Verification:**
```bash
# Check for drift — every template directory should have a bootstrap entry
diff <(ls -d registry/templates/*/ | sed 's|registry/templates/||;s|/||') \
     <(grep "source_path:" registry/templates/bootstrap-registry.yaml | \
       sed 's|.*templates/||;s|/.*||' | sort -u)
```

---

## 5. CNS Span Grounding

Every manifest must declare its CNS span for observability. The canonical span registry is `crates/hkask-types/src/cns.rs` (`CnsSpan`).

### 5.1 Manifest CNS Declaration

```yaml
manifest:
  id: my-skill-v1
  name: "My Skill"
  description: "..."
  version: "0.27.0"
  visibility: Public
  cns_span: cns.skill.my-skill    # Must match a canonical span
```

### 5.2 SKILL.md CNS References

SKILL.md files should reference CNS spans in their debug/troubleshooting sections:

```markdown
## Debug

- CNS spans: `cns.tool.condenser` for tool invocation governance
- Check `kask cns health` for current CNS state
```

**Common errors:**
- Using `kask /status` instead of `kask cns health` (hallucinated CLI command)
- Wildcard spans like `cns.*` — use concrete canonical names only
- Forgetting the `cns.` prefix

---

## 6. Testing

### 6.1 Schema Validation Test

The test in `crates/hkask-templates/tests/yaml_schema_validation.rs` validates every `registry/manifests/*.yaml` file. It checks:

- `manifest.id` is non-empty
- `manifest.name` is non-empty
- `manifest.version` is non-empty
- `manifest.visibility` is `Public` or `Private` (P11)

Run it:
```bash
cargo test -p hkask-templates yaml_schema_validation
```

**Before committing a new manifest:**
1. Ensure all required fields are present
2. Ensure `visibility` is `Public` or `Private`
3. Run the test locally

### 6.2 Contract Completeness

If a `.j2` template declares a `contract.input`/`contract.output`, the manifest should reference these types consistently. The contract-audit script checks for this:

```bash
scripts/contract-audit.sh --summary
```

### 6.3 CNS Health

After adding a new skill with CNS spans:
```bash
kask cns health
```

---

## 7. Bundles — Composing Skills

A **bundle** is a curated composition of already-active primary skills. It is NOT a first-class template type and does NOT replace FlowDef.

### 7.1 When to Create a Bundle

| Condition | Action |
|-----------|--------|
| Multiple skills compose a coherent workflow | Create a bundle manifest in `registry/manifests/` |
| A single skill needs orchestration of its own templates | Use a FlowDef in the skill's `.j2` — do NOT create a bundle |
| Every constituent skill scores ≥ 0.8 | Bundle can be created |
| Some constituents are uncalibrated (< 0.8) | Calibrate first, then bundle |

### 7.2 Bundle Manifest Location

Bundles live in `registry/manifests/`, NOT in `registry/templates/`. They are `BundleManifest` structs, not `RegistryEntry` structs.

Example: `registry/manifests/kata-pattern.yaml` bundles `kata-starter`, `kata-improvement`, and `kata-coaching`.

---

## 8. Common Pitfalls

| # | Pitfall | Symptom | Fix |
|---|---------|---------|-----|
| 1 | `template_type: FlowDef` in a `.j2` file | Malformed template — FlowDefs go in manifests only | Change to `WordAct` or `KnowAct`, or move to manifest |
| 2 | DDMVSS aliases (`Cognition`, `Prompt`, `Process`) in frontmatter | Invalid type — not recognized by the runtime | Use canonical `KnowAct`, `WordAct`, `FlowDef` |
| 3 | `visibility: Shared` in manifests or `.j2` contracts | Deprecated synonym — P11 requires canonical values | Replace with `Public` (or `Private` if explicitly private) |
| 4 | Creating a manifest for a Type 1/2 skill | Premature bundling (F5 bug pattern) | Delete the manifest; the SKILL.md alone is sufficient |
| 5 | Orphan manifest (manifest.yaml exists but no SKILL.md and no .j2 files) | "Orphan" — unreachable at runtime, maintenance burden | Either add the missing layer or delete the manifest |
| 6 | Forgetting to register in bootstrap-registry.yaml | Template directory exists but runtime can't find it | Add entries to `bootstrap-registry.yaml` and verify with diff |
| 7 | Hallucinated CNS spans or CLI commands | CNS health check fails; agents get wrong diagnostics | Use only canonical spans from `cns.rs`; use `kask cns health` not `kask /status` |
| 8 | `type: FlowDef` in `manifest.yaml` misinterpreted as prohibited | Confusion with `.j2` `template_type` rule | `manifest.yaml` `type: FlowDef` is correct; only `.j2` `template_type: FlowDef` is prohibited |

---

## 9. Lifecycle Checklist

### New Skill: From Zero to Production

1. **Classify the skill type** (§1.1) — Type 1/2/3/4?
2. **Write SKILL.md** (§2) — if Type 1, 2, or 4
3. **Create registry directory** (§3.1) — if Type 3 or 4
4. **Write manifest.yaml** (§3.2) — correct crate name, version, template types
5. **Write .j2 templates** (§3.3) — correct `template_type` (WordAct/KnowAct), no DDMVSS aliases
6. **Set visibility** (§3.5) — `Public` or `Private` (not `Shared`)
7. **Ground CNS spans** (§5) — manifest `cns_span` field, canonical names
8. **Register in bootstrap-registry.yaml** (§4) — add entries for each template
9. **Run schema validation** (§6.1) — `cargo test -p hkask-templates yaml_schema_validation`
10. **Run CNS health** (§6.3) — `kask cns health`
11. **Commit** with message: `feat(skills): add <skill-name> — <one-line purpose>`

### Existing Skill: Maintenance

1. **Check template_type correctness** — no DDMVSS aliases, no `FlowDef` in `.j2`
2. **Verify visibility** — `Public`/`Private` only
3. **Check registration** — bootstrap-registry.yaml is current (no drift)
4. **Run schema validation** after any manifest change
5. **Run CNS health** after any span change

---

## 10. Architecture Diagram

```mermaid
flowchart TD
    A[Skill Design Request] --> B{Classification}
    B -->|Type 1/2| C[SKILL.md only]
    B -->|Type 3| D[Registry layer only]
    B -->|Type 4| E[Both layers]

    C --> F[Write SKILL.md with frontmatter]
    F --> G[Ground CNS spans]

    D --> H[Create registry/templates/&lt;name&gt;/]
    H --> I[Write manifest.yaml]
    H --> J[Write .j2 templates]
    I --> K[Validate template types]
    J --> K
    K --> L[Set visibility: Public/Private]

    E --> F
    E --> H

    G --> M[Register in bootstrap-registry.yaml]
    L --> M
    M --> N[Run yaml_schema_validation]
    N --> O[Run kask cns health]
    O --> P[Commit]
```

---

## References

- [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) — P1–P12 architecture principles
- [AGENTS.md](../../AGENTS.md) — Agent operating guide and prohibitions
- [CNS Domain Specification](../architecture/core/CNS-DOMAIN-SPECIFICATION.md) — CNS span registry and health checks
- [Testing Discipline](../architecture/core/TESTING_DISCIPLINE.md) — Contract testing and REQ tagging
- `crates/hkask-types/src/cns.rs` — Canonical CNS span definitions
- `crates/hkask-templates/src/vocabulary.rs` — Canonical vocabulary terms
