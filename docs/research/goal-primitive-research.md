---
title: "Goal Primitive Research — Consolidated Report"
audience: [architects, developers]
last_updated: 2026-05-24
togaf_phase: "E"
version: "1.0.0"
status: "Active"
domain: "Application"
---

# Goal Primitive Research — Consolidated Report

**Date:** 2026-05-22  
**Status:** ✅ Research Complete — Ready for Implementation  
**Original Files:** `goal-primitive-analysis.md`, `goal-primitive-final-report.md`, `hlexicon-goal-foundation.md` + 7 task reports (archived)

---

## Executive Summary

The `/goal` primitive represents a **fundamental shift from prompting to assigning** — agents transition from one-shot commands to persistent objectives with defined completion criteria. This research analyzed implementations in Claude Code, OpenAI Codex CLI, and Hermes Agent, and evaluated whether hKask should formalize a shared understanding of "goal."

**Decision:** **ADAPT** (not adopt or reject) — hKask should implement a native goal primitive with:
1. **OCAP-Gated Delegation** — Capability tokens with attenuation
2. **CNS Monitoring** — Variety counters, algedonic alerts
3. **Hybrid Verification** — CNS + LLM + Bot verifiers
4. **Registry Routing** — `template_type: Goal` discriminator
5. **hLexicon Grounding** — Speech act theory (WordAct), workflow patterns (FlowDef), enactive cognition (KnowAct)

**Implementation Cost:** ~2,050 LOC

---

## Part 1: What /goal Actually Is

### The Primitive Shift

| Paradigm | User Role | Agent Role | Completion |
|----------|-----------|------------|------------|
| **Prompting** | Steers every turn | Responds to last message | Implicit (user decides) |
| **/goal** | Defines "done" once | Works autonomously toward target | Explicit (verifier decides) |

**Example:**
```
/goal Build the app described in SPEC.md. Done means tests pass, build passes, 
README is accurate, and git status only shows relevant project files.
```

### Hermes Agent Implementation

**Source:** `hermes_cli/goals.py` (PR #18262, May 2026) — ~650 LOC

**Core Components:**
```python
@dataclass
class GoalState:
    goal: str                        # Free-form objective
    status: str                      # active | paused | done | cleared
    turns_used: int                  # Budget tracking
    max_turns: int                   # Default: 20
    subgoals: List[str]              # Mid-loop criteria additions
    consecutive_parse_failures: int  # Judge model health
```

**Key Mechanisms:**
1. **Judge Model** — Auxiliary LLM evaluates after each turn (fail-open design)
2. **Continuation Loop** — User submits → Agent executes → Judge evaluates → Repeat
3. **Session Persistence** — Goal state stored in `SessionDB.state_meta`
4. **Commands:** `/goal`, `/goal status`, `/goal pause/resume`, `/subgoal`

**Known Bug (Issue #27585):** Judge API errors cause spam loops when agent reaches terminal response. Fix: Auto-pause on judge error + terminal phrase detection.

---

## Part 2: Academic Research on Goal Primitives

### BDI (Belief-Desire-Intention) Architecture

**Historical Foundation:** Bratman (1987), Rao & Georgeff (1991), modern implementations (Jason, AgentSpeak, GOAL, Can)

**Goal Lifecycle:** `Pending → Active → [Suspended | Aborted | Successful]`

**Key Distinctions:**
| Type | Description | Example |
|------|-------------|---------|
| **Achievement Goal** | State to bring about | `on(A, B)` |
| **Maintenance Goal** | State to preserve | `¬exceed_gradient(5%)` |
| **Declarative Goal** | Goal "to be" (belief-independent) | `¬believes(goal_achieved)` |
| **Procedural Goal** | Goal "to do" (plan-triggered) | `respond_to_event(e)` |

### Recent Advances (2020-2026)

1. **Temporally Extended Goals** (IJCAI 2024) — Mix reachability + invariant properties
2. **Goal-Conditioned RL** (AAAI 2026) — First-order representation languages, Hindsight Experience Replay
3. **Goal Representation Learning** (2025-2026) — Action-sufficient representations, dual goal representations

### Formal Specification Languages

| Language | Approach | Relevance |
|----------|----------|-----------|
| **Prism** (arXiv 2025) | Compositional metalanguage | Grammar-like specifications |
| **AgentSPEX** (arXiv 2026) | YAML syntax | Explicit workflow control |
| **FALAA** (Springer 2026) | UML + OCL | Formal agent architecture specs |

---

## Part 3: hLexicon as Foundation for Goal Primitive

### hLexicon Coverage Analysis

The hLexicon provides hKask with a **uniquely elegant framework** for implementing goals. Unlike Hermes (free-form text) or BDI (logical formulas), hLexicon grounds goals in:

1. **Speech Act Theory (WordAct)** — Goals as **commissive acts** (`commit`, `pledge`, `undertake`)
2. **Workflow Patterns (FlowDef)** — Goals as **process compositions** (`sequence`, `parallel`, `choice`)
3. **Enactive Cognition (KnowAct)** — Goals as **cognitive orientations** (`orient`, `ground`, `regulate`)

### Goal-Related Terms Already Present

| Domain | Term | Relevance | Frequency |
|--------|------|-----------|-----------|
| **WordAct** | `commit` | Core goal commitment | 414 matches |
| **WordAct** | `pledge` | Agent commitment | 0 (unused) |
| **FlowDef** | `sequence` | Goal decomposition | High |
| **FlowDef** | `choice` | Conditional paths | High |
| **KnowAct** | `orient` | Direct attention | High |
| **KnowAct** | `monitor` | Track progress | High |
| **KnowAct** | `evaluate` | Assess completion | High |

**Finding:** 12 of 80 hLexicon terms (15%) are directly relevant to goal semantics.

### hLexicon Advantage Over Competing Approaches

| Aspect | Hermes | BDI | hKask (hLexicon) |
|--------|--------|-----|------------------|
| **Representation** | Free-form text | Logical formulas | Typed lexicon terms |
| **Semantics** | Implicit (LLM) | Formal (modal logic) | Grounded (speech acts) |
| **Composition** | Subgoals (strings) | Plan libraries | FlowDef patterns |
| **Verification** | LLM judge | Plan success | Hybrid (CNS+LLM+Command) |
| **Vocabulary** | Unlimited (ambiguous) | Fixed (rigid) | ~80 terms (elegant) |
| **LLM Compatibility** | High | Low | **High** (designed for LLMs) |

**Key Advantage:** hLexicon achieves **formal precision without sacrificing LLM interpretability**.

---

## Part 4: hKask Current Architecture Analysis

### Existing Goal-Related Concepts

**CNS ν-Event Lifecycle:**
```rust
pub struct NuEvent {
    pub span: Span,              // cns.tool.*, cns.prompt.*, cns.agent_pod.*
    pub phase: Phase,            // Observe → Compare → Act
    pub outcome: Value,
    pub confidence: f64,
    pub algedonic_alert: bool,
}
```

**Lifecycle:** Observation → Regulation (Comparator checks against **goal**) → Outcome

**Variety Counter:**
```rust
pub struct VarietyCounter {
    environmental_states: usize,
    internal_states: usize,
    deficit_threshold: u64,  // Default: 100
}
```

### Gaps Identified

| Concept | Hermes | hKask Current | Gap |
|---------|--------|---------------|-----|
| **Goal Persistence** | SessionDB | None | Goals not first-class |
| **Completion Verification** | Judge LLM | CNS ν-event (undefined) | Comparator undefined |
| **Budget Tracking** | `max_turns: 20` | None | No turn/energy budget |
| **Goal Delegation** | N/A | Capability tokens | No goal-specific attenuation |
| **Goal Routing** | Manual | Unified registry | No `template_type: Goal` |

---

## Part 5: Proposed hKask Goal Primitive Design

### Database Schema

```sql
CREATE TABLE goals (
    id              UUID PRIMARY KEY,
    session_id      UUID NOT NULL,
    owner_webid     TEXT NOT NULL,
    goal_text       TEXT NOT NULL,
    template_ref    TEXT,
    status          TEXT NOT NULL,  -- active|paused|done|cleared|blocked
    turns_used      INTEGER DEFAULT 0,
    energy_budget   INTEGER,
    max_turns       INTEGER DEFAULT 20,
    created_at      INTEGER NOT NULL,
    visibility      TEXT NOT NULL DEFAULT 'private',
    INDEX idx_session (session_id),
    INDEX idx_owner (owner_webid),
    INDEX idx_status (status)
);

CREATE TABLE goal_completion_criteria (
    goal_id     UUID NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    ordinal     INTEGER NOT NULL,
    criterion_type TEXT NOT NULL,  -- command|state|semantic
    criterion_data TEXT NOT NULL,
    PRIMARY KEY (goal_id, ordinal)
);

CREATE TABLE goal_verifications (
    id              UUID PRIMARY KEY,
    goal_id         UUID NOT NULL REFERENCES goals(id),
    nu_event_id     UUID REFERENCES nu_events(id),
    verdict         TEXT NOT NULL,  -- done|continue|blocked
    reason          TEXT,
    confidence      REAL,
    verified_at     INTEGER NOT NULL,
    INDEX idx_goal (goal_id)
);
```

### Goal Lifecycle State Machine

```
Created → Active → [Paused | Delegated | Done | Blocked]
         ↑        ↓
         └────────┘ (resume/return/resolve)
```

### CNS Integration

**New Spans:**
- `cns.goal.create` — Goal instantiation
- `cns.goal.verify` — Post-turn verification
- `cns.goal.complete` — Completion detection
- `cns.goal.delegate` — Capability-attenuated handoff
- `cns.goal.block` — Escalation to Curator/human
- `cns.goal.variety_deficit` — Algedonic alert

**Variety Counter Integration:**
```rust
impl GoalManager {
    pub fn check_variety(&self, goal: &Goal) -> Result<(), AlgedonicAlert> {
        let environmental_states = goal.estimate_complexity();
        let internal_states = self.agent.capability_count();
        let deficit = environmental_states.saturating_sub(internal_states);
        
        if deficit > 100 {
            Err(AlgedonicAlert::VarietyDeficit { goal_id: goal.id, deficit })
        } else { Ok(()) }
    }
}
```

### Registry Integration

**Template Type Discriminator:**
```yaml
# registry/templates/goal_build_app.j2
[inference]
template_type: Goal
lexicon_terms: [build, compile, test, verify, deliver]
contract:
  input: {spec_path: string, verification_commands: array}
  output: {build_status: string, test_results: object}
  verification:
    - type: command
      command: "npm test"
      expected_exit_code: 0
    - type: state
      check: "git status --porcelain"
      expected_pattern: "^\\?\\s+.*$"
```

### OCAP Goal Delegation

**Capability Token Attenuation:**
```rust
pub struct GoalCapability {
    pub goal_id: GoalId,
    pub attenuation_level: u8,  // Increases per delegation
    pub max_attenuation: u8,    // Default: 7
    pub allowed_actions: Vec<CapabilityAction>,
    pub expiration: UnixTimestamp,
    pub hmac_signature: Vec<u8>,
}

impl GoalCapability {
    pub fn delegate(&self, secret_key: &[u8]) -> Result<Self, DelegationError> {
        if self.attenuation_level >= self.max_attenuation {
            return Err(DelegationError::MaxAttenuationReached);
        }
        // Attenuate actions, halve expiration, increment level
    }
}
```

---

## Part 6: Implementation Plan

### Phased Rollout

| Phase | Duration | Deliverables | LOC |
|-------|----------|--------------|-----|
| **Phase 1** | Week 1-2 | Core types, port traits | ~400 |
| **Phase 2** | Week 3-4 | Two consumers per port | ~800 |
| **Phase 3** | Week 5 | CNS integration | ~300 |
| **Phase 4** | Week 6 | Registry integration | ~200 |
| **Phase 5** | Week 7-8 | OCAP + audit | ~350 |
| **Total** | 8 weeks | All phases | **~2,050** |

### Constraint Compliance

**Principles (P1-P7):**
- P1: Two consumers per trait — ✅ (3 verifiers: CNS, LLM, Command)
- P2: Two generic instantiations — ✅ (`GoalCapability<Owner, Holder>`)
- P3: Encapsulation — ✅ (`hkask-goals/src/` module)
- P4: Fallible builders — ✅ (`GoalBuilder::build() -> Result`)
- P5: Feature activator — ✅ (`--features goals`)
- P6: No stubs — ✅ (Phase 0 types only)
- P7: Deletion > deprecation — ✅

---

## Part 7: Comparison Matrix

| Feature | Hermes | hKask (Proposed) |
|---------|--------|------------------|
| **Storage** | SessionDB (per-session) | SQLite (cross-session, multi-agent) |
| **Verification** | Judge LLM only | Hybrid (CNS + LLM + Bot) |
| **Budget** | Turns only | Turns + energy |
| **Delegation** | N/A | OCAP with attenuation |
| **Routing** | Manual | Registry (`template_type: Goal`) |
| **Security** | Trust session DB | HMAC + SQLCipher + OCAP |
| **Multi-Agent** | Single-session | ACP-enabled |
| **Escalation** | User notification | Algedonic alert → Curator |
| **Semantics** | Free-form text | hLexicon-grounded |

---

## Part 8: Open Questions (Resolved)

| Question | Resolution | Phase |
|----------|------------|-------|
| Q1: Template vs. entity? | **Both** — routing + persistence | Phase 1 |
| Q2: Minimal viable primitive? | Types + 2 consumers per port | Phase 1 |
| Q3: Goal hijacking prevention? | OCAP + HMAC | Phase 3 |
| Q4: Encryption at rest? | SQLCipher only | Phase 3 |
| Q5: Condensation pipeline? | Hybrid auto/manual | Phase 3+ |
| Q6: Algedonic alerts? | Alert, no auto-pause | Phase 3 |
| Q7: Span integration? | Parent span links | Phase 3 |
| Q8: Goal composition? | Hierarchical only | Phase 4+ |
| Q9: Goal type distinction? | Unified for MVP | Phase 4+ |
| Q10: Termination conditions? | Auto-block on failures | Phase 2 |

---

## Part 9: Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| **Goal Spam Loops** | Max parse failures → auto-pause, terminal phrase detection |
| **Capability Attenuation Too Aggressive** | Configurable `max_attenuation`, audit trail |
| **Variety Counter False Positives** | Adjustable threshold, Curator review |
| **Registry Routing Failures** | Confidence threshold (<0.7 → ask user) |

---

## Part 10: Completion Criteria

**Goal primitive is complete when:**
1. ✅ Database schema deployed
2. ✅ `GoalManager` implements full lifecycle
3. ✅ CNS spans emitted (`cns.goal.*` namespace)
4. ✅ Registry routes goals via `template_type: Goal`
5. ✅ OCAP delegation with capability attenuation
6. ✅ Variety counter integration (algedonic alerts)
7. ✅ CLI commands implemented
8. ✅ Verifier trait with hybrid verification
9. ✅ Tests passing

---

## Recommendation

**Formalize the goal primitive in hKask with:**
1. **Goals as First-Class Entities** — Database tables, Rust types, CNS spans
2. **Verification-Based Completion** — Never trust self-report; always verify
3. **OCAP Delegation** — Capability attenuation on goal handoff
4. **Registry Routing** — `template_type: Goal` in unified registry
5. **CNS Integration** — Variety counters, algedonic alerts
6. **hLexicon Grounding** — Speech acts (WordAct), workflows (FlowDef), cognition (KnowAct)

**Rationale:**
- Aligns with hKask's cybernetic roots (Beer, Ashby, Von Foerster)
- Extends BDI model with OCAP security and CNS monitoring
- Enables multi-agent goal delegation (unlike Hermes single-session)
- Leverages existing hLexicon vocabulary (12 of 80 terms directly relevant)

**Next Step:** Begin Phase 1 implementation (database schema + Rust types).

---

## Archived Files

The following files have been moved to `docs/archive/2026-05-22-documentation-refresh/goal-primitive-research/`:

**Task Reports (7 files, ~2,470 lines):**
- `task1-semantic-mapping.md` — RDF graph, Mermaid ERD
- `task2-hermes-interrogation.md` — Code paths, sequence diagram, security audit
- `task3-academic-survey.md` — Annotated bibliography (10 papers)
- `task4-alignment-analysis.md` — Gap analysis, recommendation
- `task5-architecture-design.md` — Rust types, port traits, adapters
- `task6-implementation-plan.md` — 5-phase plan, risk assessment
- `task7-open-questions.md` — 10 open questions, 3 research spikes

**Source Documents (3 files, ~1,332 lines):**
- `goal-primitive-analysis.md` — Comprehensive analysis
- `goal-primitive-final-report.md` — Executive summary
- `hlexicon-goal-foundation.md` — hLexicon grounding analysis

**This consolidated report:** ~650 lines (replaces 1,332 lines of source + preserves key findings)

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Goal primitive: from prompting to assigning.*  
*Rust is the loom. YAML/Jinja2 is the thread. OCAP is the gate. CNS is the monitor. hLexicon is the foundation.*
