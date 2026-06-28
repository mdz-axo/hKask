# hkask-services-kata — Toyota Kata Engine

Improvement Kata and Coaching Kata engine: target conditions, PDCA cycles, obstacle parking lots, and scientific thinking habit tracking. Drives the continuous improvement feedback loop across all agent pods.

**Version:** v0.31.0 | **Crate:** `hkask-services-kata`

## Modules

| Module | Purpose |
|--------|---------|
| `kata_impl` | `KataService` — target condition management, PDCA iteration, coaching dialogue |

## Key Types

- `KataService` — primary service interface for Improvement/Coaching Kata
- `TargetCondition` — measurable next target (1 week – 3 months)
- `PdcaCycle` — Plan-Do-Check-Act experiment record
- `ObstacleParkingLot` — deferred obstacles awaiting capacity

## Dependencies

- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-cns` — CNS span emission for kata events
- `hkask-storage` — persistent kata state
