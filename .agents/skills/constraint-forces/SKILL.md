---
name: constraint-forces
visibility: public
description: "Classify constraints by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis) to decide which can be relaxed and which are inviolable. Maps directly to Magna Carta P1–P4 enforcement levels. Use when deciding whether a constraint can be relaxed, when constraints conflict, or when the user asks 'can I change this rule?'"
---

# Constraint Forces

A constraint classification system for deciding what can be relaxed and what cannot. Every statement about the system falls into one of five force types, ranked from strongest to weakest. When constraints conflict, higher rank wins. Never silently relax a Prohibition or Guardrail.

## The Five Forces

| Rank | Force | Ontology | Epistemic | Relaxable? | Magna Carta Mapping |
|------|-------|----------|-----------|------------|---------------------|
| 1 | **Prohibition** | OUGHT | Declarative | Never | P1 User Sovereignty — inviolable boundary |
| 2 | **Guardrail** | IS | Declarative | Only via explicit user override | P2 Affirmative Consent — deny by default |
| 3 | **Guideline** | OUGHT | Probabilistic | Yes, with reason stated | P3 Generative Space — user may configure |
| 4 | **Evidence** | IS | Probabilistic | Always informational | Supporting data, not enforced |
| 5 | **Hypothesis** | IS | Subjunctive | Always tentative | P4 Clear Boundaries — needs verification |

### What Each Force Means

**Prohibition** — An inviolable rule. Violating it breaks a Magna Carta principle. Example: "Episodic memory must never be exposed to other agents without explicit consent" (P1). Enforcement: OCAP capability gate, fail-closed.

**Guardrail** — A measured boundary. Crossing it triggers a CNS alert but the system doesn't prevent it autonomously — the user can override with affirmative consent. Example: "Variety deficit > 100 triggers algedonic alert" (CNS). Enforcement: CNS `cns.cybernetics.backpressure` span, Curator escalation.

**Guideline** — A best practice. Relaxing it is acceptable if the user understands the tradeoff and states the reason. Example: "Prefer local models over remote for sovereign data" (P3). Enforcement: None structural — user choice.

**Evidence** — A measured fact. Not enforced, but supports decisions. Example: "CNS variety counter shows 47 distinct tool invocations this session." Use it to inform, not to constrain.

**Hypothesis** — A speculative claim. Needs verification before acting on it. Example: "Memory growth may be due to embedding cache expansion." Always mark hypotheses explicitly.

## Classification Decision Tree

```
Statement about the system?
├── States an inviolable Magna Carta principle → Prohibition (Rank 1)
├── States a measured boundary (threshold, limit) → Guardrail (Rank 2)
├── States a best practice or preference → Guideline (Rank 3)
├── States a measurement or observation → Evidence (Rank 4)
└── States a possibility or projection → Hypothesis (Rank 5)
```

When unsure between two adjacent ranks, classify at the **stronger** rank. Misclassifying a Guardrail as a Guideline is more dangerous than misclassifying a Guideline as a Guardrail.

## Conflict Resolution

When two constraints conflict:

1. **Identify** both constraints and their force types.
2. **Rank**: Higher rank wins. Prohibition > Guardrail > Guideline > Evidence > Hypothesis.
3. **State** the conflict and resolution explicitly. Never silently ignore a constraint.
4. **Log** via CNS: emit a `cns.cybernetics.backpressure` span noting the conflict and which force prevailed.
5. **Never** relax Rank 1 (Prohibition) or Rank 2 (Guardrail) without the user's explicit, informed affirmative consent.

### Example Conflicts

| Conflict | Resolution |
|----------|------------|
| Prohibition says "no remote inference for sovereign data" but Guideline says "prefer best-available model" | Prohibition wins — sovereign data stays local |
| Guardrail says "variety deficit > 50 → Warning" but Guideline says "allow focused deep work" | Guardrail wins — escalate the warning, user can override |
| Guideline says "prefer SQLCipher" but Evidence shows "performance regression in encryption layer" | Guideline holds — but investigate the regression |
| Hypothesis says "probably a cache issue" but Evidence shows "heap growth correlates with embedding requests" | Evidence wins — update the hypothesis |

## Magna Carta Enforcement Levels

The five forces map to the four Magna Carta principles as enforcement tiers:

| Principle | Default Force | Override Path |
|-----------|--------------|---------------|
| P1 User Sovereignty | Prohibition | Constitutional change (not runtime) |
| P2 Affirmative Consent | Guardrail | User explicit consent via OCAP token |
| P3 Generative Space | Guideline | User configuration |
| P4 Clear Boundaries | Guardrail | OCAP token attenuation |

P1 is the only Prohibition-level principle. P2 and P4 are Guardrails because the user *can* override them through the consent mechanism — but the system never overrides them autonomously.

## Registry Templates

This skill's runtime templates live in `registry/templates/constraint-forces/`:

| Template | Type | Purpose |
|----------|------|--------|
| `constraint-forces-classify.j2` | KnowAct | Classify a constraint into its force type and Magna Carta mapping |
| `constraint-forces-resolve.j2` | KnowAct | Resolve a conflict between two constraints by force ranking |

The SKILL.md (this file) teaches the Zed coding agent the classification methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When to Use This Skill

- **Deciding whether to relax a constraint:** Check its force rank. If Rank 1 or 2, do not relax without user consent.
- **Constraints conflict:** Apply the resolution hierarchy. State the conflict explicitly.
- **Communicating certainty:** Mark each statement with its force type so the reader knows what's enforceable vs. tentative.
- **Writing code that enforces rules:** Prohibitions become OCAP gates (fail-closed). Guardrails become CNS spans (monitor + alert). Guidelines become defaults (user-configurable).
- **Auditing compliance:** Use `magna-carta-verifier` for formal verification; use this skill for quick classification during design and review.

## Quick Reference

Before stating a constraint, ask:
1. Is it inviolable? → Prohibition
2. Is it a measured boundary? → Guardrail
3. Is it a best practice? → Guideline
4. Is it a measurement? → Evidence
5. Is it speculative? → Hypothesis

Before relaxing a constraint, ask:
1. What is its force rank?
2. Is the user explicitly consenting?
3. Is the reason for relaxation stated?
4. Is a CNS span emitted for the override?