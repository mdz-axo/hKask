---
title: "Unsloth Qwen3.6-27B Training Pipeline Flowchart"
audience: [operators, developers]
last_updated: 2026-07-10
version: "0.1.0"
status: "Active"
domain: "Training"
mds_categories: [domain, lifecycle]
---

# Training Pipeline Flowchart — Qwen3.6-27B on RunPod

This flowchart shows the end-to-end training pipeline from pod launch through self-management. Each decision node represents a validation gate; failures are handled by preserving the pod for debugging rather than silently exiting.

```mermaid
flowchart TD
    A([Launch: bash runpod_unsloth.sh]) --> B{GPU Available?}
    B -->|No| C([Exit: SUPPLY_CONSTRAINT])
    B -->|Yes| D[Pod Boots: 5 min]
    D --> E[User Pastes: curl ... | bash]
    E --> F[Install Dependencies: pip + apt]
    F --> G{SDPA FlashAttn?}
    G -->|No| H([Warn: VRAM Tight])
    G -->|Yes| I[Validate Datasets]
    I --> J{Datasets OK?}
    J -->|No| K([Exit: Format Error])
    J -->|Yes| L[Download Model: 65GB, 20 min]
    L --> M[Apply LoRA: r=64, alpha=64]
    M --> N[Format Data: Chat Templates]
    N --> O[Measure Token Lengths]
    O --> P{P95 < 50% of max_seq?}
    P -->|Yes| Q[Auto-Reduce max_seq_length]
    P -->|No| R[Keep max_seq_length]
    Q --> S[Train: SFTTrainer, 3 Epochs]
    R --> S
    S --> T{Loss Improving?}
    T -->|No for 5 evals| U[Early Stop]
    T -->|Yes| V[Complete All Epochs]
    U --> W[Save Best Checkpoint]
    V --> W
    W --> X[Save LoRA Adapter]
    X --> Y[Upload to HF: 5 Retries]
    Y --> Z{Upload OK?}
    Z -->|No| AA([Pod Kept Alive: Manual Recovery])
    Z -->|Yes| AB[60s Countdown]
    AB --> AC{Ctrl-C?}
    AC -->|Yes| AD([Pod Kept Alive])
    AC -->|No| AE[Terminate Pod]
    AE --> AF([Done: $0/hr])
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-TRAIN-001
verified_date: 2026-07-10
verified_against: scripts/train_unsloth.sh; scripts/runpod_unsloth.sh
reference_sources: unsloth.ai/docs/models/qwen3.5/fine-tune; docs.runpod.io/sdks/graphql/manage-pods
status: VERIFIED
-->
