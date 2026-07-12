# hkask-storage-guard

StorageGuard Loop — Autonomous disk space management (Loop 7).

## Purpose

Implements the HkaskLoop (sense → compare → compute → act) cycle to monitor disk usage on the `/data` volume and take corrective action. Extracted from `hkask-cns` to separate disk space management from cybernetic regulation.

## Guardrail Contract

- **Sense:** Measure disk usage percentage on the data directory
- **Compare:** Detect deviations from configurable thresholds (warn 80%, critical 95%)
- **Compute:** At warn level → log CNS span. At critical level → produce Prune action.
- **Act:** Prune old export archives. If pruning is insufficient, escalate to Curator.
- **Verify:** Re-check after dampener cooldown (5 min). If still critical → escalate.

## Dependencies

- `hkask-cns` — Loop trait

## See also

- [`hkask-cns`](../hkask-cns/) — Cybernetic Nervous System