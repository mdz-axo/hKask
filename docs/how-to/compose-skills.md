---
title: "How to Compose Skills — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Compose Skills

hKask's `BundleService` in `crates/hkask-services-skill/src/bundle.rs` composes multiple skills into a coordinated `BundleManifest` using inference-driven analysis. This guide covers creating bundles, understanding cascade ordering, and testing composed skills.

## Prerequisites

Skill composition requires:
- A running inference port (`Arc<dyn InferencePort>`)
- Skills registered in the `SkillRegistryIndex`
- An `AgentService` context with initialized storage

## Creating a Bundle

Call `BundleService::compose()` with at least 2 skill IDs:

```rust
use hkask_services_skill::bundle::BundleService;
use hkask_types::Visibility;

let result = BundleService::compose(
    &ctx,
    &["coding-guidelines".into(), "idiomatic-rust".into()],
    Some("rust-review-bundle"),
    Visibility::Shared,
    inference_port,
    "alice",
).await?;

println!("Bundle: {} — {} skills, {} steps",
    result.manifest.id,
    result.manifest.skills.len(),
    result.manifest.steps.len()
);
for warning in &result.warnings {
    println!("Warning: {}", warning);
}
```

### What Happens During Composition

The `compose()` method performs these steps:

1. **Resolve skills** from the registry — validates that all skill IDs exist
2. **Smart deduplication** — checks if a bundle with these exact skills already exists; returns it with a warning if so
3. **Polarity classification** — each skill is classified as Generative, Evaluative, Regulative, or Procedural
4. **Conflict detection** — identifies conflicting skill polarities and declares resolutions
5. **Complementarity identification** — finds skills that enhance each other
6. **Phase separation** — skills are organized into Pre → Core → Post cascade phases
7. **Inference-driven manifest generation** — the LLM produces a `BundleManifest` JSON
8. **Validation** — the manifest passes through `BundleManifest::validate()`
9. **Registration** — the bundle is stored in `BundleRegistryIndex`

## Cascade Ordering

The composition prompt enforces these ordering rules:

- **Divergent (Generative) and convergent (Evaluative) skills must not share a phase**
- **Cascade depth must not exceed 7**
- **At least one Procedural (productive) skill is required**
- Each skill may have **≤10 lexicon terms**
- A **convergence criterion** must be declared

The resulting manifest's `steps` array carries `phase` (Pre/Core/Post), `ordinal`, `action`, `gas_cap`, and `timeout_seconds` for each step in the cascade.

## Evolving a Bundle

When skills change, re-compose the bundle:

```rust
let evolved = BundleService::evolve(&ctx, "rust-review-bundle", inference_port, "alice").await?;
// Old bundle is removed, new one registered
```

## Listing and Applying Bundles

```rust
// List all bundles
let bundles = BundleService::list(&ctx).await?;

// Get a specific bundle
let bundle = BundleService::get(&ctx, "rust-review-bundle").await?;

// Apply a bundle to the current session
let manifest = BundleService::apply(&ctx, "rust-review-bundle").await?;

// Deactivate (no-op — bundles are session-scoped)
BundleService::deactivate()?;
```

## Testing Composed Skills

Validate that composed bundles produce correct behavior:

1. **Check warnings**: `result.warnings` contains zone-visibility mismatches and composition notes
2. **Verify validation**: If `manifest.validate().is_valid()` fails, inspect `validation.errors`
3. **Test cascade execution**: Run each step in order and verify outputs match expectations
4. **Check idempotency**: Composing the same skill set twice should return the existing bundle

## Bundle Manifest Structure

The resulting `BundleManifest` contains:
```json
{
  "id": "rust-review-bundle",
  "name": "Rust Review Bundle",
  "description": "Reviews Rust code against idiomatic patterns and coding guidelines",
  "version": "1.0.0",
  "editor": "alice",
  "visibility": "shared",
  "skills": [
    {"id": "coding-guidelines", "polarity": "regulative", "lexicon_terms": [...], ...},
    {"id": "idiomatic-rust", "polarity": "evaluative", "lexicon_terms": [...], ...}
  ],
  "conflicts": [...],
  "complementarities": [...],
  "steps": [
    {"ordinal": 1, "action": "analyze", "phase": "pre", "gas_cap": 2000, ...},
    {"ordinal": 2, "action": "review", "phase": "core", "gas_cap": 5000, ...},
    ...
  ]
}
```
