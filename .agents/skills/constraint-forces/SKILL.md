---
name: constraint-forces
visibility: public
description: "DEPRECATED — merged into pragmatic-semantics. Use `pragmatic-semantics` for constraint classification (Prohibition/Guardrail/Guideline/Evidence/Hypothesis), IS/OUGHT distinction, provenance tracing, and conflict resolution. The pragmatic-semantics skill provides the full epistemic framework plus CNS span emission for constraint overrides."
---

# Constraint Forces — DEPRECATED

This skill has been merged into **pragmatic-semantics**.

The pragmatic-semantics skill provides the full constraint force hierarchy (Prohibition → Hypothesis), IS/OUGHT classification, conflict resolution via Optimality Theory ranking, provenance tracing, temporal semantics, and CNS span emission for overrides.

**Use `kask run pragmatic-semantics` for all constraint classification and conflict resolution.**

## Migration

| Old | New |
|-----|-----|
| `kask run constraint-forces` | `kask run pragmatic-semantics` |
| classify → force type | `semantics-classify-statement.j2` (adds epistemic axis) |
| resolve → force ranking | `semantics-conflict-resolve.j2` (adds provenance weighting) |
| 5-force hierarchy | Available in pragmatic-semantics §Constraint Hierarchy |
| Magna Carta P1-P4 mapping | Available in pragmatic-semantics §Constraint Hierarchy |
