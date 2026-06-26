---
name: logo-builder
visibility: public
description: >
  Pragmatic and principled logo design using LLM-assisted generation.
  Synthesizes Martin (Made By James — Minimum Viable Brand), Bokhua
  (Principles of Logo Design — five formal gates), and Peters (Logos
  That Last — iterative case study method). Three-phase pipeline:
  discovery (brand-to-design mapping), formal generation (Bokhua gates),
  and iterative refinement (weighted critique loop). Uses hKask's media
  server tools (generate_image, describe_image, image_remove_background).
  Logos stored in the gallery as regular images — no separate storage.
  Activate when user says "design a logo", "create a logo", "generate a
  logo", "make a brand mark", or provides a business name with visual
  identity intent.
references_skills: [coding-guidelines, essentialist]
---

# Pragmatic and Principled Logo Design for LLM-Assisted Generation

**A Deep Research Paper for the hKask Logo Builder Skill**

**References:**
1. James Martin, *Made By James* — <https://themadebyjames.com/>
2. George Bokhua, *Principles of Logo Design* — <https://blazetype.eu/blog/principles-of-logo-design/>
3. Allan Peters, *Logos That Last* — <https://www.petersdesigncompany.com/book>

---

## 1. Introduction

A logo is a *sign* — the most compressed unit of visual identity. Bokhua: "a logo comprising simple shapes can communicate a stronger message than a complex one." Peters: logos that *last*. Martin: the centerpiece of a Minimum Viable Brand. This paper asks: **what process and prompts produce principled logos, not merely pretty ones?**

| Section | Source Emphasis | Registry Artifact |
|---------|----------------|-------------------|
| §2 — Discovery & Brief | Martin (MVB) | `logo-discovery.yaml` + `logo-discovery-map.j2` |
| §3 — Formal Design | Bokhua (formal gates) | `logo-formal-prompt.j2` |
| §4 — Iteration & Refinement | Peters (case studies) | `logo-iterative-refine.yaml` |
| §5 — Presentation | All three | `logo-presentation.yaml` |
| §6 — LLM Selection | Cross-cutting | Model parameters in all templates |

---

## 2. Discovery & Brief (Martin's MVB Layer)

### 2.1 The Minimum Viable Brand

Martin's core insight: a logo never exists in isolation. Before generating a single mark, answer:

1. **Who** is this for? (audience, not just client)
2. **What** does it stand for? (values, not features)
3. **Where** will it live? (signage, screens, embroidery, favicon)
4. **Why** does it matter? (differentiation, not description)

Martin's sketch-to-vector workflow begins with *words*, not shapes. For LLM generation, this means a structured **brief** must constrain prompt generation.

### 2.2 Registry Artifacts

**FlowDef:** `registry/templates/media/logo-discovery.yaml`
- Maps brand identity inputs → formal design parameters
- Agent-coordinated: renders `logo-discovery-map.j2`, calls inference router, parses JSON
- Selects strategy: single-shot, iterative-refine, or moodboard-first

**KnowAct:** `registry/templates/media/logo-discovery-map.j2`
- Maps `{name, industry, audience, values}` → `{style, logo_type, dominant_shape, typography_class, palette_hex, density, rationale}`
- Default model: `OR/openai/gpt-4o` (strong brand reasoning, structured output)

### 2.3 Style Taxonomy

| Style | Visual character |
|-------|-----------------|
| `minimal` | Clean lines, few elements, whitespace |
| `modern` | Geometric, sans-serif, flat |
| `playful` | Rounded, colorful, approachable |
| `corporate` | Conservative, serif, navy/blue |
| `vintage` | Retro typography, muted, badges |
| `luxury` | Gold/black, elegant serifs, thin |
| `tech` | Futuristic, gradients, geometric |
| `handmade` | Organic, imperfect, craft |
| `bold` | Heavy weights, high contrast |
| `elegant` | Thin strokes, script, sophisticated |

---

## 3. Formal Design (Bokhua's Five Gates)

### 3.1 The Gates

| Gate | Principle | Check |
|------|-----------|-------|
| **G1 — Simplicity** | "Maximize communication with minimal information" | Describable in one sentence? |
| **G2 — Monochrome Viability** | "Simple, monochromatic shapes" | Works in pure black on white? |
| **G3 — Grid Discipline** | Grid systems (Müller-Brockmann) | Geometric intentionality? |
| **G4 — Negative Space** | "Negative space, contrast, aperture" | Active or intentional space? |
| **G5 — Scalability** | "Size, legibility, composition" | Legible at 16px? At billboard? |

Bokhua's strongest point: formal research happens *before* the computer — pencil-first iteration. For LLM generation: **prompt structure before pixel generation.** The brief must be fully resolved before `generate_image` is called.

### 3.2 Registry Artifact

**KnowAct:** `registry/templates/media/logo-formal-prompt.j2`
- Core generation prompt — every logo flow delegates to this
- Encodes all five gates as explicit prompt requirements
- Accepts `{name, industry, style, logo_type, palette, dominant_shape, density, typography_class, generation_model}`
- Default model: `FA/flux-pro/v1` (best text rendering for wordmarks/lettermarks)

---

## 4. Iteration & Refinement (Peters' Case Studies)

### 4.1 The Case Study Method

Peters structures around detailed case studies from concept to completion:

1. **Multiple candidates** — never present one option
2. **Specific critique** — each iteration responds to named issues
3. **Documented decisions** — rationale becomes brand guidelines
4. **Brand system thinking** — logo is the seed; the system grows from it

### 4.2 Registry Artifact

**FlowDef:** `registry/templates/media/logo-iterative-refine.yaml`
- Generates N candidates via `generate_image` with `logo-formal-prompt.j2`
- Vision LLM critique via `describe_image` against 5 criteria (readability, scalability, distinctiveness, professionalism, text accuracy)
- Agent selects best candidate, regenerates with critique feedback
- Background removal via `image_remove_background`
- Parameters: `{name, industry, style, palette, logo_type, variants (1-5), refine_rounds (0-3), generation_model, critique_model}`

### 4.3 Weighted Critique Dimensions

The paper proposes 7 weighted dimensions for structured critique. The FlowDef implements a simplified 5-criterion version that maps to actual `describe_image` calls. The full weighted model from the paper is aspirational — it requires scoring logic not yet implemented in the media server.

| Dimension | Source | Weight (paper) | Implemented? |
|-----------|--------|---------------|-------------|
| Simplicity | Bokhua G1 | 0.25 | As "readability" in FlowDef |
| Monochrome | Bokhua G2 | 0.15 | Aspirational |
| Grid | Bokhua G3 | 0.10 | Aspirational |
| Negative Space | Bokhua G4 | 0.10 | Aspirational |
| Scalability | Bokhua G5 | 0.15 | As "scalability" in FlowDef |
| Brand Fit | Martin | 0.15 | Aspirational |
| Distinctiveness | Peters | 0.10 | As "distinctiveness" in FlowDef |

---

## 5. Presentation & Deliverables

### 5.1 Registry Artifact

**FlowDef:** `registry/templates/media/logo-presentation.yaml`
- `image_remove_background` → transparent PNG
- `generate_image` → monochrome variant
- `generate_image` → icon-only square mark (favicon)
- `generate_image` → context mockup (signage/website)

---

## 6. LLM Model Selection

| Stage | Primary | Fallback | Why |
|-------|---------|----------|-----|
| Discovery | `OR/openai/gpt-4o` | `DI/meta-llama/Llama-3.3-70B` | Structured output, brand reasoning |
| Generation | `FA/flux-pro/v1` | `FA/flux-dev` | Best text rendering |
| Critique | `OR/openai/gpt-4o` | `OR/anthropic/claude-sonnet` | Vision + structured scoring |
| Refinement | Same as generation | — | Visual consistency |

**Never cross providers within a refinement cycle.** Cost: ~$0.50-1.50 for full iterative-refine (3 variants × 2 rounds).

---

## 7. Integrated Architecture

### 7.1 Template Inventory

| Template | Type | File |
|----------|------|------|
| Logo Discovery (flow) | FlowDef | `registry/templates/media/logo-discovery.yaml` |
| Logo Discovery Map (prompt) | KnowAct | `registry/templates/media/logo-discovery-map.j2` |
| Logo Formal Prompt | KnowAct | `registry/templates/media/logo-formal-prompt.j2` |
| Logo Iterative Refine | FlowDef | `registry/templates/media/logo-iterative-refine.yaml` |
| Logo Presentation | FlowDef | `registry/templates/media/logo-presentation.yaml` |

### 7.2 Execution Flow

```
User provides { name, industry, audience?, values? }
        │
        ▼
[logo-discovery.yaml] ──► logo-discovery-map.j2 ──► design parameters
        │
        ├── single-shot ──► logo-formal-prompt.j2 ──► generate_image
        │
        └── iterative-refine ──► logo-iterative-refine.yaml
                │
                ├── generate-candidates (×N) via generate_image
                ├── critique (×N) via describe_image
                ├── agent selects best
                ├── refine (×R rounds)
                └── finalize via image_remove_background
                        │
                        ▼
              [logo-presentation.yaml] ──► deliverables package
```

---

## 8. Constraints

- Uses media server tools only — `generate_image`, `describe_image`, `image_remove_background`
- Logos stored in the gallery as regular images — no separate storage
- Discovery phase uses inference router for text LLM classification (not a media tool)
- FlowDef templates are agent-coordinated — they describe what to do; the agent executes the steps
- Vision critique costs extra inference credits — `refine_rounds: 0` skips it
- `references_skills` [coding-guidelines, essentialist] are aspirational integrations for future upgrades — they are declared but have zero call sites in any template; they will be wired when the Bokhua gate scoring logic is implemented in the media server

## References

1. Martin, James. *Made By James* — <https://themadebyjames.com/>
2. Bokhua, George. *Principles of Logo Design*. Rockport Publishers, 2022. ISBN 9780760376515.
3. Peters, Allan. *Logos That Last*. <https://www.petersdesigncompany.com/book>
4. Müller-Brockmann, Josef. *Grid Systems in Graphic Design*. Niggli, 1981.
5. Wong, Wucius. *Principles of Form and Design*. Wiley, 1993.


## Registry Manifest

**Type:** Template (one-shot) | **Manifest:** none (no registry crate — SKILL.md only)

This is a Template, not a Skill. Templates are one-shot prompt executions without PDCA convergence.

**Upgrade path:** to convert from Template to Skill, create a PDCA orchestrator at `registry/manifests/logo-builder.yaml` that wraps the existing FlowDefs (logo-discovery.yaml → logo-iterative-refine.yaml → logo-presentation.yaml) with convergence criteria based on the Bokhua gate scores.
