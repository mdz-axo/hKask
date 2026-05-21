# Toyota Kata System — Iteration Support v0.21.4

**Version:** v0.21.4 (Iteration Support Added)  
**Date:** 2026-05-21  
**Status:** Complete

---

## Iteration Budget Design

| Budget Type | Allocation | Scope |
|-------------|------------|-------|
| **Primary execution** | 100,000 tokens | Per Kata session (all core steps) |
| **Iteration budget** | 50,000 tokens | Per iteration (max 2) |
| **Total maximum** | 200,000 tokens | Primary + 2 iterations |

---

## Files Created (3)

1. `registry/manifests/kata-iteration.yaml` — Iteration manifest (50k budget, max 2 iterations)
2. `registry/templates/kata/iteration-comparison.j2` — Variance/confidence assessment
3. `registry/templates/kata/iteration-check.j2` — Iteration trigger check

## Files Modified (1)

4. `registry/manifests/kata-pattern.yaml` — Added iteration tracking, 2 new conditional steps

---

## Iteration Triggers

| Condition | Threshold | Action |
|-----------|-----------|--------|
| Variance score | > 0.3 (30%) | Trigger iteration |
| Confidence | < 0.5 (50%) | Trigger iteration |
| Automaticity trend | Declining | Trigger iteration |
| Capability delta | Vague/unspecific | Trigger iteration |
| Max iterations reached | 2 | No more iterations |

---

## Variance/Confidence Classification

| Variance Score | Confidence | Interpretation |
|----------------|------------|----------------|
| 0.0–0.2 | High | Consistent results, high confidence |
| 0.2–0.4 | Medium | Some variance, moderate confidence |
| 0.4–0.6 | Low | Significant variance, low confidence |
| 0.6–1.0 | Very Low | Highly variable, unreliable results |

---

## CNS Integration

**Variety Counters:**
| Counter ID | Baseline | Warning | Critical |
|------------|----------|---------|----------|
| `kata.iterations.used` | 0.5/session | >1.5 | >2.0 |
| `kata.variance.score` | 0.2 | >0.4 | >0.6 |

**Algedonic Alerts:**
- `iteration_budget_exceeded` — Iterations used > max_iterations → Curator
- `high_variance_detected` — Variance score > 0.5 → Curator
- `low_confidence_persistent` — Confidence low after 2 iterations → Curator

---

## Budget Semantics

| Scenario | Tokens Used | Carbon (kg CO₂e) |
|----------|-------------|------------------|
| Primary only | 100,000 | ~0.029 |
| Primary + 1 iteration | 150,000 | ~0.044 |
| Primary + 2 iterations | 200,000 | ~0.058 |

---

## Step Flow (Updated)

```
Core (always):
1. consent-and-select
2. kata-cycle
3. outcome-and-habit
4. memory-record
5. cns-emit

Conditional (when triggered):
6. habit-intervention (if intervention_needed)
7. kata-switch-check (if composition_enabled)
8. kata-switch-execute (if switch_requested)
9. iteration-check (if variance/confidence threshold exceeded) ← NEW
10. kata-iteration-execute (if iteration_needed) ← NEW
```

---

## Testing Checklist

```bash
# Validate syntax
cargo check -p hkask-templates
cargo check -p hkask-cns

# Test iteration flow
# 1. Execute Kata with high variance trigger
# 2. Verify iteration manifest invoked
# 3. Verify variance comparison
# 4. Verify best iteration recorded
# 5. Verify CNS spans emitted
```

---

*ℏKask — Toyota Kata System v0.21.4*
*Iteration support complete. Variance assessment enabled. Confidence building active.*