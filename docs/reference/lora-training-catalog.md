# LoRA Training — Method & Gate Catalog

Reference catalog for the `lora-training` skill. Extracted from
`.agents/skills/lora-training/SKILL.md` to keep the skill companion lean.
The registry crate (`registry/templates/lora-training/`) remains
authoritative (P5.1); this document is a derived reference.

## Method Catalog

| Method | When (gate) | Init | Mergeable | Source |
|--------|-------------|------|-----------|--------|
| LoRA | G4=default | `B=0`, `A~Gaussian` | Yes | arXiv:2106.09685 |
| QLoRA | G2=memory-bound | LoRA init on NF4 base | Yes (after dequant) | arXiv:2305.14314 |
| rsLoRA | G3=r>64 | LoRA init, `α/√r` scaling | Yes | arXiv:2312.03732 |
| DoRA | G4=cost-sensitive, low r | LoRA init + magnitude | Yes (PEFT ≥0.10) | arXiv:2402.09353 |
| PiSSA | G4=fast convergence | SVD of base weight | Yes (via `subtract_mutated_init`) | arXiv:2404.02948 |
| LoRA-GA | G4=fast convergence | Gradient SVD | Yes (via `save_mutated_as_lora`) | arXiv:2407.05000 |
| CorDA-KP | G5=knowledge preservation | Context-oriented | Yes | PEFT v0.19.0 `corda_config` |
| EVA | G4=data-driven init | SVD of activations | Yes | PEFT v0.19.0 `eva_config` |
| AdaLoRA | (not default) | SVD-parameterized | Yes | arXiv:2303.10512 |
| aLoRA | G1=dynamic-switching | LoRA init | No (selective) | PEFT v0.19.0 `alora_invocation_tokens` |
| IA³ | (extreme param budget) | Vector scaling | Yes | PEFT `IA3Config` |
| Prefix Tuning | (not recommended for production) | Prefix vectors | No | PEFT `PrefixTuningConfig` |

New methods can be added as PEFT exposes them and literature justifies
them (P7) — not speculatively.

## Gate Catalog

16 quality gates enforced by the `audit-config` phase. Each gate is a
single assertion with a citation.

### Math-Contract Gates (from LoRA paper, arXiv:2106.09685)

| Gate | ID | Assertion | Source |
|------|----|-----------|--------|
| No-op-at-init invariant | G-M1 | `init_lora_weights=True` produces ΔW=0 at step 0 (B=0). Non-default init requires justification. | LoRA §4.1; PEFT `init_lora_weights` docstring |
| Merge equivalence | G-M2 | `W_merged = W + (α/r)·BA` ≡ `W + adapter` within fp tolerance. Broken by `bias='all'`/`'lora_only'`, DoRA on PEFT<0.10. | LoRA §4.2; PEFT `bias` docstring |
| Scaling form | G-M3 | scaling = `α/r` (or `α/√r` if `use_rslora`). Never `α`, `r/α`, or `1`. | LoRA §4.1; rsLoRA arXiv:2312.03732 |
| Rank budget | G-M4 | `r < min(d_in, d_out)`. Warn if `r ≥ 0.5×min`. Refuse if `r ≥ min`. | LoRA §4.3 |
| Trainable param count | G-M5 | `2·r·(d_in+d_out)·n_layers` matches `print_trainable_parameters()` within 1%. | LoRA Table 5 |

### QLoRA-Specific Gates (from QLoRA paper, arXiv:2305.14314)

Only apply if QLoRA mode selected (G2).

| Gate | ID | Assertion | Source |
|------|----|-----------|--------|
| Frozen base quantized | G-Q1 | `base_param.dtype` is 4-bit NF4. | QLoRA §3 |
| Adapter dtype | G-Q2 | LoRA `A`, `B` are bf16/fp32, never 4-bit. | QLoRA §3 |
| Gradient flow | G-Q3 | `A.grad` and `B.grad` are not None after first backward. | QLoRA §3 |
| No silent upcast | G-Q4 | Frozen base not cast to fp32 by `prepare_model_for_kbit_training`. | QLoRA §3; PEFT docstring |
| Paged optimizer (conditional) | G-Q5 | `paged_adamw_8bit` only required if peak memory > VRAM. | QLoRA §3 |
| NF4 optimality assumption | G-Q6 | NF4 optimal for normally-distributed weights; warn if non-normal. | QLoRA §3 |

### Data/Eval Gates (from QLoRA paper, arXiv:2305.14314)

| Gate | ID | Assertion | Source |
|------|----|-----------|--------|
| Dataset size vs quality | G-D1 | `n<1000` requires justification; `n>100000` requires quality audit. | QLoRA §5 |
| Eval protocol | G-D2 | Vicuna/MMLU alone not trustworthy; require human or GPT-4 eval. | QLoRA §5 |
| Lemon-pick analysis | G-D3 | Report failure cases, not just aggregate score. | QLoRA §6 |

### Forgetting Gates (post-QLoRA literature)

| Gate | ID | Assertion | Source |
|------|----|-----------|--------|
| Intruder dimension check | G-F1 | Report intruder dimensions before/after training via `reduce_intruder_dimension`. | Razin et al. arXiv:2410.21228 |
| Knowledge preservation (CorDA) | G-F2 | If CorDA Knowledge-Preserved mode: assert world-knowledge eval doesn't regress. | CorDA; PEFT `corda_config` docstring |

## Convergence Metric Weights

Computed by the `convergence-check` phase. Metric ∈ [0, 1] where 0 =
fully converged (training-ready).

| Dimension | Weight | Pass condition |
|-----------|--------|----------------|
| Critical + high findings resolved | 0.40 | 0 critical/high = +0.00; 1+ = +0.40 |
| Math-contract gate coverage | 0.25 | All 5 (G-M1..G-M5) pass = +0.00; scaled by failures |
| QLoRA gate coverage | 0.15 | All 6 (G-Q1..G-Q6) pass = +0.00; only if QLoRA mode |
| Data/eval gate coverage | 0.10 | All 3 (G-D1..G-D3) pass = +0.00 |
| Forgetting gate coverage | 0.10 | G-F1 planned = +0.00; G-F2 if CorDA mode |

Converged: metric ≤ 0.10 AND ≥5% relative improvement from previous cycle.

## Source References

- **LoRA:** arXiv:2106.09685 — arxiv.org/abs/2106.09685
- **QLoRA:** arXiv:2305.14314 — arxiv.org/abs/2305.14314
- **rsLoRA:** arXiv:2312.03732 — arxiv.org/abs/2312.03732
- **DoRA:** arXiv:2402.09353 — arxiv.org/abs/2402.09353
- **PiSSA:** arXiv:2404.02948 — arxiv.org/abs/2404.02948
- **LoRA-GA:** arXiv:2407.05000 — arxiv.org/abs/2407.05000
- **AdaLoRA:** arXiv:2303.10512 — arxiv.org/abs/2303.10512
- **Razin et al. (intruder dimensions):** arXiv:2410.21228 — arxiv.org/abs/2410.21228
- **AutoPEFT (rejected alternative):** arXiv:2301.12132 — arxiv.org/abs/2301.12132
- **PEFT v0.19.0:** huggingface.co/docs/peft/v0.19.0/package_reference/lora
- **Practitioner consensus:** Raschka (magazine.sebastianraschka.com), Brenndoerfer (mbrenndoerfer.com), Spheron (spheron.network/blog/peft-methods-2026-dora-galore-pissa-vera-guide), Databricks (databricks.com/blog/efficient-fine-tuning-lora-guide-llms), Gradient Flow (gradientflow.com/lora-or-full-fine-tuning/)
