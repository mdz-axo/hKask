---
name: hypothesis-framer
visibility: public
description: >
  Research question framing and hypothesis formulation using FINER criteria
  and PICO process. Evaluates broad research topics through Feasibility,
  Interest, Novelty, Ethics, and Relevance (FINER) gates, structures questions
  via Population-Intervention-Comparison-Outcome (PICO) framework, derives
  testable hypotheses with null hypothesis formulation, and develops aligned
  research aims and objectives. Use when the user wants to formulate a research
  question, develop a testable hypothesis, frame a study protocol, or says
  'help me frame my research idea', 'is my research question good', 'write a
  hypothesis', or 'develop study aims'.
activation: "frame my research"
---

# Hypothesis Framer

Formulate and refine research questions and testable hypotheses using the FINER criteria and PICO process. Based on Willis (2023), Respiratory Care 68(8):1180–1185.

## Philosophy

Poor study design leads to fatal flaws in research methodology. The most effective prevention is rigorous question formulation before study initiation. The research question is the foundation; the hypothesis is derived from it. Both must be developed prior to any data collection, and neither should be changed post hoc to fit data.

The FINER criteria (Feasible, Interesting, Novel, Ethical, Relevant) provide a quality gate for research questions. The PICO framework (Population, Intervention, Comparison, Outcome) provides structure. Together they transform a broad topic into a testable hypothesis — moving from "I'm interested in X" to "In P, does I compared to C affect O? I predict that I will improve O."

## The Process (4-Step Kata-Style PDCA)

### Step 1: Evaluate with FINER (Plan)
Apply the five FINER criteria to a broad research topic:
- **Feasible**: Subjects, expertise, resources, support — can you actually do this?
- **Interesting**: Would your audience care? Does it address a problem worth solving?
- **Novel**: Does it fill a knowledge gap, improve on flawed methods, or confirm an important finding?
- **Ethical**: IRB requirements, risk-benefit ratio, informed consent — is it approvable?
- **Relevant**: Would results change practice? Does it generate new knowledge with clinical impact?

Each dimension scored 0–10 with rationale. The lowest-scoring dimension is the most concerning — refine the question to address it. Questions answerable with "yes/no" are generally not researchable.

### Step 2: Structure with PICO (Plan continued)
Decompose the research question into four elements:
- **P**opulation: Condition, demographics, setting, inclusion/exclusion criteria
- **I**ntervention: Treatment, diagnostic test, exposure, program — what will be done?
- **C**omparison: Placebo, standard of care, alternative, or no comparison (descriptive/observational)
- **O**utcome: Primary outcome with measurement method, timing, and clinical significance

Produce a structured question: "In [P], does [I] compared to [C] affect [O]?" Adapt the template for diagnostic, prognostic, or etiological studies. Assess each element's completeness — missing elements are blockers.

### Step 3: Hypothesis + Operationalize (Do)
A single cognitive act — "What do I predict, and how will I test it?" — in five parts:

**Formulate**: Derive the research hypothesis (H₁, a declarative prediction) and null hypothesis (H₀, postulating no difference). Classify hypothesis type (difference, association, superiority, non-inferiority, equivalence, diagnostic accuracy, or prognostic).

**Operationalize**: Write a primary aim (broad purpose), 2–4 primary objectives (specific, measurable steps), and optional secondary aims with rationale.

**Assess testability**: Now that objectives are specified, evaluate whether the hypothesis is actually testable — measurable outcome, specified population, defined comparison, suggested statistical framework, clinically meaningful effect size, sample size feasibility.

**Verify alignment**: Five-link chain — question → hypothesis → primary aim → objectives → PICO outcome → null hypothesis. Flag any misalignments.

**Recheck feasibility**: Does the operational detail introduce new resource constraints not visible at the FINER stage?

### Step 4: Convergence Check (Act)
Weighted dimensional evaluation (0.25 each):
1. FINER compliance — all scores ≥ 6?
2. PICO completeness — all elements present and specified?
3. Hypothesis coherence — testable, directional, null correctly negates?
4. Aims alignment — five-link chain intact?

Convergence metric: 0 = decision-ready. Specific blockers enumerated for each dimension. Iterate the PDCA cycle (up to 3 iterations) until the question-hypothesis-aims chain is coherent.

## When to Use

| Trigger | Action |
|---------|--------|
| "I have a research idea but don't know how to frame it" | Start with FINER evaluation to refine the broad topic |
| "Is my research question any good?" | Run through FINER → PICO → Hypothesis → Convergence |
| "Help me write a hypothesis" | Provide the structured PICO question, run from Step 3 |
| "I need to develop study aims and objectives" | Same as above — hypothesis and aims are now one step |
| "Does my hypothesis actually test my question?" | The alignment check in Step 3 catches this |
| "My grant proposal needs a research plan section" | Run full flow for structured output |
| "I'm stuck — my question keeps changing" | The iterative PDCA loop refines across cycles |

## When NOT to Use

- **Study already designed**: FINER/PICO/hypothesis are pre-study tools — applying them post hoc is circular
- **Purely exploratory work**: If you genuinely have no prediction (hypothesis-generating research), some steps don't apply — but you should still evaluate FINER and structure with PICO
- **Qualitative research**: PICO is optimized for quantitative intervention studies; adapt the population/outcome elements for qualitative designs
- **Meta-research/systematic reviews**: These have their own frameworks (PICOS, PRISMA); FINER still applies

## Cybernetic Perspective

The process maps to a cybernetic feedback loop: FINER (sensor) → PICO (model) → Hypothesis (regulator) → Aims (actuator) → Convergence (closure). Variety engineering: FINER attenuates the question space; PICO amplifies structure. The convergence check ensures the regulator's model matches the system being regulated.

## Anti-Patterns

1. **Hypothesis-fitting**: Changing the hypothesis after seeing data to match results — this is scientific fraud, not refinement
2. **"Yes/no" questions**: "Does X work?" is not researchable — the question must open inquiry
3. **Vague population**: "Patients with disease X" without inclusion/exclusion criteria is too broad to sample
4. **Missing comparator**: Without a comparison group, you can't test most hypotheses — acknowledge this as a design limitation
5. **Unmeasurable outcomes**: "Improved quality of life" without a validated instrument is not testable
6. **Over-narrowing**: A question so specific it becomes trivial loses relevance
7. **Aims as objectives**: "The aim is to measure X" — that's an objective. Aims are broader.
8. **Post hoc FINER**: Using FINER to justify a question you've already committed to, rather than evaluating it honestly

## Design Rationale: Why 4 Steps Not 5

The original 5-step design had separate templates for hypothesis formulation and aims/objectives. This was refined to 4 steps because:

- **Testability depends on objectives**: You cannot assess whether a hypothesis is testable until you know the objectives — so testability assessment belongs alongside objectives, not before them
- **Alignment is a single concern**: The five-link alignment chain evaluates the relationship between hypothesis and aims — separating them into different inference calls fragments this evaluation
- **Shared context**: Both hypothesis and aims reference the same PICO elements and FINER context — passing them across separate inference calls duplicates token cost for no analytical benefit
- **Kata fit**: The 4-step structure maps cleanly to Plan (FINER + PICO) → Do (Hypothesis + Operationalize) → Check (Convergence) → Act (Loop)

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `finer-evaluate.j2` | KnowAct | Apply FINER criteria to evaluate and refine a broad research topic |
| `pico-structure.j2` | KnowAct | Apply PICO framework to structure the research question |
| `hypothesis-operationalize.j2` | KnowAct | Derive hypothesis, formulate null, operationalize into aims and objectives, verify alignment |
| `hypothesis-framer-convergence-check.j2` | KnowAct | Compute weighted convergence metric across all four dimensions |

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/hypothesis-framer.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.10 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = question-hypothesis-aims chain is fully coherent and decision-ready

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 (manifest `rjoule.cap` — see `registry/manifests/hypothesis-framer.yaml` for canonical value)
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
