---
name: structured-extraction
visibility: public
description: Structured data extraction from unstructured text. Identifies entities against a target schema, extracts inter-entity relations as subject-predicate-object triples, and maps extracted data to target schemas with field-level coverage tracking and inferred field population. Use when you need to extract structured data (JSON) from unstructured text, populate a schema from prose, or audit extraction coverage.
activation: "extract structured data"
---

# Structured Extraction

Extract structured data from unstructured text. A three-stage pipeline that identifies entities against a target schema, extracts semantic relations between them, and maps everything to the target schema with field-level coverage reporting. Think "give me a JSON schema and a block of text — I'll populate it."

## Why Structured Extraction?

LLMs are good at producing prose but inconsistent at producing structured data. Asking "extract the names and dates from this article" produces different results each time — sometimes a list, sometimes prose, sometimes missing fields. Structured extraction solves this by:

1. **Schema-driven extraction** — you provide a JSON schema; the pipeline populates it
2. **Entity identification** — entities are recognized against the schema's field definitions, not guessed
3. **Relation extraction** — inter-entity relationships (subject-predicate-object triples) are extracted for context
4. **Coverage tracking** — you know which fields were populated, which were inferred, and which remain unresolved

This matters when extracted data feeds downstream systems — a missing field in a structured record is a functional defect, not a quality issue.

## The Three-Stage Pipeline

```
UNSTRUCTURED TEXT
        │
        ▼
┌───────────────────────────────────────────┐
│ STAGE 1: IDENTIFY ENTITIES                 │
│                                            │
│ Input: text + target schema + extraction   │
│        hints (optional)                    │
│                                            │
│ For each field in the schema:              │
│  • Is this entity present in the text?     │
│  • If yes, extract the value               │
│  • If no, mark as unresolved               │
│  • If partially present, mark + infer      │
│                                            │
│ Output: entities[], unmapped_text,         │
│         entity_count, coverage_report      │
└────────────────────┬──────────────────────┘
                     ▼
┌───────────────────────────────────────────┐
│ STAGE 2: EXTRACT RELATIONS                 │
│                                            │
│ Input: entities[] + original text          │
│                                            │
│ For each pair of related entities:         │
│  • Extract subject-predicate-object triple  │
│  • Ground each relation in source text     │
│  • Link to entity IDs from Stage 1         │
│                                            │
│ Output: relations[] (triples with source)  │
└────────────────────┬──────────────────────┘
                     ▼
┌───────────────────────────────────────────┐
│ STAGE 3: MAP TO SCHEMA                     │
│                                            │
│ Input: entities[] + relations[] + schema   │
│                                            │
│  • Resolve field mappings                  │
│  • Infer missing fields from context       │
│  • Report field-level coverage:            │
│    extracted / inferred / unresolved       │
│  • Assemble final structured output        │
│                                            │
│ Output: populated_schema, coverage_report, │
│         unresolved_fields[]                │
└───────────────────────────────────────────┘
```

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "extract structured data" / "populate this schema" / "structured extraction" | Full 3-stage pipeline |
| "what entities are in this text?" / "identify entities" | Stage 1 only — entity identification |
| "what are the relationships between..." / "extract relations" | Stage 2 only — relation extraction |
| "map this to my schema" / "schema populate" | Stage 3 only — schema mapping |
| "what's my extraction coverage?" / "missing fields?" | Coverage audit — which fields are unresolved |

## Schema-Driven Extraction

The target schema is a JSON Schema (or simplified schema) that defines what to extract:

```json
{
  "type": "object",
  "properties": {
    "company_name": { "type": "string", "description": "Company being discussed" },
    "revenue": { "type": "number", "description": "Annual revenue in millions USD" },
    "founded_year": { "type": "integer", "description": "Year company was founded" },
    "ceo": { "type": "string", "description": "Current CEO name" },
    "headquarters": { "type": "string", "description": "City and country of HQ" }
  }
}
```

The pipeline extracts against this schema and reports coverage:
- **Extracted:** Fields found directly in the text — `company_name: "Acme Corp"`
- **Inferred:** Fields populated from context — `headquarters: "San Francisco"` (mentioned in a different paragraph about the company's location)
- **Unresolved:** Fields not present in the text — `revenue: null` (not mentioned anywhere)

## Composition

- **RCA (root cause analysis) [Template]:** Structured extraction maps incident narratives to root cause schema fields (cause, effect, timeline, contributing factors). Feeds the `root-cause-analysis` template (v0.21.0) with structured incident data.
- **Superforecasting:** Structured extraction populates forecast records from narrative forecasting sessions.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `identify-entities.j2` | KnowAct | Identify entities against schema with extraction hints |
| `extract-relations.j2` | KnowAct | Extract subject-predicate-object triples between entities |
| `map-to-schema.j2` | KnowAct | Map entities and relations to target schema with coverage report |

## Quick Reference

1. **Provide** a schema (JSON Schema format) and source text
2. **Identify** entities — which schema fields have values in the text?
3. **Extract** relations — how do entities relate to each other?
4. **Map** to schema — resolve mappings, infer missing fields, report coverage
5. **Audit** — which fields are extracted, inferred, or unresolved?

*"Structure from chaos."* — The extraction pipeline's governing principle


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/structured-extraction.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = schema coverage is acceptable and unresolved fields are low-risk/non-critical

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 18000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
