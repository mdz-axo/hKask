# Open Question 5: Capability Metrics — Resolved

**Decision:** 3 primary + 2 secondary metrics schema

**Primary Metrics (tracked for all bots):**

| Metric ID | Unit | Source |
|-----------|------|--------|
| `task.success_rate` | percent | Bot execution logs |
| `response.latency` | ms_per_token | CNS tool spans |
| `thinking.automaticity` | score_0_to_1 | Kata classifier |

**Secondary Metrics (tracked on demand):**

| Metric ID | Unit | Source |
|-----------|------|--------|
| `coordination.handoff_success` | percent | Ensemble session logs |
| `energy.efficiency` | wh_per_task | CNS carbon tracking |

**Improvement Kata Target Schema:**

```yaml
target_condition:
  metric_id: string  # e.g., "thinking.automaticity"
  baseline: number   # e.g., 0.3
  target: number     # e.g., 0.75
  achieve_by: YYYY-MM-DD
```

**Implementation:**

Files modified:
- `registry/manifests/cns-carbon-tracking.yaml` — Added `capability_metrics` section
- `registry/manifests/kata-pattern.yaml` — Added `cns.capability_metrics` section
- `registry/templates/kata/improvement-cycle.j2` — References specific metrics
- `registry/templates/kata/improvement-metrics-selector.j2` — New template for metric selection

---

*ℏKask — Toyota Kata System v0.3.2*
*Open Question 5 resolved: 3 primary + 2 secondary capability metrics*