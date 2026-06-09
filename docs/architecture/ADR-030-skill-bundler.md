---
title: "ADR-030: Skill Bundler — Meta-Skill Composition"
audience: [architects, developers]
last_updated: 2026-06-04
version: "0.27.0"
status: "Proposed"
domain: "Technology"
mds_categories: [composition]
---

# ADR-030: Skill Bundler — Meta-Skill Composition

**Date:** 2026-06-04  
**Status:** Proposed  
**Supersedes:** N/A  
**Related:** ADR-024 (Unified Registry), ADR-025 (Attenuation Depth Limit), PRINCIPLES §1.5 (Composition)

## Context

hKask skills operate independently, but users frequently need multiple skills active simultaneously (e.g., `coding-guidelines` + `grill-me` for a coding review session). Without composition, skills conflict, overlap, or produce incoherent output. The skill-bundler introduces a meta-skill pattern: a FlowDef manifest that composes multiple skills into a coherent cascade with declared conflicts, complementarities, and phase assignments.

The core problem is combinatorial: two skills may issue contradictory directives (e.g., `constrain` in one vs. `decompose` in another), occupy the same cascade phase with opposite polarity, or amplify each other beyond the system's energy budget. Individual skills have no visibility into each other's lexicon signatures, polarity, or phase expectations — composition is a cross-cutting concern that requires a first-class type.

## Decision

We introduce `BundleManifest` as a first-class type in `hkask-types`, persisted in the registry alongside templates and skills. The bundler operates through three Jinja2 templates (`compose`, `apply`, `evolve`) and is accessible via CLI (`kask bundle`), slash commands (`/bundle`), and API (`/api/v1/bundles`).

### Key Architectural Decisions

**1. `BundleManifest` is a Rust type, not just YAML.** This ensures type safety, validation, and SQLite persistence — consistent with how `TemplateType`, `FlowDef`, and `KnowAct` are all first-class Rust types in `hkask-types`.

```rust
pub struct BundleManifest {
    pub id: BundleId,
    pub name: String,
    pub skills: Vec<SkillRef>,
    pub conflicts: Vec<ConflictDeclaration>,
    pub complementarities: Vec<ComplementarityDeclaration>,
    pub cascade: Vec<CascadePhase>,
    pub visibility: Visibility,
    pub content_hashes: HashMap<SkillId, ContentHash>,
    pub energy_budget: Option<EnergyBudget>,
}
```

**2. Phase separation is enforced at validation time.** P1 violation (divergent + convergent skills in the same cascade phase) is a hard error. `BundleManifest::validate()` rejects bundles that place generative and evaluative skills in the same phase — the Double Diamond pattern[^double-diamond] is structural, not advisory.

**3. Smart matching prevents duplicate bundles.** Before composing, the registry checks for existing bundles with the exact same skill set. Users can apply, evolve, or compose anew — but never accidentally create redundant manifests.

**4. Content hashes on skills enable evolution tracking.** When a skill's manifest changes, its hash changes, triggering re-composition. Stale bundles are detected, not silently applied.

**5. `BundleManifest::validate()` checks:** minimum 2 skills, matroshka depth ≤ 7, max 10 terms per skill, max 30 unique terms per bundle, P1 phase separation, conflict/complementarity reference integrity, ordinal sequencing.

**6. Compose template outputs are LLM-generated, not Jinja2-rendered.** The `conflicts` and `complementarities` sections are instructions to the LLM, not template variables. The Jinja2 template frames the task; the LLM analyzes skill interactions and produces the structured output. This follows hKask's principle that selection intelligence lives in Jinja2/LLM, not Rust code.[^principles]

**7. The registry stores bundles as JSON in SQLite.** Complex nested structures (conflict declarations, cascade phases, complementarity maps) are serialized rather than normalized across tables. This matches the existing pattern for template storage and avoids join explosion.

**8. CNS spans use namespace `cns.prompt.skill-bundler`** for compose/evolve operations, consistent with the existing `cns.prompt.*` namespace convention.

### Interface Surface

| Interface | Path / Command |
|-----------|---------------|
| CLI | `kask bundle <skill1> [skill2]...`, `kask bundle list`, `kask bundle show <name>`, `kask bundle evolve <name>`, `kask bundle skills` |
| Slash | `/bundle <skill1> [skill2]...`, `/bundle list`, `/bundle off` |
| API | `POST /api/v1/bundles/compose`, `POST /api/v1/bundles/apply`, `GET /api/v1/bundles`, `GET /api/v1/bundles/{id}`, `POST /api/v1/bundles/{id}/evolve`, `DELETE /api/v1/bundles/{id}/active` |

### Composition Principles

| # | Principle | Rationale |
|---|-----------|-----------|
| 1 | **Phase separation** | Divergent and convergent skills never share a cascade phase[^double-diamond] |
| 2 | **Default ordering** | Recognize → Act → Reflect (hLexicon Pattern 2) |
| 3 | **Domain complementarity** | Skills from different hLexicon domains compose more safely than skills from the same domain |
| 4 | **Conflict resolution hierarchy** | Domain separation → Phase separation → Specificity wins → Manifest override → User intent wins |
| 5 | **Convergence criteria** | Every iterative composition declares a convergence criterion; CNS `CalibrationThreshold` provides defaults |
| 6 | **Depth and term limits** | Cascade depth ≤ 7 (matroshka limit), ≤ 10 terms/skill, ≤ 30 unique terms/bundle |
| 7 | **Polarity classification** | Generative → early, Evaluative → late, Regulative → cross-phase, Procedural → backbone |

### Skill Polarity Extension

The `Skill` struct gains two new fields:

```rust
pub struct Skill {
    // ... existing fields ...
    pub polarity: SkillPolarity,    // Generative | Evaluative | Regulative | Procedural
    pub content_hash: ContentHash,  // SHA-256 of skill manifest for evolution tracking
}
```

## Rationale

1. **FlowDef is the composition primitive.** [^adr-024] hKask's unified registry already stores `FlowDef` templates. A skill bundle is itself a FlowDef manifest — the bundler composes other manifests into a single process flow. No new type hierarchy is needed; `BundleManifest` extends `FlowDef` with conflict resolution metadata.

2. **Phase separation prevents cancellation.** [^double-diamond] The Double Diamond model demonstrates that divergent (expansive) and convergent (evaluative) thinking must not overlap. A bundle that places `decompose` and `constrain` in the same phase produces contradictory output. Enforcing this at validation time, not runtime, catches the error before it manifests.

3. **Smart matching avoids redundancy.** Without registry deduplication, users would accumulate near-identical bundles. Checking skill sets against existing manifests is O(n) against the bundle index and prevents the "which of my 12 coding-session bundles is the right one?" problem.

4. **Content hashes enable safe evolution.** [^content-addressing] Skills change. When a skill's manifest drifts from the hash recorded in the bundle, the `evolve` operation re-composes only what changed. This is content-addressing at the skill level — consistent with hKask's Git-based approach to artifact storage.

5. **Matroshka depth limit of 7.** [^adr-025] Bundle composition is a form of nesting. A bundle of bundles (matroshka) follows the same depth limit as capability attenuation — 7 levels, constant-time verification, bounded recursion.

6. **JSON-in-SQLite for complex structures.** [^adr-024] The unified registry already stores templates as JSON blobs in SQLite. Normalizing `ConflictDeclaration`, `ComplementarityDeclaration`, and `CascadePhase` across separate tables would create 4+ join tables for a read-heavy, write-rare access pattern. JSON serialization matches the existing pattern.

7. **LLM-generated composition is intentional.** The conflict and complementarity analysis between skills requires understanding semantic relationships that Jinja2 templates cannot express. The template frames the task; the LLM does the reasoning. This is the same pattern hKask uses for template selection[^principles] — the hard kernel handles parsing and validation; the soft material handles judgment.

## Consequences

### Positive

- Users can compose, apply, evolve, and deactivate skill bundles through CLI, chat, and API
- Bundle validation prevents common composition errors (phase conflicts, term overflow, depth violations) at registration time
- Content hashes detect skill drift and trigger re-composition automatically
- Smart matching prevents duplicate bundles
- The bundler is a meta-skill: its own composition flow IS a FlowDef manifest — no special-case infrastructure
- Polarity classification gives the LLM structured metadata for composition decisions
- CNS integration (`cns.prompt.skill-bundler`) provides observability without external monitoring

### Negative

- `BundleManifest` adds a new type to `hkask-types` and a new storage table to the registry
- The `Skill` struct gains `polarity` and `content_hash` fields — a migration for existing skill manifests
- LLM-generated composition is non-deterministic: the same skill set may produce slightly different manifests on different compose runs
- JSON-in-SQLite means bundle queries are not indexable by internal fields without additional SQLite JSON functions

### Alternative Rejected

**Option A — Skill composition as runtime orchestration (no manifest).** The agent would receive all skill instructions simultaneously and resolve conflicts at inference time. Rejected because: (1) no validation guardrail, (2) no persistent composition to evolve, (3) no audit trail, (4) contradicts hKask's design principle that structure should be declared, not emergent.

**Option B — Separate `BundleRegistry` type.** A distinct registry for bundles, parallel to the unified template registry. Rejected because it violates ADR-024[^adr-024]: three registries were already rejected in favor of a single unified registry. Bundles join the existing registry as a `template_type` discriminator, not a parallel structure.

## Anti-Patterns

| Anti-Pattern | Detection | Resolution |
|-------------|-----------|------------|
| **Cancel-out** | `constrain` + `decompose` in same phase | Move divergent to pre-core, convergent to post-core |
| **Contradictory directives** | Multiple `command`/`require` for same target | Specificity wins; add `reconcile` step |
| **Ordering collision** | Same domain, same phase, same specificity | Explicit `cascade_order` in manifest |
| **Runaway feedback** | Skill A triggers Skill B triggers Skill A | Convergence criterion + matroshka depth limit (7) |
| **Scope creep** | Term count > 10 per skill or > 30 per bundle | Decompose into sub-bundles |
| **Dead letter** | No productive term (`create`, `pledge`, `instruct`) | Require at least one productive term per skill |

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (No trait without two consumers) | ✅ `BundleManifest` consumed by `Registry` and `SqliteRegistry` |
| P4 (No builder without fallibility) | ✅ `BundleManifest::validate()` is fallible — returns `Result<Self, BundleValidationError>` |
| P5 (No speculative code) | ✅ No bundle composition infrastructure until skills exist to compose |
| P6 (Delete stubs, don't publish them) | ✅ Templates are functional, not stubs |
| C4 (Repetition is missing primitive) | ✅ Skill bundler extracts the repetitive pattern of manually combining skills |
| C5 (Every error variant is unique recovery path) | ✅ `BundleValidationError::{PhaseConflict, TermOverflow, DepthExceeded, HashMismatch, SkillNotFound}` |
| §1.5 (Composition) | ✅ Bundle manifests are first-class registry entries with `template_type` discriminator |
| §1.6 (Headless) | ✅ All interaction via CLI, MCP, or API — no visual UI |

## References

[^double-diamond]: Council, D. (2005). *The Double Diamond: Design Council's Framework for Innovation*. <https://www.designcouncil.org.uk/our-resources/the-double-diamond/>
[^adr-024]: hKask Project. (2026). *ADR-024: Unified Registry Decision*. `docs/architecture/ADR-024-unified-registry.md`
[^adr-025]: hKask Project. (2026). *ADR-025: 7-Level Attenuation Depth Limit*. `docs/architecture/ADR-025-attenuation-depth-limit.md`
[^principles]: hKask Project. (2026). *Architecture Principles*. `docs/architecture/PRINCIPLES.md`
[^content-addressing]: Merkle, R. C. (1987). *A Digital Signature Based on a Conventional Encryption Function*. CRYPTO '87.

---

*ℏKask - A Minimal Viable Container for Agents — ADR-030 — v0.23.0*