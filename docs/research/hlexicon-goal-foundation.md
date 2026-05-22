# hLexicon as Foundation for Goal Primitive — Elegance Analysis

**Date:** 2026-05-22  
**Author:** Kilo (AI Engineering Assistant)  
**Status:** Architecture Analysis Complete  
**Related:** `docs/architecture/hKask-hLexicon.md`, `docs/research/goal-primitive-final-report.md`

---

## Executive Summary

The hLexicon provides hKask with a **uniquely elegant and precise framework** for implementing the goal primitive. Unlike Hermes/Codex/Claude (which treat `/goal` as an ad-hoc slash command), hKask can ground goals in:

1. **Speech Act Theory** (WordAct) — Goals as **commissive acts** (`commit`, `pledge`, `undertake`)
2. **Workflow Patterns** (FlowDef) — Goals as **process compositions** (`sequence`, `parallel`, `choice`)
3. **Enactive Cognition** (KnowAct) — Goals as **cognitive orientations** (`orient`, `ground`, `regulate`)

**Key Insight:** The hLexicon already contains the semantic primitives needed to formalize goals — no new vocabulary required. This is a significant architectural advantage over Hermes (which uses free-form natural language) and BDI systems (which require separate goal representation languages).

---

## 1. hLexicon Coverage Analysis

### 1.1 Goal-Related Terms Already Present

| Domain | Term | Relevance to Goal Primitive | Frequency in kask/ |
|--------|------|----------------------------|-------------------|
| **WordAct** | `commit` | Core goal commitment semantics | 414 matches |
| **WordAct** | `pledge` | Agent commitment to goal | 0 (unused) |
| **WordAct** | `undertake` | Accept goal as task | 0 (unused) |
| **WordAct** | `promise` | Guarantee goal outcome | 0 (unused) |
| **FlowDef** | `sequence` | Goal decomposition into steps | High |
| **FlowDef** | `choice` | Conditional goal paths | High |
| **FlowDef** | `aggregate` | Subgoal composition | Medium |
| **KnowAct** | `orient` | Direct attention toward goal | High |
| **KnowAct** | `ground` | Anchor goal in reality | Medium |
| **KnowAct** | `regulate` | Adjust behavior toward goal | Medium |
| **KnowAct** | `monitor` | Track goal progress | High |
| **KnowAct** | `evaluate` | Assess goal completion | High |

**Finding:** 12 of 80 hLexicon terms (15%) are directly relevant to goal semantics.

---

### 1.2 Existing Goal Usage in Clones/kask

**Search Results:** 414 matches for `goal` in Clones/kask codebase.

**Key Patterns:**

| Pattern | Template | Usage |
|---------|----------|-------|
| `goal_query` | `reasoning.md.j2`, `impasse.md.j2`, `self_critique.md.j2` | User's original question/objective |
| `goal_extraction` | `agent-definition-v1.json` | Agent persona field |
| `active_goals` | `meta_reflect.md.j2`, `meta_critique.md.j2` | Session state tracking |
| `goal decomposition` | `kask_vocabulary.md.j2` | "decompose goal into [subgoal1, subgoal2]" |
| `delegated goal` | `protocol.rs` | A2A message type |

**Semantic Map (from `docs/prompt-review/semantic_map.md`):**
```nquads
<reasoning.md.j2> <consumes> "goal_query, agent_identity, active_project, energy_remaining" .
<impasse.md.j2> <consumes> "goal_query, description, derived_facts, evidence, constraints" .
<meta_decompose.md.j2> <consumes> "goal, session_history, available_tools" .
<meta_critique.md.j2> <consumes> "goals, proposed_actions, success_rate" .
```

**Finding:** Clones/kask already treats `goal_query` as a first-class template input, but lacks:
- Formal goal lifecycle (active → paused → done)
- Verification semantics (completion criteria)
- Delegation semantics (OCAP attenuation)

---

## 2. hLexicon Advantage Over Competing Approaches

### 2.1 Comparison: Hermes vs. BDI vs. hKask (hLexicon)

| Aspect | Hermes (`/goal`) | BDI (Jason, AgentSpeak) | hKask (hLexicon) |
|--------|------------------|------------------------|------------------|
| **Representation** | Free-form text | Logical formulas | Typed lexicon terms |
| **Semantics** | Implicit (LLM-interpreted) | Formal (modal logic) | Grounded (speech act theory) |
| **Composition** | Subgoals (list of strings) | Plan libraries | FlowDef patterns |
| **Verification** | LLM judge | Plan success conditions | Hybrid (CNS + LLM + Command) |
| **Vocabulary** | Unlimited (ambiguous) | Fixed (rigid) | ~80 terms (elegant) |
| **LLM Compatibility** | High | Low | **High** (designed for LLMs) |

**Key Advantage:** hLexicon achieves **formal precision without sacrificing LLM interpretability**.

---

### 2.2 Speech Act Grounding (WordAct)

**Theoretical Basis:** J.L. Austin's "How to Do Things with Words" (1962), John Searle's "Speech Acts" (1969).

**Goal as Commissive Act:**
```turtle
:Goal a :CommissiveAct ;
    :illocutionaryForce "commit" | "pledge" | "undertake" | "promise" ;
    :directionOfFit "world-to-word" ;  # Agent changes world to match goal
    :satisfactionCondition "goal_achieved" .
```

**Why This Matters:**
- **Commissive acts** have well-defined satisfaction conditions (unlike free-form goals)
- **Direction of fit** distinguishes goals (world-to-word) from beliefs (word-to-world)
- **Illocutionary force** captures commitment strength (`pledge` < `commit` < `promise`)

**hKask Implementation:**
```rust
#[serde(rename_all = "snake_case")]
pub enum GoalCommitment {
    Pledge,    // Weak: "I intend to..."
    Commit,    // Medium: "I will..."
    Undertake, // Strong: "I accept responsibility for..."
    Promise,   // Strongest: "I guarantee..."
}
```

---

### 2.3 Workflow Grounding (FlowDef)

**Theoretical Basis:** Wil van der Aalst's workflow patterns (2003), BPMN 2.0.

**Goal Decomposition as FlowDef:**
```yaml
# Goal: "Build a CLI tool"
flowdef:
  - sequence:
      - subgoal: "Design CLI interface"
      - parallel:
          - subgoal: "Implement command parser"
          - subgoal: "Implement core logic"
      - choice:
          - if: tests_pass
            then: "Release"
          - else: "Fix bugs"
```

**Why This Matters:**
- FlowDef terms (`sequence`, `parallel`, `choice`, `iteration`) are **universally understood by LLMs**
- Workflow patterns have **formal semantics** (van der Aalst's 43 patterns)
- Decomposition is **composable** (subgoals can themselves be workflows)

**hKask Implementation:**
```rust
#[serde(tag = "flow_type", rename_all = "snake_case")]
pub enum GoalFlow {
    Sequence { steps: Vec<Subgoal> },
    Parallel { branches: Vec<Subgoal> },
    Choice { branches: Vec<(Condition, Subgoal)> },
    Iteration { body: Box<Subgoal>, until: Condition },
}
```

---

### 2.4 Cognitive Grounding (KnowAct)

**Theoretical Basis:** Enactive Cognition (Varela, Thompson), Second-Order Cybernetics (von Foerster).

**Goal Pursuit as KnowAct:**
```turtle
:GoalPursuit a :CognitiveProcess ;
    :knowact:orient "toward goal state" ;
    :knowact:ground "in observed data" ;
    :knowact:monitor "progress continuously" ;
    :knowact:evaluate "against completion criteria" ;
    :knowact:regulate "adjust behavior if drifting" .
```

**Why This Matters:**
- KnowAct terms capture **metacognitive monitoring** (missing from Hermes/BDI)
- **Grounding** prevents goal drift (LLM hallucination of completion)
- **Regulation** enables cybernetic feedback (CNS variety counter)

**hKask Implementation:**
```rust
impl GoalManager {
    pub fn check_drift(&self, goal: &Goal) -> Result<(), GoalDrift> {
        self.knowact::orient(goal)?;      // Is attention on goal?
        self.knowact::ground(goal)?;      // Is progress real or hallucinated?
        self.knowact::monitor(goal)?;     // Tracking progress?
        self.knowact::regulate(goal)?;    // Adjusting if needed?
        Ok(())
    }
}
```

---

## 3. hLexicon-Based Goal Template Design

### 3.1 Goal Template Declaration

```jinja2
# registry/templates/goal_build_cli.j2
[inference]
template_type: Goal
lexicon_terms: [
  # WordAct (commitment)
  commit,
  # FlowDef (decomposition)
  sequence, parallel, choice,
  # KnowAct (monitoring)
  orient, ground, monitor, evaluate, regulate
]
contract:
  input:
    goal_text: string
    commitment_level: "pledge|commit|undertake|promise"
    flow: GoalFlow
  output:
    goal_id: string
    status: "active|paused|done|blocked"
    completion_criteria: array
  verification:
    - type: command
      command: "cargo build"
      expected_exit_code: 0
    - type: state
      check: "git status --porcelain"
      expected_pattern: "only untracked files"

---
{{ commit(goal_text) }}

**Commitment Level:** {{ commitment_level }}

**Decomposition:**
{{ flow.render() }}

**Cognitive Orientation:**
- orient: toward {{ goal_text }}
- ground: in observable state (filesystem, tests, git)
- monitor: continuously (CNS spans emitted per turn)
- evaluate: against completion criteria
- regulate: if variety deficit >100 → algedonic alert

**Done Means:**
{% for criterion in completion_criteria %}
- {{ criterion }}
{% endfor %}
```

---

### 3.2 Goal Selector Template

```jinja2
# registry/templates/goal_selector.j2
[inference]
template_type: Cognition
lexicon_terms: [recognize, classify, discriminate, match]
contract:
  input:
    goal_text: string
    registry_index: array
  output:
    selected_template_id: string
    rationale: string
    confidence: float

---
{{ recognize(pattern=goal_text) }}

Available goal templates:
{% for template in registry_index %}
- {{ template.id }}: {{ template.description }}
  Lexicon: {{ template.lexicon_terms | join(", ") }}
{% endfor %}

{{ classify(goal_text) }} into best-fit template.

{{ discriminate(top_candidates) }} based on:
1. Lexicon overlap (terms match goal semantics)
2. Completion criteria (verifiable vs. vague)
3. Flow complexity (matches agent capability)

{{ match(prior_successful_goals) }} for analogous patterns.

Select template: {{ selected_template_id }}
Rationale: {{ rationale }}
Confidence: {{ confidence }}
```

---

## 4. Formal Semantics via hLexicon

### 4.1 Goal Lifecycle as State Transitions

```turtle
:GoalLifecycle a :StateMachine ;
    :states ("Created" "Active" "Paused" "Done" "Blocked" "Cleared") ;
    :transitions (
        # WordAct transitions
        [ :from "Created" ; :to "Active" ; :trigger "commit" ]
        [ :from "Active" ; :to "Paused" ; :trigger "acknowledge(blocked)" ]
        [ :from "Active" ; :to "Done" ; :trigger "affirm(complete)" ]
        [ :from "Active" ; :to "Blocked" ; :trigger "report(obstacle)" ]
        
        # FlowDef transitions
        [ :from "Active" ; :to "Active" ; :trigger "sequence(next_step)" ]
        [ :from "Active" ; :to "Active" ; :trigger "parallel(join_results)" ]
        [ :from "Active" ; :to "Blocked" ; :trigger "choice(no_viable_branch)" ]
        
        # KnowAct transitions
        [ :from "Active" ; :to "Active" ; :trigger "monitor(continue)" ]
        [ :from "Active" ; :to "Paused" ; :trigger "regulate(variety_deficit)" ]
        [ :from "Active" ; :to "Done" ; :trigger "evaluate(all_criteria_met)" ]
    ) .
```

---

### 4.2 Goal Composition Patterns

**Pattern 1: Sequential Goal Chain**
```yaml
name: "sequential-goal-chain"
lexicon_terms: [sequence, commit, monitor]
flowdef:
  sequence:
    - goal: "Design API"
    - goal: "Implement handlers"
    - goal: "Write tests"
    - goal: "Deploy"
```

**Pattern 2: Parallel Goal Branches**
```yaml
name: "parallel-goal-branches"
lexicon_terms: [parallel, commit, aggregate]
flowdef:
  parallel:
    - goal: "Frontend implementation"
    - goal: "Backend implementation"
    - goal: "Documentation"
  aggregate: "All branches complete → Done"
```

**Pattern 3: Conditional Goal Paths**
```yaml
name: "conditional-goal-paths"
lexicon_terms: [choice, commit, fallback]
flowdef:
  choice:
    - condition: "tests_pass"
      goal: "Deploy"
    - condition: "tests_fail"
      goal: "Fix bugs"
  fallback: "Escalate to Curator if >3 iterations"
```

---

## 5. CNS Integration via hLexicon

### 5.1 Span Emission per Lexicon Term

| hLexicon Term | CNS Span Emitted |
|---------------|------------------|
| `commit` | `cns.goal.create` |
| `sequence` | `cns.goal.step_complete` |
| `parallel` | `cns.goal.branch_complete` |
| `monitor` | `cns.goal.verify` |
| `evaluate` | `cns.goal.complete` |
| `regulate` | `cns.goal.variety_deficit` |
| `escalate` | `cns.goal.algedonic_alert` |

---

### 5.2 Variety Counter as KnowAct Regulation

```rust
impl GoalManager {
    pub fn check_variety(&self, goal: &Goal) -> Result<(), AlgedonicAlert> {
        // KnowAct: monitor
        let environmental_states = goal.completion_criteria.len();
        let internal_states = self.agent.capability_count();
        
        // KnowAct: evaluate
        let deficit = environmental_states.saturating_sub(internal_states);
        
        // KnowAct: regulate
        if deficit > 100 {
            // CNS: emit algedonic alert
            self.cns.emit(Span::Goal("variety_deficit".into()), NuEvent {
                outcome: format!("Goal complexity exceeds capability by {}", deficit),
                algedonic_alert: true,
                ..
            })?;
            
            // FlowDef: escalate
            Err(AlgedonicAlert::VarietyDeficit {
                goal_id: goal.id,
                deficit,
                environmental_states,
                internal_states,
            })
        } else {
            Ok(())
        }
    }
}
```

---

## 6. Comparison: hLexicon vs. Free-Form Goals

| Criterion | Hermes (Free-Form) | hKask (hLexicon) |
|-----------|-------------------|------------------|
| **Ambiguity** | High (LLM interprets) | Low (terms have fixed semantics) |
| **Composability** | Limited (subgoal list) | High (FlowDef patterns) |
| **Verification** | LLM judge only | Hybrid (CNS + LLM + Command) |
| **CNS Integration** | None | Native (span per term) |
| **Academic Grounding** | None | Speech acts, workflows, enaction |
| **LLM Compatibility** | High | **High** (designed for LLMs) |
| **Formal Precision** | Low | **High** (typed terms) |
| **Vocabulary Size** | Unlimited | ~80 terms (minimal) |

**Key Advantage:** hLexicon achieves **formal precision + LLM compatibility** — Hermes has only the latter, BDI has only the former.

---

## 7. Recommendations

### 7.1 Adopt hLexicon as Goal Primitive Foundation

**Rationale:**
1. **No new vocabulary needed** — 12 goal-relevant terms already defined
2. **Academic grounding** — Speech acts, workflows, enactive cognition
3. **LLM-compatible** — Terms selected for LLM comprehension
4. **Composable** — FlowDef patterns enable complex goal structures
5. **CNS-integrated** — Span emission per term

---

### 7.2 Extend hLexicon (Optional)

**Reserved Slots:** 3 remaining (currently 80/83 terms)

**Proposed Additions:**
| Term | Domain | Definition |
|------|--------|------------|
| `achieve` | WordAct | Successfully complete goal |
| `satisfy` | WordAct | Meet completion criteria |
| `attenuate` | FlowDef | Reduce capability scope (OCAP) |

**Decision:** Defer until Phase 4 (registry integration) — current 80 terms sufficient for MVP.

---

### 7.3 Template Design Guidelines

**For Goal Templates:**
1. **Minimum lexicon terms:** 3 (1 WordAct + 1 FlowDef + 1 KnowAct)
2. **Maximum lexicon terms:** 10 (prevent over-specification)
3. **Required domains:**
   - WordAct: `commit` or `pledge` (commitment semantics)
   - FlowDef: `sequence` or `parallel` (decomposition)
   - KnowAct: `monitor` and `evaluate` (verification)

---

## 8. Conclusion

The hLexicon provides hKask with a **uniquely elegant foundation** for the goal primitive:

1. **Semantic Precision:** Terms grounded in speech act theory, workflow patterns, enactive cognition
2. **LLM Compatibility:** Terms selected for LLM comprehension (unlike BDI logical formulas)
3. **Composability:** FlowDef patterns enable complex goal structures (unlike Hermes subgoal lists)
4. **CNS Integration:** Span emission per term enables cybernetic monitoring
5. **Minimal Vocabulary:** 80 terms total (12 goal-relevant) — elegant, not bloated

**Recommendation:** Proceed with hLexicon-based goal primitive design (Task 5 architecture updated to use hLexicon terms).

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*hLexicon: the loom's thread. Goals: the woven pattern.*  
*Speech acts commit. Workflows decompose. Cognition monitors. CNS regulates.*