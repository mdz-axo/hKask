---
title: "Template Registry — Entity Relationship Diagram"
audience: [data architects, database developers, agents]
last_updated: 2026-06-07
version: "0.23.0"
status: "Active"
domain: "Data"
ddmvss_categories: [persistence]
---

# Template Registry — Entity Relationship Diagram

## Overview

The high-temperature template registry stores templates and their invocations for anti-normative generation. This ERD documents the database schema added to `hkask-storage`.

## Schema

```mermaid
erDiagram
    templates ||--o{ template_invocations : "has many"
    template_invocations ||--o{ curation_records : "evaluated by"
    
    templates {
        string id PK
        string name
        string type
        real temperature_min
        real temperature_max
        text prompt_template
        text input_schema    -- JSON stored as TEXT (SQLite has no native JSONB type)
        text output_schema   -- JSON stored as TEXT
        text constraints     -- JSON stored as TEXT
        timestamp created_at
        timestamp updated_at
    }
    
    template_invocations {
        string id PK
        string template_id FK
        string bot_id
        real temperature
        text parameters       -- JSON stored as TEXT
        text input            -- JSON stored as TEXT
        text outputs          -- JSON stored as TEXT
        integer selected_index
        string outcome
        timestamp timestamp
    }
    
    curation_records {
        string id PK
        string curator_id
        string invocation_id FK
        string decision
        text rationale
        text ocap_boundaries -- JSON stored as TEXT
        timestamp timestamp
    }
    
    cns_variety {
        string id PK
        string span
        integer counter
        integer threshold
        timestamp last_alert
        timestamp updated_at
    }
    

    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-REG-001
verified_date: 2026-06-07
verified_against: crates/hkask-templates/src/registry.rs; crates/hkask-templates/src/registry_sqlite.rs
status: VERIFIED
-->

## Tables

### templates

Stores high-temperature template definitions.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Primary key (UUID string) |
| name | TEXT | Human-readable template name |
| type | TEXT | Template type (WordAct, KnowAct, FlowDef) |
| temperature_min | REAL | Minimum temperature for sampling |
| temperature_max | REAL | Maximum temperature for sampling |
| prompt_template | TEXT | Jinja2 template for prompt rendering |
| input_schema | TEXT | JSON schema for input validation (JSON stored as TEXT) |
| output_schema | TEXT | JSON schema for output validation (JSON stored as TEXT) |
| constraints | TEXT | Template-specific constraints (JSON stored as TEXT) |
| created_at | TIMESTAMP | Creation timestamp |
| updated_at | TIMESTAMP | Last update timestamp |

### template_invocations

Audit trail of template executions.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Primary key (UUID string) |
| template_id | TEXT | Foreign key to templates |
| bot_id | TEXT | Bot that invoked the template |
| temperature | REAL | Actual temperature used |
| parameters | TEXT | Full LLM parameters (JSON stored as TEXT) |
| input | TEXT | Input provided to template (JSON stored as TEXT) |
| outputs | TEXT | Array of generated outputs (JSON stored as TEXT) |
| selected_index | INTEGER | Index of selected output (if merged) |
| outcome | TEXT | Success, Failure, Merged, Discarded |
| timestamp | TIMESTAMP | Invocation timestamp |

### curation_records

Curator evaluation decisions.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Primary key (UUID string) |
| curator_id | TEXT | The Curator's ID |
| invocation_id | TEXT | Foreign key to template_invocations |
| decision | TEXT | Merge, Discard, Revise, Defer |
| rationale | TEXT | Optional explanation |
| ocap_boundaries | TEXT | OCAP boundaries checked (JSON stored as TEXT) |
| timestamp | TIMESTAMP | Decision timestamp |

### cns_variety

Variety counter monitoring for CNS.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Primary key |
| span | TEXT | CNS span (cns.template, cns.curation, etc.) |
| counter | INTEGER | Current variety count |
| threshold | INTEGER | Alert threshold (default: 100) |
| last_alert | TIMESTAMP | Last algedonic alert time |
| updated_at | TIMESTAMP | Last update timestamp |



## Usage Patterns

### Register Template

```sql
INSERT INTO templates (id, name, type, temperature_min, temperature_max, prompt_template)
VALUES (?, ?, ?, ?, ?, ?);
```

### Invoke Template

```sql
INSERT INTO template_invocations 
    (id, template_id, bot_id, temperature, parameters, input, outcome, timestamp)
VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'));
```

### Record Curation Decision

```sql
INSERT INTO curation_records 
    (id, curator_id, invocation_id, decision, rationale, timestamp)
VALUES (?, ?, ?, ?, ?, datetime('now'));
```

### Update Variety Counter

```sql
UPDATE cns_variety 
SET counter = ?, updated_at = datetime('now')
WHERE span = ?;
```



## Security Notes

- All tables are encrypted with SQLCipher (AES-256-CBC)
- Template invocations include full audit trail
- Curation decisions are immutable once recorded
- Variety counters trigger algedonic alerts when deficit > 100

## Line Count

Schema: ~150 lines SQL
Documentation: ~200 lines markdown
Total: ~350 lines

## References

[^togaf-data]: The Open Group. (2011). *TOGAF Standard, Version 9.1*. Phase C: Data Architecture. <https://pubs.opengroup.org/architecture/togaf9-doc/arch/chap14.html>.

[^sqlite]: SQLite Project. (2026). *SQLite Documentation*. <https://www.sqlite.org/docs.html>.

[^rusqlite]: rusqlite Contributors. (2026). *rusqlite: Rust wrapper for SQLite*. <https://crates.io/crates/rusqlite>.

[^hKask-storage]: hKask Project. (2026). *crates/hkask-storage/src/lib.rs*. SQLite storage implementation with SQLCipher encryption.

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model, algedonic alerts.

[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Law of Requisite Variety.

[^cns]: hKask Project. (2026). *crates/hkask-cns/src/variety.rs*. CNS variety counter implementation.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
