---
title: "Dual-Model Classification Flow"
diataxis: reference
---

# Dual-Model Classification Flow

How classification operates with two peer models from different jurisdictions.
Neither model is primary — both produce extractions that are integrated with
divergence detection.

Related: `crates/hkask-services-runtime/src/dual_classify.rs`, `crates/hkask-services-corpus/src/embed/service.rs`

```mermaid
flowchart TD
    S([Source Text])
    G{Guard Input Scan}
    R[Refuse + CNS Alert]
    MA[Model A\nKiloCode/Qwen\nChina]
    MB[Model B\nDeepInfra/Gemma 4\nUS]
    EA[TripleExtraction A]
    EB[TripleExtraction B]
    I[integrate_dual_triples]
    JA{Jaccard Agreement\n&gt;= 0.6?}
    D[CNS: dual_fidelity\nDivergence Alert]
    GO{Guard Output Scan}
    R2[Strip Secrets\n+ CNS Alert]
    ST[Store in Shared Memory]
    M[Memory]

    S --> G
    G -->|pass| MA
    G -->|pass| MB
    G -->|block| R
    MA --> EA
    MB --> EB
    EA --> I
    EB --> I
    I --> JA
    JA -->|yes| GO
    JA -->|no| D
    D --> GO
    GO -->|pass| ST
    GO -->|violation| R2
    R2 --> ST
    ST --> M

    subgraph "Peer Models (parallel)"
        MA
        MB
    end

    subgraph "Epistemic Integration"
        I
        JA
        D
    end
```
