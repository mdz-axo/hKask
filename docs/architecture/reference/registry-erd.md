---
title: "Template Registry — Entity Relationship Diagram"
audience: [data architects, database developers, agents]
last_updated: 2026-05-24
version: "0.21.0"
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
    cns_variety ||--o{ cns_killzone : "monitors"
    
    templates {
        string id PK
        string name
        string type
        real temperature_min
        real temperature_max
        text prompt_template
        jsonb input_schema
        jsonb output_schema
        jsonb constraints
        timestamp created_at
        timestamp updated_at
    }
    
    template_invocations {
        string id PK
        string template_id FK
        string bot_id
        real temperature
        jsonb parameters
        jsonb input
        jsonb outputs
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
        jsonb ocap_boundaries
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
    
    cns_killzone {
        string id PK
        string space_id
        real vc_investment
        integer acquisition_count
        integer kill_zone_detected
        timestamp last_updated
    }
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-REG-001
verified_date: 2026-05-24
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
| type | TEXT | Template type (code_generation, decision, framing, communication, reflection) |
| temperature_min | REAL | Minimum temperature for sampling |
| temperature_max | REAL | Maximum temperature for sampling |
| prompt_template | TEXT | Jinja2 template for prompt rendering |
| input_schema | JSONB | JSON schema for input validation |
| output_schema | JSONB | JSON schema for output validation |
| constraints | JSONB | Template-specific constraints |
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
| parameters | JSONB | Full LLM parameters (top_p, top_k, etc.) |
| input | JSONB | Input provided to template |
| outputs | JSONB[] | Array of generated outputs |
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
| ocap_boundaries | JSONB | OCAP boundaries checked |
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

### cns_killzone

Kill zone detection for catch-and-kill monitoring.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | Primary key |
| space_id | TEXT | Technology/market space being monitored |
| vc_investment | REAL | VC investment level (0.0-1.0) |
| acquisition_count | INTEGER | Number of acquisitions in space |
| kill_zone_detected | INTEGER | Boolean flag (0/1) |
| last_updated | TIMESTAMP | Last update timestamp |

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

### Detect Kill Zone

```sql
UPDATE cns_killzone
SET vc_investment = ?, 
    acquisition_count = acquisition_count + 1,
    kill_zone_detected = CASE WHEN ? < 0.5 AND acquisition_count > 0 THEN 1 ELSE 0 END,
    last_updated = datetime('now')
WHERE space_id = ?;
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

*ℏKask - A Minimal Viable Container for Agents — v0.21.0*
