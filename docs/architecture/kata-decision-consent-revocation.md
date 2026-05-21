# Open Question 7: Consent Revocation — Resolved

**Decision:** Revocable at any time with graceful abort

**Consent Model:**

| Kata Type | Grant Required From | Who Can Revoke |
|-----------|--------------------|----------------|
| Improvement | Curator | Curator |
| Coaching | Learner OR Curator | Learner OR Curator |
| Starter | Self | Self |

**Mid-Cycle Revocation Effect:**

1. **Immediate abort** — Stop further token consumption
2. **Save partial outcome** — Record to memory with `incomplete: true` flag
3. **Emit CNS span** — Include `consent_revoked: true` flag
4. **Notify parties** — Inform Curator and/or learner

**Re-consent Requirements:**
- No automatic re-consent
- Explicit grant required to restart Kata
- Previous partial outcome available for reference

**OCAP Alignment:**
- Consent is a capability grant
- Grantor retains right to revoke
- Revocation is immediate and effective
- Partial work is preserved (not discarded)

**Implementation:**

Updated `kata-pattern.yaml`:
```yaml
security:
  consent:
    model: revocable_at_any_time
    revocation:
      mid_cycle_effect: abort_and_save_partial
      save_incomplete_outcome: true
      emit_cns_span: true
      notify_parties: true
    re_consent:
      automatic: false
      requires: explicit_grant
```

**Audit Trail:**
- `consent_granted` — Logged with timestamp, grantor
- `consent_revoked` — Logged with timestamp, revoker, reason (optional)

---

*ℏKask — Toyota Kata System v0.21.2*
*Open Question 7 resolved: Revocable consent with graceful abort*