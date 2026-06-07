---
name: skill-translator
visibility: public
description: "Translate agent skills between different format systems. Analyze source format conventions (manifest type, execution model, state model, consent model) and produce target-format output. Use when adapting skills from other projects, converting between skill system versions, or when the user says 'translate this skill' or 'convert this skill'."
---

# Skill Translator

Translate agent skills between format systems by analyzing source conventions and producing target-format output. The key insight: cognitive work (reasoning, decision-making) translates to instructions; deterministic work (validation, measurement) translates to tool-callable procedures.

## Format Analysis Framework

Before translating, characterize both systems on these dimensions:

| Dimension | What to Identify | Translation Impact |
|-----------|-----------------|-------------------|
| **Manifest type** | How is the skill declared? (YAML, markdown frontmatter, JSON, etc.) | Determines structural mapping |
| **Execution model** | How does the skill run? (REPL injection, on-demand loading, orchestration steps) | Cognitive steps → instructions; deterministic steps → procedures |
| **State model** | How does the skill track state? (conversation context, step ordinals, explicit schemas) | State passing → instruction sequencing |
| **Consent model** | How are mutations authorized? (consent gates, OCAP, none) | Map consent mechanisms between systems |
| **Observability** | How are actions traced? (spans, events, logs, none) | Map observability to target's equivalent |
| **Domain references** | What system-specific concepts are referenced? | Identify what needs re-mapping vs. what drops |

## Step Classification Algorithm

When translating a step or instruction from source to target:

### Cognitive steps → Instructions in SKILL.md body

- Steps that render templates, evaluate answers, or make decisions → Convert to instructive sections that teach the agent the methodology
- Steps that select among options (escalate/hold/reprobe) → Convert to decision criteria sections
- Steps that emit events → Drop if target has its own observability; note the span namespace in references

### Deterministic steps → Tool-callable procedures

- Steps that validate schemas, check state, or compute metrics → Convert to step-by-step instructions the agent follows using built-in tools (terminal, grep, read_file)
- Steps that mutate state → Convert to guarded instructions with consent/OCAP gates noted

### Ambiguous steps

- Steps using a fast/cheap model tier → Likely deterministic-style work delegated to LLM. Convert to instructions with a note about original model tier
- When unsure → Classify as cognitive. It is safer for the agent to reason through an ambiguous step than to automate it incorrectly.

## Translation Mapping: Common Source → hKask Target

| Source Concept | hKask Equivalent | Notes |
|---------------|-----------------|-------|
| Manifest `id` | SKILL.md frontmatter `name` | Must be lowercase-hyphenated, match directory name |
| Manifest `description` | SKILL.md frontmatter `description` | Must be 1–1024 chars, specific and actionable |
| Manifest `symptoms` | Natural-language phrases in `description` | hKask matches by description, not symptom catalog |
| Manifest `probes` | Step-by-step instructions using built-in tools | Terminal commands, file reads, grep searches |
| Manifest `interventions` | Guarded instructions with consent notes | Note OCAP requirements for mutations |
| Manifest `safety.max_auto_risk` | Constraint-force classification in description | "Prohibition" = never auto-execute; "Guardrail" = user consent required |
| KNOWLEDGE.md content | SKILL.md body (after frontmatter) | Rewrite from source persona voice to imperative voice |
| `agent_persona.yaml` | Dropped | hKask agent identity comes from session, not skill |
| `hlexicon.yaml` | Dropped | hKask has its own hLexicon in the registry |
| `scripts/*.sh` | Inline procedure instructions | Agent uses built-in tools instead of shell scripts |
| Source persona voice ("I do X") | Imperative voice ("Do X") | Strip personality, keep methodology |

## What Gets Lost in Translation

Some source concepts have no hKask equivalent. Document every drop:

| Dropped Concept | Reason | Mitigation |
|----------------|--------|-----------|
| Energy/token budgets | hKask has gas budgets in CNS but not at skill level | Note in references: "Original had energy cap per step" |
| Step ordinals | hKask skills are loaded whole, not step-by-step | Convert ordinal flow to section structure |
| Symptom catalog integration | hKask matches by description, not catalog | Encode key symptoms as description phrases |
| Script-based probes | Agent uses built-in tools | Rewrite as tool-callable procedures |
| Source-specific consent gates | hKask uses OCAP | Map to OCAP-gated instructions |
| Source-specific observability | hKask uses CNS spans | Map to `cns.*` namespace references |

These are not failures — they are design differences. The translation preserves the **methodology** (what to do and why) while adapting the **mechanism** (how to do it).

## Translation Workflow

When the user asks to translate a skill:

1. **Analyze** — Read the source skill. Characterize its format on the six dimensions above.
2. **Classify** — Walk through each section/step. Classify as cognitive or deterministic.
3. **Map** — For each element, identify the target equivalent (or note the drop).
4. **Draft** — Write the SKILL.md with frontmatter and body. Use imperative voice. Include only what survives translation.
5. **Validate** — Check the output:
   - Frontmatter: `name` is lowercase-hyphenated, matches directory name, `description` is 1–1024 chars
   - Body: All cognitive steps are represented as instructions
   - Body: All deterministic steps are represented as tool-callable procedures
   - No source-specific terminology that hKask doesn't understand (unless explained)
   - No dropped concepts without documentation in references section
6. **Review** — Present to user with a translation summary: what was preserved, what was adapted, what was dropped.

## Domain Substitution Table

When the source references system-specific concepts, substitute with hKask equivalents:

| Source Domain | hKask Equivalent |
|--------------|-----------------|
| Journal / log store | hkask-storage (SQLite + SQLCipher) |
| Sentinel / sensor | MCP tool dispatch (`cns.tool.*`) |
| Baselines / EWMA | CNS variety counters |
| Nurse / regulator | Curator Agent |
| Proprioception | CNS algedonic signals (`cns.cybernetics.*`) |
| IDRS / consent gate | OCAP capability delegation |
| Manifest.yaml | SKILL.md frontmatter |
| KNOWLEDGE.md | SKILL.md body |
| Probe scripts | Built-in tool procedures |
| Intervention scripts | Guarded instructions |

## Safety

The translator refuses to translate:
- Skills with arbitrary code execution steps (no `action: execute` equivalents)
- Skills with unresolvable references (missing templates, broken paths)
- Skills that require network access the target system doesn't have
- Skills that violate Magna Carta principles (e.g., exposing episodic memory without consent)

## When to Use This Skill

- **"Translate this skill" / "Convert this skill":** Run the full workflow.
- **"Can I use this [external] skill in hKask?":** Analyze format, classify steps, assess feasibility.
- **Adapting skills from other projects:** Apply domain substitution for hKask-specific references.
- **Evaluating a skill from a different format system:** Use the format analysis framework to identify what would need to change.