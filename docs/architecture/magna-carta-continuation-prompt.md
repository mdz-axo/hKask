# hKask Magna Carta Refactoring — Continuation Prompt

## Session Context

This session performed a major refactoring of hKask's Magna Carta charter and sovereignty architecture. The work is approximately 90% complete. What remains is creating the YAML manifest files and Jinja2 templates for the Magna Carta Verifier skill, plus a small amount of wiring.

## What Was Done

### Principle Changes
The Magna Carta was reduced from 5 principles to 4:

1. **User Sovereignty** (was #2, now #1) — Grounded in SOLID architecture principles. Added: atomic consent (unbundled, ≤5 sentences per term), resource verification at onboarding, data portability.
2. **Affirmative Consent** (was #3 "Acquisition Resistance", renamed) — Default deny. Added: consent scope/versioning/expiration, hierarchical consent structures (master/per-agent/per-agent-type), fail-closed default.
3. **Generative Space** (was #5, now #3) — Expanded: settings exposure (not absence of constraints), no privileged engineer access, open-source commitment, user curation (HHH is a user tool), non-normativity (first-person vs aggregate).
4. **Clear Boundaries / OCAP** (was #1, now #4) — Repositioned as holistic verification that P1–P3 are correctly enforced through OCAP boundaries. Dual enforcement gate (require_capability + require_sovereignty), unforgeable attenuating tokens.

### Kill-Zone Removal
Principle #4 (Kill-Zone Detection) was removed entirely. Two fatal flaws identified:
1. **Speculative judgment** — pre-judged entities without affirmative violations
2. **False universal** — assumed VC investment undermines sovereignty

All kill-zone code, docs, types, API endpoints, CLI commands, CNS spans, and MCP tools have been removed. Zero downstream consumers existed.

### Renames
- "Acquisition Resistance" → "Affirmative Consent" throughout
- `acquisition_resistance: bool` → `requires_affirmative_consent: bool` in `DataSovereigntyBoundary`
- `AcquisitionResistance` enum type → removed (was already simplified to bool)
- `resistance_name()` → `consent_name()` in API (maps `true` → `"required"`, `false` → `"open"`)

### Files Modified

**Documentation (all clean, no kill-zone references remain):**
- `docs/architecture/magna-carta.md` — Fully rewritten with 4 principles, SOLID grounding, Verifier skill section, no historical notes
- `docs/README.md` — Updated Magna Carta description
- `docs/architecture/PRINCIPLES.md` — Removed `cns.killzone.*`, updated loop mapping
- `docs/architecture/domain-and-capability.md` — Removed KillZone span
- `docs/architecture/hKask-architecture-master.md` — Updated description
- `docs/architecture/loop-architecture.md` — Removed `cns.killzone.*`
- `docs/architecture/trust-security-observability.md` — Removed kill-zone from auth flow, renamed fields
- `docs/architecture/reference/hKask-erd.md` — Removed killzone subgraph
- `docs/architecture/reference/registry-erd.md` — Removed cns_killzone table, relation, SQL
- `docs/architecture/reference/subsystem-erds.md` — Renamed fields, removed KillZone from SpanCategory
- `docs/specifications/DEPLOYMENT.md` — Removed /api/sovereignty/killzone endpoint

**Rust Code (all compile cleanly with `cargo check`):**
- `crates/hkask-types/src/sovereignty.rs` — Removed `KillZoneState`, removed kill-zone fields from `UserSovereigntyState`, renamed `acquisition_resistance` → `requires_affirmative_consent`
- `crates/hkask-types/src/event.rs` — Removed `cns.killzone` from CANONICAL_NAMESPACES and SpanCategory
- `crates/hkask-types/src/cns.rs` — Removed killzone from span docs
- `crates/hkask-cns/src/kill_zone.rs` — **DELETED** entirely
- `crates/hkask-cns/src/runtime.rs` — Removed KillZoneDetector field, check_kill_zone, fire_killzone_alert, kill_zone_state methods
- `crates/hkask-cns/src/lib.rs` — Removed `mod kill_zone`
- `crates/hkask-agents/src/sovereignty.rs` — Unchanged (already clean)
- `crates/hkask-api/src/routes/sovereignty.rs` — Removed KillZoneResponse, /api/sovereignty/killzone, kill-zone fields from SovereigntyStatusResponse, renamed acquisition_resistance → requires_affirmative_consent
- `crates/hkask-api/src/routes/mod.rs` — Removed KillZoneResponse re-export
- `crates/hkask-api/src/lib.rs` — Removed killzone endpoint doc
- `crates/hkask-cli/src/cli/actions.rs` — Removed MarkAcquisition and KillZone variants
- `crates/hkask-cli/src/commands/sovereignty.rs` — Removed kill-zone commands, SOVEREIGNTY_STATE mutex
- `crates/hkask-cli/src/repl/commands.rs` — Updated sovereignty display to delegate to status handler
- `crates/hkask-storage/src/sovereignty.rs` — Already updated (no kill-zone, renamed field)
- `crates/hkask-storage/src/nu_event_store.rs` — Already updated (removed killzone from categories)
- `mcp-servers/hkask-mcp-cns/src/main.rs` — Removed KillZoneRequest, cns_kill_zone tool

**Skill (created, partial):**
- `.agents/skills/magna-carta-verifier/SKILL.md` — Created with YAML front matter and full content
- `.agents/skills/magna-carta-verifier/manifests/` — Directory created, no manifest files yet
- `.agents/skills/magna-carta-verifier/templates/` — Directory created, no template files yet

## What Remains

### 1. Create YAML Manifest Files (HIGH PRIORITY)

Four manifest files need to be written in `.agents/skills/magna-carta-verifier/manifests/`:

**p1-user-sovereignty.yaml** — Assertions:
- p1a: Every code path to sovereign data is gated by `SovereigntyChecker` (structural_audit, targets: `hkask-agents::pod::context` methods store_episodic, recall_episodic, store_semantic, recall_semantic)
- p1b: Non-owner access to sovereign data is denied (behavioral_probe, targets: sovereign DataCategories, non-owner WebID)
- p1c: Every resource is correctly categorized before platform entry (resource_verification, targets: DataSovereigntyBoundary::hkask_default)
- p1d: Sovereign data is portable (structural_audit, targets: hkask-storage export paths)
- p1e: Consent terms are atomic — unbundled, ≤5 sentences per term (structural_audit, targets: ConsentManager)

**p2-affirmative-consent.yaml** — Assertions:
- p2a: Default is deny — no access without explicit consent (structural + behavioral, targets: DenyAllConsent, SovereigntyChecker)
- p2b: Consent grants are scoped to categories and resource versions (structural, targets: consent records)
- p2c: Consent grants expire by date or version upgrade (structural + behavioral, targets: consent expiration)
- p2d: Consent structures are hierarchical: master/per-agent/per-agent-type (structural, targets: consent resolution)
- p2e: Fail-closed: misconfiguration defaults to deny (behavioral, targets: no consent port wired)

**p3-generative-space.yaml** — Assertions:
- p3a: Inference/tooling expose all probabilistic settings (structural, targets: Okapi/llama.cpp options surface via API/CLI/MCP)
- p3b: No privileged engineer access to settings (absence_check, targets: no admin-only settings exist)
- p3c: Generative resources are open-source with exposed weights (structural + behavioral, targets: inference providers)
- p3d: HHH and persona filters are user-selectable, not hardcoded (structural + behavioral, targets: HHH pipeline)
- p3e: User preference overrides take precedence over LLM defaults (absence_check, targets: no system-level normative overrides)

**p4-clear-boundaries.yaml** — Assertions:
- p4a: Every access path goes through require_capability + require_sovereignty (structural + behavioral, targets: all PodContext methods)
- p4b: Capability tokens are unforgeable and attenuating (structural, targets: token creation/delegation path)
- p4c: Generative settings tokens obtainable through P2's affirmative consent (structural, targets: consent hierarchy)
- p4d: Connected inference providers expose settings (structural, targets: Okapi settings surface)

**Recommended strategy:** Use the `magna-carta-verifier` skill itself as reference for the structure. Each manifest should declare assertions at the principle level, with targets pointing to current crate/module/method locations. The key insight from the design session: manifests declare *intentions* anchored to principles; code-specific targets evolve; principles are the stable anchor.

### 2. Create Jinja2 Template Files (HIGH PRIORITY)

Three template files in `.agents/skills/magna-carta-verifier/templates/`:

**verification-procedure.md.j2** — Renders verification procedure for each assertion. Should iterate over assertions, render method-specific steps (structural_audit → enumerate gates; behavioral_probe → generate test scenarios; resource_verification → check categorization; absence_check → search for prohibited constructs).

**verification-report.md.j2** — Renders findings, gaps, and status. Should produce a markdown report with: principle name, assertion ID, claim, method, status (pass/fail/gap), findings, and recommendations.

**test-case.rs.j2** — Renders Rust test cases as code blocks in the report. Should generate `#[test]` functions for behavioral probes and absence checks, with `// REQ:` comments linking back to the assertion.

**Recommended strategy:** Use hKask's existing Jinja2 template patterns from `hkask-templates` as reference. Keep templates simple — they render markdown that the agent reads, not executable code that runs directly.

### 3. Wire Verifier Skill to hKask Infrastructure (MEDIUM PRIORITY)

- The verifier should be triggerable from the CLI: `kask verify magna-carta` (or similar)
- Triggers defined in the SKILL.md: start-up, expiration, user change, resource/service change
- On failure: escalate to Curator → review with human user or replicant in chat session
- This requires a new CLI subcommand and potentially a new MCP tool

### 4. Database Migration (LOW PRIORITY)

- The `sovereignty_boundaries` table schema changed: `resistance` column → `requires_affirmative_consent`, `kill_zone_threshold` column removed
- Existing databases need a migration. This is a schema-breaking change.
- Consider whether to write a migration script or handle it as a fresh start (hKask may not have production databases yet)

### 5. AGENTS.md Update (LOW PRIORITY)

- The project-level `AGENTS.md` references the CNS span registry and kill-zone. It was already clean when checked, but verify after all changes.

## Recommended Skills and Tools

- **`magna-carta-verifier` skill** — Use it to validate the manifests once created
- **`coding-guidelines` skill** — Follow Karpathy principles when writing the manifest/template code (minimal, no speculative features, surgical changes)
- **`zoom-out` skill** — If you get lost in the weeds of manifest structure, zoom out to verify alignment with the four principles
- **`tdd` skill** — If you write Rust code for the CLI subcommand or MCP tool, follow the TDD workflow
- **`cargo check -p <crate>`** — Verify compilation after each crate change
- **`cargo clippy -p <crate> -- -D warnings`** — Verify lint after compilation

## Key Architectural Decisions to Preserve

1. **No kill-zone.** It's gone. Don't re-introduce any form of speculative entity judgment.
2. **"Affirmative Consent" not "Acquisition Resistance".** The name describes what the system *does*, not what it *resists*.
3. **Four principles only.** P1 (User Sovereignty, SOLID-grounded), P2 (Affirmative Consent), P3 (Generative Space), P4 (Clear Boundaries/OCAP as holistic verification of P1-P3).
4. **Atomic consent (P1).** Consent terms must be unbundled, each ≤5 sentences.
5. **Consent scope/versioning/expiration (P2).** Consent is not indefinite; it expires or re-validates on resource version changes.
6. **Hierarchical consent (P2).** Master/per-agent/per-agent-type.
7. **Generative = settings exposure (P3).** Not "no constraints" but "all settings equally exposed to all users."
8. **Open-source commitment (P3).** Closed-weight/closed-code projects structurally cannot meet the criteria.
9. **No historical notes in docs.** Documents state current practices and policies only.
10. **Verifier escalation goes to Curator → human user/replicant.** Not automated blocking.