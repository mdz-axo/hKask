---
title: "Fusion Mode — Multi-Model Deliberation"
audience: [operators, developers, users]
last_updated: 2026-07-16
version: "0.31.0"
status: "Active"
domain: "Inference"
mds_categories: [domain, composition, trust]
---

# Fusion Mode — Multi-Model Deliberation

Configure and operate hKask's provider-agnostic fusion engine: a panel of models answers in parallel, then a judge model synthesizes, picks, critiques, deliberates, or plans according to one of five deliberation modes. Fusion is **opt-in and disabled by default** — it activates only when you explicitly configure a judge and panel.

This guide covers global env-var configuration, per-skill manifest overrides, the five deliberation modes, skill anchoring, bypass semantics, and operational checks. All facts are verified against the implementation in `crates/hkask-inference/src/fusion_orchestrator.rs`, `crates/hkask-types/src/fusion.rs`, `crates/hkask-inference/src/config.rs`, and `crates/hkask-templates/src/executor.rs`.

---

## What Fusion Is

Fusion is a **hKask-side orchestration engine**, not a provider feature. hKask itself:

1. Sends the user's prompt to every panel model **in parallel** (each routing through its own provider via a 2-letter prefix).
2. Collects all panel responses.
3. Dispatches to a **judge** model operating in the configured deliberation mode.

This is distinct from the OpenRouter `FusionPlugin` (`crates/hkask-inference/src/chat_protocol.rs`), which injects a plugin into the OpenRouter request body. The hKask orchestrator is provider-agnostic and works across DeepInfra, fal.ai, Together, OpenRouter, KiloCode, and Cline.

The judge can be an **LLM** operating in one of five deliberation modes (synthesis, best-of-n, critique, deliberation, pi), or the **algorithm** `"algo"` — a deterministic JSON merge that makes no LLM call. The `algo` judge runs the panel in parallel and merges the JSON responses via a recursive union, preserving both viewpoints without applying a methodology lens. See [The Algo Judge — Algorithmic Merge](#the-algo-judge--algorithmic-merge) below.

**Why fusion exists:** Multi-model deliberation improves answer quality on hard reasoning tasks by combining diverse model perspectives under a methodologically-anchored judge. It is a quality/cost tradeoff — fusion multiplies token cost by roughly (panel size + judge calls × rounds). The `algo` judge is the exception: zero token cost, zero latency, since it skips the LLM judge call entirely.

---

## Enable Fusion Globally

Fusion is off by default. Activate it by setting two required environment variables:

| Env var | Required | Purpose |
|---------|----------|---------|
| `HKASK_FUSION_JUDGE_MODEL` | yes | Judge/fuser model. Supports provider prefix. |
| `HKASK_FUSION_PANEL_MODELS` | yes | Comma-separated panel models, 1–8. Each supports provider prefix. |
| `HKASK_FUSION_MODE` | no | Deliberation mode. One of `synthesis`, `best-of-n`, `critique`, `deliberation`, `pi`. Default: `synthesis`. |
| `HKASK_FUSION_SKILLS` | no | Comma-separated skill anchors for the judge. Default: none. |
| `HKASK_FUSION_MAX_ROUNDS` | no | Max rounds for `deliberation` mode. Default: `5`. |
| `HKASK_FUSION_DISABLED=1` | no | Force-disable fusion (overrides all other vars). |

Minimal activation (uses kask defaults for judge/panel when unset, but explicit is recommended):

```bash
export HKASK_FUSION_JUDGE_MODEL=deepseek-v4-pro
export HKASK_FUSION_PANEL_MODELS=Kimi2.7,Qwen3.7 Max,GLM5.2,Minimax3
```

With an explicit deliberation mode and skill anchor:

```bash
export HKASK_FUSION_JUDGE_MODEL=DI/deepseek-v4-pro
export HKASK_FUSION_PANEL_MODELS=OR/auto,KC/anthropic/claude-sonnet-4.5,DI/qwen/qwen3
export HKASK_FUSION_MODE=critique
export HKASK_FUSION_SKILLS=pragmatic-semantics
```

**Default model set** (when env vars are unset but fusion is enabled via `FusionConfig::kask_default()`):

| Role | Model |
|------|-------|
| Judge | `deepseek-v4-pro` |
| Panel | `Kimi2.7`, `Qwen3.7 Max`, `GLM5.2`, `Minimax3` |

---

## Provider Prefix Routing

Every model name — judge or panelist — may carry a 2-letter provider prefix. Unprefixed names use the configured default provider.

| Prefix | Provider |
|--------|----------|
| `DI/` | DeepInfra |
| `FA/` | fal.ai |
| `TG/` | Together AI |
| `OR/` | OpenRouter |
| `KC/` | KiloCode |
| `CL/` | Cline |

Mixed-provider panels are supported and encouraged for epistemic diversity. Example: a DeepInfra judge with an OpenRouter auto-routed panelist, a KiloCode Claude, and a DeepInfra Qwen:

```bash
HKASK_FUSION_JUDGE_MODEL=DI/deepseek-v4-pro
HKASK_FUSION_PANEL_MODELS=OR/auto,KC/anthropic/claude-sonnet-4.5,DI/qwen/qwen3
```

If a panel model fails to resolve or generate, the orchestrator logs a warning at `cns.inference` and drops it from the round. If **all** panel models fail, the orchestrator returns `InferenceError::Generation("All panel models failed")`.

---

## Choose a Deliberation Mode

The judge operates in one of five modes, set via `HKASK_FUSION_MODE` or the `mode:` field in a manifest's `fusion:` block.

> **When `judge: "algo"`, the mode is ignored** — the algorithmic merge has its own logic. The `skills` and `max_rounds` fields are also ignored.

### `synthesis` (default) — 1 round

The judge composes a unified response incorporating the strongest elements from each panelist and explicitly resolves contradictions. Use this as the general-purpose default when you want a merged answer rather than a picked one.

### `best-of-n` — 1 round

The judge evaluates all panel responses and outputs **only the chosen response verbatim** — no commentary, no synthesis, no justification. Use this when you want one panelist's answer, not a merge. Lowest judge token cost.

### `critique` — 2 rounds

1. **Round 1:** Judge produces a draft synthesis from panel responses.
2. **Round 2:** Panel critiques the draft (weaknesses, gaps, contradictions); judge revises into a final synthesis.

Use this for tasks where a draft-then-refine loop improves quality — design reviews, reasoning chains, documentation. Matches the diagnosis loop and the essentialist 3-gate pattern.

### `deliberation` — ≤ N rounds (default 5)

Multi-round with a convergence check. Each round, the judge either:

- Emits a final synthesis (if panel responses have converged), **or**
- Emits a line prefixed `FOLLOW_UP:` containing a follow-up question for the panel.

The orchestrator re-dispatches the follow-up to the panel and loops. If `max_rounds` is reached without convergence, the judge is forced to synthesize a final response from the last round. Configure the cap with `HKASK_FUSION_MAX_ROUNDS` (default `5`). Use this for hard problems where iterative refinement surfaces information a single pass misses.

### `pi` (Plan-Implement) — 2 phases

1. **Phase 1 (Plan):** Panel proposes high-level strategies (architecture, key decisions, tradeoffs — no implementation details). Judge synthesizes a unified strategy plan.
2. **Phase 2 (Implement):** The strategy plan is sent back to the panel, which proposes concrete implementation steps (files, functions, tests, sequencing). Judge synthesizes a unified implementation plan.

Use this for engineering tasks where strategy and execution should be separated — refactors, feature design, architectural work. Matches the `refactor-service-layer` and `improve-codebase-architecture` skills.

---

## The Algo Judge — Algorithmic Merge

`judge: algo` is a special judge value that runs a **deterministic JSON merge** instead of an LLM call. Zero token cost, zero latency, and it preserves both viewpoints rather than picking one or synthesizing a third.

**What it does.** The orchestrator dispatches the panel in parallel — exactly as in the LLM-judge modes — then merges the panelists' JSON responses via `merge_json_values` (a recursive union):

- **Objects** merge by key (recursively).
- **Arrays** concatenate with case-insensitive deduplication.
- **Diverging strings** are annotated as `[A:... B:...]` so both values survive.

**When to use.** Epistemic-integrity tasks where both models' outputs should be preserved rather than judged — e.g. classification steps where agreement and disagreement are both signal. The `algo` judge replaces the former `dual_model: true` step flag and the `DualModelPort` mechanism.

Minimal config:

```yaml
fusion:
  judge: algo
  panel:
    - KC/qwen/qwen3-235b-a22b-2507
    - DI/google/gemma-4-31b-it-turbo
```

**Limitations.**

- Designed for a **2-model** merge — annotation quality degrades with 3+ panelists.
- Requires **JSON** panel responses; non-JSON output falls back to `null`.
- **No skill anchoring** — use a judge-based mode (`synthesis`, `critique`, etc.) when you need methodology-anchored evaluation.

---

## Anchor the Judge with Skills

The judge can be anchored on hKask's pragmatic methodologies via `HKASK_FUSION_SKILLS` (comma-separated). Each anchor injects a compact methodology prompt into the judge's system context, steering its reasoning without forcing a rigid template.

| Skill anchor | Methodology injected |
|--------------|---------------------|
| `pragmatic-semantics` | IS vs OUGHT, certainty levels, provenance, constraint hierarchy |
| `pragmatic-cybernetics` | Feedback loops, variety engineering, homeostasis |
| `pragmatic-laziness` | Path of least action, delete before adding |
| `coding-guidelines` | Karpathy's 4 principles: think first, simplicity, surgical changes, goal-driven |
| `deep-module` | Ousterhout deletion test, interface minimalism (≤7 public items) |
| `essentialist` | 3-gate challenge loop: Exist → Surface → Contract |
| `superforecasting` | Fermi decomposition, Bayesian updating, dragonfly-eye synthesis, calibrated probabilities |
| `mcda` | Weighted scoring, compensation masking, sensitivity analysis |
| `tdd` | Red-Green-Refactor, contract-first, vertical tracer-bullet |

Example: a judge anchored on cybernetics and essentialism:

```bash
HKASK_FUSION_SKILLS=pragmatic-cybernetics,essentialist
```

The full methodology text for each skill is defined in `skill_prompt()` in `crates/hkask-inference/src/fusion_orchestrator.rs`. To add a new anchor, extend the `FusionSkill` enum in `crates/hkask-types/src/fusion.rs` and add a matching arm in `skill_prompt()`.

> **Note:** `hypothesis-framer` and `idiomatic-rust` appear in the skill catalog but are not yet `FusionSkill` variants. Adding them requires extending the enum and the `skill_prompt` match.

---

## Configure Per-Skill Fusion (Manifests)

A skill's flow manifest can declare its own `FusionConfig` via a `fusion:` block, overriding the global env-var config for all steps in that skill's pipeline. This lets each skill pick its own judge, panel, mode, skill anchors, and round cap without touching other skills.

```yaml
# registry/manifests/superforecasting.yaml
fusion:
  judge: deepseek-v4-pro
  panel:
    - Kimi2.7
    - Qwen3.7 Max
    - GLM5.2
    - Minimax3
  mode: synthesis
  skills:
    - superforecasting
  max_rounds: 5
```

The full `FusionConfig` shape (5 fields, YAML-clean):

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `judge` | string | required | Judge model; supports provider prefix. |
| `panel` | string[] | required | 1–8 panel models; each supports provider prefix. |
| `mode` | `synthesis` \| `best-of-n` \| `critique` \| `deliberation` \| `pi` | `synthesis` | Judge deliberation mode. |
| `skills` | string[] | `[]` | Skill anchors to inject into the judge. |
| `max_rounds` | u32 | `5` | Cap for `deliberation` mode. |

### Resolution Priority

When a step runs, fusion config is resolved in this order (highest priority first):

1. `step.fusion: Some(false)` → **bypass fusion** (single-model inference). Used for deterministic rubric evaluation and convergence checks.
2. `step.fusion: Some(true)` or `None` → inherit the manifest config.
3. `manifest.fusion: Some(config)` → per-manifest config (carried via `LLMParameters.fusion_config`).
4. `manifest.fusion: None` → global config (`HKASK_FUSION_*` env vars).
5. `params.bypass_fusion: true` → **bypass everything** (chat path, condenser, daemon narratives, summarization).

### Per-Step Bypass

Set `fusion: false` on individual steps to keep them single-model while the rest of the manifest uses fusion. This is the standard pattern for convergence checks and quality gates, which must be deterministic:

```yaml
steps:
  - id: converge
    kind: select
    fusion: false        # deterministic single-model rubric
    template: converge.j2
```

The implementation lives in `execute_select()` in `crates/hkask-templates/src/executor.rs`.

---

## Where Fusion Applies (and Doesn't)

Fusion routing is decided per inference call by `bypass_fusion` on `LLMParameters`:

| Call path | Routes through fusion? | Why |
|-----------|------------------------|-----|
| Skill `select` steps (fusion active) | ✅ yes | Skills benefit from multi-model deliberation |
| Skill tool invocations | ✅ yes | Same as above |
| `kask chat` interactive chat | ❌ no | `bypass_fusion = true` — user's chosen model is used directly |
| `kask api` chat stream | ❌ no | `bypass_fusion = true` |
| Condenser / summarization | ❌ no | Always bypass — cost/latency sensitive |
| Daemon narratives | ❌ no | Always bypass |
| Algo-judge steps (`fusion: true`, `judge: algo`) | ✅ yes | The `algo` judge is a fusion path — it dispatches the panel in parallel and merges JSON via `merge_json_values`, replacing the former dual-model mechanism |

Chat intentionally bypasses fusion so the user's explicitly-chosen model answers directly, while skills (which run autonomously) route through the fusion panel for higher quality.

---

## Operate Fusion

### REPL commands

Inside the REPL (`kask repl` or the TUI):

| Command | Effect |
|---------|--------|
| `/fusion` | Print fusion status (active/inactive, judge, panel, mode) |
| `/fusion on` | Activate fusion for the session |
| `/fusion off` | Deactivate fusion for the session |

### Startup banner

When fusion is configured and the judge model is set, `kask` prints a banner on startup:

```
  ⚡ Fusion mode active — model: deepseek-v4-pro
     4 panel models judged by deepseek-v4-pro (mode: synthesis)
```

This is emitted by `check_fusion_startup()` in `crates/hkask-cli/src/main.rs` — a P9 proactive cost-safety check so you never accidentally run fusion with an unintended model.

### Doctor check

`kask doctor` verifies the fusion judge is reachable:

```
Fusion Model
────────────
  ✅ Fusion judge reachable — deepseek-v4-pro
```

If the judge is unreachable, doctor reports `❌ Fusion judge NOT reachable` or `⚠️ Could not verify fusion model: <error>`. Fix connectivity or credentials before relying on fusion.

### Diagnostics

Fusion emits tracing spans at `target: "cns.inference"`:

- `Fusion orchestration starting` — mode, judge, panel count, skill count
- `Critique round 1 complete` / `Critique round 2` — critique mode round boundaries
- `Judge requested follow-up` / `Deliberation converged` — deliberation mode round decisions
- `P-I Phase 1 complete — strategy synthesized` — plan-implement phase boundary
- `Panel model resolution failed` / `Panel model generation failed` — per-panelist failures (warnings, not fatal unless all fail)

Filter with `RUST_LOG=cns.inference=info` to watch fusion in action.

---

## Disambiguation: Fusion vs. OpenRouter FusionPlugin

hKask has two distinct multi-model mechanisms. They are **orthogonal** and never combine on the same call:

| Mechanism | Crate | Purpose | Combines with fusion? |
|-----------|-------|---------|----------------------|
| **Fusion** (this guide) | `hkask-inference::fusion_orchestrator` | Panel → judge deliberation for quality | — |
| **OpenRouter FusionPlugin** | `hkask-inference::chat_protocol` | OpenRouter-side plugin injected into the request body | Separate path; the hKask orchestrator does not use it |

> The former `dual_model: true` step flag, `DualModelPort`, and the dual classifier module (`dual_classify.rs` with Jaccard scoring, divergence detection, drift detection) have all been removed. The algo fusion judge (`judge: "algo"`) replaces them. The corpus pipeline routes through the same fusion orchestrator — panel models in the corpus config's `fusion:` block, merged via `algo_merge()`.

---

## Reference: Crates and Types

| Artifact | Location |
|----------|----------|
| `FusionConfig`, `FusionMode`, `FusionSkill` types | `crates/hkask-types/src/fusion.rs` |
| Env-var parser (`parse_fusion_config`) | `crates/hkask-inference/src/config.rs` |
| Orchestrator entry (`orchestrate`), 5 mode implementations, and `ALGO_JUDGE` constant | `crates/hkask-inference/src/fusion_orchestrator.rs` |
| Router fusion override (`effective_model`, `orchestrate_fusion`) | `crates/hkask-inference/src/inference_router/` |
| Per-manifest `fusion:` block (`BundleManifest.fusion`, `BundleManifestStep.fusion`) | `crates/hkask-templates/src/bundle/manifest.rs` |
| Per-step resolution logic | `crates/hkask-templates/src/executor.rs` (`execute_select`) |
| `LLMParameters.fusion_config` carrier | `crates/hkask-types/src/template.rs` |
| OpenRouter `FusionPlugin` (separate) | `crates/hkask-inference/src/chat_protocol.rs` |

Types live in `hkask-types` (not `hkask-inference`) so manifests and `LLMParameters` can carry fusion config without a dependency on the inference crate.

---

## See Also

- [Architecture: Fusion — Multi-Model Deliberation](../architecture/hKask-architecture-master.md#fusion--multi-model-deliberation) — canonical architecture reference
- [Cognition and Replica: Fusion System Design Recommendations](../explanation/cognition-and-replica.md) — design rationale (why no partial inheritance, why no `fusion_mode` shorthand)
- [Install and Configure](install-and-configure.md) — env-var setup including `HKASK_FUSION_DISABLED`
- [Skills and Composition](skills-and-composition.md) — manifest authoring and the `fusion:` block in context