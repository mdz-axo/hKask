---
title: "hkask-inference — API Reference"
audience: [developers]
last_updated: 2026-07-09
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-inference — API Reference

Multi-provider inference router. Routes LLM requests to DeepInfra, fal.ai, Together AI, OpenRouter, KiloCode, RunPod, or Baseten based on a 2-letter provider prefix in the model name. Also provides embedding routing and multi-model fusion orchestration.

## Public Modules

| Module | Description |
|---|---|
| `chat_protocol` | Chat protocol types. Re-exports: `FusionPlugin` |
| `config` | Configuration types: `InferenceConfig`, `ProviderConfig`, `ProviderId`, `FusionConfig`, `FusionMode`, `FusionSkill` |
| `deepinfra_backend` | DeepInfra provider backend (`DI/` prefix → `api.deepinfra.com`) |
| `embedding_router` | Embedding routing. Type: `EmbeddingRouter` |
| `fal_backend` | fal.ai provider backend (`FA/` prefix → `api.fal.ai`) |
| `fusion_orchestrator` | Multi-model fusion orchestration: `orchestrate()` |
| `inference_router` | Inference routing. Type: `InferenceRouter` |
| `kilocode_backend` | KiloCode provider backend (`KC/` prefix → `api.kilo.ai/api/gateway`) |
| `runpod_backend` | RunPod serverless backend (`RP/` prefix, requires `RUNPOD_TEMPLATE_ID` or `RUNPOD_BASE_URL`) |
| `baseten_backend` | Baseten serverless backend (`BT/` prefix, requires `BASETEN_MODEL_ID` or `BASETEN_BASE_URL`) |
| `model_constants` | Model name constants |
| `openai_backend` | OpenAI provider backend |
| `openrouter_backend` | OpenRouter provider backend (`OR/` prefix → `openrouter.ai/api`) |
| `together_backend` | Together AI provider backend (`TG/` prefix → `api.together.xyz`) |

## Architecture

```
InferenceRouter (implements InferencePort)
  ├── DeepInfraBackend    — DI/ prefix → api.deepinfra.com
  ├── FalBackend          — FA/ prefix → api.fal.ai
  ├── TogetherBackend     — TG/ prefix → api.together.xyz
  ├── OpenRouterBackend   — OR/ prefix → openrouter.ai/api
  ├── KiloCodeBackend     — KC/ prefix → api.kilo.ai/api/gateway
  ├── RunpodBackend       — RP/ prefix → serverless (RUNPOD_TEMPLATE_ID)
  └── BasetenBackend      — BT/ prefix → serverless (BASETEN_MODEL_ID)

EmbeddingRouter
  ├── DeepInfraEmbedding  — DI/ prefix → /v1/embeddings
  └── OpenRouterEmbedding — OR/ prefix → /v1/embeddings
```

No prefix defaults to the configured default provider (default: DeepInfra).

### Vision / Multimodal Format

All backends use the OpenAI-standard multimodal content-array format via `build_vision_request()`:

```json
{"messages": [{"role": "user", "content": [
  {"type": "image_url", "image_url": {"url": "data:image/png;base64,..."}},
  {"type": "text", "text": "Extract all text from this image."}
]}]}
```

The legacy `ChatMessage.images` field was removed in v0.31.0. All vision dispatch goes through `build_vision_request()`.

### RunPod / Baseten Serverless

RunPod (`RP/`) and Baseten (`BT/`) backends require serverless endpoint provisioning:

| Env Var | Purpose |
|---------|--------|
| `RUNPOD_API_KEY` / `BASETEN_API_KEY` | API key |
| `RUNPOD_TEMPLATE_ID` → `https://api.runpod.ai/v2/{tid}/openai/v1` | Template endpoint |
| `BASETEN_MODEL_ID` → `https://model-{mid}.api.baseten.co/production/predict` | Model endpoint |
| `HKASK_OCR_MODEL=RP/allenai/olmocr-2-7b-1025` | OCR model routing |
| `HKASK_OCR_CONCURRENCY=4` | Parallel page processing (default: 4) |

Backends return empty model lists — model discovery is via template configuration, not API listing.

## Key Public Types

### `ProviderId`

Enum identifying an inference provider.

**Variants:** `DeepInfra`, `Fal`, `Together`, `Runpod`, `Baseten`, `OpenRouter`, `KiloCode`.

**Methods:**
- `parse_from_model(model: &str) -> Option<Self>` — extract provider from `"XX/model-name"` prefix
- `prefix_model(&self, model_id: &str) -> String` — prepend provider prefix to model name
- `as_str(&self) -> &'static str` — provider code string

### `InferenceConfig`

Configuration for inference routing.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `default_provider` | `ProviderId` | Default provider when no prefix |
| `deepinfra_base_url` | `String` | DeepInfra API base URL |
| `deepinfra_api_key` | `String` | DeepInfra API key |
| `fal_base_url` | `String` | fal.ai API base URL |
| `fal_media_base_url` | `String` | fal.ai media URL |
| `fal_queue_base_url` | `String` | fal.ai queue URL |
| `fal_api_key` | `String` | fal.ai API key |
| `together_base_url` | `String` | Together AI base URL |
| `together_api_key` | `String` | Together AI API key |
| `openrouter_base_url` | `String` | OpenRouter base URL |
| `openrouter_api_key` | `String` | OpenRouter API key |
| `openrouter_max_prompt_price_per_m` | `f64` | Max prompt price per million tokens |
| `openrouter_min_intelligence_index` | `f64` | Minimum intelligence index for routing |
| `kilocode_base_url` | `String` | KiloCode API base URL |
| `kilocode_api_key` | `String` | KiloCode API key |
| `timeout_secs` | `u64` | Request timeout in seconds |
| `pool_max_idle` | `usize` | Maximum idle connections in pool |
| `default_model` | `String` | Default model ID |
| `fusion` | `FusionConfig` | Fusion orchestration config |

**Methods:** `from_env()` (load from environment variables), `build_client()`, `deepinfra_config()`, `together_config()`, `openrouter_config()`, `kilocode_config()`. Implements `Default`.

### `ProviderConfig`

A single provider's connection configuration.

**Fields:** `base_url: String`, `api_key: String`.

**Methods:** `from_env(provider: &ProviderId) -> Self`, `is_configured() -> bool`.

### `FusionConfig`

Configuration for multi-model fusion orchestration.

**Fields:** `judge: String`, `panel: Vec<String>`, `mode: FusionMode`, `skills: Vec<FusionSkill>`, `max_rounds: usize`.

**Methods:** `kask_default() -> Self`, `model_id()`, `description()`.

### `FusionMode`

Fusion deliberation mode.

**Variants:**
| Variant | Description |
|---|---|
| `BestOfN` | Judge picks the single best response from the panel |
| `Synthesis` | Judge composes a unified response from all panelists |
| `Critique` | 2-round: draft → panel critique → revised final |
| `Deliberation` | Multi-round with convergence check |
| `PlanImplement` | 2-phase: strategy plan → implementation plan |

**Methods:** `as_str()` — returns lowercase string. Implements `FromStr`.

### `FusionSkill`

Skill anchors for the judge's reasoning in fusion modes. Each variant activates a specific hKask methodology prompt.

**Variants:** `PragmaticSemantics`, `PragmaticCybernetics`, `PragmaticLaziness`, `CodingGuidelines`, `DeepModule`, `Essentialist`, `Superforecasting`, `MCDA`, `TestDrivenDevelopment`, `RustExpertise`.

Implements `FromStr`.

### `InferenceRouter`

Multi-provider inference router. Implements `InferencePort`. Routes requests to the appropriate backend based on model name prefix. Constructed from `InferenceConfig`.

### `EmbeddingRouter`

Multi-provider embedding router. Routes embedding requests to DeepInfra or OpenRouter based on provider prefix.

### `RouterModelEntry`

Unified model entry from any provider with provider prefix applied.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `prefixed_name` | `String` | Full model name with provider prefix (e.g., `"OM/qwen3:8b"`) |
| `provider` | `ProviderId` | Provider this model belongs to |
| `model` | `String` | Raw model name without prefix |
| `family` | `Option<String>` | Model family (e.g., `"llama"`, `"qwen2"`) |
| `parameter_size` | `Option<String>` | Parameter count (e.g., `"8B"`, `"70B"`) |
| `quantization_level` | `Option<String>` | Quantization level (e.g., `"Q4_0"`) |
| `size_bytes` | `Option<u64>` | Model size in bytes (if available) |
| `supports_vision` | `Option<bool>` | Whether the model supports vision/multimodal input. Populated via heuristic on model family name |

**Methods:**
- `from_model_entry(provider: ProviderId, model_id: &str) -> Self` — construct from provider and model ID, with inferred vision support
- `infer_vision_support(model: &str, family: Option<&str>) -> Option<bool>` — heuristic check against compiled-in allowlist of vision-capable model families (llava, gemma3, pixtral, qwen2-vl, etc.) plus any listed in the `HKASK_VISION_FAMILIES` env var

### `FusionPlugin`

Chat protocol fusion plugin type (re-exported from `chat_protocol`).

## Public Functions

### `orchestrate()`

```rust
pub async fn orchestrate(
    router: &InferenceRouter,
    prompt: &str,
    params: &LLMParameters,
    tools: Option<&[ChatToolDefinition]>,
    fusion: &FusionConfig,
) -> Result<InferenceResult, InferenceError>
```

Provider-agnostic fusion deliberation entry point. Dispatches to the panel in parallel, then routes to the configured `FusionMode` for judge behavior.

## Model Naming Convention

Provider selection is determined by a 2-letter prefix in the model name:

| Prefix | Provider |
|---|---|
| `DI/` | DeepInfra (`api.deepinfra.com`) |
| `FA/` | fal.ai (`api.fal.ai`) |
| `TG/` | Together AI (`api.together.xyz`) |
| `OR/` | OpenRouter (`openrouter.ai/api`) |
| `KC/` | KiloCode (`api.kilo.ai/api/gateway`) |
| `RP/` | RunPod serverless (requires `RUNPOD_TEMPLATE_ID`) |
| `BT/` | Baseten serverless (requires `BASETEN_MODEL_ID`) |
| (none) | Default provider (configurable, default: DeepInfra) |

## Re-exports from Crate Root

`FusionPlugin`, `FusionConfig`, `FusionMode`, `FusionSkill`, `InferenceConfig`, `ProviderConfig`, `ProviderId`, `EmbeddingRouter`, `InferenceRouter`.
