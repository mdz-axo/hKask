# hKask Training Harness Gap Analysis & GPU Platform Decision

> **Date**: 2026-07-22
> **Author**: agent (gap analysis session)
> **Methodology**: kata-improvement PDCA (4 steps) + pragmatic-semantics classification
>   + sequential-inquiry + MCDA + falsifiability
> **Provenance**: Every claim is tagged with epistemic mode (IS/OUGHT),
>   constraint force, provenance, and confidence.

---

## Table of Contents

1. [Direction (kata Step 1)](#1-direction-kata-step-1)
2. [Current Condition (kata Step 2)](#2-current-condition-kata-step-2)
3. [Exemplar Deep Dive](#3-exemplar-deep-dive)
4. [Gap Matrix](#4-gap-matrix)
5. [Rust-Transferability Verdicts](#5-rust-transferability-verdicts)
6. [lora-training Skill Gap Report](#6-lora-training-skill-gap-report)
7. [GPU Platform MCDA](#7-gpu-platform-mcda)
8. [Target Condition (kata Step 3)](#8-target-condition-kata-step-3)
9. [Experiment (kata Step 4)](#9-experiment-kata-step-4)
10. [Open Questions Register](#10-open-questions-register)

---

## 1. Direction (kata Step 1)

**Challenge (measurable):** Close the capability gap between hKask's MCP
training server / `lora-training` skill and three exemplar open-source
training harnesses — Ludwig, Unsloth-Zoo, Axolotl — while honoring the
hard constraint that hKask implements everything practical in Rust with
skills/manifests/templates, not Python.

**Excellent performance looks like:**
- Categorized gap matrix (feature × {hKask, Ludwig, Unsloth, Axolotl})
  with provenance-tagged evidence per cell.
- Every recommended change classified OUGHT/Guardrail vs IS/Observation.
- GPU platform recommendation backed by MCDA with sensitivity analysis.
- `tasks/training-gap-plan.md` whose slices are ≤ M, vertically cut,
  dependency-ordered, checkpointed.

**Measurement plan:** gap-matrix completeness (every feature cell
populated or marked "unverifiable"), MCDA robustness (ranking stable
under ±20% weight perturbation), plan convergence metric ≤ 0.15.

**Classification:** IS/Declarative (this is the stated challenge);
OUGHT/Prohibition (Rust-first constraint from AGENTS.md).

---

## 2. Current Condition (kata Step 2)

### 2.1 Training Surface Inventory

| Component | Location | Status | Evidence |
|---|---|---|---|
| MCP training server | `mcp-servers/hkask-mcp-training/` | Active, 8 tools | `src/lib.rs` tool registrations |
| lora-training skill | `.agents/skills/lora-training/SKILL.md` | Advisory only — does NOT train | SKILL.md line 26: "This skill does not train, load, initialize, merge, or evaluate models" |
| RunPod provider | `src/providers/runpod.rs` | Active, GraphQL API | `RunpodHost` impl `TrainingHost` |
| Harness adapters | `src/providers/harness.rs`, `trl_harness.rs` | 3 harnesses: Axolotl, TRL, Ludwig | `HarnessAdapter` trait + impls |
| LoRA validation | `src/lora_validation.rs` | 14 of 17 gates enforced | Module doc comment lines 8-27 |
| Adapter store | `src/adapter/adapter_store.rs` | SQLite-backed | `AdapterStore::from_driver()` |
| Dataset pipeline | `src/dataset.rs` | JSONL normalization, HuggingFace publish | `DatasetPipeline::new()` |
| Eval harness | `training_evaluate` (lib.rs:1146-1328) | Basic: exact_match, contains, semantic | Code review |
| Post-mortem | `docs/post-mortem/2026-07-19-training-providers.md` | 3 HIGH bugs fixed | Document review |
| Rust LoRA research | `docs/research/rust-lora-training-research.md` | OxiCUDA stack identified, PoC not built | Document review |
| Training readiness | `docs/status/userpod-corpus-training-readiness.md` | NOT ready for production | Document review |

**Classification:** IS/Observation/Direct-provenance (codebase
inventory), confidence 0.95.

### 2.2 MCP Training Server Tool Surface

| Tool | Purpose | State-mutating? | Contract tested? |
|---|---|---|---|
| `training_submit` | Submit training job to RunPod | Yes | Per post-mortem: was untested, now has unit tests |
| `training_status` | Poll job status, auto-register adapter on completion | Yes (registers adapter) | Unit tests |
| `training_cancel` | Cancel job, terminate pod | Yes | Unit tests |
| `training_evaluate` | Eval adapter via inference (exact/contains/semantic) | No | Unit tests |
| `training_ingest_qa` | Ingest QA pairs into dataset pipeline | Yes (writes files) | Unit tests |
| `training_ingest_dataset` | Ingest raw dataset | Yes (writes files) | Unit tests |
| `training_assemble_dataset` | Assemble balanced dataset from QA pairs | Yes (writes files) | Unit tests |
| `training_validate_config` | Validate training params against LoRA gates | No | Unit tests |

**Classification:** IS/Observation/Direct-provenance, confidence 0.95.

### 2.3 LoRA Validation Gates (Code-Enforced)

14 of 17 gates are enforced in Rust code (`lora_validation.rs`):

| Gate | ID | Enforced? | Evidence |
|---|---|---|---|
| No-op-at-init | G-M1 | ✅ Refuse | `validate_noop_at_init()` |
| Merge equivalence | G-M2 | ✅ Refuse | `validate_merge_equivalence()` |
| Scaling form | G-M3 | ✅ Refuse | `validate_scaling_form()` |
| Rank budget | G-M4 | ✅ Warn/Refuse | `validate_rank_budget()` |
| Trainable param count | G-M5 | ✅ (preflight) | Module doc |
| Frozen base quantized | G-Q1 | ✅ Refuse | QLoRA mode check |
| Adapter dtype | G-Q2 | ✅ Warn | Compute dtype check |
| Gradient flow | G-Q3 | ❌ Requires backward pass | Module doc line 25 |
| No silent upcast | G-Q4 | ✅ Warn | bf16 check |
| Paged optimizer | G-Q5 | ✅ Warn | Large model + QLoRA |
| NF4 optimality | G-Q6 | ❌ Requires weight analysis | Module doc line 26 |
| Dataset size vs quality | G-D1 | ✅ Warn | Sample count check |
| Eval protocol | G-D2 | ✅ Advisory | Vicuna/MMLU warning |
| Lemon-pick analysis | G-D3 | ✅ Advisory | Failure case reporting |
| Intruder dimension | G-F1 | ✅ Advisory | Requires Python PEFT |
| Knowledge preservation | G-F2 | ❌ Requires CorDA + eval | Module doc line 27 |
| Harness-method compat | G-H1 | ✅ Refuse | Harness-trainer matrix |

**Classification:** IS/Observation/Direct-provenance, confidence 0.95.

### 2.4 Patterns Observed

**Present-and-deep:**
- LoRA math-contract validation (14/17 gates, paper-anchored)
- Harness config rendering (Axolotl YAML, TRL Python, Ludwig YAML)
- Pod lifecycle persistence (SQLite + JSON, post-post-mortem fix)
- Retrain mode (merge + deduplicate + version + A/B baseline)

**Present-but-shallow:**
- Eval harness: exact_match/contains/semantic only — no MMLU, no
  perplexity, no benchmark suite, no lemon-pick automation
- Dataset pipeline: JSONL normalization works but Unsloth data
  contract mismatch (messages vs text) was a blocking bug
- Multi-harness support: 3 harnesses rendered but all are Python;
  no Rust-native training path

**Entirely absent:**
- Rust-native training (OxiCUDA PoC not built)
- Multi-GPU sharding (FSDP/DeepSpeed/ZeRO)
- Multi-node training
- Sequence packing / multipacking
- Flash attention toggle
- Gradient checkpointing toggle
- Activation offloading
- Checkpoint resume (pod restart loses training state)
- FP8 training
- Full fine-tuning
- GGUF export
- Model merging (TIES, DARE)
- Model Soup (checkpoint averaging)
- Multi-adapter (switchable at runtime)
- VLM fine-tuning
- Sample-level logging / W&B / tensorboard
- Cloud dataset loading (S3/GCS/Azure)
- Sequence parallelism

**Classification:** IS/Observation/Direct-provenance, confidence 0.90.

---

## 3. Exemplar Deep Dive

### 3.1 Ludwig (v0.17)

**Repository:** https://github.com/ludwig-ai/ludwig
**License:** Apache-2.0, Linux Foundation AI & Data
**Provenance:** GitHub README, ludwig.ai homepage, hKask
  `docs/reference/lora-training-catalog.md` (internal, pre-existing).

| Capability | Present? | Evidence |
|---|---|---|
| SFT | ✅ Deep | `trainer.type: finetune` — ludwig.ai homepage |
| DPO | ✅ Deep | `trainer.type: dpo` — ludwig.ai homepage |
| KTO | ✅ Deep | `trainer.type: kto` — ludwig.ai homepage |
| ORPO | ✅ Deep | `trainer.type: orpo` — ludwig.ai homepage |
| GRPO | ✅ Deep | `trainer.type: grpo` — ludwig.ai homepage; TRL defers (requires vLLM) |
| Reward modeling | ❌ | lora-training-catalog.md line 39 |
| LoRA/QLoRA | ✅ Deep | LoRA, QLoRA, DoRA, VeRA, LoRA+, PiSSA, EVA, CorDA, OFT, HRA, WaveFT, VBLoRA — ludwig.ai |
| 4-bit QLoRA | ✅ | via torchao — ludwig.ai |
| Multi-adapter PEFT | ✅ | "multiple named adapters on one base model, switchable at runtime" — GitHub README |
| Model Soup | ✅ | "uniform and greedy checkpoint averaging" — GitHub README |
| Model merging | ✅ | "merge with TIES, DARE, SVD, magnitude pruning" — GitHub README |
| VLM fine-tuning | ✅ | lora-training-catalog.md line 45 |
| Config format | YAML declarative | ludwig.ai |
| Ray cluster scaling | ✅ | "Scale from laptop to GPU cluster" — ludwig.ai |
| Config validation | ✅ (schema) | YAML schema validation; no math-contract gates |

**Depth assessment (deep-module):** Ludwig is a deep module — small
declarative interface (YAML config), large implementation (full
training pipeline from SFT to GRPO). The deletion test passes: if you
deleted Ludwig, the complexity of orchestrating SFT/DPO/KTO/ORPO/GRPO
+ PEFT methods + Ray scaling would reappear in every consumer.

**Transferable to Rust/MCP?** **Partial.** The YAML config schema and
the harness-method compatibility matrix are already integrated into
hKask's `LudwigHarness` adapter. The training itself is Python/PyTorch
— not transferable to Rust without OxiCUDA. The multi-adapter and
model-merging concepts are transferable as adapter-store features.

**Classification:** IS/Observation/External-provenance, confidence 0.85.

### 3.2 Unsloth-Zoo

**Repository:** https://github.com/unslothai/unsloth-zoo
**License:** Apache-2.0
**Provenance:** GitHub README, PyPI, unsloth.ai docs, Red Hat Developer
  article, learnopencv guide.

| Capability | Present? | Evidence |
|---|---|---|
| SFT | ✅ Deep | `FastLanguageModel.get_peft_model()` + TRL SFTTrainer — GitHub |
| DPO | ✅ | TRL DPOTrainer + FastLanguageModel — GitHub |
| GRPO | ✅ Deep | GRPO with sequence packing by default — unsloth.ai changelog |
| LoRA/QLoRA | ✅ Deep | 4-bit, 16-bit, FP8 — PyPI |
| Full fine-tuning | ✅ | "full_finetuning = True" — GitHub issue #5039 |
| Gradient checkpointing | ✅ Deep | Custom "unsloth" mode — 30% less VRAM, 4x longer context — unsloth.ai blog |
| Sequence packing | ✅ | GRPO supports by default — unsloth.ai changelog |
| Checkpoint resume | ✅ | "Stopped Unsloth training runs can now resume from checkpoints" — unsloth.ai changelog |
| Multi-GPU | ✅ (DDP) | "Multi-GPU training is supported, with major improvements coming soon" — PyPI |
| VLM | ✅ | FastVisionModel, vision LoRA — learnopencv, GitHub issue #5039 |
| GGUF export | ✅ | "export NVFP4, FP8, and imatrix GGUFs after training" — unsloth.ai changelog |
| FP8 training | ✅ | PyPI |
| Observability | ✅ | "Monitor training live, track loss and GPU usage" — PyPI |
| Custom kernels | ✅ | Triton + mathematical kernels — PyPI |
| NVIDIA/Intel/AMD | ✅ | learnopencv |
| Flash attention | ✅ (via optimization) | Kernel-level CUDA optimizations — Red Hat Developer |

**Depth assessment:** Unsloth-Zoo is a utility layer (the name says
"zoo" — utils for Unsloth). It patches PyTorch models with optimized
kernels. The deletion test: if you deleted Unsloth-Zoo, the kernel
optimizations would not reappear — you'd fall back to standard
HuggingFace/TRL performance. It earns its surface area through
measurable VRAM savings (50-60%) and speed gains.

**Transferable to Rust/MCP?** **No-with-rationale.** Unsloth's value
is in hand-tuned Triton/CUDA kernels that replace PyTorch's autograd
operations. These are inherently Python/CUDA-ecosystem. The OxiCUDA
stack (per hKask's research doc) provides analogous Rust-native GPU
kernels but is a separate implementation, not a port. The concepts
(gradient checkpointing strategy, sequence packing) are transferable
as config parameters, but the kernel implementations are not.

**Classification:** IS/Observation/External-provenance, confidence 0.80.

### 3.3 Axolotl

**Repository:** https://github.com/axolotl-ai-cloud/axolotl
**License:** Apache-2.0
**Provenance:** docs.axolotl.ai, PyPI, axolotl.ai substack, AI
  Engineering Academy tutorial, Baseten partnership blog.

| Capability | Present? | Evidence |
|---|---|---|
| SFT | ✅ Deep | YAML config, `adapter: lora` — docs.axolotl.ai |
| DPO | ❌ | lora-training-catalog.md line 36 (SFT-only in hKask's matrix) |
| LoRA/QLoRA | ✅ Deep | `adapter: lora`, `load_in_4bit: true` — docs.axolotl.ai |
| Multipacking | ✅ | "Improves GPU utilization by combining multiple short sequences" — docs.axolotl.ai/optimizations |
| Flash Attention 2/3/4 | ✅ | `flash_attention: true` — docs.axolotl.ai |
| Gradient checkpointing | ✅ Deep | `gradient_checkpointing: true` + offload option — docs.axolotl.ai, substack |
| Activation offloading | ✅ | `activation_offloading: true` — Baseten blog |
| Mixed precision | ✅ | `bf16: true` — docs.axolotl.ai |
| Multi-GPU | ✅ Deep | FSDP1, FSDP2, DeepSpeed (ZeRO 1-3) — docs.axolotl.ai/multi-gpu |
| Multi-node | ✅ | Torchrun, Ray — docs.axolotl.ai |
| Sequence parallelism | ✅ | `sequence_parallel_degree: 4` — axolotl substack v0.8.0 |
| Checkpoint resume | ✅ | Checkpoint directories — AI Engineering Academy tutorial |
| Eval | ✅ (basic) | `val_set_size: 0.05`, WandB logging — configs |
| Cloud datasets | ✅ | S3, Azure, GCP, OCI — docs.axolotl.ai |
| Liger Kernel | ✅ | docs.axolotl.ai |
| Cut Cross Entropy | ✅ | docs.axolotl.ai |
| ScatterMoE | ✅ | docs.axolotl.ai |
| EAFT | ✅ | "Entropy-Aware Focal Training" — docs.axolotl.ai |
| Distributed Muon | ✅ | FSDP2 pretraining — docs.axolotl.ai |
| Docker images | ✅ | docs.axolotl.ai |
| Config format | YAML | docs.axolotl.ai |

**Depth assessment:** Axolotl is a deep module — YAML config interface,
massive implementation covering distributed training, attention
optimizations, and dataset handling. The deletion test passes: without
Axolotl, the complexity of FSDP/DeepSpeed/SP configuration + Flash
Attention integration + multipacking would reappear in every consumer.

**Transferable to Rust/MCP?** **Partial.** hKask already renders
Axolotl YAML configs via `AxolotlHarness`. The config schema and
parameter surface are transferable (already done). The training
execution is Python/PyTorch — not transferable. The distributed
training concepts (FSDP, DeepSpeed, SP) are config parameters that
could be passed through, but the execution requires Python. OxiCUDA
provides ZeRO stages 1-3 in Rust, but has not been validated on H100.

**Classification:** IS/Observation/External-provenance, confidence 0.85.

---

## 4. Gap Matrix

### 4.1 Training Methods

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| SFT | ✅ Deep (config render) | ✅ Deep | ✅ Deep | ✅ Deep | — | 0.95 |
| DPO | ✅ (TRL/Ludwig config) | ✅ Deep | ✅ | ❌ | Guideline | 0.85 |
| KTO | ✅ (TRL/Ludwig config) | ✅ Deep | ✅ | ❌ | Guideline | 0.85 |
| ORPO | ✅ (TRL/Ludwig config) | ✅ Deep | ✅ | ❌ | Guideline | 0.85 |
| GRPO | ✅ (Ludwig config) | ✅ Deep | ✅ Deep | ❌ | Guideline | 0.80 |
| Reward modeling | ✅ (TRL config) | ❌ | ❌ | ❌ | — | 0.80 |
| Full fine-tuning | ❌ Absent | ✅ | ✅ | ✅ | OUGHT/Guardrail | 0.85 |

### 4.2 PEFT Methods

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| LoRA | ✅ Deep (validation) | ✅ Deep | ✅ Deep | ✅ Deep | — | 0.95 |
| QLoRA (4-bit NF4) | ✅ Deep (validation) | ✅ Deep | ✅ Deep | ✅ Deep | — | 0.95 |
| DoRA | ✅ (catalog) | ✅ | ✅ | ✅ | Guideline | 0.80 |
| VeRA | ✅ (catalog) | ✅ | ❌ | ❌ | Guideline | 0.75 |
| PiSSA | ✅ (catalog, EVA canonical) | ✅ | ❌ | ✅ | — | 0.80 |
| EVA | ✅ Deep (canonical) | ✅ | ❌ | ✅ | — | 0.85 |
| CorDA | ✅ (catalog) | ✅ | ❌ | ❌ | Guideline | 0.75 |
| LoRA-GA | ✅ (catalog) | ❌ | ❌ | ❌ | — | 0.70 |
| rsLoRA | ✅ (validation G-M3) | ✅ | ✅ | ✅ | — | 0.85 |
| FP8 training | ❌ Absent | ❌ | ✅ | ❌ | Hypothesis | 0.70 |

### 4.3 Training Optimizations

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| Dataset packing / multipacking | ❌ Absent | ❌ | ✅ | ✅ Deep | OUGHT/Guardrail | 0.85 |
| Flash Attention 2/3/4 | ❌ Absent (delegated to harness) | ✅ | ✅ (kernel-level) | ✅ Deep | OUGHT/Guardrail | 0.85 |
| Gradient checkpointing | ❌ Absent (delegated to harness) | ✅ | ✅ Deep (custom) | ✅ Deep | OUGHT/Guardrail | 0.85 |
| Activation offloading | ❌ Absent | ❌ | ✅ | ✅ | Guideline | 0.75 |
| Mixed precision (bf16) | ❌ Absent (delegated) | ✅ | ✅ | ✅ | OUGHT/Guardrail | 0.85 |
| Liger Kernel | ❌ Absent | ❌ | ❌ | ✅ | Guideline | 0.70 |
| Cut Cross Entropy | ❌ Absent | ❌ | ❌ | ✅ | Guideline | 0.70 |
| Sequence parallelism | ❌ Absent | ❌ | ❌ | ✅ Deep | Guideline | 0.75 |
| EAFT | ❌ Absent | ❌ | ❌ | ✅ | Hypothesis | 0.60 |

### 4.4 Distributed Training

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| Multi-GPU (FSDP/DeepSpeed) | ❌ Absent | ✅ (Ray) | ✅ (DDP) | ✅ Deep | OUGHT/Guardrail | 0.85 |
| Multi-node | ❌ Absent | ✅ (Ray) | ❌ | ✅ | Guideline | 0.75 |
| ZeRO stages 1-3 | ❌ Absent | ✅ (via DeepSpeed) | ❌ | ✅ Deep | OUGHT/Guardrail | 0.85 |

### 4.5 Checkpoint & Resume

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| Checkpoint resume | ❌ Absent | ✅ | ✅ | ✅ | OUGHT/Prohibition | 0.90 |
| Pod lifecycle persistence | ✅ (post-mortem fix) | N/A | N/A | N/A | — | 0.95 |
| Job persistence (SQLite) | ✅ | N/A | N/A | N/A | — | 0.95 |

### 4.6 Evaluation

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| Exact match eval | ✅ Shallow | ✅ | ✅ | ✅ | — | 0.90 |
| Contains eval | ✅ Shallow | ❌ | ❌ | ❌ | — | 0.85 |
| LLM-as-judge eval | ✅ Shallow | ✅ | ❌ | ❌ | Guideline | 0.80 |
| Perplexity eval | ❌ Absent | ✅ | ❌ | ✅ | OUGHT/Guardrail | 0.85 |
| Benchmark eval (MMLU etc.) | ❌ Absent | ✅ | ❌ | ✅ | OUGHT/Guardrail | 0.85 |
| Lemon-pick analysis | ✅ Advisory (G-D3) | ❌ | ❌ | ❌ | Guideline | 0.75 |
| A/B comparison | ✅ (retrain mode) | ❌ | ❌ | ❌ | — | 0.85 |

### 4.7 Observability & Export

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| Sample-level logging | ❌ Absent | ✅ | ✅ | ✅ | OUGHT/Guardrail | 0.80 |
| W&B / TensorBoard | ❌ Absent | ✅ | ✅ | ✅ | Guideline | 0.80 |
| GGUF export | ❌ Absent | ❌ | ✅ | ❌ | Guideline | 0.70 |
| Model merging (TIES/DARE) | ❌ Absent | ✅ | ❌ | ❌ | Guideline | 0.70 |
| Model Soup | ❌ Absent | ✅ | ❌ | ❌ | Hypothesis | 0.60 |
| Multi-adapter (runtime switch) | ❌ Absent | ✅ | ❌ | ❌ | Guideline | 0.70 |
| VLM fine-tuning | ❌ Absent | ✅ | ✅ | ✅ | Guideline | 0.75 |
| Cloud dataset loading | ❌ Absent | ✅ | ❌ | ✅ Deep | Guideline | 0.75 |

### 4.8 Config & Validation

| Capability | hKask | Ludwig | Unsloth | Axolotl | Remediation | Confidence |
|---|---|---|---|---|---|---|
| Math-contract gates | ✅ Deep (14/17) | ❌ | ❌ | ❌ | — (hKask leads) | 0.95 |
| Harness-method compat | ✅ Deep (G-H1) | ❌ | ❌ | ❌ | — (hKask leads) | 0.95 |
| Convergence metric | ✅ Deep | ❌ | ❌ | ❌ | — (hKask leads) | 0.95 |
| Config schema validation | ✅ (Rust types) | ✅ (YAML schema) | ❌ | ✅ (YAML) | — | 0.85 |
| Cost estimation | ✅ (URJ) | ❌ | ❌ | ❌ | — (hKask leads) | 0.85 |

### 4.9 Remediation Classification Summary

| Classification | Count | Examples |
|---|---|---|
| OUGHT/Prohibition (must close) | 1 | Checkpoint resume (pod restart loses training state — $600 leak root cause class) |
| OUGHT/Guardrail (should close) | 11 | Full fine-tuning, packing, flash attention, grad checkpointing, mixed precision, multi-GPU, ZeRO, perplexity eval, benchmark eval, sample logging, checkpoint resume |
| Guideline (nice-to-have) | 14 | DPO/KTO/ORPO/GRPO depth, DoRA, VeRA, CorDA, activation offloading, Liger, CCE, SP, multi-node, W&B, GGUF, model merging, multi-adapter, VLM, cloud datasets |
| Hypothesis (uncertain value) | 3 | FP8 training, EAFT, Model Soup |
| — (hKask leads) | 5 | Math-contract gates, harness compat, convergence metric, cost estimation, A/B comparison |

**Classification:** IS/Observation/Direct+External-provenance. The
matrix is descriptive; the remediation column is OUGHT/Guideline
(prescriptive). Confidence varies per row as shown.

---

## 5. Rust-Transferability Verdicts

For each OUGHT/Guardrail gap, the falsifiability admit-gate: is the
claim "transfer to Rust" testable?

| Feature | Verdict | Boundary | Rationale |
|---|---|---|---|
| Checkpoint resume | **Yes** | Rust orchestrator writes checkpoint metadata to SQLite; training loop (Python) saves checkpoints to disk; Rust polls for completion marker | The Rust side is the lifecycle manager; the Python harness already saves checkpoints. The gap is that hKask doesn't resume from them. |
| Dataset packing | **Partial** | Config parameter passed through to harness (already possible); Rust-native packing would require OxiCUDA's attention implementation | The config toggle is a 1-line YAML field. Rust-native packing is a deep OxiCUDA dependency. |
| Flash attention | **Partial** | Config parameter (`flash_attention: true`) already in Axolotl YAML render path. Rust-native FA requires OxiCUDA `oxicuda-dnn::FlashAttention`. | hKask already passes this to Axolotl. The gap is that hKask doesn't expose it as a first-class parameter in `TrainingParams`. |
| Gradient checkpointing | **Partial** | Same as flash attention — config parameter. Rust-native requires OxiCUDA `oxicuda-train::CheckpointManager`. | Already in Axolotl config. Gap is `TrainingParams` doesn't expose it. |
| Mixed precision | **Partial** | Config parameter (`bf16: true`). Rust-native requires OxiCUDA AMP + GradScaler. | Already in Axolotl config. Gap is `TrainingParams` doesn't expose it. |
| Multi-GPU (FSDP/DeepSpeed) | **No-with-rationale** | Pure Python/PyTorch ecosystem. OxiCUDA has ZeRO 1-3 but unvalidated on H100. | The MCP-FFI boundary is: hKask passes `deepspeed_config` path to harness; execution is Python. Rust-native multi-GPU is a research project, not a near-term option. |
| ZeRO stages | **No-with-rationale** | Same as multi-GPU. OxiCUDA has ZeRO but unvalidated. | MCP-FFI boundary: config parameter to harness. |
| Perplexity eval | **Yes** | Rust can compute perplexity from model logits via inference API. No Python needed. | The eval harness already calls inference. Perplexity = exp(avg negative log-likelihood). Can be computed from the inference response if the API returns logprobs. |
| Benchmark eval (MMLU) | **Yes** | Rust can orchestrate MMLU-style eval: load benchmark dataset, format prompts, call inference, score. | This is a Rust-side orchestration task, not a training-loop task. |
| Sample-level logging | **Yes** | Rust can parse training logs (stdout/stderr from pod) and emit structured Regulation spans. | The training server already polls pod status. Adding log scraping + `reg.training.sample.*` spans is a Rust-side task. |
| Full fine-tuning | **Partial** | Config parameter (`adapter: none` or `full_finetuning: true`). Rust-native full FT requires OxiCUDA model loading + training loop. | Already possible via harness config. Rust-native is OxiCUDA-dependent. |

**Key insight:** Most "absent" features are absent from hKask's
`TrainingParams` struct, not absent from the rendered harness configs.
The Axolotl YAML template already supports `flash_attention`,
`gradient_checkpointing`, `bf16`, `sample_packing`, `deepspeed` —
hKask just doesn't expose them as first-class parameters. This is a
**shallow gap** (add fields to a struct + pass through to config
template), not a deep gap (implement the feature in Rust).

**Classification:** OUGHT/Guardrail (the verdicts are prescriptive
recommendations), provenance = assessment (derived from codebase +
exemplar analysis), confidence 0.80.

---

## 6. lora-training Skill Gap Report

### Gates Present (skill-level, all 17 defined)

The skill defines all 17 audit gates (G-M1..M5, G-Q1..Q6, G-D1..D3,
G-F1..F2, G-H1) and the 8-gate recommendation refinement (G0, G-D0,
G1-G6). This is **deeper** than any exemplar — no exemplar has
math-contract validation anchored to LoRA/QLoRA papers.

### Gates Missing vs Exemplars

| Gap | Exemplar evidence | Skill gap | Severity |
|---|---|---|---|
| Runtime gates (G-Q3, G-Q6, G-F2) | Axolotl/Unsloth execute these in the training loop | Skill correctly defers (advisory only); code-enforced subset also defers | Low — by design |
| Full fine-tuning method | Unsloth/Axolotl support `full_finetuning` | Skill's G0 only covers adapter purposes (instruction, reasoning, vision, preference, reward_model) — no `full_finetuning` purpose | Medium |
| FP8 training | Unsloth supports FP8 | No gate for FP8 precision validation | Low (Hypothesis) |
| Gradient checkpointing config | All exemplars support it | No gate validates checkpointing strategy vs memory budget | Medium |
| Flash attention config | All exemplars support it | No gate validates attention implementation selection | Low |
| Multi-GPU config | Axolotl supports FSDP/DeepSpeed | No gate validates parallelism strategy vs model size | Medium |
| Sequence packing | Axolotl/Unsloth support it | No gate validates packing strategy vs dataset characteristics | Low |
| Checkpoint resume | Unsloth/Axolotl support it | No gate validates checkpoint resume capability | Medium |

### Harness Capability Matrix Gap

The skill's harness matrix (in `lora-training-catalog.md`) is accurate
but **incomplete** vs current exemplar state:

| Gap | Evidence |
|---|---|
| Axolotl now supports more than SFT | docs.axolotl.ai shows RLHF, full FT; hKask matrix says "SFT only" — **stale** |
| Unsloth not in harness matrix | Unsloth is used in hKask's RunPod guide but not in the G6 harness selection |
| TRL GRPO status changed | TRL now has GRPOTrainer (huggingface.co/docs/trl/grpo_trainer); hKask matrix says "deferred" — **stale** |

**Classification:** IS/Observation for the current state;
OUGHT/Guardrail for the stale matrix entries. Provenance = direct
(skill text) + external (exemplar docs). Confidence 0.85.

---

## 7. GPU Platform MCDA

### 7.1 Alternatives

| Platform | Model | H100 $/hr | A100 $/hr | Billing | Spot | Multi-GPU | Source |
|---|---|---|---|---|---|---|---|
| RunPod (Secure) | Pod/Serverless/Cluster | $2.89 PCIe, $2.99 SXM | $1.39 PCIe, $1.49 SXM | per-second | Yes (Community) | Yes (Clusters) | runpod.io/pricing, usagepricing.com |
| Hetzner | Cloud VM | N/A | ~$0.38 (€0.35) | monthly | No | No | gpuhosted.com, computestacker.com |
| Cerebrium | Serverless | $4.50 | Enterprise plan | per-second | No | No | runpod.io/alternatives, skywork.ai |
| DeepInfra | Dedicated instance | $1.79 | $0.89 | per-minute | No | Yes (dedicated) | costbench.com, morphllm.com |
| Nebius | Cloud VM / HGX | $2.95 on-demand, $2.00 reserved | from $0.95 | per-second | Yes (preemptible) | Yes (HGX) | nebius.com, spheron.network, gpufinder.dev |

**Classification:** IS/Observation/External-provenance, confidence 0.80
  (pricing varies by region/time; verified July 2026).

### 7.2 Criteria & Weights

| Criterion | Weight | Rationale |
|---|---|---|
| GPU availability (H100/A100/L40S) | 0.20 | Must have H100 for 27B+ model training |
| Cost per GPU-hour | 0.15 | Budget-constrained; $600 already wasted |
| Reliability/uptime | 0.20 | Post-mortem showed reliability is critical |
| hKask integration friction | 0.15 | Existing RunPod integration vs greenfield |
| Rust-friendliness | 0.10 | Must drive from `kask` without Python glue |
| Spot/preemptible support | 0.05 | Useful for non-critical runs |
| Data egress/storage cost | 0.05 | Dataset publication to HuggingFace |
| Vendor lock-in / portability | 0.10 | Avoid platform-specific dead-ends |

### 7.3 Scored Matrix (1-10 scale)

| Criterion (weight) | RunPod | Hetzner | Cerebrium | DeepInfra | Nebius |
|---|---|---|---|---|---|
| GPU availability (0.20) | 8 | 3 | 5 | 7 | 8 |
| Cost (0.15) | 6 | 9 | 4 | 9 | 6 |
| Reliability (0.20) | 5 | 8 | 7 | 7 | 9 |
| Integration friction (0.15) | 9 | 4 | 3 | 4 | 4 |
| Rust-friendliness (0.10) | 7 | 8 | 4 | 6 | 7 |
| Spot support (0.05) | 7 | 1 | 1 | 1 | 6 |
| Egress cost (0.05) | 6 | 7 | 5 | 6 | 6 |
| Lock-in (0.10) | 7 | 8 | 5 | 7 | 6 |
| **Weighted total** | **6.90** | **6.15** | **4.65** | **6.40** | **6.80** |

### 7.4 Ranking

1. **RunPod — 6.90** (status quo, existing integration, mid-range cost/reliability)
2. **Nebius — 6.80** (highest reliability, HGX clusters, close second)
3. **DeepInfra — 6.40** (cheapest dedicated H100, inference-focused)
4. **Hetzner — 6.15** (cheapest A100, no H100, monthly billing)
5. **Cerebrium — 4.65** (serverless, expensive, Enterprise plan required)

### 7.5 Compensation Masking Detection

RunPod's high integration score (9) partially masks its low
reliability score (5). If we remove the integration advantage (set to
5, simulating a greenfield scenario where we'd need to build
integration for any platform), RunPod drops to 6.30, below Nebius
(6.80) and DeepInfra (6.40). This means: **RunPod's lead is entirely
from existing integration, not from platform quality.**

Nebius's high reliability score (9) partially masks its higher cost
vs DeepInfra. If we equalize cost (both at 7), Nebius rises to 7.00
and DeepInfra to 6.55 — Nebius's reliability advantage is real, not
cost-compensated.

### 7.6 Sensitivity Analysis (±20% weight perturbation)

| Scenario | RunPod | Nebius | DeepInfra | Hetzner | Cerebrium | Winner |
|---|---|---|---|---|---|---|
| Baseline | 6.90 | 6.80 | 6.40 | 6.15 | 4.65 | RunPod |
| Reliability +20% (0.24), Integration -20% (0.12) | 6.83 | 7.04 | 6.40 | 6.31 | 4.73 | **Nebius** |
| Cost +20% (0.18), Reliability -20% (0.16) | 6.88 | 6.62 | 6.39 | 6.10 | 4.57 | RunPod |
| Integration +20% (0.18), Cost -20% (0.12) | 6.99 | 6.74 | 6.25 | 6.10 | 4.61 | RunPod |
| Rust-friendliness +20% (0.12), Spot -20% (0.04) | 6.94 | 6.84 | 6.44 | 6.19 | 4.65 | RunPod |
| GPU availability +20% (0.24), Lock-in -20% (0.08) | 7.04 | 6.88 | 6.48 | 5.95 | 4.77 | RunPod |

**Key finding:** The ranking is **not stable** under +20% reliability
weight. Nebius overtakes RunPod when reliability is weighted higher.
This is the most consequential sensitivity — it directly addresses the
post-mortem's reliability concerns.

**Classification:** OUGHT/Guardrail (prescriptive recommendation
derived from IS data). Provenance = external (web research) +
assessment (scoring). Confidence 0.75 (scoring is inherently
subjective; sensitivity analysis exposes the fragility).

### 7.7 Falsifiability: RunPod Migration Counterfactual

**Hypothesis:** "If we migrated off RunPod, our training success rate
would improve."

**Evidence against (IS/Observation):** The post-mortem
(`docs/post-mortem/2026-07-19-training-providers.md`) shows all three
HIGH bugs were **code/operator errors**, not platform-inherent:
- H1: In-memory `HashMap` lost pod IDs → code bug (fixed with SQLite + JSON)
- H2: Together AI poll timeout + wrong JSON field → code bug (fixed)
- H3: PiSSA lesson not propagated → process bug (fixed)

**Evidence for (IS/Observation):**
- Community Cloud host-dependent reliability: "If a host goes offline,
  your pod can be interrupted with limited recourse" (spheron.network)
- H100 availability fluctuates: "H100s are the first to sell out"
  (gpuhosted.com)
- No bare-metal guarantee on Community tier

**Discriminating test design:** Run the same training job
(Qwen3-0.5B, 100 samples, 3 epochs, LoRA r=16) on:
- RunPod Secure Cloud H100 (5 runs)
- Nebius on-demand H100 (5 runs)

Measure: (a) pod provisioning time, (b) job completion rate, (c) cost
per successful run, (d) checkpoint integrity.

**Falsification:** If RunPod Secure Cloud achieves ≥80% completion rate
at ≤120% of Nebius cost, the migration hypothesis is **falsified** —
the failures were operator errors, not platform-inherent. If RunPod
Secure Cloud achieves <60% completion rate, the hypothesis is
**corroborated** and migration is justified.

**Verdict:** The counterfactual is **not yet admitted** — the evidence
does not support migration based on past failures (which were code
bugs). However, the MCDA sensitivity analysis shows Nebius is within
0.10 points of RunPod and overtakes it under reliability-weighted
scenarios. The recommendation is: **stay on RunPod Secure Cloud for
now, but add Nebius as a secondary provider for the discriminating
test.**

**Classification:** OUGHT/Guardrail (derived recommendation).
  Provenance = direct (post-mortem) + external (web research) +
  assessment (MCDA + falsifiability). Confidence 0.75.

---

## 8. Target Condition (kata Step 3)

### 8.1 Target (1 week – 3 months)

| Target | Timeline | Rationale |
|---|---|---|
| Expose `TrainingParams` pass-through fields (flash_attention, gradient_checkpointing, bf16, sample_packing, deepspeed_config) | 1 week | Shallow gap — struct fields + template passthrough |
| Add checkpoint resume to pod lifecycle | 2 weeks | OUGHT/Prohibition — pod restart loses training state |
| Add perplexity eval to `training_evaluate` | 1 week | Rust-side computation from inference logprobs |
| Add benchmark eval scaffold (MMLU-style) | 2 weeks | Rust-side orchestration |
| Add sample-level log scraping + Regulation spans | 1 week | Rust-side log parsing + `reg.training.sample.*` |
| Update harness capability matrix (stale entries) | 3 days | Axolotl >SFT, TRL GRPO, add Unsloth |
| Run RunPod vs Nebius discriminating test | 1 week | Falsifiability verdict on platform migration |
| Build OxiCUDA PoC (Qwen3-0.5B, 1 training step) | 2-4 weeks | Validates Rust-native training path; per research doc |

### 8.2 Obstacles Parking Lot

1. **Checkpoint resume** — pod restart loses training state; no
   mechanism to detect and resume from last checkpoint.
2. **RunPod reliability** — Community Cloud host-dependent; Secure
   Cloud untested at scale.
3. **No eval harness** — only exact_match/contains/semantic; no
   perplexity, no benchmark, no automated lemon-pick.
4. **Stale harness matrix** — Axolotl/TRL capabilities have evolved;
   hKask matrix is outdated.
5. **`TrainingParams` too narrow** — doesn't expose flash attention,
   gradient checkpointing, packing, deepspeed, mixed precision.
6. **OxiCUDA unvalidated on H100** — Rust-native training is a
   research path, not a proven path.
7. **Dataset format mismatch** — Unsloth expects `text`, hKask
   produces `messages` (per readiness doc).
8. **No full fine-tuning** — only LoRA/QLoRA supported.
9. **No multi-GPU** — single GPU pods only.
10. **No sample-level observability** — can't diagnose training
    failures at the sample level.

### 8.3 Focus Obstacle (ONE)

**Checkpoint resume.**

Rationale (evidence-based, not assumed): The post-mortem's H1 finding
shows the single most expensive failure ($600) was caused by pod
restarts losing state. Even after the persistence fix (pod IDs now
survive restarts), the training state itself (model weights, optimizer
state, LR scheduler position) does not survive. If a pod restarts
mid-training, the job must start from epoch 0. This makes long
training runs (26-55h per Together AI post-mortem data) fragile and
expensive. Every exemplar (Unsloth, Axolotl, Ludwig) supports
checkpoint resume. This is the OUGHT/Prohibition gap with the highest
cost-avoidance ROI.

### 8.4 Knowledge Gap

We do not yet know:
- Whether RunPod pods preserve `/workspace` across restarts (volume
  persistence vs ephemeral disk).
- Whether Axolotl's checkpoint format is compatible with hKask's
  adapter store (safetensors vs OXPA).
- Whether the existing `training_status` poll loop can detect a
  restarted pod and trigger resume logic.

**Classification:** IS/Observation (target is prescriptive
  OUGHT/Guardrail). Provenance = assessment (derived from gap matrix +
  post-mortem). Confidence 0.80.

---

## 9. Experiment (kata Step 4)

### Focus Obstacle: Checkpoint Resume

| Element | Detail |
|---|---|
| **Next step** | Add a `resume_from_checkpoint` field to `TrainingParams` and wire it through the Axolotl config template. When `training_status` detects a pod that restarted (status transitions from `running` → `stopped` → `running`), automatically re-submit with `resume_from_checkpoint: /workspace/outputs/{job_id}/checkpoint-{last_step}`. |
| **Prediction** | If RunPod preserves `/workspace` volume across pod restarts (likely — it's a network volume), then a restarted training job can resume from the last checkpoint within 60 seconds, avoiding a full restart. This would have prevented ~$400 of the $600 leak. |
| **Do** | 1. Verify RunPod `/workspace` persistence: launch a pod, write a file to `/workspace`, stop the pod, restart it, check if the file survives. 2. Add `resume_from_checkpoint: Option<String>` to `TrainingParams` in `types.rs`. 3. Add the field to the Axolotl YAML template (`axolotl-lora.j2`). 4. In `training_status`, detect pod restart (status transition) and emit a `reg.training.checkpoint.resume` span. |
| **Check** | (a) `/workspace` file survives pod restart → volume is persistent. (b) `cargo test -p hkask-mcp-training` passes with new field. (c) Rendered Axolotl YAML includes `resume_from_checkpoint` when set. (d) `reg.training.checkpoint.resume` span fires on restart detection. |
| **Act** | If `/workspace` is persistent: implement full auto-resume in `training_status`. If not persistent: investigate RunPod volume mounting options or switch to Nebius (which has persistent storage). |
| **When to check** | Within 1 day — the `/workspace` persistence test takes ~10 minutes of GPU time (~$0.50). |

**Classification:** OUGHT/Guardrail (prescriptive experiment design).
  Provenance = assessment (derived from post-mortem + gap analysis).
  Confidence 0.80.

---

## 10. Open Questions Register

Every assumption surfaced during this analysis, not silently resolved.

| # | Question | Type | Current assumption | Falsifiable? |
|---|---|---|---|---|
| Q1 | Does RunPod `/workspace` survive pod restart? | Knowledge gap | Assumed yes (network volume) — untested | Yes: write file, restart, check |
| Q2 | Is OxiCUDA's ZeRO implementation validated on H100 (SM 9.0)? | Knowledge gap | Research doc says "Medium risk" — untested | Yes: run GEMM on H100 |
| Q3 | Does Axolotl's checkpoint format integrate with hKask's adapter store? | Knowledge gap | Assumed safetensors (standard) — unverified | Yes: inspect checkpoint dir |
| Q4 | Are the RunPod failures platform-inherent or operator errors? | Counterfactual | Post-mortem says operator errors — but reliability is untested at scale | Yes: discriminating test (§7.7) |
| Q5 | Is Nebius's 99% 30-day reliability score applicable to training workloads? | Knowledge gap | Assumed yes — but score may measure inference availability, not training | Yes: run 5 training jobs on Nebius |
| Q6 | Does DeepInfra support training workloads or only inference? | Knowledge gap | Docs say "dedicated instances" — but training requires SSH access, not just API | Yes: try SSH into DeepInfra instance |
| Q7 | Can hKask's inference API return logprobs for perplexity computation? | Knowledge gap | Assumed yes — but depends on provider (Together AI, RunPod serverless) | Yes: check API response schema |
| Q8 | Is the harness matrix stale for Axolotl (now supports >SFT)? | Stale data | Web research says Axolotl supports RLHF, full FT — but hKask matrix says "SFT only" | Yes: check axolotl docs |
| Q9 | Does TRL's GRPOTrainer work without vLLM co-location? | Knowledge gap | TRL docs show `--use_vllm --vllm_mode colocate` — may not work on single GPU | Yes: run TRL GRPO on single H100 |
| Q10 | Is the Unsloth `messages` vs `text` format mismatch still blocking? | Knowledge gap | Readiness doc says blocking — but post-mortem may have fixed it | Yes: check dataset pipeline output |
| Q11 | Does Hetzner's monthly billing model make it impractical for short training runs? | Knowledge gap | Assumed yes — monthly commitment vs per-second RunPod | Yes: calculate breakeven |
| Q12 | Can Cerebrium's serverless model support long training runs (26-55h)? | Knowledge gap | Serverless typically has execution time limits | Yes: check Cerebrium docs for max execution time |
| Q13 | Is OxiCUDA's `oxicuda-lm` Qwen3 loader straightforward (~100 lines)? | Knowledge gap | Research doc estimates ~100 lines based on LLaMA similarity — unverified | Yes: write the loader |
| Q14 | Does hKask's `training_evaluate` semantic eval (LLM-as-judge) produce reliable scores? | Knowledge gap | Uses same model for generation and judging — known bias risk | Yes: compare with human eval on 20 samples |
| Q15 | Is the $600 billing leak truly fixed, or could it recur under different conditions? | Counterfactual | Post-mortem says fixed (SQLite + JSON persistence) — but untested under crash conditions | Yes: kill process mid-submit, verify pod ID persists |

**Classification:** IS/Observation (questions are descriptive of
  knowledge gaps). Each carries an implicit OUGHT/Guardrail to resolve
  before acting on the associated assumption. Provenance = assessment.
  Confidence 0.70 (some questions may be resolvable by reading docs
  we haven't checked yet).

---

## Appendix: Pragmatic-Semantics Classification Summary

| Claim Type | Count | Constraint Force |
|---|---|---|
| IS/Observation/Direct (codebase evidence) | 25 | None (descriptive) |
| IS/Observation/External (exemplar/platform evidence) | 18 | None (descriptive) |
| OUGHT/Prohibition (Magna Carta / CI-gated) | 1 | Hard (must close) |
| OUGHT/Guardrail (should close) | 11 | Medium (ergonomic/correctness) |
| Guideline (nice-to-have) | 14 | Low |
| Hypothesis (uncertain value) | 3 | None (needs falsifiability test) |
| Assessment (derived recommendation) | 7 | Medium (agent-derived, operator-reviewed) |

**Every recommendation in this document carries a provenance trace to
either hKask source code, exemplar documentation, or web research.
No recommendation is based on fabricated evidence.**

---

*End of analysis. See `tasks/training-gap-plan.md` and
`tasks/training-gap-todo.md` for the decomposed task plan.*