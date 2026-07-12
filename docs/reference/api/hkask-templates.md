---
title: "hkask-templates — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-templates` is the template registry and manifest execution engine. It provides the unified registry for WordAct (Jinja2 prompts), KnowAct (cognition templates), and FlowDef (YAML pipeline manifests). Rust is the loom; YAML/Jinja2 is the thread.

## Public Modules

| Module | Purpose |
|---|---|
| `bundle` | `BundleManifest` and `BundleRegistryIndex` — composed skill bundles |
| `capability_validator` | Validates capability requirements in manifests |
| `crate_loader` | `TemplateCrateLoader` — loads templates from crate directories |
| `executor` | `ManifestExecutor` — deterministic multi-step cascade execution |
| `manifest_loader` | YAML manifest loading and resolution (`load_manifest_from_yaml`, `resolve_manifest`) |
| `ports` | Port traits: `McpPort`, `TemplateError`, `Result` |
| `prompt_strategy` | `PromptStrategy` for template-driven prompt construction |
| `registry` | In-memory `Registry` (read-through cache over `SqliteRegistry`) |
| `registry_sqlite` | `SqliteRegistry` — persistent SQLite-backed registry |
| `skill_loader` | `SkillLoader`, `SkillFrontMatter`, `SkillLoadResult` — two-zone skill discovery |
| `vocabulary` | Known vocabulary terms bootstrapped from manifest `lexicon_terms` |

## Key Types

### `Registry`

Thin in-memory wrapper (read-through cache) around `SqliteRegistry`. Implements three index traits: `RegistryIndex`, `SkillRegistryIndex`, and `BundleRegistryIndex`. Stores templates as `RegistryEntry`, skills as `Skill`, and bundles as `BundleManifest` in `HashMap`s. Constructed via `Registry::new()`; provides `invalidate_cache()` for hot-reload.

### `SqliteRegistry`

Persistent SQLite-backed registry implementing `RegistryIndex`, `SkillRegistryIndex`, and `BundleRegistryIndex`. Used in tandem with the in-memory `Registry` for fast lookups with durability. Supports `register_bundle()` to persist composed skill bundles.

### `ManifestExecutor`

Deterministic multi-step orchestrator that executes a `BundleManifest` cascade: select → populate → execute. Dispatches each `BundleManifestStep` by its `action` field:

- **select**: Render a selector template, call inference, parse JSON to choose the next step.
- **populate**: Render a template with accumulated context map, producing a filled prompt.
- **execute**: Invoke an MCP tool with parameters bound from the context map.
- **choice**: Evaluate a condition against context, branch by setting `_next_ordinal`.
- **loop**: Re-enter cascade from `loop_target` ordinal, incrementing iteration counter. Bounded by the matryoshka depth limit (7).
- **abort**: Exit cascade with a convergence status. Emits `cns.skill.converged`.
- **escalate**: Exit cascade with an escalation error. Emits `cns.skill.escalated`.

Respects iterative convergence (`manifest.convergence`), gas budgets (`manifest.gas.cap` with per-token deduction), timeout constraints (`step.timeout_seconds` enforced via `tokio::time::timeout`), and conditional step execution (`step.condition`). Template rendering supports two modes: **minijinja** (load and render full Jinja2 templates from file) and **inline** (simple `{{key}}` substitution).

### `SkillLoader`

Discovers, parses, and registers SKILL.md files from two zones (private `.agents/skills/` and public `skills/`). Parses YAML front matter into `SkillFrontMatter`, validates zone-vs-visibility consistency, and registers skills via `SkillRegistryIndex`.

### `SkillFrontMatter`

Parsed SKILL.md front matter. Fields: `name` (`String`), `visibility` (`Option<String>`), `namespace` (`Option<String>`), `description` (`Option<String>`). All fields default to empty.

### `SkillLoadResult`

Result of loading skills from both zones. Fields: `loaded` (`Vec<Skill>`) and `warnings` (`Vec<String>`).

### `BundleManifest`

Composed skill bundle manifest supporting recursive composition via `BundleManifestStep` entries, gas budgets, and convergence thresholds. Indexed via `BundleRegistryIndex`.

## Key Functions

| Function | Signature | Purpose |
|---|---|---|
| `load_manifest_from_yaml` | `(yaml: &str) -> Result<BundleManifest, ManifestLoadError>` | Parse a YAML string into a `BundleManifest` |
| `resolve_manifest` | See `manifest_loader` module | Resolve and validate manifest references |

## Key Re-exports

| Re-export | Source |
|---|---|
| `InferencePort` | `hkask_ports::InferencePort` |
| `Skill` | `hkask_ports::Skill` |
| `SkillZone` | `hkask_ports::SkillZone` |
| `SkillPolarity` | `hkask_types::SkillPolarity` |
| `PromptStrategy` | `crate::prompt_strategy::PromptStrategy` |
| `TemplateError` | `crate::ports::TemplateError` |
| `McpPort` | `crate::ports::McpPort` |

## Error Types

### `TemplateError`

Defined in `crate::ports`. Returned by registry operations, manifest loading, and executor lifecycle.

### `ManifestLoadError`

Returned by `load_manifest_from_yaml` when YAML parsing or validation fails.

### Type Alias: `Result<T>`

`std::result::Result<T, TemplateError>` — the standard result type for template operations.

## Feature Flags

No feature flags are defined. This crate is a core dependency.

## Vocabularies

The `vocabulary` module defines `KNOWN_TERMS`, a bootstrapped lexicon of action verbs (e.g., "analyze", "audit", "classify", "compose", "critique", "decompose", "synthesize") sorted alphabetically for binary-search lookup. Terms are drawn from manifest `lexicon_terms` across the skill corpus.
