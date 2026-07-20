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

1. Read the training config: `LoraConfig` (or equivalent YAML/JSON),
   `BitsAndBytesConfig` if present, accelerate config, training script
   arguments (`--learning_rate`, `--num_train_epochs`, `--per_device_train_batch_size`).
2. Extract the five gate inputs:
   - `inference_constraint`: must-merge (no latency) | dynamic-switching |
     either-ok
   - `memory_budget_gb`: available VRAM
   - `model_size_b`: base model parameter count
   - `task_distance`: light (style/format/vocab) | moderate (new domain
     knowledge) | heavy (new reasoning/new language)
   - `quality_vs_cost`: cost-sensitive | quality-critical | default
   - `knowledge_preservation_required`: bool
3. Apply the 5-gate decision (deterministic, not a search):

   ```
   G1. inference_constraint?
       - must-merge → LoRA-family (LoRA | DoRA | PiSSA | CorDA | LoRA-GA)
       - dynamic-switching → Adapters | aLoRA (if invocation-token-gated)
       - either-ok → continue

   G2. memory_budget_gb vs model_size_b?
       - model_size_b × 2 (bf16) > memory_budget_gb → QLoRA (NF4 + LoRA)
       - else → LoRA (bf16, no quantization)

   G3. task_distance?
       - light → r ∈ [8, 16]
       - moderate → r ∈ [32, 64]
       - heavy → r ∈ [64, 128] OR recommend full fine-tune
       - r > 128 → require justification (defeats low-rank premise)

   G4. quality_vs_cost?
       - cost-sensitive, can afford 2× adapter size → DoRA (use_dora=True)
       - quality-critical, want fast convergence, can modify base at init
         → PiSSA (init_lora_weights='pissa') or LoRA-GA (lora_ga_config set)
       - default → LoRA with init_lora_weights=True

   G5. knowledge_preservation_required?
       - true → CorDA Knowledge-Preserved mode (init_lora_weights='corda',
         corda_config.mode='knowledge_preserved')
                OR plan post-training intruder mitigation
                (peft.tuners.lora.intruders.reduce_intruder_dimension)
       - false → standard LoRA
   ```

4. Emit the selected method as a structured `LoraConfig`-shaped JSON:
   `r`, `lora_alpha`, `target_modules`, `lora_dropout`, `bias`,
   `init_lora_weights`, `use_rslora`, `use_dora`, `use_qalora`,
   `lora_ga_config`, `corda_config`, `loftq_config`, `modules_to_save`,
   `quantization_config` (if QLoRA), `optimizer`, `learning_rate`,
   `lr_scheduler_type`, `num_train_epochs`, `justification` (per-gate
   citation).
5. Refuse with a cited reason if:
   - `r >= 0.5 × min(d_in, d_out)` (defeats low-rank premise — LoRA
     paper §4).
   - `model_size_b × 2 > memory_budget_gb` and QLoRA not selected
     (will OOM).
   - `bias='lora_only'` or `bias='all'` and `inference_constraint='must-merge'`
     (breaks merge equivalence — PEFT docs: "even when disabling the
     adapters, the model will not produce the same output as the base
     model would have without adaptation").
   - `init_lora_weights='pissa'` or `'loftq'` or `'olora'` or `'corda'`
     and the training script does not call the corresponding
     preprocessing function (`preprocess_loraga`, `replace_lora_weights_loftq`,
     etc.) — these inits modify base weights and require explicit save
     handling.
6. Emit `cns.lora.select` CNS span (P9) with gate inputs, selected
   method, justification citations, refusal reason (if any),
   `replicant_host`, latency metric.

### lora-training/audit-config

1. Read each file in the training config set. Quote config lines for
   evidence (not synthetic). Record concrete line numbers.
2. For `LoraConfig`: extract `r`, `lora_alpha`, `target_modules`,
   `lora_dropout`, `bias`, `init_lora_weights`, `use_rslora`,
   `use_dora`, `modules_to_save`, `layers_to_transform`,
   `rank_pattern`, `alpha_pattern`, `lora_ga_config`, `corda_config`,
   `loftq_config`, `eva_config`.
3. For `BitsAndBytesConfig` (if QLoRA): extract `load_in_4bit`,
   `bnb_4bit_quant_type` (must be `'nf4'`), `bnb_4bit_compute_dtype`
   (must be `torch.bfloat16` or `torch.float16`), `bnb_4bit_use_double_quant`
   (should be `True`), `llm_int8_skip_modules`.
4. For training script: extract `learning_rate`, `lr_scheduler_type`,
   `num_train_epochs`, `per_device_train_batch_size`,
   `gradient_accumulation_steps`, `optim` (`'paged_adamw_8bit'` if QLoRA),
   `gradient_checkpointing`, `bf16`/`fp16`.
5. Apply the math-contract gates (each is a single assertion with a
   citation):

   **Math/contract gates (from LoRA paper, arXiv:2106.09685):**
   - **G-M1 No-op-at-init invariant:** `init_lora_weights=True` (default)
     produces ΔW=0 at step 0 (B initialized to zero). Test: forward
     pass with adapter enabled == forward pass with adapter disabled at
     step 0. If `init_lora_weights` is `'gaussian'`, `'eva'`, `'olora'`,
     `'pissa'`, `'corda'`, `'orthogonal'`, or `False`, the adapter is
     NOT a no-op at init — flag and require explicit justification.
     Source: LoRA paper §4.1; PEFT v0.19.0 `LoraConfig.init_lora_weights`
     docstring.
   - **G-M2 Merge equivalence:** `W_merged = W + (α/r)·BA` produces
     identical outputs to `W + adapter` within fp tolerance. Test:
     `‖merge(W) − forward_with_adapter(W)‖ < ε`. Broken by `bias='all'`
     or `bias='lora_only'` (PEFT docs explicit warning). Broken by
     `use_dora=True` on PEFT < 0.10 (magnitude component applied
     incorrectly — Spheron 2026 guide). Source: LoRA paper §4.2;
     PEFT v0.19.0 `LoraConfig.bias` docstring.
   - **G-M3 Scaling form:** scaling is `α/r` (or `α/√r` if
     `use_rslora=True`). Never `α`, never `r/α`, never `1`. Test:
     `scaling == lora_alpha / (math.sqrt(r) if use_rslora else r)`.
     Source: LoRA paper §4.1 (`α/r`); rsLoRA paper arXiv:2312.03732
     (`α/√r` for high rank).
   - **G-M4 Rank budget:** `r < min(d_in, d_out)`. Warn if
     `r >= 0.5 × min(d_in, d_out)` (defeats low-rank premise). Refuse
     if `r >= min(d_in, d_out)` (not low-rank at all). Source: LoRA
     paper §4.3 (rank sufficiency experiments).
   - **G-M5 Trainable param count:** `2 · r · (d_in + d_out) · n_target_layers`
     matches `model.print_trainable_parameters()`. Test: assert
     computed count == reported count within 1%. Source: LoRA paper
     Table 5.

   **Memory/quantization gates (from QLoRA paper, arXiv:2305.14314):**
   - **G-Q1 Frozen base is quantized (QLoRA mode only):** assert
     `base_param.dtype` is 4-bit (`torch.uint8` container with
     `bnb_4bit_quant_type='nf4'`) when QLoRA mode is on. Source: QLoRA
     paper §3 (NF4).
   - **G-Q2 Adapter dtype:** LoRA `A`, `B` are bf16 or fp32, never
     4-bit. Test: `lora_A.dtype in (torch.bfloat16, torch.float32)`.
     Source: QLoRA paper §3 (compute in bf16 through frozen base).
   - **G-Q3 Gradient flow:** gradients reach `A` and `B`. Test: after
     first backward, `A.grad is not None and B.grad is not None`.
     Source: QLoRA paper §3 (backprop through frozen 4-bit base).
   - **G-Q4 No silent upcast:** frozen base params are not cast to
     fp32 by `prepare_model_for_kbit_training` or accelerate config.
     Test: assert `base_param.dtype != torch.float32` when QLoRA mode
     is on (silent 2× memory). Source: QLoRA paper §3; PEFT
     `prepare_model_for_kbit_training` docstring.
   - **G-Q5 Paged optimizer (conditional):** `optim='paged_adamw_8bit'`
     only required if peak memory > available VRAM. Gate is conditional,
     not absolute. Source: QLoRA paper §3 (paged optimizers).
   - **G-Q6 NF4 optimality assumption:** NF4 is information-theoretically
     optimal for normally-distributed weights. Warn if base model is
     known to have non-normal weights (e.g., post-quantization-pruned,
     sparse). Source: QLoRA paper §3 (NF4 derivation).

   **Data/eval gates (from QLoRA paper, arXiv:2305.14314):**
   - **G-D1 Dataset size vs quality:** if `n_samples < 1000`, require
     explicit justification. If `n_samples > 100000`, require quality
     audit (dedup, contamination check). Source: QLoRA paper §5 (small
     high-quality > large noisy).
   - **G-D2 Eval protocol:** chatbot benchmarks (Vicuna, MMLU alone)
     are not trustworthy. Require held-out human eval or GPT-4 eval.
     Source: QLoRA paper §5 (GPT-4 eval as cheap proxy).
   - **G-D3 Lemon-pick analysis:** report failure cases, not just
     aggregate score. Source: QLoRA paper §6 (lemon-picked analysis).

   **Forgetting gates (post-QLoRA literature):**
   - **G-F1 Intruder dimension check (post-training):** report intruder
     dimensions before/after training using
     `peft.tuners.lora.intruders.reduce_intruder_dimension`. Source:
     Razin et al. arXiv:2410.21228 ("LoRA vs Full Fine-tuning: An
     Illusion of Equivalence").
   - **G-F2 Knowledge preservation (CorDA mode):** if
     `init_lora_weights='corda'` with `corda_config.mode='knowledge_preserved'`,
     assert world-knowledge eval doesn't regress beyond threshold.
     Source: CorDA paper (PEFT v0.19.0 `LoraConfig.corda_config`
     docstring).

6. Apply pragmatic-semantics (embedded — like `bug-hunt` `oracle` phase):
   - IS vs OUGHT: describe config content (`IS`) vs training contract
     (`OUGHT` — no-op init, merge equivalence, correct scaling).
   - Epistemic mode: `Declarative` (config read), `Probabilistic`
     (memory estimate — only when config provides explicit batch size
     and model size), `Subjunctive` (potential training failure —
     labeled clearly, not presented as fact).
   - Provenance: `Direct measurement` (read config), `Inference`
     (param count math), `Assessment` (contract violation mapping) —
     label each finding explicitly.
7. Apply grill-me self-challenge: Could this config choice be
   intentional? Would a reviewer dismiss? If yes, downgrade or omit.
   Only propose concrete findings with quoted config evidence.
8. Apply pragmatic-cybernetics analysis (feedback loops): trace
   scaling×rank interaction (does higher r need lower LR? — yes per
   Brenndoerfer 2024), check variety (alternative init strategy
   available?), Good Regulator (is init strategy matched to training
   goal?).
9. For each config entry, produce structured finding:
   `parameter`, `value`, `source_line`, `config_path`, `gate_id`
   (e.g., `G-M3`), `gate_result` (pass | warn | fail | refuse),
   `severity` (critical/high/medium/low — justified by evidence),
   `provenance`, `epistemic_mode`, `evidence_snippet` (quoted config
   line + file path), `source_citation` (arXiv paper section, PEFT
   docs section), `remediation` (concrete fix), `replicant_host`.
10. Emit `cns.lora.audit` CNS span per gate evaluated
    (`target: "cns.lora.audit"`, message: `"CNS"`, operation:
    `"audit_gate"`, gate_id, parameter, value, gate_result,
    replicant_host, latency_ms).

CONSTRAINT — Evidence integrity (P8):
- No synthetic config quotes. Every `evidence_snippet` verifiable by
  reading cited config file at cited line.
- No synthetic benchmark numbers. Only reference paper-reported results
  with explicit citation (paper, table/section).
- Source citations must reference concrete URLs or documents actually
  consulted: arXiv paper URLs (abs/2106.09685, abs/2305.14314,
  abs/2312.03732, abs/2402.09353, abs/2404.02948, abs/2407.05000,
  abs/2410.21228), PEFT v0.19.0 docs
  (huggingface.co/docs/peft/v0.19.0/package_reference/lora).
- Every finding must include `replicant_host` identity (P12) — no
  anonymous config auditing.
- When referencing `security/regressions/`, read actual YAML files; do
  not invent regression entries. Only propose new entries when concrete
  evidence supports them.
- This skill complements `kali-audit` (security surface) and `tdd`
  (training-loop code) by governing training configuration and
  contracts. State relationship explicitly in reports.
- Minimal (P5): 4 templates (`select-method`, `audit-config`, `report`,
  `convergence-check`), no bundle, no sub-agent delegation, no actual
  training execution. Each template answers specific 5W1H: select
  (What + Why), audit (How), report (Why + What), convergence (When +
  Why).

### lora-training/report

1. Synthesize `findings` array from `audit-config` phase. Group by
   severity:
   - critical (refuse gates: `r >= min(d_in, d_out)`, OOM-certain
     memory mismatch, merge-equivalence break with must-merge
     constraint, init strategy without preprocessing call)
   - high (fail gates: wrong scaling form, silent fp32 upcast, no
     gradient flow, no-op-at-init break without justification)
   - medium (warn gates: `r > 128`, NF4 on non-normal weights, no
     lemon-pick analysis, no intruder check)
   - low (informational: suboptimal LR for rank, missing
     `modules_to_save` for embeddings)
2. For each finding: include `parameter`, `config_path`, `line`,
   `evidence_snippet`, `severity`, `gate_id`, `source_citation`
   (arXiv paper section + PEFT docs section), `remediation`
   (citing concrete fix pattern: set `use_rslora=True`, switch to
   `paged_adamw_8bit`, add `preprocess_loraga()` call, etc.),
   `replicant_host`.
3. Propose regression entry for findings with severity >= medium (only
   when evidence is concrete — no synthetic findings). Use exact YAML
   format from `security/regressions/README.md`:
   `surface: training`, `gate: G-M3` (or G-Q1, G-F1, etc.),
   `discovered_in: <config_path>`, `status: pending`,
   `detection: kind: grep` (with concrete pattern against config
   content) or `detection: kind: runtime-assert` (with Python
   assertion code). Each proposal must include concrete `pattern`
   referencing config content (e.g., regex for `bias:\s*['"]?(all|lora_only)`
   combined with `inference_constraint:\s*['"]?must-merge`) — not
   vague description.
4. Identify contract gaps (e.g., missing no-op-at-init test, missing
   merge-equivalence test, missing intruder-dimension check). Propose
   top 3 highest-priority fixes based on severity.
5. Produce verdict:
   - Pass: zero critical/high findings, all math-contract gates pass,
     all QLoRA-specific gates pass (if QLoRA mode).
   - Conditional: medium findings present or 1-2 gates warn.
   - Fail: critical/high findings present or any gate refuses.
6. Emit `cns.lora.report` CNS span with findings count by severity,
   gate results summary, proposed regression count, replicant host,
   verdict, latency metric.

### lora-training/convergence-check

1. Compute normalized convergence metric [0, 1] where 0 = fully
   converged (training-ready).
2. Score dimensions (weighted):
   - Critical + high findings resolved (0.40): 0 critical/high = +0.00;
     1+ critical/high unresolved = +0.40; partial resolution =
     proportional.
   - Math-contract gate coverage (0.25): all 5 math gates (G-M1..G-M5)
     pass = +0.00; 4 pass = +0.05; 3 pass = +0.10; 2 pass = +0.15;
     <2 pass = +0.25.
   - QLoRA gate coverage (0.15, only if QLoRA mode): all 6 QLoRA gates
     (G-Q1..G-Q6) pass = +0.00; 5 pass = +0.03; 4 pass = +0.06; <4
     pass = +0.15. (If not QLoRA mode, this dimension is 0.00.)
   - Data/eval gate coverage (0.10): G-D1, G-D2, G-D3 all pass = +0.00;
     2 pass = +0.03; 1 pass = +0.06; 0 pass = +0.10.
   - Forgetting gate coverage (0.10): G-F1 intruder check planned =
     +0.00; not planned = +0.05. G-F2 knowledge-preservation assert
     planned (if CorDA mode) = +0.00; not planned = +0.05.
3. Start at 0.00, add contributions, clamp to [0, 1].
4. Converged: metric ≤ 0.10 AND relative improvement ≥ 5% from previous
   cycle. If metric has not improved by ≥5%, identify blocker (unfixed
   finding, missing gate, evidence gap).
5. Return JSON: `{convergence_metric, dimensions, rationale, blockers,
   gate_results, proposed_regressions}`.
6. Emit `cns.lora.convergence` CNS span (registered in
   `CANONICAL_NAMESPACES` — `crates/hkask-types/src/event.rs`).

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
