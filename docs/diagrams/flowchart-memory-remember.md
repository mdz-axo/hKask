---
title: "Memory Remember — Dual-Model Template Cascade"
diataxis: reference
---

# Memory Remember — Dual-Model Template Cascade

FlowDef manifest for agent memory formation. Three-step cascade with dual-model
rendering on every step. The `operation-selector.j2` classifies and routes to
episodic or semantic extraction. Both peer models render the same template in
parallel; outputs are merged via `merge_json_values()`.

Related: `registry/manifests/memory_remember.yaml`, `crates/hkask-templates/src/executor.rs`

```mermaid
flowchart TD
    OP([Agent Operation])
    OS{operation-selector.j2\nClassify + Route}
    EP["remember-episodic.j2\nFirst-Person Extraction"]
    SE["remember-semantic.j2\nThird-Person Extraction"]
    MAE["Model A\nQwen/KiloCode"]
    MBE["Model B\nGemma/DeepInfra"]
    MAS["Model A\nQwen/KiloCode"]
    MBS["Model B\nGemma/DeepInfra"]
    ME["merge_json_values\nUnion + Dedup"]
    MS["merge_json_values\nUnion + Dedup"]
    EM[("Episodic Memory\nPrivate, Agent-Scoped")]
    SM[("Semantic Memory\nShared, Cross-Agent")]

    OP --> OS
    OS -->|episodic| EP
    OS -->|semantic| SE

    EP --> MAE
    EP --> MBE
    MAE --> ME
    MBE --> ME
    ME --> EM

    SE --> MAS
    SE --> MBS
    MAS --> MS
    MBS --> MS
    MS --> SM

    subgraph "Step 1: Classify"
        OS
    end

    subgraph "Step 2: Episodic (dual_model: true)"
        EP
        MAE
        MBE
        ME
    end

    subgraph "Step 3: Semantic (dual_model: true)"
        SE
        MAS
        MBS
        MS
    end
```
