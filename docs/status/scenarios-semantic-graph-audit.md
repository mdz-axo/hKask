---
title: "Scenarios Semantic Graph Audit"
audience: [developers, architects]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting"
mds_categories: [domain, composition, lifecycle, curation]
---

# Scenarios Semantic Graph Audit

## Cross-Skill/Server Dependency Graph

This status inventory is a code-review snapshot, not an executable contract. The authoritative interfaces are the server's tool definitions and registry manifests.

### Servers

```
hkask-mcp-companies (FIBO) ──→ hkask-mcp-scenarios (Dublin Core)
  calibrate_forecast              scenario_from_companies
  Schwartz 2×2 scenarios     →    scenario_quantify
                                  scenario_calibrate
                                  scenario_score

hkask-mcp-scenarios ──→ hkask-mcp-memory
  record_experience          →    store_experience (daemon)
  ontology_anchor                  episodic (PKO) / semantic (DC)

hkask-mcp-scenarios ──→ CNS (tracing subscriber)
  execute_tool_semantic      →    cns.mcp.scenarios.* spans
  check_sequence                  cns.mcp.scenarios.sequence
```

### Skills → Server Tools

```
superforecasting skill
  Stage 1 (Fermi)           → scenario_calibrate
  Stage 2 (Outside view)    → scenario_calibrate + base_rate
  Stage 4 (Bayesian)        → scenario_update
  Stage 5 (Dragonfly-eye)   → scenario_synthesize
  Stage 7 (Record)          → scenario_score
  Cross-validate            → scenario_cross_validate
                               └─ next_action: {skill: "grill-me"}

scenario-builder skill
  Stage 1 (Focal question)  → scenario_frame
  Stage 3 (Driving forces)  → scenario_brainstorm
  Stage 6 (Implications)    → scenario_assess

mcda skill
  Scenario weights          → scenario_synthesize
  Strategy alternatives     → scenario-builder Stage 6 → ranked by MCDA
  Compensation masking      → scenario_sensitivity
  Learning loop             → scenario_assess → MCDA re-rank
```

### Tool Call Sequence (expected pipeline)

```
scenario_frame                ─┐
scenario_frame_document        │ framing (PKO)
                               │
scenario_brainstorm            │ ideation (PKO)
scenario_build                 │ structuring
                               │
scenario_triage     ←── independent (anytime)
scenario_research   ←── independent (anytime)
                               │
scenario_quantify              │ computation
scenario_calibrate             │ (Dublin Core)
scenario_update      ←── after calibrate
scenario_sensitivity ←── after quantify
                               │
scenario_synthesize            │ aggregation
scenario_cross_validate ←── after calibrate
                               │
scenario_score                 │ tracking
scenario_calibration           │
                               │
scenario_assess                │ evaluation
                               │
scenario_full       ←── orchestrator (independent)
scenario_from_companies ←── bridge (independent)
```

### Constraint Forces (pragmatic-semantics)

| Edge | Force | Reason |
|------|-------|--------|
| Server → CNS spans | Guardrail | CNS must receive spans for regulation |
| Server → Memory (daemon) | Guardrail | Episodic encoding must uphold consent (P1) |
| Cross-validate → grill-me | Guideline | Agent should interrogate, not required to |
| Companies → Scenarios bridge | Guideline | One-way works; bidirectional needs MCP client |
| Skills → Server dispatch | Guideline | Agent decides; server provides tools |

### Orphans & Gaps

| Artifact | Status |
|----------|--------|
| `ScenarioForecast` struct | Removed — dead code |
| `WeightedScenario` struct | Removed — dead code |
| `ForecastQuestion` struct | Removed — dead code |
| `EventCandidate` struct | Removed — dead code |
| `store_forecast` fn | Removed — dead code, ForecastStore replaces |
| `resolve_forecast` fn | Removed — dead code |
| Companies → Scenarios MCP client | Not wired — requires an rmcp client in the companies server |
| Sequence enforcement | Informational spans only; no per-session state prevents out-of-order calls |
| MCP integration tests | Unit tests cover computation; protocol-level tests are still absent |
