---
title: "HHH Mode — Open Design Questions"
date: 2026-06-04
version: "2.0.0"
status: "Pre-Implementation"
audience: [architects, developers]
related:
  - docs/research/hhh-alignment-research.md
---

# HHH Mode — Open Design Questions

Questions that must be resolved before implementation begins. The architecture is now settled: **HHH is a toggle on the existing inference flow, not a new pipeline.** The remaining questions are about the details of the insertion points.

---

## Blockers — Must Decide Before Writing Code

### B1. How Does `chat_with_agent` Accept HHH-Augmented Prompts?

`chat_with_agent` currently takes `input: &str` and constructs the system prompt internally. When HHH is active, we need to:
- Pass the reframed input instead of the raw input
- Append HHH directives to the system prompt

**Options:**

| Option | How | Pros | Cons |
|---|---|---|---|
| **A. New parameters** | Add `hhh_mode: Option<HhhMode>` and `hhh_config: Option<HhhConfig>` params to `chat_with_agent` | Clean, explicit, no hidden behavior | Adds params to an already-long function signature |
| **B. Wrap input externally** | Transform `input` and system prompt in the REPL turn before calling `chat_with_agent`, pass the already-transformed strings | No changes to `chat_with_agent` | `chat_with_agent` still constructs its own system prompt internally — we'd need to pass a pre-built system prompt instead |

**Recommendation:** Option B, but it requires a small change to `chat_with_agent`: add an optional `system_prompt_override: Option<String>` parameter. When HHH is active, the REPL constructs the full system prompt (agent prompt + HHH directives) and passes it as the override. When HHH is off, `None` is passed and the existing behavior is unchanged. This keeps `chat_with_agent` generic — it doesn't know about HHH, it just accepts a system prompt override.

**Impact:** `chat_with_agent` signature changes from 8 params to 9 (adding `system_prompt_override`). The function body already has `let mut system_prompt = match agent { ... }` at line 156 — the override replaces this.

---

### B2. Tool Calls and the Evaluation Gate

When the generation model produces tool calls, the HHH gate should evaluate the response *after* tool processing (Option B from v1 of this doc). The flow becomes:

```
User Input → [HHH Reframe] → LLM → Tool Processing → [HHH Gate] → (Pass) → User
                                                            → (Fail) → [Correction without tools] → [Gate] → ...
```

Correction iterations skip tool processing — the correction prompt asks the model to revise the text response, not to re-invoke tools.

**Question:** Should the gate evaluate `ProcessedResponse.text` (text with tool calls stripped) or the raw model response including tool directives?

**Recommendation:** Evaluate `ProcessedResponse.text`. This is what the user actually sees, and it's the right target for HHH compliance. Tool call arguments (like search queries) that contain sycophantic framing would be caught by the gate since they appear in the text portion.

---

### B3. Gate Inference Port Lifecycle

The REPL creates one `InferencePort` at startup. The gate needs a second one for `qwen3.5:397b-cloud`.

**Recommendation:** Eager creation at REPL init. Add `gate_inference_port: Arc<dyn InferencePort>` to `ReplState`. Created alongside the main port using `OkapiInference::new("qwen3.5:397b-cloud", okapi_config.clone())`. The `OkapiConfig` is already available at init time. If the gate model is unavailable, print a warning and fall back to skipping the gate (same as gas exhaustion).

---

### B4. JSON Parsing Robustness

The gate model may produce malformed JSON, wrap it in markdown fences, or add conversational text.

**Recommendation:** Three-layer parse:
1. Strict `serde_json::from_str`
2. Lenient extraction of first `{...}` block
3. Strip markdown fences, then extract `{...}` again
4. Fallback: `overall_pass: true` + `tracing::warn!(target: "cns.hhh.gate", ...)`

This ensures the gate never blocks all responses due to a parse failure.

---

### B5. Gas Accounting for Gate Calls

Each HHH iteration adds 1-2 inference calls (evaluation + correction). These share the session's gas budget.

**Recommendation:** Use the existing hold-settle pattern. Each gate call goes through `CyberneticsLoop.reserve_gas()` → `InferencePort.generate_with_model()` → `CyberneticsLoop.settle_gas()` with actual token counts from `InferenceResult.usage`. If gas runs out mid-iteration, return the last available response with the uncertainty marker and log `cns.hhh.gas_exhausted`.

---

## Important — Should Decide Before Phase 2

### I1. Gate Model Parameters

| Parameter | Generation | Gate | Rationale |
|---|---|---|---|
| `temperature` | 0.7 | 0.1 | Near-deterministic evaluation |
| `top_p` | 0.9 | 0.95 | Ensure JSON completion |
| `top_k` | 40 | 5 | Narrow selection |
| `max_tokens` | 512 | 512 | Enough for JSON + prose |
| `seed` | None | 42 | Deterministic |

Correction prompts use **generation** parameters (0.7 temperature) since they produce creative responses.

### I2. Session History and Episodic Memory

Session history (`ReplState.session_history`) stores only the corrected response. Episodic memory stores the full audit trail (original, evaluation, correction, final) under type `hhh_chat_turn`.

### I3. Latency and UX

Progress indicators during HHH evaluation:

```
  ℏKask [Curator]> What is the capital of France?
  [HHH] Generating response...
  [HHH] Evaluating response for HHH compliance...
  [HHH] ✓ Passed (iteration 1)
  
  The capital of France is Paris.
```

Or for corrections:

```
  [HHH] ✗ Failed: honesty_uncertainty (0), helpfulness (1)
  [HHH] Correcting (iteration 2)...
  [HHH] ✓ Passed (iteration 2)
```

---

## Deferrable

- **Prompt decomposition integration** (Phase 3: use `decompose_prompt` to detect sycophancy triggers)
- **API exposure** (Phase 4: `POST /api/v1/hhh/toggle`)
- **Cross-agent evaluation in ensemble** (Phase 4)
- **Confidence-verb integration** (Phase 3: require confidence prefixes in responses)
- **Streaming responses** (future: buffer full response for now)
- **Per-agent HHH mode** (Phase 4: per-session is sufficient for Phase 1)

---

*See `hhh-alignment-research.md` (v2.0.0) for the full research paper.*