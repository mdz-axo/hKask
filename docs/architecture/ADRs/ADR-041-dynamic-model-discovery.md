---
title: "ADR-041: Dynamic Model Discovery Pipeline"
audience: [architects, developers]
last_updated: 2026-06-28
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle, curation]
---

# ADR-041: Dynamic Model Discovery Pipeline

**Date:** 2026-06-28
**Status:** Active

## Context

hKask's onboarding previously used a hardcoded list of model names in `crates/hkask-cli/src/onboarding.rs`. This list was stale — models like "Qwen3.5 397B" and "Kimi2.6" were listed as simple names without provider-specific model IDs, thinking/instruct classification, or freshness validation. The default provider was also hardcoded to DeepInfra.

**Problem Statement:** Onboarding presented stale, provider-agnostic model names that did not reflect the current near-frontier open-weight model landscape or the user's configured cloud inference provider.

**Stakeholders:** CLI users (first-run onboarding), `kask onboard` users, userpod creators.

**Constraints:**
- The pipeline must run during CLI onboarding, before the agent runtime is operational (no template executor available)
- Must work with any configured provider (KiloCode, DeepInfra, Together, fal.ai)
- Must filter to models updated in the last 6 months
- Must present one Thinking and one Instruct/flash model per model family
- Must fall back gracefully when APIs are unreachable

## Decision

**Chosen Approach:** A 5-layer Rust pipeline in `crates/hkask-cli/src/onboarding/discovery.rs`:

```
Layer 1: HuggingFace API — fetch text-generation models sorted by recency
Layer 2: Classify & filter — >5000 followers, Thinking vs Instruct, 6-month recency gate
Layer 3: Per-family dedup — keep best Thinking + best Instruct per model family
Layer 4: Cross-reference — fuzzy-match HF models against configured provider's API
Layer 5: Display — UI with family grouping, 🧠/📋 icons, hKask fusion integration
```

Three-tier fallback chain: HF → provider API → static curated lists (provider-specific).

**Alternatives Considered:**
1. **Jinja2 template** — Rejected because the runtime is unavailable during onboarding. A `registry/templates/onboarding/refresh-models.j2` template is planned for post-onboarding model refresh.
2. **Provider API only** — Rejected because provider APIs don't expose follower counts or unified model family metadata. HuggingFace provides follower counts and cross-provider metadata.
3. **Static curated list only** — Rejected because it goes stale and requires manual updates.

**Rationale:** The HuggingFace API provides the richest metadata (follower counts, recency, model family) for quality filtering. The provider API cross-reference ensures the output is provider-correct. The Rust implementation is necessary because onboarding runs before the agent runtime.

## Consequences

### Positive
- Onboarding always shows current near-frontier models (≤6 months, >5000 followers)
- Provider-agnostic: works with KiloCode, DeepInfra, Together, fal.ai
- Family deduplication prevents model catalog overload (max 2 per family)
- Thinking/Instruct classification helps users choose the right model type
- hKask's own fusion orchestrator is integrated (not just OpenRouter)
- Graceful fallback: HF → provider API → static lists

### Negative
- HuggingFace API requires N+1 HTTP calls (1 for models, N for author follower counts). Adds ~5-10s latency during onboarding.
- Fuzzy cross-reference matching may produce false negatives for models with unusual naming (e.g., dots in version numbers like "qwen3.5")
- The static fallback lists still contain hardcoded model IDs (provider-structural, not model names — bounded by provider count)
- No post-onboarding model refresh mechanism (planned as a `.j2` template)

### Neutral
- `author_to_family` mapping requires occasional maintenance when new orgs publish frontier models
- Scoring heuristics (`compute_quality_score`) use substring matching; not based on actual benchmark data

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| P6 (Delete stubs, don't publish) | ✅ | All unused functions deleted (`is_likely_llm`, `is_likely_small_model`, `map_to_hkask_provider`) |
| P7 (Prefer deletion over deprecation) | ✅ | Deprecated `FRONTIER_FAMILIES`, `ONBOARDING_MODELS`, `describe_model` — all deleted, not annotated |

### Constraints

| Constraint | Compliance | Evidence |
|-----------|-----------|----------|
| C4 (Repetition is missing primitive) | ✅ | `classify_and_score` + `compute_quality_score` replaced 4 separate functions in the original code |
| C6 (Stub is debt receipt) | ✅ | No stubs; pipeline is complete with 3-tier fallback |

## Verification

```bash
# Compilation
cargo check -p hkask-inference -p hkask-cli

# Unit tests
cargo test -p hkask-cli -- onboarding::discovery
# Expected: 8 passed

# Dead code audit
grep -r "is_likely_llm\|is_likely_small\|FRONTIER_FAMILIES" crates/hkask-cli/src/
# Expected: 0 matches
```

**Expected Results:**
- 8 unit tests pass (classification, family extraction, scoring, display, small-model rejection)
- hkask-inference compiles with 6-month freshness filter on KiloCode backend
- No dead code in discovery or onboarding modules

## Related Documents

- `docs/architecture/core/hKask-architecture-master.md` — Architecture master
- `docs/architecture/core/PRINCIPLES.md` — P9 (Homeostatic Self-Regulation) covers inference configuration
- `crates/hkask-cli/src/onboarding/discovery.rs` — Implementation
- `crates/hkask-inference/src/kilocode_backend.rs` — KiloCode 6-month freshness filter

---

*ℏKask - A Minimal Viable Container for Replicants — v0.31.0*
