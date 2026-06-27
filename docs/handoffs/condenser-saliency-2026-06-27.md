# Handoff: Condenser Saliency Refactoring — 2026-06-27 ✅ COMPLETED

## Completion (2026-06-27 session)

### Condenser Refactoring

- **`SaliencyRankAlgorithm` renamed to `WordRankAlgorithm`** — now correctly reflects TF-IDF bag-of-words compression. Not saliency.
- **`domain_saliency` extracted as public free function** — graph proximity scoring via ontology graph (P5.4). Pure: no I/O, no async, no MCP.
- **`persona_to_anchor` added** — maps persona description text to `OntologyAnchor` for domain-aware scoring.
- **`condenser_score_saliency` MCP tool** — three anchoring modes:
  - `persona`: charter-derived anchor (persona description → domain)
  - `episodic`: PKO process domain anchor (first-person experiences)
  - `semantic`: DC+BIBO document domain anchor (shared knowledge)
- All paths use `domain_saliency()` — graph proximity. Zero word counting.

### Metacognition CAT Wiring

- `MetacognitionLoop` calls `condenser/condenser_score_saliency(against: "persona")` before `cat::evaluate()`
- Persona saliency score modulates `convergence_bias`: `effective_bias = (bias + score × (1.0 − bias)).min(1.0)`
- Graceful degradation: defaults to 0.5 if condenser unavailable

### Documentation Updated

- `docs/architecture/ADRs/matrix-server-administration.md` — architecture diagram, CAT model, saliency anchoring table
- `docs/architecture/hKask-architecture-master.md` — Pattern C: CAT communication posture
- `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` — condenser MCP server, communication section
- `crates/hkask-condenser/README.md` — algorithm table, word_rank section
- `mcp-servers/hkask-mcp-condenser/README.md` — tools (8), algorithm table, context categories

### Test Results

| Crate | Tests | Status |
|-------|-------|--------|
| `hkask-condenser` | 65 + 2 fuzz | All pass |
| `hkask-mcp-condenser` | 11 contract | All pass |
| `hkask-agents` | 53 unit + 12 integration | All pass |
| `hkask-agents` (CAT) | 6 | All pass |
| Full workspace (9 crates) | compile | Clean |

### Key Decisions

1. `convergence_bias` IS the speak/silent decision (CAT). Saliency modulates it.
2. The engagement gate is a pure function (`cat::evaluate`). MCP calls happen in the metacognition loop, not the gate.
3. Episodic memory = PKO anchor (process domain). Semantic memory = DC+BIBO anchor (document domain).
4. Word frequency (`WordRankAlgorithm`) is compression, not saliency. Never conflated.
5. `domain_saliency` is the canonical saliency function — graph proximity via ontology graph (P5.4).

### Remaining

- MEDIUM: YAML persona posture wiring (convergence_bias from persona YAML into MetacognitionLoop construction)
- TODO: 7R7 receptor specification (6 remaining receptors)
