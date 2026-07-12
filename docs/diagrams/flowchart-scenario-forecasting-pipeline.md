---
title: "Scenario Forecasting Pipeline"
audience: [developers, operators, agents]
last_updated: 2026-07-10
version: "0.31.0"
status: "Active"
domain: "Scenario forecasting"
mds_categories: [domain, composition, lifecycle, curation]
---

# Scenario Forecasting Pipeline

The scenarios MCP server accepts explicit research and event inputs, calculates the event-tree and calibration artifacts, and records resolved forecasts for later calibration. The diagram distinguishes the optional exploratory framing path from the computational path; `scenario_research` and `scenario_build` return scaffolds for an agent to review rather than silently collecting research or creating final events.[^mcp]

```mermaid
flowchart TD
    Start([Decision or forecast question])
    Frame[scenario_frame]
    FrameDoc[scenario_frame_document]
    Brainstorm[scenario_brainstorm]
    Research[Agent supplies research text]
    Scaffold[scenario_research or scenario_build]
    Events[Reviewed ScenarioEvent JSON]
    Quantify[scenario_quantify]
    Calibrate[scenario_calibrate]
    Update[scenario_update]
    Synthesize[scenario_synthesize]
    Score[scenario_score]
    Calibration[scenario_calibration]
    Assess[scenario_assess]
    End([Decision learning])

    Start --> Frame
    Frame --> FrameDoc
    FrameDoc --> Brainstorm
    Brainstorm --> Research
    Research --> Scaffold
    Scaffold --> Events
    Events --> Quantify
    Events --> Calibrate
    Calibrate --> Update
    Calibrate --> Synthesize
    Quantify --> Score
    Update --> Score
    Synthesize --> Assess
    Score --> Calibration
    Calibration --> Assess
    Assess --> End
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-FW-007
verified_date: 2026-07-10
verified_against: mcp-servers/hkask-mcp-scenarios/src/lib.rs:459-1708; mcp-servers/hkask-mcp-scenarios/src/superforecast.rs:165-400
reference_sources: mcp
status: VERIFIED
-->

## Scope and constraints

`scenario_quantify` validates event probabilities and dependency references before computing marginals. Its all-events value uses parent-true conditionals for a single-parent edge and a documented average proxy for a multi-parent edge; it is not a general joint-distribution engine. Forecasts are only persisted when `scenario_score` receives explicit outcomes, then `scenario_calibration` derives Brier and reliability signals from stored records.[^brier]

[^mcp]: Model Context Protocol. (2025). *Specification*. https://modelcontextprotocol.io/specification/2025-06-18
[^brier]: Brier, G. W. (1950). Verification of forecasts expressed in terms of probability. *Monthly Weather Review*, 78(1), 1–3. https://doi.org/10.1175/1520-0493(1950)078%3C0001:VOFEIT%3E2.0.CO;2
