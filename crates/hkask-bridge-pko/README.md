# hkask-bridge-pko

PKO (Procedural Knowledge Ontology) bridge — shared vocabulary constants for hKask's process (flow) ontological axis.

Part of the dual-axis ontological framework (P5.4): every MCP server uses this crate alongside `hkask-bridge-dublincore` (state axis). Shared by all 14 MCP servers.

Reference: Carriero et al. (2025, arXiv:2503.20634) — https://w3id.org/pko

## Concepts (30+)

- **Procedure:** Procedure, ProcedureType, ProcedureStatus, ProcedureTarget
- **Steps:** Step, MultiStep, Action, Function, requiresAction, requiresFunction, requiresTool
- **Execution:** ProcedureExecution, StepExecution, ProcedureExecutionStatus
- **Issues/Feedback:** IssueOccurrence, UserFeedbackOccurrence, UserQuestionOccurrence, Error
- **Verification:** StepVerification
- **Agents/Roles:** Agent, Role, RoleInTime, ExpertiseLevel
- **Resources:** references, wasExtractedFrom
- **Versioning:** hasVersion, nextVersion, previousVersion

## Mapping Helpers

- `kanban_status_to_pko_execution(status)` — kanban task status → PKO execution status
- `docproc_stage_to_pko_step(stage)` — docproc stage → PKO step concept
- `research_stage_to_pko(stage)` — research workflow stage → PKO concept

## Usage

```rust
use hkask_bridge_pko::{PROCEDURE, STEP_EXECUTION, kanban_status_to_pko_execution};

let status = kanban_status_to_pko_execution("in_progress"); // → pko:ProcedureExecutionStatus/inProgress
```
