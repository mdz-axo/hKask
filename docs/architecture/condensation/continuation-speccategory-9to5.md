---
title: "Condensation Continuation — SpecCategory Enum 9→5"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Deferred"
domain: "Architecture"
mds_categories: [domain, lifecycle]
---

# Condensation Continuation — SpecCategory Enum 9→5

**Status:** ✅ Complete (2026-06-09). The Rust `SpecCategory` enum was already collapsed to 5 MDS categories. Audit confirmed zero remaining DDMVSS references in code.

---

## Background

The DDMVSS→MDS migration updated all documentation (categories, tools, completeness predicate, template manifests) but did not update the Rust code. The `SpecCategory` enum, `Spec` struct, `SpecStore` trait, and `SqliteSpecStore` implementation all still reference 9 DDMVSS categories.

## Current State

### `SpecCategory` enum (hkask-storage/src/spec_types.rs)

```rust
pub enum SpecCategory {
    Domain,
    Capability,      // → Composition
    Interface,       // → Composition
    Composition,
    Trust,
    Observability,   // → Lifecycle
    Persistence,     // → Lifecycle
    Lifecycle,
    Curation,
}
```

### Target State

```rust
pub enum SpecCategory {
    Domain,
    Composition,     // Absorbs Capability + Interface + Composition
    Trust,
    Lifecycle,       // Absorbs Observability + Persistence + Lifecycle
    Curation,
}
```

## Impact Map

| Crate | Affected Types | Change |
|-------|---------------|--------|
| `hkask-storage` | `SpecCategory` enum, `Spec` struct, `SpecStore` trait, `SqliteSpecStore` | Remove 4 variants, update match arms |
| `hkask-storage` | `SpecCurationRecord`, `GoalSpec`, `CompletenessDomain`, `DriftReport` | Update category references |
| `hkask-services::spec` | Spec capture/validate/cultivate functions | Update category parameters |
| `hkask-services::verification` | Magna Carta verification | May reference spec categories |
| `hkask-agents` | `DefaultSpecCurator`, `SpecCurator` trait | Update category parameters |
| `hkask-mcp-spec` | Tool implementations | Update category parameters in tool dispatch |
| `hkask-cli` | `kask spec` commands | Update category flags/options |
| `hkask-api` | Spec routes | Update category parameters |
| Tests | All spec-related tests | Update expected categories |

## Approach

### Phase 1 — Audit

1. Find every reference to `SpecCategory::Capability`, `SpecCategory::Interface`, `SpecCategory::Observability`, `SpecCategory::Persistence`
2. Map each reference to the correct MDS category
3. Identify match arms that group multiple old categories (e.g., `Capability | Interface | Composition` → simply `Composition`)

### Phase 2 — Collapse (Strangler Fig)

1. Add new MDS variants to `SpecCategory` alongside old DDMVSS variants
2. Mark old variants with `#[deprecated]` 
3. Update match arms to handle both old and new variants
4. Migrate callers one by one to use new variants
5. Remove old variants
6. Run `cargo check --workspace && cargo test --workspace` at each step

### Phase 3 — Backward Compatibility

Existing spec data in SQLite may use old category strings. The `SpecCategory` serialization (serde) must handle both old and new names:
- Old `"Capability"` → deserialize as `Composition`
- Old `"Interface"` → deserialize as `Composition`
- Old `"Observability"` → deserialize as `Lifecycle`
- Old `"Persistence"` → deserialize as `Lifecycle`

### Phase 4 — Verify

1. Run `cargo check --workspace && cargo test --workspace`
2. Verify spec capture with new categories
3. Verify spec query with new categories
4. Verify existing spec data deserializes correctly

## Risks

1. **SQLite data migration:** Existing specs stored with old category names must deserialize correctly. Serde aliases should handle this without data migration.
2. **Spec server tests:** `hkask-mcp-spec` tests reference old category names. These tests currently have 1 pre-existing failure — fix both issues together.
3. **Blast radius:** `SpecCategory` is referenced across 6+ crates. The collapse must be systematic.

## Verification

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
# Verify SpecCategory has exactly 5 variants
# Verify existing spec data deserializes with old category names
# Verify spec capture works with new category names
```

## Predecessor Tasks

- [x] MDS specification (5 categories documented)
- [x] Documentation cleanup (DDMVSS→MDS)
- [x] MDS tools renamed (bind deleted, coherence renamed)

---

*This continuation prompt captures all context needed for the SpecCategory enum collapse.*
