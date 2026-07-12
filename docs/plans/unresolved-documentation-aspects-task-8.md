---
title: "Documentation Initiative — Unresolved Aspects for Follow-Up"
audience: [architects]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Documentation Initiative — Unresolved Aspects (Task 8)

**Purpose:** Identify underspecified aspects of the documentation initiative that require decisions before the full documentation set can be declared complete. These questions should be resolved in a dedicated decision session before or during Task 3 implementation.

---

## Question A: Canonical Location for Architecture Documentation

**Current state:** The `AGENTS.md` states: "Architecture docs (`docs/architecture/`) are under reconstruction. CI invariants and crate-level doc comments are the current authority for design constraints."

The `README.md` echoes: "The authoritative source for principles and invariants is the CI pipeline (`ci.yml` invariants job) and crate-level doc comments."

**Open question:** What is the canonical location — `docs/architecture/`, inline `//!` crate-level doc comments, or both — and what is the single-source-of-truth policy when they conflict?

**Options:**
1. **Both, with clear precedence chain:** crate-level doc comments are the executable truth (verified by `cargo test --doc`), `docs/architecture/` is the narrative truth. When they conflict, `cargo doc` output wins.
2. **Architecture docs only:** remove architecture content from crate-level doc comments; everything lives in `docs/architecture/`. CI verifies cross-references.
3. **Crate-level doc comments only:** migrate architecture content into `//!` module docs; `docs/architecture/` becomes an index pointing to `rustdoc` output.

**Impact:** Decision affects the explanation quadrant structure, CI verification design, and the workload for Task 4 document creation.

---

## Question B: Primary Audience Persona

**Current state:** The existing documentation serves three audiences unevenly: end-users (14 user guides), skill-authors (skill-designer-guide, skill-composition-guide), and core developers (architecture master, PRINCIPLES, ADRs). The tutorial quadrant has only 3 documents vs. 14 how-to guides.

**Open question:** Which audience has primacy, and how should the tutorial quadrant weight its content?

**Options:**
1. **End-user first:** Tutorial focuses on running `kask`, invoking skills, managing pods. Skill-authoring and MCP development are advanced tutorials.
2. **Skill-author first:** Tutorial focuses on writing a skill from scratch. Running `kask` is a prerequisite appendix.
3. **Core developer first:** Tutorial focuses on understanding the architecture, contributing to crates. Skills and user operations are documented in how-to guides.

**Impact:** Determines the structure and content of `getting-started.md` and whether we need separate tutorials for each persona.

---

## Question C: Documentation Staleness Threshold

**Current state:** The enhanced `verify-docs.sh` uses 30 commits as the staleness threshold for `last-verified-against` hashes. This is an initial setting, not a considered threshold.

**Open question:** What threshold triggers a mandatory documentation update, and should staleness be a CI gate (fail the build) or a CNS advisory (warn in the Curator feed)?

**Options:**
1. **Commit-based:** N commits since `last-verified-against` → CI WARNING; 3N commits → CI ERROR
2. **Calendar-based:** 30 days → WARNING; 90 days → ERROR
3. **Symbol-based:** Count commits that touch symbols referenced in a doc → if any symbol authoring commit is newer than `last-verified-against`, flag
4. **CNS advisory:** Don't fail CI on staleness; emit `cns.doc.stale` spans that the Curator monitors. Documentation staleness becomes a variety signal.

**Impact:** Affects CI pipeline severity, developer workflow (does a doc update block a merge?), and the cybernetic feedback loop design.

---

## Question D: Generated API Docs vs. Hand-Written Reference

**Current state:** `cargo doc` generates rustdoc output for all workspace crates. The Diataxis architecture design proposes hand-written reference documents under `docs/reference/api/`. These are two separate surfaces describing the same API.

**Open question:** Should generated API docs be considered part of the reference quadrant or a separate surface?

**Options:**
1. **Unified:** `docs/reference/api/*.md` files are the canonical reference; they embed links to `rustdoc` for details. CI verifies hand-written docs match rustdoc.
2. **Separate:** `rustdoc` is the canonical API reference; `docs/reference/api/` contains only narrative guides and cross-crate relationship documentation.
3. **Bridge:** `docs/reference/api/README.md` points to `rustdoc` output as the primary reference; crate-specific pages exist only for crates with complex public APIs that benefit from narrative description.

**Impact:** Determines the scope of Task 4 reference document creation (22 planned documents vs. a single bridge index).

---

## Question E: Skill Documentation Depth

**Current state:** 38 skills have SKILL.md files in `.agents/skills/` (generated companions for the agent runtime). 22 of 38 skills have no `docs/` entry beyond AGENTS.md catalog listing.

**Open question:** Which skills require individual tutorial/how-to pages vs. being covered by the registry-style reference listing alone?

**Options:**
1. **All 38 get individual pages:** Every skill gets a reference page with its FlowDef parameters, gas budget, and convergence threshold. Top 10 most-used skills get how-to guides.
2. **Tiered approach:** Guardrails + Core Development skills get how-to guides (10 skills). Reasoning + Specialized skills get reference pages only (20 skills). Kata + Meta skills get reference pages with workflow context (8 skills).
3. **Reference-only:** All 38 skills are covered by the registry listing. Tutorial content is covered by the `design-a-skill.md` how-to, which teaches the reader to read any skill's manifest.

**Who decides:** The Curator (via metacognition analysis of skill invocation frequency) or human architects? If Curator-decided, this becomes a feedback loop — skill docs are generated when invocation frequency crosses a threshold.

---

## 6. Decision Cadence

**Recommended approach:** Hold a 2-hour decision session covering all five questions. Each question gets 20 minutes: 5 min presentation of options, 10 min discussion, 5 min decision. Document decisions as ADRs (ADR-047 through ADR-051) before proceeding with Task 3 implementation.

**Pre-reading:** This document, the `diataxis-architecture-design.md` blueprint, and the `documentation-inventory-2026-07-07.md` inventory report.

---

*Task 8 deliverable. To be resolved before Task 3 implementation proceeds to full document creation.*
