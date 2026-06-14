---
title: "Document Category Ownership"
audience: [project maintainers, documentation stewards]
last_updated: 2026-06-14
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Document Category Ownership

**Purpose:** Assigns clear ownership to each documentation directory, defines maintenance responsibilities, and establishes the review cadence for the documentation corpus.

**Governing Principles:** P5 (Pared Surface), P6 (No Dead Docs), P8 (Semantic Grounding)

---

## 1. Category Ownership Map

| Directory | Owner | Review Cadence | Auto-Generated? | Notes |
|-----------|-------|---------------|-----------------|-------|
| `docs/architecture/` | Architecture steward | Per-release | No | Framework docs, ADRs, reference artifacts |
| `docs/specifications/` | Documentation steward | Per-release | No | Standards, templates, governance |
| `docs/plans/` | Workstream lead | Weekly | No | Active work plans and roadmaps |
| `docs/handoffs/` | Agent sessions (transient) | Per-session | No | Self-cleaning via lifecycle policy |
| `docs/status/` | CI/CD steward | Per-build | Mixed | Inventories auto-generated; PROJECT_STATUS.md manual |
| `docs/user-guides/` | User advocate | Per-release | No | End-user facing documentation |
| `docs/guides/` | Methodology steward | Per-release | No | Research and practice guides |
| `docs/audit/` | Audit steward | Per-audit-cycle | No | **Archived 2026-06-14.** Completed audit bundles moved to `docs/archive/2026-06-14-crate-audit/`. Directory retained for future audit cycles. |
| `docs/generated/` | Build system | Per-build | **Yes** | CLI reference, OpenAPI spec (future) |
| `docs/ci/` | CI/CD steward | Per-build | No | Verification scripts |
| `docs/archive/` | Documentation steward | Per-sweep | No | Gitignored snapshot; git history is canonical |

---

## 2. Steward Responsibilities

### Architecture Steward
- Maintains `docs/architecture/hKask-architecture-master.md` as the authoritative index
- Ensures all ADRs are indexed in the master document
- Reviews framework documents for staleness per release
- Approves new ADRs for structural correctness

### Documentation Steward
- Maintains `DOCUMENTATION_STANDARDS.md`, `MDS_SCAFFOLD.md`, `WRITING_EXCELLENCE.md`
- Runs document corpus hygiene sweeps (per release or when corpus grows >10%)
- Owns the `corpus_inventory.yaml` regeneration process
- Owns the `docs/archive/MANIFEST.md`
- Owns `docs/README.md` portal accuracy

### CI/CD Steward
- Maintains `docs/ci/` scripts (`check-links.sh`, `check-metadata.sh`)
- Ensures status inventories are regenerated on build
- Owns `docs/status/PROJECT_STATUS.md`
- Owns version synchronization automation

### Workstream Lead
- Maintains `docs/plans/TODO.md`
- Reviews plan documents weekly for staleness
- Escalates completed plans for archival

### User Advocate
- Maintains `docs/user-guides/`
- Ensures guides are tested against current CLI behavior
- Identifies gaps: runbooks, onboarding guides, replicant setup

### Methodology Steward
- Maintains `docs/guides/`
- Ensures research guides reference current project state

### Audit Steward
- Maintains `docs/audit/` bundles during active audit cycles
- Transitions completed audit bundles to archive when implementation is verified
- **Current state:** All prior audit bundles archived 2026-06-14. Directory empty, ready for next audit cycle.

---

## 3. Version Synchronization

### Rule

Every formal document (with YAML frontmatter) MUST carry `version` matching the workspace `Cargo.toml` version.

### Exceptions

- Documents tracking their own semantic version (e.g., `MDS_SCAFFOLD.md` at 2.5.0, `test-inventory.md` at 2.4.0) may diverge intentionally — but the divergence must be documented in `corpus_inventory.yaml` notes.
- Auto-generated files (`docs/generated/cli-reference.md`) inherit workspace version automatically.
- Handoffs and audit documents have no frontmatter (exempt).

### Procedure

On workspace version bump:
1. Update `Cargo.toml` workspace version.
2. Run `docs/ci/check-metadata.sh` to identify version anomalies.
3. For documents intentionally tracking their own version: verify the divergence is documented.
4. For all other documents: bump `version` field to match workspace.

### Automation Opportunity

A `sync-versions.sh` script could automate Step 4. See `docs/plans/DOCUMENT_ROADMAP.md` for priority.

---

## 4. Auto-Generated vs. Maintained Documents

| Document | Generation Source | Refresh Trigger |
|----------|-----------------|-----------------|
| `status/corpus_inventory.yaml` | Document hygiene sweep (manual agent run) | Per release or corpus change >10% |
| `status/test-inventory.md` | `cargo test --list` piped to markdown | `cargo test` on CI |
| `status/mcp-tools-inventory.md` | MCP tool enumeration (manual or script) | Per MCP server change |
| `status/skill-inventory.md` | **Manual** — updated by skill steward | Per skill change |
| `generated/cli-reference.md` | CLI `--help` text extraction | `cargo build` |
| `generated/openapi.json` | utoipa annotation extraction | `cargo build` (future) |

All auto-generated documents MUST include a header comment noting their generation source and the last generation timestamp.

---

## 5. Document Types Without Current Homes

| Document Type | Recommended Home | Priority | Owner |
|---------------|-----------------|----------|-------|
| Runbooks / operational guides | `docs/guides/` or new `docs/runbooks/` | Medium | User advocate |
| Replicant onboarding guide | `docs/user-guides/` | Low | User advocate |
| OpenAPI specification | `docs/generated/openapi.json` | Medium | CI/CD steward |
| New MCP server specifications | `docs/specifications/` | High | Workstream lead |

---

## 6. Review Cadence

| Trigger | Action | Owner |
|---------|--------|-------|
| Pre-release | Full corpus hygiene sweep: run inventory, re-run CI scripts, archive stale docs | Documentation steward |
| Post-merge (≥10 new docs) | Re-run `corpus_inventory.yaml` regeneration | Documentation steward |
| Weekly | Review `docs/plans/` for staleness | Workstream lead |
| Per-session | Clean superseded handoffs from working tree | Agent / replicant |
| Ad-hoc | Archive completed audit bundles | Audit steward |

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
