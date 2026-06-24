# hkask-services-skill — Skill Service

Skill discovery, publishing, hashing, and lifecycle management. Powers the skill registry — the canonical source of truth for all agent capabilities.

**Version:** v0.30.0 | **Crate:** `hkask-services-skill`

## Modules

| Module | Purpose |
|--------|---------|
| `skill_impl` | `SkillService` — discovery, install, hash verification, publish |

## Key Types

- `SkillService` — primary service interface for skill operations
- `SkillManifest` — skill metadata and content hash

## Dependencies

- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-templates` — template registry and rendering
- `hkask-storage` — persistent skill index
- `hkask-cns` — CNS span emission for skill operations
