---
title: "Classification + Guard Architecture Overview"
diataxis: reference
---

# Classification + Guard Architecture Overview

How dual-model classification, content safety guard, drift detection, and
memory storage compose. P3.1 Social Generativity governs the entire pipeline —
no single model gates shared memory, and no LLM boundary is unguarded.

Related: `crates/hkask-guard/src/lib.rs`, `crates/hkask-services-runtime/src/dual_classify.rs`, `docs/architecture/core/PRINCIPLES.md` §P3.1

```mermaid
flowchart TD
    subgraph "P3.1 Social Generativity"
        direction TB
        G["ContentGuard\n(llm-guard, OWASP Top 10)"]
        MA["Model A\nQwen/KiloCode\nChina"]
        MB["Model B\nGemma 4/DeepInfra\nUS"]
        I["Dual Integrator\nJaccard + Merge"]
        CD["Drift Detection\ncns.classify.drift"]
        FD["Fidelity Check\ncns.classify.dual_fidelity"]
        SM[("Shared Memory\nhMems + provenance")]
    end

    IN([Source Text]) --> G
    G -->|"cns.guard.violation\n(refuse)"| RJ([Refused])
    G -->|pass| MA
    G -->|pass| MB
    MA --> I
    MB --> I
    I --> FD
    FD -->|"agreement < 0.6\ncns.classify.dual_fidelity"| CD
    FD --> SM
    I --> CD
    CD -->|"divergence > 30%\nor asymmetry > 2.0\ncns.classify.drift"| CNS([CNS Alert])

    subgraph "Never Disableable"
        G
    end

    subgraph "Mandatory Peer Models"
        MA
        MB
        I
    end

    subgraph "Observable"
        FD
        CD
        CNS
    end
```

## Subsystems

| Subsystem | Crate | OWASP Alignment |
|---|---|---|
| ContentGuard | `hkask-guard` | LLM01, LLM02, LLM04, LLM06 |
| Dual Classifier | `hkask-services-runtime` | LLM09 (Misinformation — cross-jurisdiction) |
| Drift Detection | `hkask-services-runtime` | Operational monitoring |
| Memory Storage | `hkask-storage` | Provenance-tagged hMems |
