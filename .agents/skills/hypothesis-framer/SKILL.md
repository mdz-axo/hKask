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

## The Process (Kata-Style PDCA)

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

### Step 3: Formulate Hypothesis (Do)
Derive testable hypotheses from the structured question:
- **Research hypothesis (H₁/Hₐ)**: A declarative statement predicting the expected outcome. Directional when appropriate ("will decrease" > "will differ"). Must be falsifiable.
- **Null hypothesis (H₀)**: Restates to postulate no difference/no relationship. This is what the statistical test directly evaluates.

Classify the hypothesis type: difference, association, superiority, non-inferiority, equivalence, diagnostic accuracy, or prognostic. Select the appropriate statistical framework. Assess testability on six dimensions including measurability and sample size implications.

### Step 4: Develop Aims & Objectives (Check)
Operationalize the hypothesis into research aims and objectives:
- **Primary aim**: Broad, overarching purpose — "to determine/evaluate/compare..."
- **Primary objectives**: Specific, measurable steps to accomplish the aim (2–4 objectives)
- **Secondary aims**: Related but distinct questions with rationale (not "fishing expeditions")
- **Alignment check**: Five-link chain — question → hypothesis → aim → objectives → outcome → null
- **Feasibility recheck**: Does the operational detail introduce new resource constraints?

### Step 5: Convergence Check (Act)
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
| "Is my research question any good?" | Run through FINER → PICO → hypothesis → aims flow |
| "Help me write a hypothesis" | Provide the structured PICO question, use Step 3 |
| "I need to develop study aims and objectives" | Run the full flow, focus on Step 4 |
| "Does my hypothesis actually test my question?" | Run the alignment check in Step 4 |
| "My grant proposal needs a research plan section" | Run full flow for structured output |
| "I'm stuck — my question keeps changing" | The iterative PDCA loop refines across cycles |

## When NOT to Use

- **Study already designed**: FINER/PICO/hypothesis are pre-study tools — applying them post hoc is circular
- **Purely exploratory work**: If you genuinely have no prediction (hypothesis-generating research), some steps don't apply — but you should still evaluate FINER and structure with PICO
- **Qualitative research**: PICO is optimized for quantitative intervention studies; adapt the population/outcome elements for qualitative designs
- **Meta-research/systematic reviews**: These have their own frameworks (PICOS, PRISMA); FINER still applies

## Cybernetic Perspective

The process implements a cybernetic feedback loop:
- **Sensor**: FINER evaluation detects weaknesses in the question
- **Model**: PICO structures the question into testable components
- **Regulator**: Hypothesis formulation makes a falsifiable prediction — the key regulatory mechanism of science
- **Actuator**: Aims and objectives operationalize the prediction into measurable actions
- **Variety engineering**: FINER attenuates the space of possible questions; PICO amplifies structure; the hypothesis is the Good Regulator's model of expected reality

The convergence check closes the loop — if question, hypothesis, and aims are misaligned, the regulator's model doesn't match the system being regulated. Iterate until coherence.

## Anti-Patterns

1. **Hypothesis-fitting**: Changing the hypothesis after seeing data to match results — this is scientific fraud, not refinement
2. **"Yes/no" questions**: "Does X work?" is not researchable — the question must open inquiry
3. **Vague population**: "Patients with disease X" without inclusion/exclusion criteria is too broad to sample
4. **Missing comparator**: Without a comparison group, you can't test most hypotheses — acknowledge this as a design limitation
5. **Unmeasurable outcomes**: "Improved quality of life" without a validated instrument is not testable
6. **Over-narrowing**: A question so specific it becomes trivial loses relevance
7. **Aims as objectives**: "The aim is to measure X" — that's an objective. Aims are broader.
8. **Post hoc FINER**: Using FINER to justify a question you've already committed to, rather than evaluating it honestly

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `finer-evaluate.j2` | KnowAct | Apply FINER criteria to evaluate and refine a broad research topic |
| `pico-structure.j2` | KnowAct | Apply PICO framework to structure the research question |
| `hypothesis-formulate.j2` | KnowAct | Derive testable hypothesis and null hypothesis from PICO question |
| `aims-objectives.j2` | KnowAct | Develop research aims, objectives, alignment check, feasibility recheck |
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
- **rJoule (inference energy):** cap 28000 rJ, 0.25 rJ/token
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
