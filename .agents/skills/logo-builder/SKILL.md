---
name: logo-builder
visibility: public
description: >
  Generate professional logos for businesses and groups using hKask's media
  server tools. Two approaches: single-shot (quick) or iterative-refine
  (critique-and-improve loop). Logos are stored in the gallery as regular
  images — no separate storage.
composes_skills: [coding-guidelines]
---

# Logo Builder

Generate professional logos from business identity inputs using the media
server's `generate_image` and `describe_image` tools. Logos are stored in
the gallery alongside all other images.

## When to Activate

- User says "design a logo", "create a logo", "generate a logo"
- User provides a business name and wants visual identity
- User asks for logo variations or refinement of an existing logo concept

## Two Approaches

### Single-Shot (`media/logo-single-shot`)

Fast. One call to `generate_image` with a curated prompt template. Best for
exploration and rapid iteration.

```
Inputs: name, industry?, style?, palette?, logo_type?
Output: one logo image URL
```

### Iterative Refinement (`media/logo-iterative-refine`)

Higher quality. Generates N candidates, uses a vision LLM to critique each
on readability/scalability/distinctiveness/professionalism, selects the
best, and regenerates with critique feedback. Repeats up to 3 cycles.

```
Inputs: name, industry?, style?, palette?, logo_type?, variants (1-5), refine_rounds (0-3)
Output: array of final logo image URLs with scores
After: background removed for transparent PNG
```

## Style Reference

| Style | Visual character |
|-------|-----------------|
| `minimal` | Clean lines, few elements, lots of whitespace |
| `modern` | Geometric, sans-serif, flat design |
| `playful` | Rounded, colorful, approachable |
| `corporate` | Conservative, serif, navy/blue palette |
| `vintage` | Retro typography, muted colors, badges/emblems |
| `luxury` | Gold/black, elegant serifs, thin lines |
| `tech` | Futuristic, gradients, circuit/network motifs |
| `handmade` | Organic, imperfect, craft aesthetic |
| `bold` | Heavy weights, high contrast, assertive |
| `elegant` | Thin strokes, script fonts, sophisticated |

## Logo Types

| Type | Description |
|------|------------|
| `wordmark` | Text-only logo (the name styled as the logo) |
| `lettermark` | Initials or abbreviation (e.g., "IBM", "CNN") |
| `emblem` | Text inside a symbol/badge (e.g., university seals) |
| `abstract` | Geometric symbol, no text (e.g., Nike swoosh) |
| `combination` | Symbol + wordmark together |
| `mascot` | Illustrated character or figure |

## Prompt Examples

```
"Design a minimalist wordmark logo for a coffee shop called 'Slow Pour'"
→ style: minimal, logo_type: wordmark, industry: food

"Create 3 logo candidates for a fintech startup named 'Ledger' with a modern tech style, then refine the best one"
→ style: tech, logo_type: combination, variants: 3, refine_rounds: 1

"Generate an elegant emblem logo for a luxury hotel 'The Ashford' in gold and navy"
→ style: luxury, logo_type: emblem, palette: "#c9a84c,#1a2744"
```

## Constraints

- Uses media server tools only — no new crate, no new storage
- Logos are images in the gallery — searchable, taggable, removable
- Prompt templates live in `registry/templates/media/`
- Vision critique costs extra inference credits — `refine_rounds: 0` skips it
