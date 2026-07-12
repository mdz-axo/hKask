---
title: "Scenario Forecasting and Planning"
audience: [developers, operators, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting"
mds_categories: [domain, composition, lifecycle, curation]
---

# Scenario Forecasting and Planning

## Theoretical Foundations [^schwartz]

The `hkask-mcp-scenarios` server integrates three complementary frameworks:

| Framework | Author | Core Question | Tool Stage |
|-----------|--------|---------------|------------|
| **Schwartz Method** | Peter Schwartz, *The Art of the Long View* (1991) | "What could happen?" | `scenario-builder` skill's 2×2 pipeline; companies 2×2 bridge |
| **Superforecasting** | Philip Tetlock, *Superforecasting* (2015) | "How likely is each event?" | `scenario_calibrate`, `scenario_update`, `scenario_score`, `scenario_synthesize`, `scenario_calibration` |
| **Performance-Based Scenario System** | Thomas Chermack, *Scenario Planning in Organizations* (2011) | "Did the project improve decision quality?" | `scenario_frame`, `scenario_assess` |

### Why Three Frameworks?

**Schwartz alone**: Builds compelling narratives but doesn't measure accuracy. You might be wrong with a good story.

**Tetlock alone**: Measures accuracy but doesn't help construct the futures being forecast. Precision without imagination is brittle.

**Chermack alone**: Evaluates effectiveness but doesn't provide the scenario construction or calibration methods. Assessment without methodology is empty.

**Together**: Build futures (Schwartz), forecast their likelihood (Tetlock), and measure whether the whole project improved decisions (Chermack).

### What Each Framework Contributes

#### Schwartz (1991) — The Art of Scenario Construction

Developed at Royal Dutch Shell, the Schwartz Method anticipated the 1973 oil crisis, 1986 price collapse, and Soviet collapse — not by predicting any of them, but by having strategies ready for worlds where they could happen.

**Key concepts**:
- Focal question: decision-relevant, time-bounded, scope-bounded
- Driving forces: STEEP (Society, Technology, Economy, Environment, Politics)
- Critical uncertainties: mapped on importance × uncertainty → 2×2 matrix axes
- Scenario narratives: internally consistent stories for each quadrant
- Robust strategies: actions that work across all scenarios
- Early-warning indicators: signals that tell you which future is unfolding

**Reference**: Schwartz, P. (1991). *The Art of the Long View: Planning for the Future in an Uncertain World*. Currency Doubleday.

#### Tetlock (2015) — The Science of Forecasting

The Good Judgment Project identified what makes the top 2% of forecasters different: better process, not higher IQ. The Ten Commandments encode this process.

**Key concepts** (Commandments 1–10):
1. **Triage**: Focus on Goldilocks-zone questions — not too easy, not too hard
2. **Fermi-ize**: Break intractable problems into tractable sub-questions
3. **Outside view first**: Start with base rates, then adjust for specifics
4. **Incremental belief updating**: Move probabilities 0.05 at a time as evidence arrives
5. **Dragonfly-eye**: Integrate multiple perspectives; steel-man dissenting views
6. **Degrees of doubt**: Use the full 0–100% scale — "60%" is more informative than "maybe"
7. **Under/overconfidence balance**: Calibrate both calibration and resolution
8. **Postmortems**: Analyze errors behind mistakes; also postmortem successes
9. **Team management**: Perspective-taking, precision questioning, constructive confrontation
10. **Error-balancing**: Master the bicycle — balance miss rates and false-alarm rates

**Reference**: Tetlock, P.E. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*. Crown.

#### Chermack (2011) — The Organizational Learning Dimension

Chermack's contribution is the **evaluation framework**. Most scenario planning literature describes how to build scenarios. Chermack asks: "Did it work? How do you know?" He grounds scenario planning in learning theory, mental model theory, and decision-making theory.

**Key concepts** — Five-Phase Performance-Based Scenario System:
1. **Project Preparation** (Ch. 5): Scope, stakeholders, resources, conversation quality
2. **Scenario Exploration** (Ch. 6): Driving forces, trends, uncertainties surfaced through dialogue
3. **Scenario Development** (Ch. 7): Scenario logic, internal consistency, narrative quality
4. **Scenario Implementation** (Ch. 8): Strategies applied, wind-tunneling, early warning systems
5. **Project Assessment** (Ch. 9): Did the project improve decision quality? Learning outcomes?

**Chermack's critique of the field**: "Very few projects reflecting scholarly research or theoretical examination have been produced." He demands: scenario planning must be evaluated — not just described.

**Reference**: Chermack, T.J. (2011). *Scenario Planning in Organizations: How to Create, Use, and Assess Scenarios*. Berrett-Koehler.

---


## Conversational Design — Why the Framing Is Different [^chermack]

### The Problem with Diagnostic Framing

Most scenario projects fail at the framing stage — not because the questions are wrong,
but because they're asked in the wrong way. Formal diagnostic questions ("What is your
focal question?") create resistance. They feel like a test. Users who don't have
ready answers feel inadequate. Users who DO have answers often haven't surfaced their
real assumptions.

The `scenario_frame` tool is designed to **invite** rather than **interrogate**.

### Design Principles

The 7-turn conversational protocol is built on three pillars:

**Improv Postures** (hKask improv skill):
- **Plussing** (default): Accept what the user says, build on what's useful, silently let go of what isn't. Never explicitly negate.
- **Yes, And**: Accept their answer and extend it with the next natural layer.
- **Yes, But**: Constrain without contradicting ("that's off the table — given that, what IS on?").

**Kata Coaching** (hKask kata-starter skill):
- The agent is a **coach**, not an interviewer. The user is the domain expert.
- The coach helps the expert articulate what they already know but haven't yet made explicit.
- Target: 15-20 minutes. "20 minutes daily, not 2 hours weekly."

**Behavioral Psychology**:
- **Foot-in-the-door** (Cialdini): Turn 1 — "What's on your mind?" — is the easiest. Anyone can answer it.
- **Curiosity gap** (Loewenstein): Turns 2-3 build intrigue before asking for commitment.
- **Loss aversion** (Kahneman): Turn 4 asks what's OFF the table. People find exclusions easier to identify than inclusions.
- **Social proof** (Cialdini): Turn 5 uses "who else" to normalize multiple perspectives. The "I told you so" framing activates contrarian thinking through social dynamics.
- **Peak-end rule** (Kahneman): Turn 1 opens warmly; Turn 7 closes with a provocative but supportive question. The first and last moments define the experience.
- **Processing fluency**: Everyday language. No "focal question," no "stakeholder analysis." Just "what's on your mind?" and "who would say I told you so?"
- **IKEA effect** (Norton, Mochon, Ariely): The user co-creates the frame. They value it more because they built it.

### The 7 Conversational Turns

| Turn | Opening | Improv Mode | Psychology | What it captures |
|------|---------|-------------|------------|------------------|
| 1 | "So — tell me a bit about what's on your mind. What situation are you looking at?" | Plussing | Foot-in-the-door | Subject, context, emotional stakes |
| 2 | "If you had a clearer picture, what would you actually do differently? What decision is hanging on this?" | Yes, And | Curiosity gap | Decision at stake, focal question |
| 3 | "When do you actually need to make this call? Over what timeframe do the key events play out?" | Coaching | Temporal anchoring | Time horizon, action deadline |
| 4 | "Let's start with what's definitely NOT on the table. What are we explicitly not trying to figure out?" | Yes, But | Loss aversion | Out-of-scope, then in-scope |
| 5 | "Who else has skin in this game? If this goes wrong, who would say 'I told you so'?" | Plussing | Social proof + contrarian | Stakeholders and perspectives |
| 6 | "What does 'good enough' look like? What would make you say this was worth the time?" | Yes, And | Peak-end begins | Success criteria, use case |
| 7 | "What are we assuming that might be completely wrong? What constraints are we working within?" | Yes, But | Peak-end closes | Assumptions, constraints |

### Agent Posture

- **Overall**: Coach, not interviewer. Socratic but warm. Never fill the silence.
- **When stuck**: Reframe, don't skip. "Let me ask it differently..."
- **When flowing**: Let the user go. The turns are a scaffold, not a script.
- **Never**: "That's too broad." "That's not a focal question." "You need to be more specific." Instead: "That's interesting — let's dig into that."

## The Integrated Pipeline [^tetlock]

### Phase 0: FRAME — Establish Purpose and Scope

**scenario_frame** → 7-question Socratic interview protocol
  - Focal question + decision at stake (Schwartz Stage 1)
  - Time horizon + action deadline (Chermack Phase 1)
  - Scope boundaries — what's in, what's out (Schwartz)
  - Stakeholders and their perspectives (Chermack: stakeholder diversity)
  - Use case — how will output be consumed?
  - Success criteria — defined BEFORE building scenarios (Chermack)
  - Constraints and surfaced assumptions

**Principle**: Frame before you forecast. The quality of the framing determines the quality of everything that follows. If the focal question doesn't change any decision, stop — the project has no purpose.

### Phase 1: BRAINSTORM — Generate Events Collaboratively

**scenario_brainstorm** → 4-round temperature-shifting protocol
  - Round 1: DIVERGE (high temp) — 4+ personas generate 12+ candidate events
  - Round 2: GROUND (medium temp) — anchor in verified facts, base rates, reference classes
  - Round 3: LINK (low temp) — build causal chains, dependency relationships
  - Round 4: PRUNE (analytical) — merge overlaps, remove isolates, converge to 4-8 events

**scenario_research** → Produce an evidence-aware extraction prompt from web-search text; the agent remains responsible for creating and reviewing candidate events
**scenario_triage** → Classify questions as clocklike/goldilocks/cloudlike (Tetlock #1)

### Phase 2: QUANTIFY — Resolve Probabilities

**scenario_quantify** → Resolve conditional probability tree
  - Topological sort, marginal probabilities, joint probability
  - Sensitivity ranking by variance contribution

**scenario_calibrate** → Fermi decomposition plus an optional outside-view base-rate blend (Tetlock #2-3); case-specific reasoning remains explicit in the calling workflow
**scenario_update** → Bayesian evidence revision (Tetlock #4)

### Phase 3: SYNTHESIZE — Aggregate Perspectives

**scenario_synthesize** → Dragonfly-eye aggregation (Tetlock #5)
  - Empirical-Bayes weighted average across perspectives
  - Disagreement scoring (Chermack: conversation quality)
  - Dissent identification

**scenario_sensitivity** → Variance contribution ranking (Tetlock #6)

### Phase 4: TRACK — Calibration and Scoring

**scenario_score** → Brier scoring + auto-update suggestions (Tetlock #7-8)
**scenario_calibration** → Calibration curve with over/underconfidence detection

### Phase 5: ASSESS — Did It Work?

**scenario_assess** → Five-phase evaluation (Chermack)
  - Per-phase scores with strengths, gaps, recommendations
  - Answers: "Did this project improve decision quality?"
## Tool Reference

| # | Tool | Framework | What it does |
|---|------|-----------|-------------|
| 1 | `scenario_frame` | Chermack Phase 1 + Schwartz Stage 1 | 7-turn conversational protocol with improv postures + behavioral psychology design |
| 2 | `scenario_brainstorm` | Kahneman + Schwartz + Chermack | 4-round temperature-shifting protocol with multiple personas |
| 3 | `scenario_triage` | Tetlock #1 | Classify question as clocklike/goldilocks/cloudlike |
| 4 | `scenario_research` | Schwartz | Extract candidate events from web research text |
| 5 | `scenario_build` | Schwartz + Chermack | Construct event tree with dependencies and Fermi scaffolding |
| 6 | `scenario_quantify` | Schwartz | Resolve conditional probability tree; compute marginals, joint, sensitivity |
| 7 | `scenario_calibrate` | Tetlock #2-3 | Fermi decomposition + outside/inside view calibration |
| 8 | `scenario_update` | Tetlock #4 | Bayesian evidence revision |
| 9 | `scenario_synthesize` | Tetlock #5 | Dragonfly-eye multi-perspective aggregation |
| 10 | `scenario_sensitivity` | Tetlock #6 | Rank events by variance contribution |
| 11 | `scenario_score` | Tetlock #7-8 | Brier scoring + forecast store + auto-update suggestions |
| 12 | `scenario_calibration` | Tetlock #7 | Calibration curve with over/underconfidence detection |
| 13 | `scenario_assess` | Chermack | Five-phase project effectiveness evaluation |
## Key Design Decisions [^brier]

### 1. Events, not axes (inspired by MAIA)

Traditional Schwartz scenario planning crosses two critical uncertainties to produce four quadrant scenarios. Our event-tree approach (informed by MAIA's investment process design) uses **binomial events with conditional dependencies** instead. This is more granular, more testable, and maps directly to Tetlock's Fermi decomposition methodology.

Each event is: (name, yes/no question + deadline, probability, basis, dependencies, Fermi sub-questions).

### 2. Computed certainty, not stored certainty

The `certainty_tier` is derived from `probability` on access via `ScenarioEvent::certainty_tier()`. This prevents the stale-field divergence bug where a Bayesian update changes the probability but leaves the tier unchanged.

### 3. Error types, not strings

All error paths use the `ScenarioError` enum (via `thiserror`), following hKask conventions. The CI pipeline enforces this — `String` errors are prohibited by `scripts/check-string-errors.sh`.

### 4. Journal-persisted forecast store with calibration tracking

Forecasts are stored when scored (`scenario_score`) and can be queried for calibration curves (`scenario_calibration`). The store uses append-only journal persistence (O(1) writes) with automatic snapshot compaction at 100 entries. Survives server restarts. `ForceCompact()` triggers manual compaction.

### 5. Conditional independence assumption in event trees

When an event has multiple parents, the server averages their single-parent conditional contributions for marginalization and for its all-events proxy. This is an explicit heuristic, not conditional independence. Real-world scenario events may be correlated through latent factors; represent a known joint mechanism as a compound event until a joint-distribution model exists.

### 6. Agent does research; server does math

The scenarios server does not collect web research. The agent supplies raw research text to `scenario_research`; the server returns an extraction scaffold and validates or computes only explicit structured inputs. The agent must create and review candidate events before quantification.

---

## Quick Start [^tetlock]

```text
# 0. FRAME: Establish purpose and scope BEFORE building anything
scenario_frame(subject="NVIDIA")
  → 7-question interview protocol
  → Agent asks user: focal question, time horizon, scope, stakeholders, use case

# After the framing interview, the user has answered:
#   focal_question: "Should we increase our NVIDIA position given uncertainty
#                    about CUDA lock-in and custom chip displacement?"
#   time_horizon: strategic (3-5 years)
#   stakeholders: Portfolio manager, tech analyst, macro strategist
#   use_case: investment_thesis

# 1. TRIAGE: Is the focal question forecastable?
scenario_triage("Will NVIDIA maintain >50% AI training market share through 2028?",
  has_deadline=true, has_reference_class=true, has_resolution_criteria=true)
→ goldilocks — proceed

# 2. RESEARCH: Gather evidence
[agent searches web for NVIDIA AI chip market, CUDA competition, custom silicon]
scenario_research(subject="NVIDIA", research_text=<search results>)
→ detected themes: competitive_dynamics, technology_evolution, supply_chain

# 3. BRAINSTORM: Generate events with stakeholders as personas
scenario_brainstorm(subject="NVIDIA", time_horizon="strategic",
  research_context=<search results>,
  personas="Portfolio Manager,Tech Analyst,Macro Strategist,Contrarian")
→ 4-round protocol: DIVERGE → GROUND → LINK → PRUNE

# 4. QUANTIFY: Resolve the tree
scenario_quantify(events=<JSON array of ScenarioEvents>)
→ EventTree with marginal probabilities, joint probability, sensitivity ranking

# 5. CALIBRATE: Fermi decomposition per event
scenario_calibrate(question="Will CUDA competitor achieve >10% training share by end 2027?",
  sub_questions=[...], base_rate=0.15, reference_class="Platform transitions")
→ calibrated probability + confidence

# 6. SYNTHESIZE: Aggregrate stakeholder perspectives
scenario_synthesize(event_id="evt-2", perspectives=[pm_view, tech_view, macro_view])
→ aggregated probability, disagreement score, dissent summary

# 7. SCORE: Track outcomes
scenario_score(forecast_id="nvda-2027", events=..., outcomes=[...])
→ Brier scores + auto-update suggestions
scenario_calibration()
→ Calibration curve: "am I overconfident?"

# 8. ASSESS: Did the project work?
scenario_assess(project_id="nvda-2027", perspective_count=3, ...)
→ Five-phase evaluation against the success criteria defined in framing
```
## Architecture [^rust]

```
mcp-servers/hkask-mcp-scenarios/
├── Cargo.toml              # Package manifest

├── src/
│   ├── main.rs             # Binary entrypoint (bootstrap + run)
│   ├── lib.rs              # Server struct, 18 MCP tools, request types
│   ├── types.rs            # Core data model (events, trees, forecasts, assessment)
│   └── superforecast.rs    # Computation engine (Fermi, Bayes, Brier, trees, assessment)
```

### Dependencies

- `hkask-mcp` — MCP server framework, tool macros, credential management
- `hkask-types` — Core types (WebID, time utilities)
- `rmcp` — MCP protocol (tool registration, parameter deserialization)
- `thiserror` — Error type derivation
- `serde` / `serde_json` — Serialization for MCP tool I/O
- `schemars` — JSON Schema generation for tool parameters
- `chrono` — Date handling (event deadlines, forecast timestamps)

---

## References

[^schwartz]: Schwartz, P. (1991). *The Art of the Long View*. Currency Doubleday. https://www.penguinrandomhouse.com/books/30008/the-art-of-the-long-view-by-peter-schwartz/
[^tetlock]: Tetlock, P. E., & Gardner, D. (2015). *Superforecasting*. Crown. https://www.goodjudgment.com/about/
[^chermack]: Chermack, T. J. (2011). *Scenario Planning in Organizations*. Berrett-Koehler. https://www.bkconnection.com/books/title/scenario-planning-in-organizations
[^brier]: Brier, G. W. (1950). Verification of forecasts expressed in terms of probability. *Monthly Weather Review*, 78(1), 1–3. https://doi.org/10.1175/1520-0493(1950)078%3C0001:VOFEIT%3E2.0.CO;2
[^rust]: Rust Project. (2026). *The Rust Programming Language*. https://doc.rust-lang.org/book/

- Schwartz, P. (1991). *The Art of the Long View: Planning for the Future in an Uncertain World*. Currency Doubleday.
- Tetlock, P.E. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*. Crown.
- Chermack, T.J. (2011). *Scenario Planning in Organizations: How to Create, Use, and Assess Scenarios*. Berrett-Koehler.
- Chermack, T.J. & Lynham, S.A. (2004). "A Theoretical Model of Scenario Planning." *Human Resource Development Review*, 3(4).
- Chermack, T.J. (2003). "A Methodology for Assessing Performance-Based Scenario Planning." *Journal of Leadership & Organizational Studies*, 10(2).
- Brier, G.W. (1950). "Verification of Forecasts Expressed in Terms of Probability." *Monthly Weather Review*, 78(1), 1-3.
- Murphy, A.H. (1973). "A New Vector Partition of the Probability Score." *Journal of Applied Meteorology*, 12(4), 595-600.
- Rappaport, A. & Mauboussin, M.J. (2001). *Expectations Investing*. Harvard Business School Press.
