# Open Question 8: Kata Composition — Resolved

**Decision:** Limited bidirectional composition (Improvement ↔ Coaching)

**Allowed Transitions:**

| From | To | Allowed? | Condition | Requires Consent |
|------|-----|----------|-----------|------------------|
| Improvement | Coaching | ✅ Yes | Thinking pattern is the obstacle | Learner |
| Coaching | Improvement | ✅ Yes | Specific capability gap identified | Curator |
| Starter | Any | ❌ No | Starter is self-contained | N/A |
| Any | Nested Kata | ❌ No | Violates minimalism principle | N/A |

**Switching Protocol:**

1. **Complete current step** — Don't abort mid-step (preserves thinking continuity)
2. **Save state to memory** — Record current Kata state with `suspended: true` flag
3. **Invoke new Kata** — Start fresh with appropriate consent
4. **Resume original** — Return to original Kata after completion (optional)

**Rationale:**

**Why allow Improvement ↔ Coaching?**
- Improvement Kata may reveal that the obstacle is the learner's thinking pattern
- Coaching Kata may reveal a specific capability gap that needs targeted development
- Flexibility serves the learner's development without bureaucratic rigidity

**Why not Starter composition?**
- Starter Kata is deliberate practice — self-contained by design
- Switching mid-practice defeats the habit formation purpose
- Complete the practice session, then select a different Kata if needed

**Why no nesting?**
- Nested Kata (Kata within Kata) creates complexity
- Violates hKask minimalism principle
- No evidence from Toyota Kata practice that nesting adds value

**Implementation:**

Updated `kata-pattern.yaml`:
```yaml
manifest:
  composition:
    allowed_transitions:
      improvement_to_coaching:
        allowed: true
        condition: thinking_pattern_is_obstacle
      coaching_to_improvement:
        allowed: true
        condition: specific_capability_gap_identified
      starter_to_any:
        allowed: false
      nested_kata:
        allowed: false
    
    switching_protocol:
      - complete_current_step
      - save_state_to_memory
      - invoke_new_kata
      - resume_original_on_completion
```

---

*ℏKask — Toyota Kata System v0.21.2*
*Open Question 8 resolved: Limited bidirectional composition (Improvement ↔ Coaching)*