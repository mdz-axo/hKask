---
title: "ADR-026: Bitemporal Triple Schema"
audience: [architects, database developers]
last_updated: 2026-05-29
version: "1.0.0"
status: "Active"
domain: "Technology"
ddmvss_categories: [persistence]
---

# ADR-026: Bitemporal Triple Schema

**Date:** 2026-05-29 (retroactive)  
**Status:** Implemented  
**Supersedes:** N/A

## Context

hKask stores knowledge as subject-predicate-object triples. A simple triple store records the current state but loses history. For an agent platform where learning is incremental and provenance is critical, the system must track both *when a fact was true in the domain* (valid time) and *when we learned about it* (transaction time).

## Decision

**Bitemporal triple schema** with valid time, transaction time, confidence, and observer identity.

```sql
CREATE TABLE triples (
    id          TEXT PRIMARY KEY,
    subject     TEXT NOT NULL,
    predicate   TEXT NOT NULL,
    object      TEXT NOT NULL,
    confidence  REAL DEFAULT 1.0,
    valid_from  TEXT NOT NULL,
    valid_to    TEXT,           -- NULL = still valid
    tx_from     TEXT NOT NULL DEFAULT (datetime('now')),
    tx_to       TEXT,           -- NULL = current record
    observer_id TEXT NOT NULL,
    source      TEXT            -- provenance reference
);
```

## Rationale

1. **Snodgrass bitemporality.** [^snodgrass] Two time dimensions capture both domain truth and knowledge acquisition. This enables queries like "What did agent X know about Y on date Z?" and "When did we learn that fact W was no longer true?"

2. **Episodic/semantic split.** Episodic memory is scoped to an observer — you query triples by `observer_id` for private, agent-specific history. Semantic memory ignores `observer_id` for public, shared knowledge. The same schema serves both pipelines.

3. **Confidence as first-class.** Every triple carries a Bayesian confidence [0.0, 1.0]. Multiple observers can assert the same fact with different confidences. The combined confidence follows Bayesian updating — independent evidence increases confidence.

4. **Provenance chain.** The `source` field references the origin of the fact — template invocation ID, inference call, human annotation. Combined with `observer_id`, this forms a complete provenance trail.

5. **Immutable history.** Rows are never updated — new knowledge creates a new row with `tx_from` = now and the previous row gets `tx_to` = now. Git history provides an additional layer of immutability for critical audit paths.

## Consequences

### Positive

- Full audit trail: "who knew what when"
- Episodic/semantic split uses the same schema, different query filters
- Bayesian confidence tracking
- Append-only = crash-safe (no in-place updates)

### Negative

- Storage grows monotonically (append-only)
- Two time dimensions require careful query design
- Confidence combination requires explicit Bayesian math

### Alternative Rejected

**Simple triple store without time dimensions** loses all history and provenance. Not suitable for an agent platform where learning correctness depends on knowing the order and timing of knowledge acquisition.

## Compliance

| Principle | Compliance |
|-----------|-----------|
| C5 (Every error variant is unique) | ✅ `TripleError` has distinct variants for schema, confidence, time parse |
| P3 (No module directory without encapsulation) | ✅ `triples.rs` encapsulates all triple semantics |

## References

[^snodgrass]: Snodgrass, R. T. (1999). *Developing Time-Oriented Database Applications in SQL*. Morgan Kaufmann.

---

*ℏKask - A Minimal Viable Container for Agents — ADR-026 — v0.21.0*
