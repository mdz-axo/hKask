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
  server tools (generate_image, describe_image, remove_background).
  Logos stored in the gallery as regular images — no separate storage.
  Activate when user says "design a logo", "create a logo", "generate a
  logo", "make a brand mark", or provides a business name with visual
  identity intent.
composes_skills: [coding-guidelines, essentialist]
---

# Pragmatic and Principled Logo Design for LLM-Assisted Generation

**A Deep Research Paper for the hKask Logo Builder Skill**

**References:**
1. James Martin, *Made By James* — <https://themadebyjames.com/>
2. George Bokhua, *Principles of Logo Design* — <https://blazetype.eu/blog/principles-of-logo-design/>
3. Allan Peters, *Logos That Last* — <https://www.petersdesigncompany.com/book>

---

## Abstract

This paper synthesizes three authoritative perspectives on professional logo design into a structured framework for LLM-assisted logo generation. Each source contributes a distinct layer: **Martin** (Made By James) provides the strategic brand-discovery lens through his Minimum Viable Brand methodology; **Bokhua** (*Principles of Logo Design*) contributes the formal design discipline — geometric grids, monochromatic reduction, and pencil-first iteration; **Peters** (*Logos That Last*) supplies the process rigor — client briefs, case study documentation, and brand system extension. Together they define what makes a logo not merely visually pleasing but *principled*: simple, scalable, memorable, and grounded in a coherent creative process. This paper translates these principles into the operational language of hKask's skill registry — **YAML flow manifests** for process orchestration and **Jinja2 prompt templates** for creative generation, with explicit LLM model selection at each stage.

---

## 1. Introduction

A logo is not a picture. It is a *sign* — the most compressed unit of visual identity. As Bokhua notes, a logo comprising simple shapes can communicate a stronger message than a complex one, leaving a lasting impression. Peters reinforces this with his title: logos that *last*. Martin frames it as the centerpiece of a Minimum Viable Brand.

LLM-assisted logo generation inherits both the promise and the peril of this compression. An image generation model (Flux, SDXL, DALL-E) can produce a visually striking mark in seconds. But without process, it produces decoration, not identity. The question this paper addresses is: **what process and what prompts produce *principled* logos, not merely pretty ones?**

### 1.1 Structure

| Section | Source Emphasis | Skill Artifact |
|---------|----------------|---------------|
| §2 — Discovery & Brief | Martin (MVB) | YAML FlowDef |
| §3 — Formal Design | Bokhua (grids, reduction) | Jinja2 KnowAct prompts |
| §4 — Iteration & Refinement | Peters (case studies) | YAML FlowDef (critique loop) |
| §5 — Presentation | All three | YAML FlowDef (deliverables) |
| §6 — LLM Selection | Cross-cutting | Model parameterization |

---

## 2. Discovery & Brief (Martin's MVB Layer)

### 2.1 The Minimum Viable Brand

Martin's core insight is that a logo never exists in isolation. It is the visual center of a Minimum Viable Brand — the smallest coherent expression of identity. Before generating a single mark, the designer must answer:

1. **Who** is this for? (audience, not just client)
2. **What** does it stand for? (values, not features)
3. **Where** will it live? (contexts: signage, screens, embroidery, favicon)
4. **Why** does it matter? (differentiation, not description)

Martin's sketch-to-vector workflow begins with *words*, not shapes. He maps brand attributes to visual language before touching a tool. For LLM generation, this translates into a structured **brief** that constrains the prompt generation stage.

### 2.2 YAML Manifest: `logo-discovery.yaml` (FlowDef)

This flow runs *before* any image generation call. It gathers business identity inputs and maps them to visual design parameters.

```yaml
# Logo Discovery — Brand Identity Mapping (FlowDef)
template_type: FlowDef
name: media_logo_discovery
version: "1.0.0"
description: "Map business identity inputs to formal logo design parameters before generation"
parameters:
  - name: name
    type: string
    required: true
    description: "Business or group name"
  - name: industry
    type: string
    required: true
    description: "Industry context (tech, food, legal, creative, etc.)"
  - name: audience
    type: string
    required: false
    description: "Who is this for? (e.g., 'young professionals', 'enterprise CTOs')"
  - name: values
    type: array
    required: false
    description: "Core brand values (e.g., [trust, innovation, craft])"
  - name: contexts
    type: array
    default: [digital, print, signage]
    description: "Where the logo will appear"
input_type: text
output_type: design_parameters
steps:
  - id: map-attributes
    description: "Map industry + audience + values to visual attributes using a brand lexicon"
    tool: template_render
    template: media/logo-discovery-map

  - id: resolve-constraints
    description: "Resolve context constraints: minimum readable size, color gamut limits, format requirements"
    condition: "contexts contains 'signage' or 'embroidery'"
    output:
      min_size_mm: integer
      color_limit: integer
      needs_monochrome: boolean

  - id: select-strategy
    description: "Choose between single-shot, iterative-refine, or moodboard-first based on brand complexity"
    condition: "always"
    output:
      strategy: enum[single-shot, iterative-refine, moodboard-first]
```

### 2.3 Jinja2 Prompt: `logo-discovery-map.j2` (KnowAct)

Maps qualitative brand attributes to formal design parameters using a vision LLM.

```jinja2
# Logo Discovery — Brand-to-Visual Mapping (KnowAct)
template_type: KnowAct
name: media_logo_discovery_map
version: "1.0.0"
model: "{{ model | default('OR/openai/gpt-4o') }}"
parameters:
  - name: name
    type: string
    required: true
  - name: industry
    type: string
    required: true
  - name: audience
    type: string
    required: false
  - name: values
    type: array
    required: false
  - name: model
    type: string
    required: false
    description: "LLM for classification (see §6 for selection guidance)"
---
You are a brand strategist. Given a business identity brief, recommend formal logo design parameters.

Business: {{ name }}
Industry: {{ industry }}
{% if audience %}Target audience: {{ audience }}{% endif %}
{% if values %}Core values: {{ values | join(', ') }}{% endif %}

Return a JSON object with these fields:

{
  "style": one of [minimal, modern, playful, corporate, vintage, luxury, tech, handmade, bold, elegant],
  "logo_type": one of [wordmark, lettermark, emblem, abstract, combination, mascot],
  "dominant_shape": one of [circle, square, triangle, organic, line-based, negative-space],
  "typography_class": one of [serif, sans-serif, slab, script, display, geometric],
  "palette_direction": one of [monochrome, warm, cool, earth, neon, high_contrast],
  "palette_hex": array of 2-5 hex codes,
  "density": one of [sparse, balanced, dense],
  "rationale": one-sentence justification citing the brand attributes
}

Base your recommendation on brand strategy principles: a tech startup needs different signals than a law firm. Consider the audience's expectations and the industry's visual conventions — recommend differentiation, not imitation.
```

---

## 3. Formal Design (Bokhua's Layer)

### 3.1 The Five Design Gates

Bokhua's book structures logo design around five chapters, but distilled into a generation pipeline, they form **five design gates** every generated logo must pass:

| Gate | Bokhua's Principle | Generation Check |
|------|-------------------|-----------------|
| **G1 — Simplicity** | "Maximize communication with minimal information" | Can the mark be described in 1 sentence? |
| **G2 — Monochrome Viability** | "Simple, monochromatic shapes" | Does it work in pure black on white? |
| **G3 — Grid Discipline** | Grid systems (Müller-Brockmann lineage) | Are key elements aligned to a geometric structure? |
| **G4 — Optical Correction** | "Negative space, contrast, aperture, optical correction" | Does negative space carry meaning? |
| **G5 — Scalability** | "Size, legibility, composition" | Is it legible at 16px? At billboard scale? |

Bokhua's emphasis on **pencil-first iteration** is crucial: his strongest point is that formal research happens *before* the computer. For LLM generation, this translates to **prompt structure before pixel generation** — the brief and design parameters must be fully resolved before the first `generate_image` call.

### 3.2 Jinja2 Prompt: `logo-formal-prompt.j2` (KnowAct)

Bokhua's formal discipline encoded as a prompt template. This is the core generation prompt — every logo flow delegates to this.

```jinja2
# Logo Formal Prompt — Bokhua-Inspired Generation (KnowAct)
template_type: KnowAct
name: media_logo_formal_prompt
version: "1.0.0"
model: "{{ generation_model | default('FA/flux-pro/v1') }}"
parameters:
  - name: name
    type: string
    required: true
  - name: tagline
    type: string
    required: false
  - name: industry
    type: string
    required: false
  - name: style
    type: enum
    values: [minimal, modern, playful, corporate, vintage, luxury, tech, handmade, bold, elegant]
    default: minimal
  - name: logo_type
    type: enum
    values: [wordmark, lettermark, emblem, abstract, combination, mascot]
    default: wordmark
  - name: palette
    type: string
    required: false
  - name: dominant_shape
    type: enum
    values: [circle, square, triangle, organic, line-based, negative-space]
    required: false
  - name: density
    type: enum
    values: [sparse, balanced, dense]
    default: balanced
  - name: typography_class
    type: enum
    values: [serif, sans-serif, slab, script, display, geometric]
    required: false
  - name: generation_model
    type: string
    required: false
    description: "Image generation model (see §6 for selection guidance)"
---
Design a professional logo for a {% if industry %}{{ industry }}{% else %}business{% endif %} called "{{ name }}"{% if tagline %}, with the tagline "{{ tagline }}"{% endif %}.

## Design Parameters
- **Style:** {{ style }}
- **Logo type:** {{ logo_type }}
- **Dominant shape language:** {{ dominant_shape | default('derived from style') }}
- **Density:** {{ density }}
- {% if typography_class %}**Typography:** {{ typography_class }}{% endif %}
{% if palette %}**Color palette:** {{ palette }}{% endif %}

## Formal Requirements (Bokhua's Gates)

### G1 — Simplicity
- The mark must be describable in one sentence. If it requires explanation, it's too complex.
- Prefer one strong idea over three weak ones.

### G2 — Monochrome Viability
- The logo must work in pure black on pure white. Color is additive, not structural.
- If the mark collapses without color, the form is insufficient.

### G3 — Grid Discipline
- Key elements should align to geometric relationships: golden ratio, root-2 rectangles, or simple integer ratios.
- No accidental placement — every anchor point is intentional.

### G4 — Negative Space
- The space between elements is as designed as the elements themselves.
- Negative space should be active (carrying meaning) or neutral — never accidental.

### G5 — Scalability
- Legible at 16px icon size. Balanced at full-bleed billboard.
- No hairline strokes that vanish at small sizes. No details that muddle at scale.

## Technical Requirements
- Clean, scalable vector-style design
- No realistic photography or 3D rendering — flat or subtly shaded illustration
- Solid or gradient background preferred — no transparent/checkerboard
- No text artifacts, garbled letters, or misspelled words
- High contrast, legible at small sizes
- Professional, not clip-art

## Negative Constraints
- Do NOT reproduce any known trademarked logo
- Do NOT use photographic elements or photo-bashing
- Do NOT include UI elements, buttons, or interface chrome
- Do NOT produce multiple variations in a single image

Return a single logo image.
```

---

## 4. Iteration & Refinement (Peters' Layer)

### 4.1 The Case Study Method

Peters' *Logos That Last* is structured around detailed case studies that follow designs from concept to completion. His process emphasizes:

1. **Multiple candidates** — never present one option. Generate variants across different conceptual approaches.
2. **Client/audience feedback loops** — each iteration responds to specific critique, not general "make it better."
3. **Documentation of decisions** — why was one direction chosen over another? The rationale becomes part of the brand guidelines.
4. **Brand system thinking** — the logo is the seed; the brand system (colors, typography, pattern, photography style) grows from it.

For LLM-assisted generation, this translates into an **iterative critique-and-refine pipeline** with structured critique criteria and decision traceability.

### 4.2 YAML Manifest: `logo-iterative-refine.yaml` (FlowDef)

```yaml
# Logo Iterative Refinement — Peters-Inspired Pipeline (FlowDef)
template_type: FlowDef
name: media_logo_iterative_refine
version: "1.0.0"
description: "Generate N candidates, critique with structured criteria, select best, refine through critique cycles"
primary_model: "FA/flux-pro/v1"
critique_model: "OR/openai/gpt-4o"
parameters:
  # Identity inputs
  - name: name
    type: string
    required: true
  - name: tagline
    type: string
    required: false
  - name: industry
    type: string
    required: false
  - name: style
    type: enum
    values: [minimal, modern, playful, corporate, vintage, luxury, tech, handmade, bold, elegant]
    default: minimal
  - name: palette
    type: string
    required: false
  - name: logo_type
    type: enum
    values: [wordmark, lettermark, emblem, abstract, combination, mascot]
    default: wordmark
  # Iteration controls
  - name: variants
    type: integer
    default: 3
    description: "Number of initial candidates (1-5)"
  - name: refine_rounds
    type: integer
    default: 1
    description: "How many critique-and-refine cycles (0-3)"
  # Model selection
  - name: generation_model
    type: string
    required: false
    description: "Image generation model (see §6)"
  - name: critique_model
    type: string
    required: false
    description: "Vision LLM for critique (see §6)"
  # Design parameters (from discovery phase)
  - name: dominant_shape
    type: string
    required: false
  - name: density
    type: string
    required: false
  - name: typography_class
    type: string
    required: false
input_type: text
output_type: image_urls
latency: async
cost_tier: high
steps:
  - id: generate-candidates
    tool: generate_image
    description: "Generate {variants} initial logo candidates using the formal prompt template"
    template: media/logo-formal-prompt
    count: "{{ variants }}"

  - id: structured-critique
    tool: describe_image
    description: "Structured critique of each candidate against the five Bokhua gates plus Peters' brand-fit criteria"
    condition: "refine_rounds > 0"
    critique_dimensions:
      - id: g1-simplicity
        label: "Simplicity"
        weight: 0.25
        question: "Can this mark be described in one sentence? Is there a single clear idea?"
      - id: g2-monochrome
        label: "Monochrome Viability"
        weight: 0.15
        question: "Does the form hold up in pure black on white?"
      - id: g3-grid
        label: "Grid Discipline"
        weight: 0.10
        question: "Are key elements geometrically intentional?"
      - id: g4-negative-space
        label: "Negative Space"
        weight: 0.10
        question: "Is the negative space active or intentional?"
      - id: g5-scalability
        label: "Scalability"
        weight: 0.15
        question: "Is it legible at 16px? At large scale?"
      - id: p1-brand-fit
        label: "Brand Fit"
        weight: 0.15
        question: "Does this feel appropriate for the stated industry and audience?"
      - id: p2-distinctiveness
        label: "Distinctiveness"
        weight: 0.10
        question: "Is this memorable? Does it look like something else?"

  - id: select-best
    tool: internal
    description: "Select highest-scoring candidate across weighted dimensions"
    condition: "refine_rounds > 0"
    output: best_candidate_index

  - id: synthesize-critique
    tool: template_render
    description: "Synthesize critique into natural language refinement guidance"
    condition: "refine_rounds > 0"
    template: |
      Redesign this logo concept addressing the following critique.
      Strengths to preserve: {{ strengths_summary }}
      Weaknesses to fix: {{ weaknesses_summary }}
      Keep the same business name, industry, and style direction.
      Fix the identified weaknesses while preserving the strengths.

  - id: refine
    tool: generate_image
    description: "Regenerate best candidate incorporating synthesized critique feedback"
    condition: "refine_rounds > 0"

  - id: finalize
    tool: remove_background
    description: "Remove background from final logo for transparent PNG"
    condition: "refine_rounds > 0"
```

### 4.3 Critique Prompt Design

Peters' case study method requires *structured* critique, not vague feedback. The critique prompt uses weighted dimensions so the scoring is reproducible across rounds. Each dimension maps to a specific author's principle:

| Dimension | Source | Weight | Rationale |
|-----------|--------|--------|-----------|
| Simplicity | Bokhua G1 | 0.25 | The most predictive of logo longevity |
| Monochrome | Bokhua G2 | 0.15 | Structural test of form quality |
| Grid | Bokhua G3 | 0.10 | Geometric intent — harder for LLMs to judge |
| Negative Space | Bokhua G4 | 0.10 | Active space — also hard for LLM vision |
| Scalability | Bokhua G5 | 0.15 | Practical usability gate |
| Brand Fit | Martin | 0.15 | Does it match the brief? |
| Distinctiveness | Peters | 0.10 | Memorable vs. generic |

Total: 1.0. Bokhua's formal gates account for 0.75 of the weight — reflecting his thesis that formal quality is the foundation. Martin's strategic fit and Peters' distinctiveness are essential but secondary to formal soundness.

---

## 5. Presentation & Deliverables (All Three)

### 5.1 The Presentation Phase

All three authors agree: a good logo poorly presented fails. Bokhua devotes his final chapter to presentation and basic brand manuals. Peters emphasizes brand system extension. Martin's MVB includes context mockups.

The logo-builder skill should produce not just image files but a **logo package**:

| Deliverable | Tool | Description |
|-------------|------|-------------|
| Primary logo | `generate_image` | Full-color primary mark |
| Monochrome variant | `generate_image` + `palette: monochrome` | Black/white version |
| Icon-only mark | `generate_image` | Square cropped for favicon/app icon |
| Transparent PNG | `remove_background` | Final logo with alpha channel |
| Context mockup | `generate_image` | Logo applied to signage/business card |
| Brand card | template render | Palette hex codes, typography notes, rationale |

### 5.2 YAML Manifest: `logo-presentation.yaml` (FlowDef)

```yaml
# Logo Presentation — Deliverables Package (FlowDef)
template_type: FlowDef
name: media_logo_presentation
version: "1.0.0"
description: "Generate a complete logo deliverables package: primary, monochrome, icon, transparent, mockups"
parameters:
  - name: logo_url
    type: string
    required: true
    description: "URL of the final refined logo"
  - name: name
    type: string
    required: true
  - name: palette_hex
    type: array
    required: false
  - name: generation_model
    type: string
    required: false
steps:
  - id: remove-background
    tool: remove_background
    description: "Produce transparent PNG from final logo"
    input: "{{ logo_url }}"

  - id: monochrome-variant
    tool: generate_image
    description: "Generate monochrome version for single-color applications"
    prompt: "Monochrome (pure black on white) version of this logo: {{ logo_url }}. Same design, no color."

  - id: icon-mark
    tool: generate_image
    description: "Generate square icon-only version for favicon/app icon"
    prompt: "Square icon-only version of this logo mark: {{ logo_url }}. Remove all text, keep only the symbol/icon element. 1:1 aspect ratio. Works at 64x64px."

  - id: context-mockup
    tool: generate_image
    description: "Generate a realistic context mockup showing the logo in use"
    condition: "contexts defined"
    prompt: >
      Photorealistic mockup showing the logo "{{ name }}" applied in context.
      Show it on a {% if 'signage' in contexts %}storefront sign, {% endif %}
      {% if 'digital' in contexts %}website header, {% endif %}
      {% if 'print' in contexts %}business card, {% endif %}.
      Professional product photography style. Good lighting. No other brands visible.
```

---

## 6. LLM Model Selection

### 6.1 Generation Model Taxonomy

Different stages of the logo pipeline benefit from different models. The skill supports explicit model selection at each stage.

| Stage | Primary Model | Fallback | Why |
|-------|-------------|----------|-----|
| **Discovery (classification)** | `OR/openai/gpt-4o` | `DI/meta-llama/Llama-3.3-70B-Instruct` | Strong structured output, brand reasoning |
| **Generation (image)** | `FA/flux-pro/v1` | `FA/flux-dev` | Best text rendering, photorealistic quality |
| **Critique (vision)** | `OR/openai/gpt-4o` | `OR/anthropic/claude-sonnet` | Structured scoring + natural language critique |
| **Refinement (image)** | `FA/flux-pro/v1` | Same as generation | Consistency with initial generation style |

### 6.2 Model Selection Principles

1. **Classification tasks** (discovery mapping, critique scoring) → text/vision LLMs with strong reasoning and structured output support. GPT-4o preferred for its brand-strategy reasoning. Claude preferred when detailed visual analysis is needed.

2. **Image generation** → Flux Pro for text-heavy logos (Bokhua's wordmarks and lettermarks depend on accurate typography). Flux Dev or SDXL acceptable for abstract/pictographic marks where text accuracy matters less.

3. **Never cross providers within a refinement cycle** — if initial generation used Flux Pro, refinement must too, or the visual style will shift.

4. **Cost-awareness**: Discovery ($0.01-0.05) << Generation ($0.05-0.15/image) << Critique ($0.02-0.10/image) << Refinement (same as generation × rounds). A full iterative-refine cycle (3 variants × 2 rounds) costs approximately $0.50-1.50 in inference credits.

### 6.3 Model Override Syntax

All templates accept a `model` (KnowAct) or `generation_model`/`critique_model` (FlowDef) parameter. This flows through to the inference router using hKask's standard provider prefix convention:

```bash
# Use OpenRouter's GPT-4o for classification
DI/meta-llama/Llama-3.3-70B-Instruct  # DeepInfra
OR/openai/gpt-4o                        # OpenRouter

# Use Flux Pro for generation
FA/flux-pro/v1                          # fal.ai
```

Default: `FA/flux-pro/v1` for generation, `OR/openai/gpt-4o` for critique.

---

## 7. Integrated Skill Architecture

### 7.1 Template Inventory

| Template | Type | Purpose | Section |
|----------|------|---------|---------|
| `logo-discovery.yaml` | FlowDef | Identity mapping → design parameters | §2 |
| `logo-discovery-map.j2` | KnowAct | Brand-to-visual classification prompt | §2 |
| `logo-formal-prompt.j2` | KnowAct | Core generation prompt (Bokhua gates) | §3 |
| `logo-iterative-refine.yaml` | FlowDef | Candidate generation + critique loop | §4 |
| `logo-presentation.yaml` | FlowDef | Deliverables package | §5 |

### 7.2 Execution Flow

```
User provides: { name, industry, audience?, values? }
        │
        ▼
[logo-discovery.yaml] ──► logo-discovery-map.j2 ──► design parameters
        │
        ├── strategy: single-shot ──► logo-formal-prompt.j2 ──► generate_image
        │
        └── strategy: iterative-refine ──► logo-iterative-refine.yaml
                │
                ├── generate-candidates (×N)
                ├── structured-critique (×N)
                ├── select-best
                ├── refine (×R rounds)
                └── finalize
                        │
                        ▼
              [logo-presentation.yaml] ──► deliverables package
```

### 7.3 Design Decision Trace

Every Peters-inspired case study requires documenting *why* choices were made. The iterative-refine flow captures this automatically via CNS spans:

```
cns.media.logo.discovery    → brand attributes mapped to design parameters
cns.media.logo.critique     → per-candidate scores across 7 dimensions
cns.media.logo.selection    → which candidate was selected and why
cns.media.logo.refinement   → critique feedback applied, rounds completed
cns.media.logo.presentation → deliverables generated
```

---

## 8. Conclusion

LLM-assisted logo design can produce decoration or identity. The difference is **process**. Martin, Bokhua, and Peters each contribute an irreplaceable layer to that process:

- **Martin**: The brief. Without understanding who the mark is for, the most beautiful shape is meaningless.
- **Bokhua**: The form. Simplicity, grids, monochrome, negative space — these are not preferences but principles. They are what make a logo *last*.
- **Peters**: The process. Candidates, critique, refinement, documentation — these are what make a logo *defensible*.

The hKask Logo Builder skill encodes these three layers into a composable, model-flexible pipeline. The YAML manifests define *what happens and in what order*. The Jinja2 templates define *how the LLM thinks at each step*. Together they produce logos that are not merely generated, but *designed*.

---

## References

1. Martin, James. *Made By James* — People's Branding Mentor & Reputation Specialist. <https://themadebyjames.com/>
2. Bokhua, George. *Principles of Logo Design: A Practical Guide to Creating Effective Signs, Symbols, and Icons*. Rockport Publishers, 2022. ISBN 9780760376515.
3. Peters, Allan. *Logos That Last: How to Create Iconic Visual Branding*. <https://www.petersdesigncompany.com/book>
4. Müller-Brockmann, Josef. *Grid Systems in Graphic Design*. Niggli, 1981. (Referenced by Bokhua as foundational.)
5. Wong, Wucius. *Principles of Form and Design*. Wiley, 1993. (Referenced by Bokhua as foundational.)
