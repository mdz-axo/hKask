# Training Pipeline Flowchart

This diagram traces the control flow of the hKask adapter training pipeline, from pod launch through training completion and upload. It covers both the reasoning distillation path (`train_unsloth.sh`) and the Rust adapter path (`train_rust_adapter.sh`), which share the same pod launcher (`runpod_unsloth.sh`) but diverge at the dataset formatting stage.

```mermaid
flowchart TD
    A([Start: bash runpod_unsloth.sh]) --> B{Mode?}
    B -->|train| C[Deploy H100 NVL pod]
    B -->|eval| C
    B -->|rust-coding| C
    B -->|rust-analysis| C
    B -->|rust-both| C
    C --> D[Wait for SSH endpoint]
    D --> E{Pod running?}
    E -->|No| F[Retry up to 5 min]
    F --> E
    E -->|Yes| G[Print SSH + curl commands]
    G --> H([User pastes curl command])

    H --> I[Download script from HF]
    I --> J[Install Unsloth + deps]
    J --> K[Install FLA kernels]
    K --> L{MODE?}

    L -->|train| M[Load opus-dsv4 dataset]
    L -->|eval| N[Download adapter from HF]
    L -->|coding| O[Load Strandset-Rust-v1]
    L -->|analysis| P[Load introspector/rust-analyser]
    L -->|both| Q[Load both datasets]

    M --> R[Format to ChatML]
    O --> R
    P --> R
    Q --> R

    R --> S[Create 90/10 train/eval split]
    S --> T[Load Qwen3.6-27B via FastLanguageModel]
    T --> U[Apply LoRA: r=16, alpha=32, dropout=0]
    U --> V[Format with chat template]
    V --> W[SFTTrainer with early stopping]

    W --> X[Train: LR=1e-4, warmup=50, eval=50]
    X --> Y{Early stop?}
    Y -->|Yes| Z[Load best checkpoint]
    Y -->|No, epochs done| Z
    Z --> AA[Save adapter + tokenizer]
    AA --> BB{HF_TOKEN?}
    BB -->|Yes| CC[Upload to HF model repo]
    BB -->|No| DD[Save locally only]
    CC --> EE{Exit code 0?}
    DD --> EE
    EE -->|Yes| FF[Auto-terminate pod in 60s]
    EE -->|No| GG[Pod stays alive for debugging]
    FF --> HH([End])
    GG --> HH

    N --> II[Load model via Unsloth]
    II --> JJ[Apply adapter via PeftModel]
    JJ --> KK[Verify LoRA B weights non-zero]
    KK --> LL[Run GPQA + MATH-500 + MMLU-Pro]
    LL --> MM[Save baseline vs adapter delta]
    MM --> HH
```

## Key Decision Points

| Decision | Condition | Branches |
|----------|-----------|----------|
| Mode selection | CLI flag (`--rust-coding`, `--eval`, etc.) | 5 paths: train, eval, coding, analysis, both |
| Pod readiness | RunPod `desiredStatus == RUNNING` | Retry loop, max 5 min |
| Dataset selection | `MODE` env var | 3 formatting paths + 1 eval path |
| Early stopping | `eval_loss` no improvement for 10 evals | Load best checkpoint vs continue |
| Upload | `HF_TOKEN` present | Upload vs local-only |
| Pod lifecycle | Exit code 0 | Auto-terminate vs keep alive |
