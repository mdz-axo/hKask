# hkask-mcp-scenarios

Scenario-planning MCP server. It turns a framed decision into candidate events, calibrated probabilities, sensitivity rankings, and learning signals. The server is a thin MCP surface over `hkask-forecast` calculations and follows the standard hKask bootstrap path.

## Configuration

| Variable | Purpose |
|---|---|
| `HKASK_MCP_HOST` | Required host identity used during MCP bootstrap |
| `HKASK_SCENARIOS_DATA` | Optional path for forecast store persistence (snapshot + append-only journal). When unset, forecasts are kept in-memory only. |

## Tools

### Frame and explore

| Tool | Description |
|---|---|
| `scenario_status` | Return the current scenario-server state snapshot, including forecast count, calibration curve, cached event tree, and persistence health flag. |
| `scenario_full` | Run the complete scenario pipeline in one call. |
| `scenario_from_companies` | Convert company data into forecast events. |
| `scenario_frame` | Start the seven-turn framing conversation. |
| `scenario_frame_document` | Convert framing answers into a typed document. |
| `scenario_brainstorm` | Produce a four-round scenario brainstorming protocol. |
| `scenario_build` | Build an event-tree scaffold from research. |

### Quantify and calibrate

| Tool | Description |
|---|---|
| `scenario_research` | Extract candidate events from research text. |
| `scenario_quantify` | Resolve event probabilities and dependency order. |
| `scenario_update` | Apply a Bayesian probability update. |
| `scenario_score` | Score resolved forecasts with Brier scoring. |
| `scenario_calibrate` | Calibrate a forecast with Fermi and outside/inside views. |
| `scenario_sensitivity` | Rank events by uncertainty score. |
| `scenario_synthesize` | Combine independent perspectives into one forecast. |

### Learn and assess

| Tool | Description |
|---|---|
| `scenario_calibration` | Calculate a calibration curve from stored forecasts. |
| `scenario_triage` | Classify a question as clocklike, Goldilocks, or cloudlike. |
| `scenario_cross_validate` | Compare independent probability estimates. |
| `scenario_assess` | Assess a scenario project across five performance phases. |

## Operational boundaries

- Scenario calculations are explicit inputs and outputs; external research remains an input to `scenario_research`, rather than hidden server-side collection.
- Each tool outcome is recorded once via `record_experience` with full provenance (tool, input summary, outcome, detail, ontology anchor). The infrastructure `record_tool_outcome` is suppressed to avoid double daemon writes.
- Pipeline sequence violations (calling a tool without its expected predecessor) are logged as CNS warnings but do not block execution — tool flexibility is preserved for exploratory workflows.
- Forecast store persistence errors are logged via `tracing::error!` and surfaced in `scenario_status` as `persistence_healthy: false`. Silent data loss is prevented by this feedback signal.
- Request types use typed structs (`Vec<ScenarioEvent>`, `Vec<Perspective>`, `Vec<SubQuestion>`, `Vec<OutcomeEntry>`) for structured parameters, giving MCP clients JSON Schema validation. Freeform JSON blobs (`companies_output`, `answers`) remain as `String`.

## Type changes from v0.31.0 review

- `ScenarioEvent.basis`: `Option<String>` → `Option<Basis>` enum (`technical_feasibility`, `scaling_distribution`, `financial_model`)
- `ScenarioEvent.depends_on`: `Vec<EventDependency>` → `Option<EventDependency>` (only one dependency group was ever processed)
- `EventTreeNode.variance_contribution` → `uncertainty_score` (renamed to match computation: `|P - 0.5| * 2`)
- `EventTree.joint_probability` → `all_events_probability` (renamed to clarify it's the all-events-occur path probability)
- `parse_time_horizon` / `parse_scenario_type`: Return `Result` — unrecognized values now error instead of silently defaulting

## Related documentation

- [`docs/architecture/core/PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) — P2, P4, and P9 constraints
- [`docs/explanation/scenario-forecasting.md`](../../docs/explanation/scenario-forecasting.md) — Schwartz, Tetlock, and Chermack integration
- [`docs/architecture/scenarios-companies-bridge.md`](../../docs/architecture/scenarios-companies-bridge.md) — companies-server bridge status
- [`docs/diagrams/flowchart-scenario-forecasting-pipeline.md`](../../docs/diagrams/flowchart-scenario-forecasting-pipeline.md) — code-anchored pipeline flow
- [`docs/reference/mcp-servers/README.md`](../../docs/reference/mcp-servers/README.md) — built-in server registry
- [`docs/status/scenarios-adversarial-review.md`](../../docs/status/scenarios-adversarial-review.md) — code review findings and action items