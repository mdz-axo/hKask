# Open Question 6: CNS Variety Counter Baseline — Resolved

**Decision:** Percentage-based thresholds (not absolute deficit)

**Baseline Values:**

| Counter ID | Expected | Unit | Warning (<60%) | Critical (<40%) |
|------------|----------|------|----------------|-----------------|
| `kata.practices.completed` | 5 | per week | <3/week | <2/week |
| `kata.habit.formation` | 1 | per 21 days | <1/35 days | <1/52 days |
| `kata.automaticity.score` | +0.05 | per week gain | <0.03/week | <0.01/week |

**Rationale:**

The hKask architecture specifies "variety deficit >100" as the algedonic alert threshold. However, this absolute number doesn't map well to Kata counters with different scales:
- `practices.completed` counts discrete events (0, 1, 2...)
- `habit.formation` counts milestones (0, 1, 2...)
- `automaticity.score` is a continuous 0.0–1.0 scale

**Percentage-based thresholds** provide consistent interpretation:
- **Warning at 60%** — Activity is notably below expected
- **Critical at 40%** — Activity is severely below expected

**Algedonic Escalation:**
- Warning → Curator (adjust intensity, offer support)
- Critical → hKask-Administrator (system degradation detected)

**Implementation:**

Updated `kata-pattern.yaml`:
```yaml
cns:
  variety_counters:
    baseline:
      kata.practices.completed:
        expected: 5
        unit: per_week
        warning_threshold: 0.6
        critical_threshold: 0.4
```

---

*ℏKask — Toyota Kata System v0.21.2*
*Open Question 6 resolved: Percentage-based variety counter baselines*