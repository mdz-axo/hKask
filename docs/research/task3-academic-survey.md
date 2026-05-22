# Task 3: Academic Research Survey

## 3.1 Annotated Bibliography (10 Key Papers)

---

### 1. **Intention Progression with Temporally Extended Goals** (IJCAI 2024)
**Authors:** [Not listed in search results]  
**URL:** https://www.ijcai.org/proceedings/2024/33

**Summary:** Extends BDI architecture to support temporally extended goals that mix reachability and invariant properties (e.g., "travel to location A while not exceeding a gradient of 5%"). Allows human-authored plans and RL policies to be freely mixed.

**Key Findings:**
- Traditional BDI limited to simple achievement goals
- Temporally extended goals enable neuro-symbolic architectures
- Subgoals can be specified at run-time or as plan prerequisites

**Relevance to hKask:**
- Supports hKask's `goal_subgoals` design
- Validates CNS variety counter (invariant monitoring)
- Informs `CompletionCriteria` with temporal constraints

---

### 2. **BDI Agent Architectures: A Survey** (IJCAI 2020)
**Authors:** Lavindra de Silva, Felipe Meneguzzi, Brian Logan  
**URL:** https://www.ijcai.org/proceedings/2020/0684.pdf

**Summary:** Comprehensive survey of BDI architectures from philosophical foundations (Bratman 1987) to modern implementations (Jason, AgentSpeak, GOAL, Can).

**Key Findings:**
- **Declarative goals** (GOAL, 3APL): Goals "to be" — belief-independent
- **Procedural goals** (PRS, Jason): Goals "to do" — plan-triggered
- Goal lifecycle: Pending → Active → [Suspended | Aborted | Successful]

**Relevance to hKask:**
- Validates distinction between declarative/procedural goals (Open Question 5)
- Informs `GoalState` lifecycle design
- Supports CNS span emission at lifecycle transitions

---

### 3. **Goal Space Abstraction in Hierarchical Reinforcement Learning via Reachability Analysis** (arXiv 2023)
**Authors:** [Not listed]  
**URL:** https://ar5iv.labs.arxiv.org/html/2309.07168

**Summary:** Introduces GARA (Goal Abstraction via Reachability Analysis) — feudal HRL algorithm that learns discrete interpretable goal representations from continuous observations using reachability analysis.

**Key Findings:**
- Goal representation learned through developmental process
- Reachability relations computed over sets of states
- Bottom-up abstraction emerges from exploration data

**Relevance to hKask:**
- Informs CNS variety counter (environmental vs. internal states)
- Supports goal complexity estimation for algedonic alerts
- Validates hierarchical goal decomposition

---

### 4. **First-Order Representation Languages for Goal-Conditioned RL** (AAAI 2026)
**Authors:** Ståhlberg, S., & Geffner, H.  
**URL:** https://ojs.aaai.org/index.php/AAAI/article/view/40960

**Summary:** Uses first-order relational languages for goal-conditioned RL. Hindsight Experience Replay (HER) relabels failures as successes by replacing original goal with achieved goal.

**Key Findings:**
- Goals as full states vs. subsets vs. lifted subgoals
- Automatic curriculum generation from easier goals
- Sparse reward environments benefit from goal relabeling

**Relevance to hKask:**
- Informs `GoalOutcome` representation
- Supports goal relabeling for failed attempts (audit trail)
- Validates subgoal decomposition strategies

---

### 5. **Dual Goal Representations for Goal-Conditioned Reinforcement Learning** (arXiv 2025)
**Authors:** [Not listed]  
**URL:** https://arxiv.org/pdf/2510.06714

**Summary:** Proposes dual goal representations characterizing state by temporal distances from all other states. Proves action sufficiency and noise invariance.

**Key Findings:**
- Ideal goal representation must be: (1) action-sufficient, (2) noise-invariant
- Dual representation: goal as functional (state → temporal distance)
- Retains sufficient information for optimal policy while discarding exogenous noise

**Relevance to hKask:**
- Informs `GoalVerifier` design (action sufficiency check)
- Supports CNS comparator (temporal distance from goal state)
- Validates noise-invariant verification

---

### 6. **Prism: A Compositional Metalanguage for Agent Behaviour** (arXiv 2025)
**Authors:** [Not listed]  
**URL:** https://arxiv.org/pdf/2512.00611

**Summary:** Minimalist metalanguage for agent behavior with grammar-like specifications. Separation: natural language understanding (LLM) + formal control (Prism policy).

**Key Findings:**
- Compositional semantics (Fregean principle)
- Decisions as expressions selecting between alternatives
- Inspectable policies without model internals access

**Relevance to hKask:**
- Informs registry template design (YAML/Jinja2 as "thread")
- Supports formal verification of goal workflows
- Validates separation: Rust kernel (loom) + YAML policies (thread)

---

### 7. **AgentSPEX: An Agent Specification and Execution Language** (arXiv 2026)
**Authors:** [Not listed]  
**URL:** https://arxiv.org/pdf/2604.13346

**Summary:** YAML syntax for explicit agent workflow control. Supports typed steps, branching, loops, parallel execution, reusable submodules, explicit state management.

**Key Findings:**
- Workflow specification includes `goal` field
- Visual editor with synchronized graph/workflow views
- Formal verification via pre/post-conditions

**Relevance to hKask:**
- Validates YAML-based goal specification
- Informs dispatch manifest design
- Supports formal verification of goal completion

---

### 8. **FALAA: Framework for the Abstraction of Language Agent Architectures** (Springer 2026)
**Authors:** Brandstetter, N., Bravo-Marquez, F., Olmedo, F.  
**URL:** https://link.springer.com/chapter/10.1007/978-3-032-18011-7_4

**Summary:** Standardizes language agent architecture description via UML + OCL. Components: Planner, Executor, Evaluator, Reflector, Memory, Environment.

**Key Findings:**
- Formal precision reveals ambiguities in existing architectures
- OCL constraints enable automated verification
- Dual-level methodology (conceptual + formal)

**Relevance to hKask:**
- Informs port trait specifications (OCL-style contracts)
- Supports CNS component mapping (Evaluator → Verifier)
- Validates hexagonal architecture documentation

---

### 9. **The Belief-Desire-Intention Ontology for Modelling Mental Reality and Agency** (arXiv 2025)
**Authors:** [Not listed]  
**URL:** https://arxiv.org/pdf/2511.17162

**Summary:** Formal BDI Ontology as modular Ontology Design Pattern. Captures cognitive architecture through beliefs, desires, intentions, and their dynamic interrelations.

**Key Findings:**
- Goals modeled as descriptions (not mental states)
- Intentions reflect agent's commitment to achieving goals
- Temporal reasoning: goals emerge, persist, evolve over time

**Relevance to hKask:**
- Informs RDF/OWL goal ontology (Task 1)
- Supports temporal tracking (`created_at`, `completed_at`)
- Validates goal-as-description (vs. mental state)

---

### 10. **Program-Based Goal Selection** (OpenReview 2024)
**Authors:** [Not listed]  
**URL:** https://openreview.net/pdf?id=DfFE7hfnEb

**Summary:** Proposes program induction for tractable selection of novel goals. Goals as `(goal_program, reward_function)` pairs sampled from generative grammar.

**Key Findings:**
- RL has no formal goal representation (assumes Markov rewards)
- Program-based agents sample goals from grammar `G`
- Captures intrinsic reward from goal achievement

**Relevance to hKask:**
- Informs goal template selection (registry routing)
- Supports goal grammar (template_type discriminator)
- Validates goal-induced intrinsic rewards (CNS satisfaction signal)

---

## 3.2 Synthesis: What Research Says About Effective Goal Primitives

### 3.2.1 Formal Representations

**Consensus:** Goals require formal representation beyond natural language.

| Approach | Representation | Verification |
|----------|----------------|--------------|
| **BDI** | Logical formulas (achievement/maintenance) | Plan success conditions |
| **GCRL** | State embeddings / temporal distances | Value function convergence |
| **Prism/AgentSPEX** | YAML/grammar specifications | Pre/post-conditions |
| **hKask (Proposed)** | Registry templates + completion criteria | CNS comparator + verifier bot |

**Recommendation:** Hybrid approach — natural language goal text + formal completion criteria (commands, state checks, semantic evaluation).

---

### 3.2.2 Goal Decomposition Strategies

**Research Findings:**
1. **Hierarchical decomposition** (GARA, BDI): Goals → subgoals → actions
2. **Temporal abstraction** (Temporally Extended Goals): Mix reachability + invariants
3. **First-order lifting** (AAAI 2026): Goals as full states → subsets → lifted subgoals
4. **Program induction** (OpenReview 2024): Goals sampled from generative grammar

**hKask Adaptation:**
- Support `goal_subgoals` table (hierarchical)
- Temporally extended criteria (invariant monitoring via CNS)
- Registry-driven template selection (grammar-like routing)

---

### 3.2.3 Goal Conflict Resolution

**Identified Conflicts:**
1. **Resource conflicts:** Multiple goals compete for limited budget
2. **Authority conflicts:** Overlapping capability grants
3. **Temporal conflicts:** Incompatible deadlines
4. **Semantic conflicts:** Contradictory completion criteria

**Resolution Mechanisms:**
- **Priority ordering** (BDI): Goal priorities resolve conflicts
- **Capability attenuation** (OCAP): Delegation reduces authority
- **Budget partitioning** (hKask): Energy budget per goal
- **Variety monitoring** (CNS): Algedonic alert on deficit

---

### 3.2.4 Verification of Goal Satisfaction

**Verification Approaches:**

| Method | Strengths | Weaknesses |
|--------|-----------|------------|
| **LLM Judge** (Hermes) | Semantic understanding | Fail-open spam, API errors |
| **Command Execution** | Deterministic, auditable | Limited to executable checks |
| **State Inspection** | Filesystem/git verification | Requires environment access |
| **CNS Comparator** | Cybernetic feedback (variety) | Requires formal goal encoding |
| **Hybrid** (hKask) | Best of all worlds | Increased complexity |

**Recommendation:** Hybrid verification with fallback chain:
```
1. Command verification (exit codes)
2. State verification (filesystem/git)
3. CNS comparator (variety check)
4. LLM judge (semantic evaluation)
```

---

### 3.2.5 Security Considerations

**Research Gap:** Academic papers rarely address security of goal primitives.

**Exception:** OCAP / capability security literature (Mark Miller) — not goal-specific.

**hKask Contribution:** First goal primitive with:
- OCAP capability tokens
- Attenuation on delegation
- Visibility gating (private/public/shared)
- HMAC-signed persistence
- SQLCipher encryption

---

### 3.2.6 Open Research Questions

1. **Goal Composition:** Can goals be composed (goal-of-goals)? Security implications?
2. **Intrinsic Rewards:** Should goal completion generate CNS satisfaction signals?
3. **Goal Learning:** Can agents learn goal templates from experience?
4. **Formal Verification:** Can goal workflows be verified (OCL, Prism)?
5. **Cross-Agent Goals:** How do goals span multiple agent pods?

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*  
*Task 3 Complete: Academic research validates hybrid verification, hierarchical decomposition, and formal representation — hKask adds OCAP security and CNS monitoring.*