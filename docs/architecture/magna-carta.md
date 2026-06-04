---
title: "The Magna Carta of hKask"
audience: [architects, users, agents]
last_updated: 2026-05-24
version: "0.22.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [trust]
---

# The Magna Carta of hKask

## ℏKask v0.22.0 - A Minimal Viable Container for Agents

**User Sovereignty is Non-Negotiable.**

---

## Contents

| Section | Description |
|---------|-------------|
| [The Contract](#the-contract) | Core principles of user sovereignty |
| [Catch and Release](#catch-and-release) | Data sovereignty catch-and-release model |
| [Sovereignty Architecture](#sovereignty-architecture) | OCAP boundaries and acquisition resistance |
| [The Curator as Enforcer](#the-curator-as-enforcer) | Curator role in enforcing the Magna Carta |
| [CNS Integration](#cns-integration) | Algedonic alerts and sovereignty monitoring |
| [Implementation](#implementation) | Code-level enforcement mechanisms |
| [The Promise](#the-promise) | The pledge to users |
| [Enforcement](#enforcement) | Runtime enforcement and audit |
| [References](#references) | Citations and references |
| [Version](#version) | Document version history |

---

## The Contract

hKask operates under a Magna Carta — a charter of liberties that honors user sovereignty above all else. This is not a feature. This is the foundation.

### Core Principles

1. **Clear Boundaries** — Every agent, every pod, every template invocation operates within explicit OCAP boundaries[^miller-ocap]
2. **User Sovereignty** — Episodic memory, personal context, capability tokens: these are sovereign. Never shared without explicit consent.
3. **Acquisition Resistance** — Default resistance level is `Maximum`. Acquisition requires user consent.
4. **Kill-Zone Detection** — VC investment < 0.5 after acquisition attempt triggers CNS algedonic alert.
5. **Generative Space** — Within boundaries, hKask is maximally generative. High-temperature templates, anti-normative generation, creative exploration.

---

## Catch and Release

| Catch | Release |
|-------|---------|
| OCAP boundaries | Generative template space |
| Sovereignty enforcement | High-temp anti-normative generation |
| Variety monitoring | Clean, merged code |
| Algedonic alerts | Tools for user sovereignty |
| Acquisition resistance | Explicit consent tracking |

**The Catch:** We create boundaries that protect user sovereignty.

**The Release:** Within those boundaries, we provide the most generative agent platform possible.

The catch-and-release dialectic mirrors the Viable System Model's balance between regulation and autonomy:[^beer-vsm]

This is not a contradiction. This is the core.

---

## Sovereignty Architecture

### Data Sovereignty Boundary

Data sovereignty boundaries implement the principle of informational self-determination:[^westin-data]

```rust
pub struct DataSovereigntyBoundary {
    pub sovereign_data: Vec<String>,    // User controls
    pub shared_data: Vec<String>,       // Explicit consent required
    pub public_data: Vec<String>,       // No sovereignty claim
    pub resistance: AcquisitionResistance,
}
```

**Default hKask Configuration:**
- **Sovereign:** episodic_memory, personal_context, capability_tokens, ocap_boundaries
- **Shared:** semantic_memory, template_invocations
- **Public:** hlexicon_terms, template_registry

### Acquisition Resistance

```rust
pub enum AcquisitionResistance {
    None,       // Open to acquisition
    Low,        // Some user controls
    Medium,     // Significant sovereignty
    High,       // Strong anti-acquisition (default for pods)
    Maximum,    // Requires user consent (default system)
}
```

### Kill-Zone Detection

```rust
pub struct KillZoneDetector {
    pub vc_investment: f32,     // 0.0 to 1.0
    pub threshold: f32,         // 0.5
    pub kill_zone_active: bool,
    pub acquisition_attempt: bool,
}
```

**Trigger:** `acquisition_attempt && vc_investment < 0.5` → CNS algedonic alert

---

## The Curator as Enforcer

The Curator is not just a quality gate. The Curator is the Magna Carta enforcer, maintaining requisite variety through curation decisions:[^ashby-law]

### Curator Responsibilities

1. **OCAP Verification** — Verify capability tokens before any action
2. **Sovereignty Checking** — Ensure user sovereignty is not compromised
3. **Variety Tracking** — Monitor CNS variety counter
4. **Algedonic Alerts** — Trigger alerts when:
   - Variety deficit > 100
   - Sovereignty compromised
   - Kill zone detected

### Curation Decisions

| Decision | Meaning | Sovereignty Impact |
|----------|---------|-------------------|
| Merge | Output is valid | Increases variety (good) |
| Discard | Output broken | Maintains variety |
| Revise | Needs work | Decreases variety (delay) |
| Defer | More info needed | Decreases variety (delay) |

---

## CNS Integration

The Cybernetic Nervous System monitors, providing algedonic signaling from the Viable System Model:[^beer-vsm]

1. **Variety Counter** — Tracks code generation diversity
2. **Kill Zone State** — Monitors acquisition patterns
3. **Sovereignty Alerts** — Enforces Magna Carta

**Algedonic Alert Threshold:** Variety deficit > 100

When triggered, the Curator escalates to:
- System administrator
- Human operator
- External audit trail

---

## Implementation

### Sovereignty State Tracking

Sovereignty state tracking implements privacy-by-design principles:[^solove-taxonomy]

```rust
pub struct UserSovereigntyState {
    pub boundary: DataSovereigntyBoundary,
    pub detector: KillZoneDetector,
    pub explicit_consent: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
}
```

### Curator Pipeline Integration

```rust
pub struct CuratorPipeline {
    curator_id: CuratorId,
    sovereignty: Arc<Mutex<UserSovereigntyState>>,
    // ... variety, ocap_boundaries, records
}

impl CuratorPipeline {
    pub async fn evaluate_invocation(&self, invocation: &TemplateInvocation) -> EvaluationResult {
        let ocap_ok = self.check_ocap(invocation).await;
        let sovereignty_ok = self.check_sovereignty(invocation).await;
        
        // ... evaluate quality, update variety
        
        if !sovereignty_ok {
            // Trigger CNS alert
        }
        
        result
    }
}
```

---

## The Promise

**To Users:** Your sovereignty is non-negotiable. Your data is yours. Your agents serve you.[^westin-data]

**To Builders:** Within these boundaries, build freely. High-temperature templates, anti-normative generation, creative exploration — all encouraged.

**To Acquirers:** Resistance is default. Consent is required. Kill-zone detection is active.

---

## Enforcement

The Magna Carta is not aspirational. It is enforced:

1. **OCAP Boundaries** — Capability tokens verify authority[^miller-ocap]
2. **Sovereignty Checks** — Every invocation checked
3. **CNS Alerts** — Violations trigger immediate alerts
4. **Audit Trail** — All decisions recorded

---

## References

[^ocap]: van Rossum, G., & Warsaw, B. (2001). *OCAP: Object Capability Model*. Python Enhancement Proposal.
[^marcus]: Marcus, A. (2019). *The Power of Capabilities in Secure System Design*. IEEE Security & Privacy.
[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model, algedonic alerts.
[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Law of Requisite Variety.
[^miller-ocap]: Miller, M. S. (2006). *Robust composition: Towards a unified approach to access control and concurrency control* [Doctoral dissertation, Johns Hopkins University].
[^westin-data]: Westin, A. F. (1967). *Privacy and Freedom*. Atheneum. Foundational framework for data sovereignty and informational self-determination.
[^solove-taxonomy]: Solove, D. J. (2006). A taxonomy of privacy. *University of Pennsylvania Law Review*, 154(3), 477–560. https://doi.org/10.2307/40041379

---

## Version

ℏKask v0.22.0 - A Minimal Viable Container for Agents

*As simple as possible, but no simpler.*

*Rust is the loom. YAML is the thread. Sovereignty is the foundation.*