---
title: "hLexicon Terminology Propagation Plan — Russell & Okapi"
audience: [architects, developers]
last_updated: 2026-05-24
togaf_phase: "E"
version: "1.0.0"
status: "Active"
domain: "Application"
created: "2026-05-23"
updated: "2026-05-23"
scope: "Russell, Okapi"
depends_on: "INTEGRATED_SPEC_PLAN.md Phase 0"
---

# hLexicon Terminology Propagation Plan — Russell & Okapi

## Executive Summary

This plan documents the propagation of hKask's hLexicon terminology change ("budget" → "allocation") to downstream ecosystem repositories: **Russell** (ACP agent) and **Okapi** (inference engine).

**Key Finding:** Neither Russell nor Okapi currently references hLexicon vocabulary governance, term counts, or the "budget" constraint. All "budget" references in both repositories are for **consumable resources** (reflex budgets, token budgets, GPU budgets, error budgets) and should remain unchanged.

**Impact:** Minimal — documentation-only updates to inform downstream consumers of the terminology and governance model. No code changes required.

**Effort:** 2-3 hours across both repositories.

---

## 1. Impact Assessment

### 1.1 Russell Repository

**hLexicon Integration Points:**
- **13 skill manifests** (`skills/*/hlexicon.yaml`) — declare primary domain and terms
- **Code references** in `russell-acp-server/src/types.rs` and `russell-skills/src/lib.rs`
- **Documentation** in ADR-0027 (ACP integration) and ecosystem-integration.md

**Terminology Usage:**
- ✅ **No "budget" references** in hLexicon vocabulary context
- ✅ **No term count constraints** referenced (75-term allocation)
- ✅ **No governance model** documented (Russell consumes hLexicon, doesn't govern it)

**"Budget" References Found:**
- `reflex_budget` — execution rate limiting (consumable)
- `error budget` — SRE SLO tracking (consumable)
- `budget_tokens` — LLM token allocation (consumable)

**Conclusion:** All "budget" references are correct — they describe consumable resources, not vocabulary allocation.

### 1.2 Okapi Repository

**hLexicon Integration Points:**
- **Contract validator** in `OKAPI_HKASK_IMPLEMENTATION_SPEC.md` (error messages reference hLexicon terms)
- **Validation code** checks template `lexicon_terms` against canonical vocabulary

**Terminology Usage:**
- ✅ **No "budget" references** in hLexicon vocabulary context
- ✅ **No governance model** documented (Okapi validates terms, doesn't govern vocabulary)

**"Budget" References Found:**
- `token budget` — LLM context window allocation (consumable)
- `GPU budget` — compute resource allocation (consumable)
- `context budget` — prompt section allocation (consumable)

**Conclusion:** All "budget" references are correct — they describe consumable resources, not vocabulary allocation.

---

## 2. Propagation Strategy

### 2.1 What Changes

**Russell:**
1. Add terminology note to `docs/architecture/ecosystem-integration.md`
2. Add hLexicon governance reference to ADR-0027 (informational only)

**Okapi:**
1. Add terminology note to `fork-docs/STRATEGY.md`
2. Update `OKAPI_HKASK_IMPLEMENTATION_SPEC.md` error message (optional, low priority)

### 2.2 What Does NOT Change

- **Skill manifests** (`skills/*/hlexicon.yaml`) — no governance metadata
- **Code** — no vocabulary constraint logic in either repo
- **Consumable resource "budget" references** — all correct as-is

---

## 3. Implementation Tasks

### 3.1 Russell: Ecosystem Integration Note

**File:** `docs/architecture/ecosystem-integration.md`

**Location:** After "## 1. hKask MCP Tool Exposure" section

**Add:**

```markdown
### 1.1 hLexicon Terminology & Governance

Russell skills declare hLexicon terms (WordAct/FlowDef/KnowAct) in their `hlexicon.yaml` manifests. These terms are **allocated** (not budgeted) by hKask's governance model:

- **Allocation:** 75 terms allocated across 3 domains (currently 80, with 5 git evolution terms exceeding allocation)
- **Governance:** `hKask/registry/registries/hlexicon-governance.yaml` defines the `term_allocation` section
- **Expansion:** When hKask extends hLexicon (e.g., spec-curation terms), Russell skills may adopt new terms without code changes

**Key distinction:** Vocabulary terms are allocated, not consumed. A term like `curate` is not depleted by use — it is deliberately placed and reused infinitely. This differs from consumable resources like `reflex_budget` or `token_budget`, which are depleted by use.

**Reference:** hKask `docs/architecture/hKask-hLexicon.md` and `docs/plans/LEXICON_TERMINOLOGY_CHANGE.md`
```

### 3.2 Russell: ADR-0027 Reference Update

**File:** `docs/adr/0027-acp-integration.md`

**Location:** In "References" section at end

**Add:**

```markdown
- hKask hLexicon Governance — `hKask/registry/registries/hlexicon-governance.yaml` (term allocation model)
- hKask Terminology Change — `hKask/docs/plans/LEXICON_TERMINOLOGY_CHANGE.md` ("budget" → "allocation" for vocabulary)
```

### 3.3 Okapi: Strategy Note

**File:** `fork-docs/STRATEGY.md`

**Location:** After "## hKask Integration" section (if exists) or in new "## Ecosystem Alignment" section

**Add:**

```markdown
## hLexicon Terminology Alignment

Okapi's contract validator checks template `lexicon_terms` against hKask's canonical hLexicon vocabulary. As of hKask v0.22.0, the vocabulary governance uses "allocation" (not "budget") terminology:

- **Term allocation:** 75 terms allocated across 3 domains (WordAct, FlowDef, KnowAct)
- **Current usage:** 80 terms (5 over allocation for git evolution terms)
- **Expansion policy:** Formal expansion with written justification (as git terms did)

**Why this matters for Okapi:** When hKask extends hLexicon (e.g., spec-curation terms like `curate`, `elicit`, `reconcile`), Okapi's contract validator must recognize the new terms. The validator's error message references "hLexicon" — this remains correct, but the governance model is now explicitly "allocation" rather than "budget."

**No code changes required:** Okapi's validator checks term membership, not governance metadata. The terminology change is informational for Okapi developers.

**Reference:** hKask `docs/architecture/hKask-hLexicon.md` and `docs/plans/LEXICON_TERMINOLOGY_CHANGE.md`
```

### 3.4 Okapi: Error Message Update (Optional, Low Priority)

**File:** `fork-docs/plans/OKAPI_HKASK_IMPLEMENTATION_SPEC.md`

**Current error message:**
```rust
#[error("Invalid lexicon term '{term}' - not found in hLexicon. Available terms: {available_terms:?}. Use only canonical hLexicon terms to ensure consistent LLM interpretation.")]
UnknownLexiconTerm { term: String, available_terms: Vec<String> },
```

**Proposed update (optional):**
```rust
#[error("Invalid lexicon term '{term}' - not found in hLexicon allocation. Available terms: {available_terms:?}. Use only canonical hLexicon terms from the allocated vocabulary to ensure consistent LLM interpretation.")]
UnknownLexiconTerm { term: String, available_terms: Vec<String> },
```

**Rationale:** "Allocation" is more precise than implicit "vocabulary." However, this is a cosmetic change — the error is already clear. **Recommendation:** Defer until next Okapi contract validator refactor.

---

## 4. Verification

### 4.1 Russell Verification

```bash
# Verify no "budget" references in hLexicon vocabulary context
cd /home/mdz-axolotl/Clones/russell
grep -r "term.*budget\|vocabulary.*budget\|lexicon.*budget" docs/ skills/ --include="*.md" --include="*.yaml"
# Expected: 0 matches

# Verify ecosystem integration note added
grep -A 5 "hLexicon Terminology & Governance" docs/architecture/ecosystem-integration.md
# Expected: Section present with allocation terminology

# Verify ADR-0027 references added
grep "LEXICON_TERMINOLOGY_CHANGE" docs/adr/0027-acp-integration.md
# Expected: Reference present
```

### 4.2 Okapi Verification

```bash
# Verify no "budget" references in hLexicon vocabulary context
cd /home/mdz-axolotl/Clones/okapi
grep -r "term.*budget\|vocabulary.*budget\|lexicon.*budget" fork-docs/ --include="*.md"
# Expected: 0 matches

# Verify strategy note added
grep -A 5 "hLexicon Terminology Alignment" fork-docs/STRATEGY.md
# Expected: Section present with allocation terminology

# Verify consumable resource "budget" references unchanged
grep -r "token.*budget\|GPU.*budget\|context.*budget" fork-docs/ --include="*.md" | wc -l
# Expected: ≥10 matches (all correct, unchanged)
```

---

## 5. Downstream Communication

### 5.1 Russell Maintainers

**Message:**
> hKask v0.22.0 updates hLexicon governance terminology from "budget" to "allocation" (vocabulary terms are allocated, not consumed). This is a documentation-only change — Russell's skill manifests and code are unaffected. We've added informational notes to `docs/architecture/ecosystem-integration.md` and ADR-0027 to align terminology. All "budget" references in Russell (reflex_budget, error budget) remain correct as they describe consumable resources.

**Channel:** GitHub PR comment or direct message to Russell maintainers

### 5.2 Okapi Maintainers

**Message:**
> hKask v0.22.0 updates hLexicon governance terminology from "budget" to "allocation." Okapi's contract validator is unaffected — it checks term membership, not governance metadata. We've added an informational note to `fork-docs/STRATEGY.md` to align terminology. All "budget" references in Okapi (token budget, GPU budget) remain correct as they describe consumable resources.

**Channel:** GitHub PR comment or direct message to Okapi maintainers

---

## 6. Future hLexicon Extensions

When hKask extends hLexicon (e.g., Phase 8 spec-curation terms), downstream repositories should:

1. **Russell:** Update skill manifests to adopt new terms (optional, per-skill decision)
2. **Okapi:** Update contract validator's `hlexicon_terms` HashSet to recognize new terms
3. **Both:** No governance changes required — hKask owns the allocation model

**Example:** When `curate`, `elicit`, `reconcile` are added to hLexicon:
- Russell's `skill-manager` might adopt `curate` in its `hlexicon.yaml`
- Okapi's validator must add these terms to its canonical set
- Neither repo needs to update governance documentation

---

## 7. Acceptance Criteria

- [ ] Russell `docs/architecture/ecosystem-integration.md` includes hLexicon terminology note
- [ ] Russell `docs/adr/0027-acp-integration.md` references hKask terminology change
- [ ] Okapi `fork-docs/STRATEGY.md` includes hLexicon terminology note
- [ ] Zero "budget" references in hLexicon vocabulary context in either repo
- [ ] All consumable resource "budget" references unchanged in both repos
- [ ] Downstream maintainers notified of terminology change
- [ ] Verification commands pass in both repositories

---

## 8. Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Downstream maintainers reject terminology note | Low | Low | Frame as informational, not prescriptive |
| Confusion between vocabulary "allocation" and resource "budget" | Medium | Low | Clear documentation with examples |
| Future hLexicon extensions not propagated | Medium | High | Document extension process in this plan |

---

## 9. Deferred Questions

| # | Question | Trigger |
|---|----------|---------|
| D1 | Should Russell skill manifests declare `allocation_aware: true` metadata? | hLexicon governance becomes machine-readable |
| D2 | Should Okapi's validator error message reference "allocation" explicitly? | Next Okapi contract validator refactor |
| D3 | Should hKask publish hLexicon terms as a versioned artifact (e.g., `hlexicon-terms-v1.0.yaml`)? | Multiple downstream consumers need canonical term list |

---

*ℏKask hLexicon Terminology Propagation Plan v0.22.0 — 2026-05-23*
*Two repositories, zero code changes, minimal documentation updates.*
*Vocabulary is allocated, not budgeted. Resources are budgeted, not allocated.*
