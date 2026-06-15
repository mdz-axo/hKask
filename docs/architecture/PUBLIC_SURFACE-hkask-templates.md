---
title: "Public Surface Justification — hkask-templates"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-templates

**Crate:** `hkask-templates`  
**Public items in lib.rs:** 22  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-templates` is the **template registry and execution crate** — YAML/Jinja2 template management with SQLite persistence. Its surface is large because it spans registry, validation, and rendering:

1. **Registry adapters** — In-memory `Registry` and `SqliteRegistry` with full CRUD, search, and skill/bundle management.
2. **Template types** — `TemplateType` (WordAct, FlowDef, KnowAct), `RegistryEntry`, `RegistryIndex` trait.
3. **Validation** — `CapabilityAwareValidator` (OCAP enforcement), `ContractValidator` (lexicon compliance).
4. **Lexicon** — hLexicon vocabulary management with markdown→YAML pipeline.
5. **Rendering** — Jinja2 template rendering with sandbox enforcement.

## Mitigations

- **Adapter pattern:** `RegistryIndex` trait enables swapping between in-memory and SQLite backends.
- **Separate validation:** Capability and contract validation are independent modules.

## Deletion Test

Delete `hkask-templates` and template registration, SQLite persistence, capability validation, lexicon management, and Jinja2 rendering reappear scattered across CLI, API, and MCP servers. The crate earns its existence.
