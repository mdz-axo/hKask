---
name: chain-of-density
visibility: public
description: Iterative density-increase summarization based on Gao et al. (2024). Produces increasingly dense summaries by identifying missing entities from prior summaries, merging them in, and removing redundant language — all while maintaining fixed length. Converges when density improvement falls below threshold between passes. Use when you need a maximally information-dense summary at a fixed length, or when "summarize this" needs to preserve more entities than a single-pass summary can hold.
activation: "summarize this densely"
---

# Chain-of-Density Summarization

An iterative summarization method based on Gao et al. (2024). Unlike single-pass summarization that produces one summary and stops, Chain-of-Density runs multiple **density passes** — each pass takes the prior summary, identifies entities it missed, and weaves them in while keeping the summary the same length. The result: progressively higher **entity density** (more information per word) without increasing word count.

## Why It Matters

A typical single-pass summary might have entity density of 0.08 — one named entity every ~12 words. After 5 Chain-of-Density passes, density can reach 0.15–0.25 — nearly **double the information density** at the same token cost. This matters when:

- You're paying per token (API costs) and need maximum information per token
- You're feeding summaries into context windows and can't afford verbosity
- You need to preserve specific entities (people, dates, quantities, concepts) that single-pass summaries tend to drop
- You're condensing for agent consumption and stale/incomplete entity references produce incorrect behavior

This is a brachistochrone operation: each density pass looks like extra work, but the final summary communicates far more per unit length. The cycloid dips below the endpoint — more iterations, denser result.

## vs. Caveman

| | Caveman | Chain-of-Density |
|---|---|---|
| **Compresses** | Tone — drops filler, articles, hedging, pleasantries | Information — increases entity count at fixed length |
| **Preserves** | Technical substance, code, errors, URLs | Entities, facts, relationships between concepts |
| **Method** | Single-pass stylistic compression | Iterative entity-density optimization |
| **Use when** | You need ultra-compact, direct prose | You need maximum factual coverage at fixed length |
| **Converges?** | No — single pass | Yes — δDensity < threshold between passes |

They're complementary: caveman for style compression, chain-of-density for information compression.

## How It Works

```
SOURCE TEXT (e.g., 2000 words)
        │
        ▼
┌───────────────────────────────────────────┐
│ PASS 0: INITIAL SUMMARY                    │
│ Identify all entities in source.           │
│ Generate baseline summary at target length. │
│ Compute entity density. Mark missing.       │
│ Density: 0.08 (entities/word)              │
└────────────────────┬──────────────────────┘
                     ▼
┌───────────────────────────────────────────┐
│ PASS 1: DENSITY INCREASE                   │
│ Take missing entities from pass 0.         │
│ Weave them into summary.                   │
│ Remove redundant language to make room.     │
│ Keep same word count.                      │
│ Density: 0.12 → +0.04 improvement           │
└────────────────────┬──────────────────────┘
                     ▼
┌───────────────────────────────────────────┐
│ PASS 2: DENSITY INCREASE                   │
│ Take remaining missing entities.           │
│ Merge in. Remove redundancies.             │
│ Density: 0.17 → +0.05 improvement           │
└────────────────────┬──────────────────────┘
                     ▼
         ...repeat until...
                     ▼
┌───────────────────────────────────────────┐
│ CONVERGENCE                                │
│ Density improvement < threshold (e.g.,     │
│ < 0.02) OR no missing entities remain.     │
│ → Summary is at maximum entity density     │
│   for the given length constraint.          │
└───────────────────────────────────────────┘
```

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "summarize this densely" / "chain of density" / "CoD this" | Full multi-pass density optimization |
| "make this denser" / "pack more in" on an existing summary | Density pass only — merge missing entities into prior summary |
| "what entities am I missing?" / "entity check" | Entity identification only — no summarization |
| "max density this" / "densest possible summary" | Full multi-pass until convergence or density ceiling |

## What "Entity" Means Here

"Entity" in Chain-of-Density is broader than NER (named entity recognition). It includes:
- **Named entities**: people, organizations, places
- **Key concepts**: technical terms, domain-specific ideas
- **Quantities**: numbers, dates, measurements, statistics
- **Relationships**: causal links, comparisons, dependencies between concepts

The goal is to preserve *everything that carries information*, not just proper nouns.

## Convergence

The loop converges when:
- **Density improvement < threshold** (e.g., < 0.02) — adding more entities would require removing others; you've hit the density ceiling for this length
- **No missing entities remain** — everything worth preserving is already in the summary
- **Max passes reached** — practical limit to prevent infinite optimization

The `converged` flag in the output indicates whether further passes would help.

## Composition

- **Caveman:** Chain-of-Density first (max entity density), then Caveman on the final summary (compress tone). Together: maximum information in minimum prose.
- **Pragmatic-laziness:** CoD is a brachistochrone operation — more passes (apparent effort) produce denser output (lower actual action per information unit). Pragmatic laziness recognizes this as genuine action reduction.
- **Structured-extraction:** CoD's entity identification feeds structured extraction's schema mapping pipeline.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `initial-summary.j2` | KnowAct | Baseline summary with entity identification and density calculation |
| `density-pass.j2` | KnowAct | Iterative density increase — merge missing entities, remove redundancies, maintain length |

## Quick Reference

1. **Identify** all entities in the source text
2. **Summarize** at target length, noting which entities are missing
3. **Densify** — merge missing entities while keeping length fixed
4. **Repeat** until density improvement falls below threshold
5. **Converge** — deliver the densest summary at the given length

*"Chain-of-Density enables summaries with substantially higher entity density without increasing length."* — Gao et al., 2024
