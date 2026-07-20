---
name: lora-training
visibility: public
description: >
  LoRA/QLoRA training skill for hKask (v0.31.0). Selects a PEFT method
  via a deterministic 5-gate decision (inference constraint, memory
  budget, task distance, quality/cost trade-off, knowledge preservation)
  and enforces the LoRA/QLoRA math contracts as quality gates (no-op
  init, merge equivalence, scaling form, rank budget, quant dtype,
  gradient flow, intruder dimensions). Anchored to LoRA (arXiv:2106.09685),
  QLoRA (arXiv:2305.14314), rsLoRA (arXiv:2312.03732), DoRA
  (arXiv:2402.09353), PiSSA (arXiv:2404.02948), LoRA-GA (arXiv:2407.05000),
  CorDA, EVA, Razin et al. intruder dimensions (arXiv:2410.21228), and
  PEFT v0.19.0 config surface. Emits cns.lora.* spans (P9). Decomposed
  into 4 phases matching bug-hunt, kali-audit, supply-chain-sentinel
  pipeline. Minimal (P5): answers all 5W1H; single skill, no bundle;
  complements kali-audit (security surface) and tdd (training-loop code)
  with zero overlap — this skill governs training configuration and
  contracts, not Rust code structure or security posture.
---

# LoRA Training

{# goal: Select a PEFT method via a deterministic 5-gate decision (inference constraint, memory budget, task distance, quality/cost trade-off, knowledge preservation) within user workspace boundaries (P4 OCAP). Enforce LoRA/QLoRA math contracts (no-op init, merge equivalence, scaling form, rank budget, quant dtype, gradient flow, intruder dimensions) as quality gates. Map findings to LoRA (arXiv:2106.09685), QLoRA (arXiv:2305.14314), rsLoRA (arXiv:2312.03732), DoRA (arXiv:2402.09353), PiSSA (arXiv:2404.02948), LoRA-GA (arXiv:2407.05000), CorDA, EVA, Razin et al. (arXiv:2410.21228). Emit cns.lora.* spans (P9). Compute convergence metric from real config evidence only. No synthetic benchmarks; no actual training execution; replicant_host mandatory (P12). #}

LoRA/QLoRA training configuration and contract enforcement. Reads
training config (`LoraConfig`, `BitsAndBytesConfig`, accelerate config,
training script arguments) as concrete evidence. Selects a PEFT method
via a deterministic 5-gate decision (not a search — AutoPEFT-style BO is
infeasible per training job). Enforces the LoRA/QLoRA math contracts as
quality gates. Emits `cns.lora.*` spans. Computes a training-readiness
convergence metric.

## When to Use

- Before launching a LoRA/QLoRA training job — to select the method and
  verify the config.
- When auditing an existing training config (`LoraConfig`,
  `BitsAndBytesConfig`, training script) for math-contract violations.
- When deciding between LoRA, QLoRA, DoRA, PiSSA, LoRA-GA, CorDA, EVA,
  AdaLoRA, IA³, Prefix Tuning, or full fine-tuning.
- When verifying that the chosen init strategy (`init_lora_weights`)
  matches the training goal (fast convergence vs knowledge preservation
  vs quantization-error minimization).
- When checking for intruder dimensions after training (Razin et al.
  2024 — LoRA ≠ full fine-tune; forgetting is structured).
- When proposing `security/regressions/` entries for training-config
  anti-patterns (e.g., `bias='lora_only'` breaks merge equivalence).

## Design Constraints (Grounded in Project Principles)

- **P5 Essentialism (5W1H gate):** Who = training operator / replicant
  host (P12); What = PEFT config / training hyperparameters / dataset
  characteristics; Where = training script / config file / model card;
  When = pre-training gate (config audit) and post-training gate
  (intruder check, merge equivalence); Why = P3.1 safe container /
  P1 user sovereignty over training choices / P4 explicit training
  boundaries; How = select method → audit config → verify contracts →
  post-train check → emit CNS span → compute convergence. All 6
  present — passes gate.
- **P5.1 Registry canonical:** Registry (`manifest.yaml` + `.j2`) is
  source of truth. SKILL.md derived from it.
- **P5.3 Minimalist test:** No actual training execution; config and
  contract analysis only (P4 boundary). No extra abstractions.
- **P5.4 Dual-axis:** Each finding has state identity (config line) and
  process identity (gate flow).
- **P7 Evolutionary:** Method catalog and gates emerge from real
  config patterns and cited literature, not speculation.
- **P8 Semantic grounding:** Every claim: file path, config line,
  parameter value, evidence snippet, source citation (arXiv paper,
  PEFT docs section, regression YAML). No fabricated benchmark numbers.
- **P9 CNS regulation:** Emits `cns.lora.select`, `cns.lora.audit`,
  `cns.lora.report`, `cns.lora.convergence` spans. All four are
  registered in `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`)
  and emitted unconditionally.
- **P10 Bot/replicant taxonomy:** `visibility: public` — transparent
  training governance.
- **P11 Visibility:** Regression proposals default `status: pending`
  (human-curated ratchet, per `security/regressions/README.md`).
- **P12 Replicant host mandate:** Every action includes `replicant_host`.
- **P3.1 Safety floor:** Training-config errors (wrong scaling, broken
  init, silent fp32 upcast) silently degrade model quality — this skill
  is the safety floor for training.
- **P4 OCAP boundaries:** Reads only declared workspace config paths;
  no ambient model download; no network calls to HuggingFace Hub without
  explicit consent (P2).

## Instructions

### lora-training/select-method

1. Read the training config (`LoraConfig`, `BitsAndBytesConfig`, accelerate config, training script args).
2. Extract the five gate inputs: `inference_constraint`, `memory_budget_gb`, `model_size_b`, `task_distance`, `quality_vs_cost`, `knowledge_preservation_required`.
3. Apply the deterministic 5-gate decision (G1 inference → G2 memory → G3 task distance → G4 quality/cost → G5 knowledge preservation). Full gate logic and refusal conditions live in `select-method.j2`.
4. Emit a `LoraConfig`-shaped JSON with per-gate justification citations, or refuse with a cited reason.
5. Emit `cns.lora.select` CNS span.

### lora-training/audit-config

1. Read each config file; quote lines for evidence (not synthetic).
2. Extract `LoraConfig`, `BitsAndBytesConfig`, and training script parameters.
3. Apply the 16 quality gates (13 implemented: G-M1..G-M5, G-Q1, G-Q2, G-Q4, G-Q5, G-D1, G-D2, G-D3, G-F1; 3 deferred to runtime: G-Q3, G-Q6, G-F2). Pre-submit gates run in `lora_validation.rs`; post-training gates run in `training_preflight_check`. Full gate definitions live in `audit-config.j2` and `docs/reference/lora-training-catalog.md`.
4. Apply pragmatic-semantics (IS/OUGHT, epistemic mode, provenance) and grill-me self-challenge.
5. Emit `cns.lora.audit` spans per gate evaluated.

### lora-training/report

1. Group findings by severity (critical/high/medium/low).
2. Propose `RR-NNNN.yaml` entries with `surface: training` for findings ≥ medium.
3. Identify contract gaps and top 3 fixes.
4. Produce verdict: Pass (zero critical/high, all math gates pass), Conditional (medium or 1-2 warn), Fail (critical/high or refuse).
5. Emit `cns.lora.report` CNS span.

### lora-training/convergence-check

1. Compute normalized convergence metric [0, 1] where 0 = training-ready.
2. Weighted dimensions: critical/high findings (0.40), math gates (0.25), QLoRA gates (0.15), data/eval (0.10), forgetting (0.10). Full weights in `convergence-check.j2`.
3. Converged when metric ≤ 0.10 with ≥5% relative improvement.
4. Emit `cns.lora.convergence` CNS span.

## Registry Templates

| Template | Type | Purpose |
|----------|------|---------|
| `select-method.j2` | KnowAct | Apply 5-gate decision (inference, memory, task distance, quality/cost, knowledge preservation); emit `LoraConfig`-shaped JSON with per-gate justification; emit `cns.lora.select` span. |
| `audit-config.j2` | KnowAct | Read config evidence; apply 15 math/quant/data/forgetting gates (G-M1..G-M5, G-Q1..G-Q6, G-D1..G-D3, G-F1..G-F2); apply pragmatic-cybernetics; emit `cns.lora.audit` spans. |
| `report.j2` | KnowAct | Synthesize findings with arXiv citations and PEFT doc references; propose `RR-NNNN.yaml` entries (`surface: training`); emit `cns.lora.report` span. |
| `convergence-check.j2` | KnowAct | Compute training-readiness convergence metric (math-contract coverage + QLoRA coverage + data/eval + forgetting). Emit `cns.lora.convergence` span. |

## Method & Gate Catalog

The full method catalog (12 PEFT methods), gate catalog (16 quality
gates: G-M1..G-M5, G-Q1..G-Q6, G-D1..G-D3, G-F1..G-F2), convergence
metric weights, and source references live in
[`docs/reference/lora-training-catalog.md`](../../docs/reference/lora-training-catalog.md).

Summary:

- **Methods:** LoRA, QLoRA, rsLoRA, DoRA, PiSSA, LoRA-GA, CorDA-KP,
  EVA, AdaLoRA, aLoRA, IA³, Prefix Tuning.
- **Math gates (G-M1..G-M5):** no-op-at-init, merge equivalence,
  scaling form, rank budget, trainable param count.
- **QLoRA gates (G-Q1..G-Q6):** frozen base quantized, adapter dtype,
  gradient flow, no silent upcast, paged optimizer, NF4 optimality.
- **Data/eval gates (G-D1..G-D3):** dataset size vs quality, eval
  protocol, lemon-pick analysis.
- **Forgetting gates (G-F1..G-F2):** intruder dimension check,
  knowledge preservation (CorDA).
- **Convergence weights:** critical/high (0.40), math (0.25), QLoRA
  (0.15), data/eval (0.10), forgetting (0.10).

## Relationship to Existing Skills

- **`kali-audit`:** `kali-audit` covers security surfaces (Rust code,
  templates, MCP, supply chain). This skill covers training
  configuration and contracts. Zero overlap — different surface
  (`surface: training` vs `surface: code`/`template`/`mcp`/`supply-chain`).
  Both consume `security/regressions/` as input; both propose new
  entries for human review.
- **`tdd`:** `tdd` covers training-loop code (test-driven development
  of Rust crates). This skill governs the training *configuration*
  passed to that code, not the code itself. Complementary: `tdd`
  builds the training loop; `lora-training` audits the config the loop
  consumes.
- **`bug-hunt`:** Provides decomposed pipeline structure (`Charter` →
  `Probe` → `Oracle` → `Taxonomize` → `Report`). This skill replicates
  that structure (`select-method` ≈ charter; `audit-config` ≈ probe +
  oracle; `report` ≈ taxonomize + report; `convergence-check` ≈
  convergence). Uses same pragmatic-cybernetics and pragmatic-semantics
  reasoning embedded in instructions.
- **`supply-chain-sentinel`:** Audits dependency manifests. This skill
  audits training configs. Both are pre-flight gates (supply chain
  before deploy; training config before training). Both emit CNS spans
  and propose regressions.
- **`diagnose`:** If a training run fails or produces a degraded model,
  `diagnose` handles the cybernetic debugging loop (reproduce →
  hypothesize → instrument → fix → regression-test). This skill is the
  *pre-flight* gate; `diagnose` is the *post-failure* loop. This skill
  proposes regressions that `diagnose` can use to anchor hypotheses.
- **`lora-training` does NOT replace any of these:** It fills the gap
  between `tdd` (training-loop code correctness) and `kali-audit`
  (security posture) by governing training *configuration* and *math
  contracts* — the layer where silent quality degradation happens
  (wrong scaling, broken init, fp32 upcast, intruder dimensions).

## Constraints (Concrete — Not Aspirational)

- `select-method.j2`: `visibility: public`.
- `audit-config.j2`: `visibility: public`.
- `report.j2`: `visibility: public`.
- `convergence-check.j2`: `visibility: public`.
- Every finding includes concrete file path, config line, parameter
  value, quoted evidence snippet, source citation — not summary
  description.
- Every proposed regression uses exact YAML format
  (`security/regressions/`) with `surface: training`, concrete
  `pattern` (grep regex against config content, or Python assertion
  code for runtime gates), `status: pending`, `gate: G-XX`.
- No synthetic config quotes; read file before quoting.
- No synthetic benchmark numbers; only reference paper-reported results
  with explicit citation (paper, table/section).
- No fabricated training-run results; this skill does not execute
  training (P4 boundary enforcement).
- Registry (`manifest.yaml` + `.j2`) is authoritative over this SKILL.md
  (P5.1).
- Do NOT invent config entries not present in the training config.
- Do NOT claim actual training execution or model evaluation capability
  — config and contract analysis only (P4 boundary enforcement).
- Every audit action includes `replicant_host` identity (P12).
- Every training-config audit emits `cns.lora.*` span. All four
  namespaces (`select`, `audit`, `report`, `convergence`) are
  registered in `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`).
- Apply pragmatic-cybernetics feedback loop analysis: scaling×rank
  interaction, LR-vs-rank variety, Good Regulator (init strategy
  matched to training goal), delay (init preprocessing overhead vs
  convergence speedup).
- Apply `grill-me` self-challenge before proposing findings.
- Apply `IS/OUGHT` classification and label `epistemic_mode` and
  `provenance` for every finding.
- Convergence metric computed from real evidence: unresolved
  critical/high findings (0.40), math-contract coverage (0.25), QLoRA
  coverage (0.15), data/eval coverage (0.10), forgetting coverage (0.10).
- Do NOT fabricate findings — only report what was discovered through
  actual config reading (like `kali-audit` constraint).
- Source citations must reference concrete sources (not aspirational):
  arXiv paper URLs (abs/2106.09685, abs/2305.14314, abs/2312.03732,
  abs/2402.09353, abs/2404.02948, abs/2407.05000, abs/2410.21228,
  abs/2303.10512), PEFT v0.19.0 docs
  (huggingface.co/docs/peft/v0.19.0/package_reference/lora),
  `security/regressions/README.md` for regression format.
- If config discovery finds zero training config files, return empty
  `config_paths` and recommend `surface: training` defaults based on
  workspace evidence (`.py` training script, `axolotl` config, `trl`
  config) — do NOT invent config content.
- Before proposing any regression entry, verify config line exists and
  evidence snippet can be quoted from actual file content.
- This skill does NOT execute training, load models, or run evals. It
  audits training configuration within user-defined workspace
  boundaries (P4 OCAP enforcement perimeter — config must be explicitly
  declared, not ambient authority).
- Propose `surface: training` regression entries only; do NOT reuse
  `surface: code`, `surface: template`, `surface: mcp`, `surface: config`,
  or `surface: supply-chain` — training findings have distinct gate
  catalog (16 gates: 5 math, 6 QLoRA, 3 data/eval, 2 forgetting)
  distinct from `kali-audit`'s 8-layer LLM/code defense catalog and
  `supply-chain-sentinel`'s 4-layer manifest catalog.
- Convergence metric must reflect actual coverage, not aspirational:
  gates only count as pass when config evidence confirms them
  (`init_lora_weights=True` = no-op-at-init; `bias='none'` = merge
  equivalence; `lora_alpha/r` or `lora_alpha/sqrt(r)` = scaling form;
  etc.).

## Source References

Anchored to concrete, verifiable literature and documentation (P8).
Full citations with gate anchors live in
[`docs/reference/lora-training-catalog.md`](../../docs/reference/lora-training-catalog.md).

Key sources:

- **LoRA:** arXiv:2106.09685 — anchors G-M1..G-M5.
- **QLoRA:** arXiv:2305.14314 — anchors G-Q1..G-Q6, G-D1..G-D3.
- **rsLoRA:** arXiv:2312.03732 — anchors G-M3 (`α/√r`).
- **DoRA:** arXiv:2402.09353 — anchors G4 method choice.
- **PiSSA:** arXiv:2404.02948 — anchors G4 method choice.
- **LoRA-GA:** arXiv:2407.05000 — anchors G4 method choice.
- **AdaLoRA:** arXiv:2303.10512 — method catalog (not default).
- **Razin et al. (intruder dimensions):** arXiv:2410.21228 — anchors G-F1.
- **AutoPEFT (rejected alternative):** arXiv:2301.12132 — multi-objective
  BO infeasible per training job; deterministic gate used instead.
- **PEFT v0.19.0:** huggingface.co/docs/peft/v0.19.0/package_reference/lora
  — config surface (`LoraConfig` fields, `init_lora_weights` options,
  `use_rslora`, `use_dora`, `use_qalora`, `lora_ga_config`, `corda_config`,
  `loftq_config`, `eva_config`, `alora_invocation_tokens`,
  `reduce_intruder_dimension`).
- **Practitioner consensus (2024-2026):** Raschka, Brenndoerfer, Spheron,
  Databricks, Gradient Flow — anchors G3 rank heuristics (r=16 default,
  sweep {8,16,32,64}), α=2r heuristic, all-linear targets consensus,
  LR-vs-r interaction.
