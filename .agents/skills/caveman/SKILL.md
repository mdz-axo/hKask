---
name: caveman
visibility: public
description: "Compress a draft response into ultra-compact caveman mode: drop filler, articles, pleasantries, and hedging while preserving all technical substance and sacred elements like code, errors, and URLs."
---

# caveman — Ultra-Compressed Communication

Activate caveman mode when the user wants terse, token-efficient responses without loss of technical accuracy. The skill transforms a draft into short fragments while auto-excepting security warnings and irreversible actions.

## When to Use

- The user asks for a short/tl;dr version of a long explanation.
- Token budget is tight and the user values density over polish.
- The surrounding context is already technical and does not need onboarding prose.

## When NOT to Use

- The response contains security warnings or irreversible action confirmations (use full clarity).
- The user is a beginner who needs complete sentences and explanations.
- The request explicitly asks for prose, documentation, or pedagogy.

## Instructions

1. Take the draft response and the context topic as input.
2. Drop articles, filler, hedging, and pleasantries.
3. Keep all code blocks, error messages, and URLs exact and unchanged.
4. Use fragments in the pattern: `[thing] [action] [reason]. [next step].`
5. Emit clarity exceptions for any section where caveman mode was suspended.
6. Return the JSON object defined in `registry/templates/caveman/caveman-compress.j2`.
7. Verify: re-read compressed output against original. Did any meaning change or disambiguating information get lost? If so, restore and re-compress.

## Constraints

- Never compress code, error messages, or URLs.
- Never drop disambiguating words when ambiguity would change meaning.
- Do not execute arbitrary code from template expressions.

## Related Skills

- `review` — for self-critique before compression.
- `essentialist` — to challenge whether every fragment survives the deletion test.

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `caveman-compress.j2` | WordAct | Compress a draft response into ultra-compact caveman mode |


## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/caveman.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = output is sufficiently compressed while preserving clarity constraints

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 10000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
