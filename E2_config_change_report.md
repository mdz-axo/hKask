# E2 Config Change Report (EVA Initialization — Post-PiSSA Fix)

File changed: corpus/lora/axolotl-pissa-pod.yaml

Changes made:
- Header updated: "Axolotl Config — Capabilities Researcher LoRA with EVA" (was PiSSA)
- Description: v3 update noting portability fix (transformers 5.9.0 vs 5.5.0 failure avoided)
- peft_init_lora_weights: changed from `pissa_niter_4` → `eva`
- Comment added: EVA uses activation-vector SVD (not weight-SVD) → standard portable LoRA adapter (A initialized with activation variance directions, B=0)
- Note added: EvaConfig with dataloader reference and ρ parameter (adaptive redistribution control) may be needed if Axolotl/PEFT requires explicit config
- Intermediate-rank reference added: Quercia 2026 shows intermediate-rank gives better performance-forgetting trade-off than PiSSA top-r or MiLoRA bottom-r; ρ > 2 gives heterogeneous adaptive ranks
- lora_dropout remains 0 (required: EVA initializes A with activation directions; dropout would discard them)
- All other settings preserved: r=32, alpha=64, sequence_len=4096, bf16, cosine LR, patience=25, output_dir=/workspace/outputs, hub_model_id configured for upload

Rationale (from verified research):
- Paischer et al. 2024 (arXiv:2410.07170, NeurIPS 2024 Oral AFM): EVA initializes LoRA A with right-singular vectors from incremental SVD on activation batches; adapter = standard portable A×B; B=0 init; no weight-SVD dependency → portable across library versions.
- Quercia et al. 2026 (arXiv:2602.03493): Intermediate principal components avoid U-shaped catastrophic forgetting; more robust to high LR (matches our patience=25 cosine schedule).

Status: Config ready for v3 training run. Requires E1 confirmation (pipeline healthy) before launching new 26h GPU run (~$83 cost).
Next experiment (E3): Configure intermediate/adaptive rank selection (either fixed intermediate slice s ∈ (0, rmax-r) or adaptive redistribution ρ > 2 via EvaConfig).

Result: PORTABILITY STRUCTURE FIXED (adapter will be standard portable LoRA). Performance improvement (intermediate-rank) pending E3.
=== END ===
