# Open Question 4: Kata Retreat Criteria — Resolved

**Decision:** Three-tier intensity model with graduation at 0.75 automaticity

**Intensity Tiers:**

| Tier | Automaticity | Kata Frequency | Curator Attention |
|------|--------------|----------------|-------------------|
| **Intensive** | 0.0–0.5 | Daily | Full coaching cycle |
| **Maintenance** | 0.5–0.75 | Weekly | Check-in only |
| **Graduated** | 0.75–1.0 | Monthly | Spot-check |

**Graduation Criteria:**
1. Automaticity ≥0.75 for 21 consecutive days
2. No algedonic alerts in 30 days
3. Switch to monthly Kata frequency

**Re-entry Triggers:**
- Automaticity drops below 0.5
- CNS variety counter shows stagnation (14 days no improvement)
- Bot requests coaching
- Curator detects thinking pattern degradation

**Implementation:**

Updated `kata-pattern.yaml`:
```yaml
habit_formation:
  automaticity:
    baseline: 0.3
    target: 0.75
    graduation_threshold: 0.75
  
  intensity_tiers:
    intensive:
      automaticity_range: [0.0, 0.5]
      frequency: daily
    maintenance:
      automaticity_range: [0.5, 0.75]
      frequency: weekly
    graduated:
      automaticity_range: [0.75, 1.0]
      frequency: monthly
  
  graduation_criteria:
    - automaticity_ge: 0.75
    - consecutive_days: 21
    - no_alerts_days: 30
  
  re_entry_triggers:
    - automaticity_below: 0.5
    - variety_stagnation_days: 14
    - bot_requests_coaching: true
    - curator_detects_degradation: true
```

**Files Modified:**
- `registry/manifests/kata-pattern.yaml` — Added intensity tiers, graduation criteria
- `registry/templates/kata/habit-automaticity-classifier.j2` — Updated score ranges

---

*ℏKask — Toyota Kata System v0.21.2*
*Open Question 4 resolved: Graduation at 0.75, monthly frequency*