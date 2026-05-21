# Open Question 3: Multi-bot Coaching — Resolved

**Decision:** 1:1 coaching only

**Rationale:**

Coaching Kata requires deep attention to the learner's thinking pattern. The 5 questions are designed to reveal subtle cognitive habits:
- Distinguishing fact from interpretation
- Selecting obstacles strategically
- Formulating testable experiments

Multi-bot coaching dilutes this attention and creates coordination overhead that interferes with the coaching dialogue.

**Alternative for Multi-bot Coordination:**

Use the existing `hkask-ensemble` standing session pattern (`registry/manifests/standing-ensemble-session.yaml`). The Curator orchestrates multi-bot coordination through the ensemble, while individual bots receive 1:1 Coaching Kata sessions for capability development.

**Implementation:**

No code changes required. The `kata-pattern.yaml` manifest already assumes single `learner_bot` in the schema:

```yaml
output_schema:
  type: object
  properties:
    kata_type:
      type: string
    learner_bot:
      type: string  # Single bot, not array
```

---

*ℏKask — Toyota Kata System v0.21.2*
*Open Question 3 resolved: 1:1 coaching only*