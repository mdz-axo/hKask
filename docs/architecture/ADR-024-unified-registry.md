---
title: "ADR-024: Unified Registry Decision"
audience: [architects, developers]
last_updated: 2026-05-29
version: "0.27.0"
status: "Active"
domain: "Technology"
ddmvss_categories: [composition]
---

# ADR-024: Unified Registry Decision

**Date:** 2026-05-29 (retroactive)  
**Status:** Implemented  
**Supersedes:** N/A (captures implicit decision from v0.21.0)

## Context

The hKask template system needs to register, discover, and dispatch templates. Three domain contexts — WordAct, FlowDef, KnowAct — each produce templates. The architecture faced a choice between three separate registries (one per domain) or a single unified registry with a type discriminator.

## Decision

**Single unified registry with `template_type` discriminator.**

```rust
pub enum TemplateType {
    WordAct,  // "Say" — prompt templates
    KnowAct,  // "Think" — reasoning templates
    FlowDef,  // "Do / Define" — workflow & specification templates
}
```

The registry stores all three types in a single SQLite table with `template_type` as a column. Discovery, search, and cascade all operate on the unified index. The type discriminator provides domain-specific filtering without requiring separate indices.

## Rationale

1. **Single source of truth.** The registry is the loom. Templates are the thread. Having one index means one bootstrap sequence, one cache, one query path.

2. **Fowler registry pattern.** [^fowler-poeaa] A single registry with a type discriminator is simpler than three separate registries with cross-references. The `RegistryIndex` trait exposes `list(domain_hint: Option<TemplateType>)` — one method serves all domains.

3. **Cascade requires unified view.** Template cascade (matroshka nesting) crosses domains: a FlowDef template may invoke a WordAct template, which may reference a KnowAct template. A unified registry makes cross-domain resolution a single lookup.

4. **Constraint compliance.** Three separate registries would create three traits with one consumer each — violating P1 (no trait without two consumers). The unified `RegistryIndex` trait has two consumers (`Registry` and `SqliteRegistry`).

5. **hLexicon grounding.** All templates reference hLexicon terms. Unified search by lexicon term works across all template types.

## Consequences

### Positive

- Single bootstrap path: `Database → hLexicon → Registry`
- Cross-domain cascade resolution is a single lookup
- P1 compliance (two `RegistryIndex` implementations)
- hLexicon search works across all template types

### Negative

- `template_type` discriminator adds a filter parameter to list methods

### Alternative Rejected

**Three separate registries** (one per domain) would require:
- Three separate SQLite tables or databases
- Three separate bootstrap steps
- Cross-domain cascade requires cross-registry resolution
- Violates P1 (three traits, one consumer each)

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (No trait without two consumers) | ✅ `RegistryIndex` used by `Registry` and `SqliteRegistry` |
| P4 (No builder without fallibility) | ✅ `SqliteRegistry::new()` is fallible |
| C4 (Repetition is missing primitive) | ✅ Unified registry eliminated domain-specific repetition |

## References

[^fowler-poeaa]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture*. Addison-Wesley. Registry pattern (pp. 490–494).

---

*ℏKask - A Minimal Viable Container for Agents — ADR-024 — v0.21.0*
