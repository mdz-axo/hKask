# Open Question 0: Version Numbering Policy — Resolved

**Decision:** Pre-release versioning at third decimal place

**Policy:**

| Phase | Version Format | Increment Trigger |
|-------|---------------|-------------------|
| **Pre-release** | 0.21.x | Each substantive change (x = 2, 3, 4...) |
| **Release** | 1.0.0 | When Administrator declares MVP complete |
| **Post-release** | 1.x.y | Major/minor/patch semver |

**Current Version:** 0.21.2

**Rationale:**
- First decimal (0.21) = architecture version (fixed until 1.0)
- Second decimal (0.21.**2**) = pre-release iteration counter
- Third decimal reserved for hotfixes if needed (0.21.2.1)

**Version History (Pre-release):**

| Version | Date | Changes |
|---------|------|---------|
| 0.21.0 | 2026-05-21 | Initial Kata system (3 manifests, 13 templates) |
| 0.21.1 | 2026-05-21 | Remediation (unified manifest, 5 templates, GHG Protocol) |
| 0.21.2 | 2026-05-21 | Habit formation + capability metrics + carbon accounting |

**Implementation:**

All files updated to v0.21.2:
- `registry/manifests/kata-pattern.yaml`
- `registry/manifests/cns-carbon-tracking.yaml`
- `docs/architecture/carbon-accounting-methodology.md`
- `docs/architecture/kata-system-summary.md`
- All decision docs

**Next Version:** 0.21.3 (upon resolution of Open Questions 6–8 or substantive changes)

---

*ℏKask — Toyota Kata System v0.21.2*
*Pre-release versioning policy established*