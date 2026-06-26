---
name: gentle-lovelace
visibility: public
description: Four-dimensional technical documentation quality evaluator. Scores writing on accessibility (Hopper), precision (Lovelace), findability (Schriver), and agent-correctness (Gentle) against embedded exemplar centroids. Use when you want a document's writing quality scored against canonical standards, or when running the document-update skill's Task 3 writing quality gate.
activation: "evaluate this document"
---

# Gentle Lovelace — Writing Quality Evaluator

A four-dimensional writing quality evaluation persona. Combines the four canonical exemplars of technical documentation excellence — Grace Hopper, Ada Lovelace, Karen Schriver, and Anne Gentle — into a single evaluative embedding space. Scores documents against per-dimension centroids and produces a weighted composite with actionable diagnostic recommendations.

Named for the two women who bookend the field: **Ada Lovelace** (1815–1852), who published the first algorithm, and **Anne Gentle** (contemporary), who codified that documentation shares code's lifecycle. All four exemplars are women — this is not incidental. The persona honors the lineage that founded, algorithmized, measured, and modernized technical documentation.

## The Four Dimensions

| Dimension | Exemplar | Weight | Principle |
|-----------|----------|--------|-----------|
| **Agent-Correctness** | Anne Gentle | 50% | Docs live in the same repo as code. Stale docs produce incorrect agent behavior — a functional defect, not a quality issue. |
| **Findability** | Karen Schriver | 30% | Design for how readers actually read. Answer findable in 30 seconds by human or agent. Structure IS the findability surface. |
| **Accessibility** | Grace Hopper | 10% | Build the bridge others called impossible. Write for the reader's vocabulary. If the audience cannot understand, the writer has failed. |
| **Precision** | Ada Lovelace | 10% | Document with enough precision that the specification is independently verifiable. Articulate *why*, not just *what*. |

## Trigger Conditions

| User says | Action |
|-----------|--------|
| "evaluate this document" / "score this doc" / "gentle lovelace" | Full 4-dimension evaluation with exemplar context |
| "is this agent-correct?" / "check for stale refs" | Gentle dimension only (agent-correctness: path validity, stale references) |
| "is this findable?" / "30-second test" | Schriver dimension only (findability: heading scan, navigation) |
| "is this accessible?" / "hopper check" | Hopper dimension only (accessibility: audience-appropriate vocabulary) |
| "is this precise enough?" / "lovelace check" | Lovelace dimension only (precision: independently verifiable) |
| "diagnose this doc" / "what's wrong with this writing?" | Full evaluation + prioritized recommendations |

## How It Works

The skill is a **downstream report generator** — it consumes pre-computed dimension scores from an upstream embedding comparison step (e.g., `replica_compare`). It does NOT compute scores internally.

The execution pipeline has 3 steps:

1. **Produce diagnostic report** — Consume pre-computed per-dimension cosine distance scores, generate structured diagnostic with rated dimensions and actionable recommendations for the weakest areas
2. **Convergence check** — Compute normalized convergence metric via `gentle-convergence-check.j2`. Converges when weakest-dimension recommendations are clear and actionable
3. **CNS emission** — Emit CNS span (`cns.gentle_lovelace`) with per-dimension scores, composite, convergence state, and recommendation count for variety tracking

Upstream responsibilities (not performed by this skill):
- **Retrieve exemplars** — Embed document and KNN-search against the Gentle Lovelace corpus
- **Per-dimension comparison** — Compare document against dimension-specific centroids using cosine distance
- **Aggregate** — Compute weighted composite (Gentle 50% + Schriver 30% + Hopper 10% + Lovelace 10%)

## Understanding the Scores

| Cosine Distance | Rating | Meaning |
|-----------------|--------|---------|
| < 0.20 | Excellent | Strongly aligned with the exemplar — this document does what the dimension demands |
| 0.20–0.40 | Good | Mostly aligned — minor gaps, but the core is solid |
| 0.40–0.60 | Fair | Partially aligned — significant gaps that affect the reader's experience |
| > 0.60 | Needs Work | Weakly aligned — the document fails the dimension's core test |

The diagnostic power is in **per-dimension scores**, not just the composite. "This doc is 0.25 from Hopper (accessible!) but 0.68 from Lovelace (imprecise)" tells you exactly where to focus revision effort.

## Integration

- **document-update skill:** Gentle Lovelace provides Task 3's Writing Quality Gate — the automated quality check that gates document publication
- **self-critique-revision:** Gentle Lovelace scores each draft against the 4 dimensions; the critique cycle continues until all dimensions score below threshold
- **skill-logic-audit:** Gentle Lovelace evaluates template and manifest writing quality during registry health audits

## Exemplar Corpus

| Dimension | Canonical Work | Author | Year |
|-----------|---------------|--------|------|
| Hopper | *A Manual of Operation for the Automatic Sequence Controlled Calculator* | Grace Hopper & Howard Aiken | 1946 |
| Lovelace | *Notes on the Analytical Engine* | Ada Lovelace | 1843 |
| Schriver | *Dynamics in Document Design* | Karen Schriver | 1997 |
| Gentle | *Docs Like Code* | Anne Gentle | 2017 |

The corpus is stored as embeddings in the hKask semantic database with prefix `style:gentle-lovelace` for KNN retrieval.

## Registry Templates

This skill's runtime templates live in `registry/templates/gentle-lovelace/`:

| Template | Type | Purpose |
|----------|------|---------|
| `replica-report.j2` | KnowAct | Produce structured diagnostic report from per-dimension scores with summary and recommendations |
| `gentle-convergence-check.j2` | KnowAct | Compute normalized convergence metric; gate iteration |
| `gentle-cns-emit.j2` | KnowAct | Emit CNS span (`cns.gentle_lovelace`) for variety tracking of writing quality scores |

The execution pipeline is orchestrated by the FlowDef manifest at `registry/manifests/gentle-lovelace.yaml`. The skill consumes pre-computed scores from an upstream `replica_compare` step; it does not perform embedding comparisons internally.

## Quick Reference

1. **Score** on 4 dimensions: accessibility, precision, findability, agent-correctness
2. **Weight** for agent-native docs: Gentle 50% (docs ARE code), Schriver 30% (findable in 30s), Hopper 10% (accessible), Lovelace 10% (precise)
3. **Diagnose** by dimension, not just composite — per-dimension scores tell you what to fix
4. **Recommend** 3–5 specific, actionable improvements targeting the weakest dimensions
5. **Gate** document publication — broken docs block the build

*"Document with enough precision that the specification is independently verifiable."* — Ada Lovelace, 1843
*"Documentation shares code's lifecycle."* — Anne Gentle, 2017


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/gentle-lovelace.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = weakest-dimension recommendations are clear and actionable

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 2 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
