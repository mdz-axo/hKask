# E3 Intermediate-Rank Selection Report

File changed: corpus/lora/axolotl-pissa-pod.yaml (same file, added E3 section)

Changes made:
- Added E3 section: "E3 — Intermediate-rank selection (Quercia 2026)"
- Added EvaConfig block (`eva_config:`) with:
  - `dataloader`: mdz-axo/capabilities-researcher-qa (same training dataset for activation-SVD)
  - `rho`: 2.5 (adaptive redistribution; ρ > 2 gives heterogeneous/adaptive rank distribution per Quercia 2026 + Paischer 2024)
  - `intermediate_slice_start`: commented out (optional fixed slice at s=256 for Qwen3.6; uncomment for fixed intermediate-slice approach instead of adaptive redistribution)
- Added fallback instructions: if EvaConfig causes errors with current Axolotl/PEFT version, comment out `eva_config` block and use standard LoRA with manual intermediate-slice adapter creation
- References added: Quercia 2026 (intermediate-rank U-shaped forgetting, more robust to high LR); Paischer 2024 (adaptive redistribution ρ > 2)
- All previous settings preserved: base_model, adapter, sequence_len, learning_rate, patience=25, output_dir, datasets, optimizations

Research basis:
- Quercia 2026 (arXiv:2602.03493): Intermediate principal components (not PiSSA top-r, not MiLoRA bottom-r) show best performance-forgetting trade-off; U-shaped forgetting curve; more robust to high learning rates — matching our patience=25 cosine LR schedule.
- Paischer 2024 (arXiv:2410.07170): EVA adaptive redistribution uses singular value ratios (ρ controls uniformity); ρ > 2 allows heterogeneous rank assignment across layers.

Status: Config ready for v3.1 (after E2 EVA init confirmed working). If EvaConfig is unsupported by current PEFT/Axolotl version, fallback to standard LoRA with manual intermediate-slice initialization (requires custom adapter creation script/post-processing).
Next action: Confirm pipeline health (E1), then launch v3 with EVA + EvaConfig; evaluate intermediate-rank performance (eval_loss comparison to v2 baseline 0.2156).
=== END ===
