---
name: caveman
visibility: public
description: "Multi-mode text compression: caveman mode drops filler/hedging for ultra-compact fragments, dense mode applies iterative entity-preserving density increase (Gao et al. 2024 Chain-of-Density). Supports single-mode or pipelined dense→caveman for maximum compression. Use when the user wants terse output, dense summaries, or 'caveman mode'."
---

# caveman — Multi-Mode Concise Communication

Two compression strategies, one skill. Caveman mode compresses *style* (drops filler, articles, pleasantries). Dense mode compresses *information* (iteratively packs more entities into fixed length). Combined: maximum information in minimum prose.

## Modes

### Mode 1: `caveman` — Stylistic Compression

Drop filler, articles, hedging, and pleasantries. Preserve all technical substance. Use when token budget is tight or the user values density over polish.

**Instructions:**
1. Take the draft response and context topic as input.
2. Drop articles, filler, hedging, and pleasantries.
3. Keep all code blocks, error messages, and URLs exact and unchanged.
4. Use fragments in the pattern: `[thing] [action] [reason]. [next step].`
5. Emit clarity exceptions for any section where caveman mode was suspended.
6. Verify: re-read compressed output against original. Did any meaning change or disambiguating information get lost? If so, restore and re-compress.

### Mode 2: `dense` — Entity-Preserving Density (Chain-of-Density)

Based on Gao et al. (2024). Iteratively increases entity density — each pass identifies missing entities from the prior summary, weaves them in, and removes redundant language while maintaining fixed length. Converges when density improvement drops below threshold or no missing entities remain.

**What "entity" means:** named entities (people, organizations, places), key concepts (technical terms, domain-specific ideas), quantities (numbers, dates, measurements), and relationships (causal links, comparisons, dependencies).

**Instructions:**
1. **Pass 0 — Initial summary:** Identify all entities in source. Generate baseline summary at target length. Compute entity density. Mark missing entities.
2. **Pass N — Density increase:** Take missing entities from prior pass. Weave them into summary. Remove redundant language to make room. Keep same word count.
3. **Converge when:** Density improvement < threshold (e.g., < 0.02) OR no missing entities remain OR max passes reached (5).
4. Return the densest summary with entity density score and convergence flag.

### Mode 3: `dense+caveman` — Maximum Compression (Pipeline)

Run dense mode first (max entity density at target length), then caveman-compress the result (strip stylistic filler). This is always the correct order — caveman drops entities for style; dense preserves them. Densify first, compress second.

## When to Use

| User says | Mode |
|-----------|------|
| "caveman mode", "compress this", "make this terse", "tl;dr" | `caveman` |
| "summarize this densely", "chain of density", "CoD this", "pack more in" | `dense` |
| "max density", "densest possible summary", "compress completely" | `dense+caveman` |
| Token budget tight, need max info per token | `dense+caveman` |

## When NOT to Use

- Response contains security warnings or irreversible action confirmations (use full clarity).
- User is a beginner who needs complete sentences and explanations.
- Request explicitly asks for prose, documentation, or pedagogy.

## Constraints

- Never compress code, error messages, or URLs.
- Never drop disambiguating words when ambiguity would change meaning.
- In dense mode: never fabricate entities — only weave in entities actually present in the source.
- Pipeline order is always dense → caveman (never reverse).

## Related Skills

- `review` — self-critique before compression.
- `essentialist` — challenge whether every fragment survives the deletion test.
- `structured-extraction` — dense mode's entity identification feeds structured extraction's schema mapping.
- `pragmatic-laziness` — dense mode is a brachistochrone operation (more passes = denser output = lower action per information unit).

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `caveman-compress.j2` | WordAct | Caveman mode — compress draft into ultra-compact fragments |
| `caveman-density-pass.j2` | KnowAct | Dense mode — iterative entity-preserving density increase (Gao et al. 2024) |
| `caveman-convergence-check.j2` | KnowAct | Compute normalized convergence metric for compression cycles |

*"Chain-of-Density enables summaries with substantially higher entity density without increasing length."* — Gao et al., 2024


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/caveman.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 5 (dense mode benefits from deeper iteration; caveman mode typically converges in 1-2)
- **Convergence meaning:** 0 = output is sufficiently compressed; in dense mode, density gains have plateaued; in caveman mode, no further stylistic compression is possible without meaning loss

### Energy Budgets
- **Gas (compute cycles):** cap 120000, 100 per iteration
- **rJoule (inference energy):** cap 18000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
