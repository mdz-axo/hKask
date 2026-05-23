---
title: "Terminology Change: hLexicon 'Budget' → 'Allocation'"
togaf_phase: "Phase C-App"
status: "proposed"
author: "hKask Architecture Team"
created: "2026-05-23"
updated: "2026-05-23"
scope: "hKask, Russell, Okapi"
priority: "medium"
effort: "low"
related_docs:
  - hKask-hLexicon.md
  - spec-curation-hypothesis.md
  - PRINCIPLES.md
  - hlexicon-governance.yaml
---

# Terminology Change: hLexicon "Budget" → "Allocation"

## Executive Summary

This plan documents a terminology refinement in hKask's hLexicon governance: replacing "budget" with "allocation" when referring to the vocabulary term count constraint. The change is scoped to hLexicon vocabulary governance only; all other uses of "budget" (energy, tokens, LOC, iterations) remain unchanged as they correctly describe consumable resources.

**Rationale:** Vocabulary terms are not consumed by use — they are deliberately placed and reused infinitely. "Budget" implies depletion; "allocation" implies intentional distribution across domains.

---

## 1. Scope Analysis

### 1.1 What Changes

**hLexicon vocabulary constraint only:**

| Current Phrasing | New Phrasing | Rationale |
|-----------------|--------------|-----------|
| "≤75 terms total" | "75 terms allocated across 3 domains" | Allocation implies distribution, not ceiling |
| "term budget" | "term allocation" | Vocabulary is not consumed |
| "vocabulary budget" | "vocabulary allocation" | Same rationale |
| "exceeds budget" | "exceeds allocation" | Same rationale |

### 1.2 What Does NOT Change

**All consumable resource budgets remain "budget":**

| Resource Type | Example Usage | Why "Budget" is Correct |
|--------------|---------------|------------------------|
| Energy budget | `energy_budget` in manifests | Tokens consumed during inference |
| Token budget | `token_budget` in manifests | Context window depleted by use |
| LOC budget | "30k Rust line budget" | Lines of code are a complexity constraint |
| Iteration budget | `iteration_budget` in kata | Iterations consumed per session |
| GPU budget | `gpuBudget` in Okapi | VRAM/layers consumed by models |
| Reflex budget | `ReflexBudget` in Russell | Interventions per hour (rate limit) |
| Error budget | SRE error budget | Reliability consumed by incidents |

**These are genuinely consumable resources.** You spend tokens, deplete VRAM, accumulate LOC, exhaust iterations. "Budget" is the accurate metaphor.

### 1.3 Affected Files

**hKask (4 files, 6 instances):**

1. `docs/architecture/hKask-hLexicon.md` (2 instances)
   - Line 5: "minimal vocabulary (~75 terms)"
   - Line 12: "Minimal: ≤75 terms total (currently 72)"

2. `docs/architecture/spec-curation-hypothesis.md` (2 instances)
   - Line 259: "75-term budget (already 5 over at 80)"
   - Line 617: "The weakest link is the term budget"

3. `docs/architecture/PRINCIPLES.md` (1 instance)
   - Line 135: "hLexicon grounding (~75 terms)"

4. `registry/registries/hlexicon-governance.yaml` (0 instances, but should add allocation language)
   - Currently silent on term count constraint
   - Should add explicit allocation policy

**Russell (0 files):**
- All "budget" usages are for consumable resources (reflex, tokens, LOC, error budget)
- No hLexicon vocabulary references found
- No changes required

**Okapi (0 files):**
- All "budget" usages are for consumable resources (GPU, tokens, LOC, thinking)
- No hLexicon vocabulary references found
- No changes required

---

## 2. Implementation Plan

### Phase 1: hKask Core Documentation (hKask repository)

#### 2.1 Update `docs/architecture/hKask-hLexicon.md`

**Current:**
```markdown
**hLexicon** is the minimal vocabulary (~75 terms) for composing templates in the hKask system.

**Design Principles:**
- **Minimal:** ≤75 terms total (currently 72)
```

**Proposed:**
```markdown
**hLexicon** is the minimal vocabulary (75 terms allocated across 3 domains) for composing templates in the hKask system.

**Design Principles:**
- **Minimal:** 75 terms allocated across WordAct, FlowDef, KnowAct (currently 80 active)
```

**Rationale:** "Allocated across 3 domains" emphasizes intentional distribution. "Currently 80 active" acknowledges the existing overage (git evolution terms deemed essential).

#### 2.2 Update `docs/architecture/spec-curation-hypothesis.md`

**Instance 1 (Line 259):**

**Current:**
```markdown
Given the orthogonality principle and the 75-term budget (already 5 over at 80), **distribution is more architecturally consistent**.
```

**Proposed:**
```markdown
Given the orthogonality principle and the 75-term allocation (already 5 over at 80), **distribution is more architecturally consistent**.
```

**Instance 2 (Line 617):**

**Current:**
```markdown
**The weakest link** is the term budget. The governance doc allows 75 terms; we're already at 80 and proposing 89.
```

**Proposed:**
```markdown
**The weakest link** is the term allocation. The governance doc allocates 75 terms; we're already at 80 and proposing 89.
```

#### 2.3 Update `docs/architecture/PRINCIPLES.md`

**Current:**
```markdown
- hLexicon grounding (~75 terms)
```

**Proposed:**
```markdown
- hLexicon grounding (75 terms allocated across 3 domains)
```

#### 2.4 Update `registry/registries/hlexicon-governance.yaml`

**Add new section after `validation_rules`:**

```yaml
  # Term Allocation Policy
  term_allocation:
    total_allocation: 75
    distribution:
      wordact: 25
      flowdef: 32
      knowact: 23
    current_usage: 80  # 5 over due to git evolution terms (deemed essential)
    policy: |
      hLexicon terms are allocated, not budgeted. Terms are not consumed by use;
      they are deliberately placed in the vocabulary and reused infinitely across
      templates and manifests. The allocation constraint enforces cognitive coherence
      and composition clarity, not resource depletion.
      
      When the vocabulary exceeds allocation, the governance body must either:
      1. Retire underused terms (zero codebase matches)
      2. Formally expand the allocation with justification
      3. Accept the overage as essential (as with git evolution terms)
      
      This policy aligns with the curation reframing: vocabulary management is
      curation (invitation-based, gradient evaluation), not governance (constraint-based,
      binary gates).
    enforcement: quarterly_review
```

**Rationale:** Makes the allocation policy explicit, documents the curation framing, and provides decision criteria for overage.

### Phase 2: Ecosystem Propagation (Russell, Okapi)

#### 2.5 Russell Integration

**Finding:** Russell does not reference hLexicon vocabulary constraints. All "budget" usages are for consumable resources (reflex execution, tokens, LOC, error budget).

**Action:** No code changes required. However, Russell's integration with hKask should be aware of the terminology distinction for future hLexicon-related features.

**Documentation update (optional):**

Add a note to `docs/architecture/ecosystem-integration.md`:

```markdown
### hLexicon Terminology

When Russell integrates with hKask's hLexicon system, note the terminology distinction:

- **Allocation:** hLexicon vocabulary terms (75 allocated across 3 domains). Terms are not consumed; they are deliberately placed and reused.
- **Budget:** Consumable resources (tokens, energy, LOC, iterations). These are depleted by use.

Russell's skill manifests may reference hLexicon terms in `lexicon_terms` arrays. The allocation constraint is enforced by hKask's governance system, not Russell.
```

#### 2.6 Okapi Integration

**Finding:** Okapi does not reference hLexicon vocabulary constraints. All "budget" usages are for consumable resources (GPU, tokens, LOC, thinking).

**Action:** No code changes required. Okapi is an inference engine and does not participate in hLexicon governance.

**Documentation update (optional):**

Add a note to `fork-docs/STRATEGY.md` or `fork-docs/plans/MASTER_PLAN.md`:

```markdown
### hLexicon Terminology

Okapi does not enforce hLexicon vocabulary constraints. When Okapi documentation references hKask's hLexicon system, use the following terminology:

- **Allocation:** hLexicon vocabulary terms (managed by hKask governance)
- **Budget:** Consumable resources (tokens, GPU VRAM, LOC — managed by Okapi)

Okapi's context utilization metrics (`/api/engine/status`) report token consumption, which is a budget concern, not an allocation concern.
```

### Phase 3: Verification

#### 2.7 Validation Steps

1. **Search for remaining "budget" in hLexicon context:**
   ```bash
   cd /home/mdz-axolotl/Clones/hKask
   grep -r "term.*budget\|vocabulary.*budget\|lexicon.*budget" docs/ registry/ --include="*.md" --include="*.yaml"
   ```
   **Expected:** 0 matches

2. **Verify "allocation" is used correctly:**
   ```bash
   grep -r "term.*allocation\|vocabulary.*allocation\|lexicon.*allocation" docs/ registry/ --include="*.md" --include="*.yaml"
   ```
   **Expected:** Matches in updated files only

3. **Verify consumable resource "budget" is unchanged:**
   ```bash
   grep -r "energy_budget\|token_budget\|iteration_budget\|gpuBudget" crates/ registry/ --include="*.rs" --include="*.yaml"
   ```
   **Expected:** All existing matches remain (these are correct)

4. **Run hLexicon validation script:**
   ```bash
   ./scripts/validate-hlexicon-alignment.sh
   ```
   **Expected:** No errors (script validates functional_role, not term count)

5. **Run hLexicon balance test:**
   ```bash
   cargo test -p hkask-templates -- hlexicon_balance
   ```
   **Expected:** Pass (test checks distribution balance, not total count)

---

## 3. Rationale

### 3.1 Why the Distinction Matters

**"Budget" is a resource constraint metaphor:**
- You spend from it
- You run out
- You exceed it
- It implies depletion

**Vocabulary terms are not consumed:**
- You don't "spend" `curate` by using it in a template
- Terms are deliberately *placed* in the vocabulary
- They are reused infinitely across templates and manifests
- The constraint is about cognitive coherence, not depletion

**"Allocation" is the accurate frame:**
- It describes intentional distribution across domains
- It implies deliberate placement, not ceilings
- It aligns with the curation reframing (invitation-based, not constraint-based)

### 3.2 Alignment with Spec-Curation Hypothesis

The spec-curation hypothesis reframes hLexicon management as **curation** (not governance). Curation is:
- Invitation-based (users participate in vocabulary evolution)
- Gradient evaluation (merge/revise/defer, not pass/fail)
- Cultivation over time (vocabulary grows with the system)

"Budget" implies governance (constraint-based, binary gates). "Allocation" implies curation (deliberate distribution, adjustable).

### 3.3 Consistency with Existing Practice

The hLexicon doc already says "Minimal: ≤75 terms total (currently 72)" — but the word "minimal" is doing the real work, not "budget." Minimality is a design principle (Occam's razor for vocabulary). Allocation is the mechanism for enforcing it.

The current reality:
- Allocation: 75 terms
- Active: 80 terms (5 over due to git evolution)
- Proposed: 89 terms (with spec-curation extension)

The system already treats the constraint as advisory, not absolute. "Allocation" accurately reflects this practice.

---

## 4. Migration Strategy

### 4.1 Backward Compatibility

**No breaking changes.** This is a documentation-only terminology refinement. No code changes, no API changes, no schema changes.

**Existing references:**
- Code comments using "budget" for consumable resources remain unchanged
- Manifest fields like `token_budget`, `energy_budget` remain unchanged
- Rust types like `ReflexBudget`, `EnergyBudget` remain unchanged

### 4.2 Communication

**Internal (hKask team):**
- Update AGENTS.md with terminology note
- Add to CONTRIBUTING.md if it exists
- Mention in next release notes

**External (Russell, Okapi):**
- Update ecosystem integration docs (optional)
- No action required from Russell/Okapi maintainers

### 4.3 Rollback Plan

If the terminology change causes confusion or is rejected:
1. Revert the 4 documentation files
2. Remove the `term_allocation` section from `hlexicon-governance.yaml`
3. No code rollback required (no code was changed)

---

## 5. Success Criteria

### 5.1 Completion Checklist

- [ ] `docs/architecture/hKask-hLexicon.md` updated (2 instances)
- [ ] `docs/architecture/spec-curation-hypothesis.md` updated (2 instances)
- [ ] `docs/architecture/PRINCIPLES.md` updated (1 instance)
- [ ] `registry/registries/hlexicon-governance.yaml` updated (add `term_allocation` section)
- [ ] `docs/architecture/ecosystem-integration.md` updated (Russell, optional)
- [ ] `fork-docs/STRATEGY.md` or `fork-docs/plans/MASTER_PLAN.md` updated (Okapi, optional)
- [ ] Validation steps pass (Section 2.7)
- [ ] No "budget" in hLexicon context (grep check)
- [ ] Consumable resource "budget" unchanged (grep check)

### 5.2 Verification Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| hLexicon "budget" instances | 0 | `grep -r "term.*budget" docs/ registry/` |
| hLexicon "allocation" instances | ≥4 | `grep -r "term.*allocation" docs/ registry/` |
| Consumable "budget" instances | Unchanged | `grep -r "energy_budget\|token_budget" crates/` |
| Validation script | Pass | `./scripts/validate-hlexicon-alignment.sh` |
| Balance test | Pass | `cargo test -p hkask-templates -- hlexicon_balance` |

---

## 6. Timeline

| Phase | Task | Effort | Owner |
|-------|------|--------|-------|
| 1 | Update hKask documentation | 30 min | hKask-Administrator |
| 2 | Update Russell/Okapi docs (optional) | 15 min | hKask-Administrator |
| 3 | Verification | 10 min | hKask-Administrator |
| **Total** | | **55 min** | |

**Recommended execution:** Single commit, single PR, immediate merge.

---

## 7. Risks and Mitigations

### 7.1 Risk: Confusion Between Allocation and Budget

**Risk:** Users may confuse "allocation" (vocabulary) with "budget" (consumable resources).

**Mitigation:**
- Clear documentation in `hlexicon-governance.yaml` explaining the distinction
- Examples in `hKask-hLexicon.md` showing both concepts
- Consistent usage across all documentation

### 7.2 Risk: Incomplete Propagation

**Risk:** Some documentation may still use "budget" in hLexicon context.

**Mitigation:**
- Grep-based validation (Section 2.7)
- Quarterly review process (already exists in `hlexicon-governance.yaml`)
- Linting rule (optional, future enhancement)

### 7.3 Risk: Resistance to Change

**Risk:** Team members may prefer "budget" as it's more familiar.

**Mitigation:**
- Clear rationale (Section 3)
- Alignment with spec-curation hypothesis (already accepted)
- Low effort, high clarity gain

---

## 8. Future Enhancements

### 8.1 Linting Rule (Optional)

Add a custom lint to `scripts/validate-hlexicon-alignment.sh`:

```bash
# Check for "budget" in hLexicon context
if grep -r "term.*budget\|vocabulary.*budget\|lexicon.*budget" docs/ registry/ --include="*.md" --include="*.yaml" | grep -v "consumable\|token\|energy\|LOC\|iteration"; then
    echo "ERROR: Found 'budget' in hLexicon context. Use 'allocation' instead."
    exit 1
fi
```

### 8.2 Glossary Entry

Add to a future `docs/GLOSSARY.md`:

```markdown
**Allocation:** The intentional distribution of hLexicon vocabulary terms across domains (WordAct, FlowDef, KnowAct). Terms are not consumed by use; they are deliberately placed and reused infinitely. The allocation constraint enforces cognitive coherence, not resource depletion.

**Budget:** A constraint on consumable resources (tokens, energy, LOC, iterations, GPU VRAM). Budgets are depleted by use and must be replenished or expanded.
```

### 8.3 Spec-Curation Integration

When `hkask-mcp-spec` is implemented (per spec-curation-hypothesis.md), the `spec/graph/validate` tool should check vocabulary allocation as part of graph validation:

```rust
async fn handle_graph_validate(server: &SpecCurationServer, args: &Value) -> Result<String, String> {
    // ... existing validation ...
    
    // Check vocabulary allocation
    let term_count = graph.unique_hlexicon_terms().len();
    let allocation = 75; // from governance config
    
    if term_count > allocation {
        warnings.push(format!(
            "Vocabulary allocation exceeded: {} terms active, {} allocated. \
             Consider retiring underused terms or formally expanding allocation.",
            term_count, allocation
        ));
    }
    
    // ... return results ...
}
```

---

## 9. Conclusion

This terminology change is a low-effort, high-clarity refinement that:

1. **Accurately describes the constraint:** Vocabulary terms are allocated, not budgeted
2. **Aligns with the curation reframing:** Allocation implies deliberate distribution, not depletion
3. **Maintains consistency:** Consumable resource "budget" remains unchanged where correct
4. **Requires no code changes:** Documentation-only, no breaking changes
5. **Propagates cleanly:** Russell and Okapi require no changes (no hLexicon vocabulary references)

The change is ready for immediate implementation.

---

## Appendix A: Complete File Change List

| File | Repository | Change Type | Instances |
|------|-----------|-------------|-----------|
| `docs/architecture/hKask-hLexicon.md` | hKask | Modify | 2 |
| `docs/architecture/spec-curation-hypothesis.md` | hKask | Modify | 2 |
| `docs/architecture/PRINCIPLES.md` | hKask | Modify | 1 |
| `registry/registries/hlexicon-governance.yaml` | hKask | Modify (add section) | 0 → 1 |
| `docs/architecture/ecosystem-integration.md` | Russell | Modify (optional) | 0 → 1 |
| `fork-docs/STRATEGY.md` or `MASTER_PLAN.md` | Okapi | Modify (optional) | 0 → 1 |

**Total:** 4 required changes (hKask), 2 optional changes (Russell, Okapi)

---

## Appendix B: Grep Commands for Verification

```bash
# 1. Verify no "budget" in hLexicon context
cd /home/mdz-axolotl/Clones/hKask
grep -r "term.*budget\|vocabulary.*budget\|lexicon.*budget" docs/ registry/ --include="*.md" --include="*.yaml"
# Expected: 0 matches

# 2. Verify "allocation" is used
grep -r "term.*allocation\|vocabulary.*allocation\|lexicon.*allocation" docs/ registry/ --include="*.md" --include="*.yaml"
# Expected: ≥4 matches

# 3. Verify consumable "budget" unchanged
grep -r "energy_budget\|token_budget\|iteration_budget\|gpuBudget\|ReflexBudget" crates/ registry/ --include="*.rs" --include="*.yaml" | wc -l
# Expected: Same count as before changes

# 4. Run validation
./scripts/validate-hlexicon-alignment.sh
# Expected: Pass

# 5. Run balance test
cargo test -p hkask-templates -- hlexicon_balance
# Expected: Pass
```

---

*Document Status: Proposed*  
*Next Steps: Review, approve, execute Phase 1-3*
