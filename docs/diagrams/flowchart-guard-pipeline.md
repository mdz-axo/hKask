---
title: "Content Safety Guard Pipeline"
diataxis: reference
---

# Content Safety Guard Pipeline

Mandatory input/output scanning aligned with OWASP LLM Top 10. Core scanners
are always active — not configurable off. Powered by `llm-guard` (pure Rust,
zero-copy, sub-millisecond).

Related: `crates/hkask-guard/src/pipeline.rs`, OWASP LLM Top 10

```mermaid
flowchart TD
    IN([Input Text])
    TL{Token Limit Gate\n32K tokens}
    RO{Role Override\nDetection}
    DO{Deobfuscated\nInjection Check}
    CR[CNS: guard.violation\nInput Refused]
    CL([Model API Call])
    OR([Model Output])
    SL{Secret Leakage\nDetection}
    SK[Strip Secrets\nCNS: guard.violation]
    PS([Parse + Store])

    IN --> TL
    TL -->|pass| RO
    TL -->|exceeded| CR
    RO -->|pass| DO
    RO -->|injection| CR
    DO -->|pass| CL
    DO -->|injection| CR
    CL --> OR
    OR --> SL
    SL -->|pass| PS
    SL -->|leak detected| SK
    SK --> PS

    subgraph "Input Pipeline (First-Hit)"
        TL
        RO
        DO
    end

    subgraph "Output Pipeline (All-Hits)"
        SL
        SK
    end
```
