---
title: "Okapi Integration — API Contract"
version: "0.23.0"
status: "Active"
last_updated: 2026-06-07
audience: [architects, developers]
domain: "Application"
mds_categories: [domain]
---

# Okapi Integration — API Contract

> **Note:** This reference document provides implementation detail supplementary to the authoritative specification in [`MDS.md §7.2`](../MDS.md §7.2) §2.5.

**Version:** 0.23.0
**Last Updated:** 2026-06-07
**Status:** Active

---

## Overview

Okapi is the default LLM inference backend for hKask, providing text generation with temperature-controlled parameters for anti-normative template execution [^ollama-api]. All inference flows through the `OkapiInference` implementation of the `InferencePort` trait.

**Source:** `crates/hkask-templates/src/inference_port.rs`
**Config:** `crates/hkask-templates/src/okapi_config.rs`

---

## API Endpoint

### POST /api/generate

**Base URL:** Configurable via `OkapiConfig.base_url` (default: `http://127.0.0.1:11435`) [^openai-chat-api]

#### Request Schema

```json
{
  "model": "string (required) — Model identifier, e.g. 'ollama/llama-3.1-8b-instruct'",
  "messages": [
    {
      "role": "string — 'user' | 'system' | 'assistant'",
      "content": "string — Prompt text"
    }
  ],
  "temperature": "float32 — Sampling temperature (0.0–1.0)",
  "top_p": "float32 — Nucleus sampling threshold (0.0–1.0)",
  "top_k": "int32 — Top-k sampling parameter",
  "frequency_penalty": "float32 — Frequency penalty (0.0–2.0)",
  "presence_penalty": "float32 — Presence penalty (0.0–2.0)",
  "max_tokens": "int32 — Maximum tokens to generate",
  "seed": "uint64|null — Deterministic seed for reproducibility",
  "n_probs": "int32|null — Number of top token probabilities to return (default: 5)"
}
```

#### Response Schema

```json
{
  "model": "string — Model used for generation",
  "choices": [
    {
      "message": {
        "role": "string",
        "content": "string — Generated text"
      },
      "finish_reason": "string — 'stop' | 'length' | 'content_filter'",
      "token_probs": [
        {
          "token": "string",
          "prob": "float64",
          "top_k": [
            {
              "token": "string",
              "prob": "float64"
            }
          ]
        }
      ]
    }
  ],
  "usage": {
    "prompt_tokens": "uint32",
    "completion_tokens": "uint32",
    "total_tokens": "uint32"
  }
}
```

#### Authentication

If `OkapiConfig.api_key` is set, requests include:
```
Authorization: Bearer <api_key>
```

---

## Error Handling

### Error Types (InferenceError)

| Variant | HTTP Status | Description |
|---------|-------------|-------------|
| `Connection` | N/A | Network/connection failure, circuit breaker open |
| `Model` | 400 | Invalid model identifier |
| `Generation` | N/A | Empty response, prompt validation failure |
| `Json` | N/A | Response parse error |
| `RateLimitExceeded` | ~~Removed~~ | Consolidated into energy budget enforcement |

### Retryable Status Codes

The following HTTP status codes trigger automatic retry:

| Code | Meaning |
|------|---------|
| 408 | Request Timeout |
| 429 | Too Many Requests |
| 500 | Internal Server Error |
| 502 | Bad Gateway |
| 503 | Service Unavailable |
| 504 | Gateway Timeout |

### Retry Behavior

- **Max retries:** 3 (configurable via `OkapiRetryConfig.max_retries`)
- **Backoff:** Exponential with base 500ms, capped at 30s
- **Formula:** `min(backoff_base_ms * 2^attempt, max_delay_ms)`
- **Attempt delays:** 500ms → 1000ms → 2000ms → 4000ms → ...

### Circuit Breaker

Optional resilience layer (`CircuitBreaker`) [^nygard-release]:
- Tracks consecutive failures/successes
- When open, immediately returns `InferenceError::Connection("Circuit breaker is open")`
- Records failure on non-success HTTP responses
- Records success on valid responses
- Emits `cns.connector.circuit_open` span when tripped

---

## Configuration

### OkapiConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_url` | String | `http://127.0.0.1:11435` | Okapi API endpoint |
| `api_key` | Option<String> | `None` | Bearer token for authentication |
| `timeout_secs` | u64 | `30` | HTTP request timeout |
| `pool_max_idle` | usize | `10` | Max idle connections per host |

### OkapiRetryConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_retries` | u32 | `3` | Maximum retry attempts |
| `backoff_base_ms` | u64 | `500` | Base delay in milliseconds |
| `max_delay_ms` | u64 | `30000` | Maximum delay cap |
| `retryable_status` | Vec<u16> | `[408, 429, 500, 502, 503, 504]` | Status codes that trigger retry |

### Presets

| Preset | base_url | api_key | timeout | pool_max_idle |
|--------|----------|---------|---------|---------------|
| `local_dev()` | `http://127.0.0.1:11435` | None | 30s | 5 |
| `default()` | `http://127.0.0.1:11435` | None | 30s | 10 |

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OKAPI_BASE_URL` | `http://127.0.0.1:11435` | Okapi API base URL |
| `OKAPI_API_KEY` | (none) | API key for authentication |
| `OKAPI_TIMEOUT_SECS` | `30` | Request timeout in seconds |
| `OKAPI_POOL_MAX_IDLE` | `10` | Max idle connections per host |

---

## Model Catalog

Okapi supports any model identifier string [^gguf-spec]. Convention: `<provider>/<model-name>`.

| Tier | Model ID | Use Case |
|------|----------|----------|
| `fast_local` | `fast-local-model` | Quick local inference, template selection |
| `balanced` | (configurable) | Standard template execution |
| `ollama/llama-3.1-8b-instruct` | (example) | Full inference with instruction following |

Models are selected by:
1. **Runtime override** — `/model` CLI slash command or `model` field in API `POST /api/chat` request
2. `ModelRequirements.required` if provided (via `generate_with_model`)
3. `OkapiInference.model` default (set at construction)
4. `ModelTierSelection` rules in registry mapping config

### Discovering Models

Available models are discovered from Okapi's `GET /api/tags` endpoint:

- **CLI:** `/model` shows the current model; `/model <query>` performs fuzzy search against locally loaded models
- **API:** `GET /api/models` lists all available models; `GET /api/models/search?q=<query>` performs fuzzy search
- **Source:** `crates/hkask-templates/src/okapi_config.rs` (`list_okapi_models`, `search_okapi_models`)

### Switching Models

| Interface | How |
|-----------|-----|
| CLI flag | `kask chat -m qwen3:8b` |
| CLI slash | `/model qwen3:8b` inside `kask chat` |
| API request | `{ "input": "...", "model": "qwen3:8b" }` in `POST /api/chat` |
| API search | `GET /api/models/search?q=qwen` to find matching models |

When Okapi is unreachable, the model name is still stored — it will be used
on the next inference attempt (graceful degradation).

---

## Prompt Validation

Prompts are validated before API calls [^white-prompt]:
- Must be non-empty
- Must not exceed 1,000,000 characters

---

## Rate Limiting

Rate limiting is now handled by energy budget enforcement via `EnergyBudget.try_consume()`. The `RateLimiter` and `InferenceError::RateLimitExceeded` types have been removed from the inference path. `McpErrorKind::RateLimited` remains for external API HTTP 429 responses where downstream services impose rate limits.

---

## CNS Integration

Okapi inference emits CNS spans at key boundary points:

| Span | When |
|------|------|
| `cns.connector.circuit_open` | Circuit breaker trips |

---

## Vision Inference

### `OkapiInference::generate_vision()`

Sends base64-encoded images along with a text prompt to a vision-capable model via Okapi. This is a direct `impl OkapiInference` method — **not** part of the `InferencePort` trait, which remains text-only.

**Source:** `crates/hkask-templates/src/inference_port.rs` (lines 414–510)

#### Request Schema

The `Message` struct includes an optional `images` field for vision inference:

```json
{
  "role": "user",
  "content": "Extract all text from this document.",
  "images": ["base64-encoded-image-data"]
}
```

- `images` is `Option<Vec<String>>` with `#[serde(skip_serializing_if)]` — text-only requests omit it entirely.
- Images are base64-encoded file bytes (PDF, PNG, JPEG, etc.).
- The Okapi chat API receives images inline in the message content.

#### Method Signature

```rust
pub async fn generate_vision(
    &self,
    prompt: &str,
    images: &[String],          // base64-encoded
    model_override: Option<&str>,
    fallback_model: Option<&str>,
    parameters: &LLMParameters,
) -> Result<InferenceResult, InferenceError>
```

#### Usage: OCR Pipeline

The `hkask-mcp-markitdown` server uses `generate_vision` for OCR fallback:

1. `markitdown_convert` extracts text from PDF/MD/HTML/TXT
2. If text extraction yields < 50 words (likely a scanned PDF), falls back to OCR
3. OCR sends the file bytes (base64) to a vision model via `generate_vision`
4. Vision model returns extracted text

**Environment variable:** `HKASK_OCR_MODEL` — must be set to a vision-capable model (e.g., `minicpm-v`). If unset, OCR requests return an error with guidance.

**MCP tool:** `inference_generate_vision` (via `InferencePort` trait, internal cognition layer) (prompt, images, model, fallback_model, temperature, max_tokens, caller_id)

---

## Architecture Notes

- `InferencePort` is the single async inference trait; the synchronous `SyncInferencePort` was removed in v0.21.0-p4.
- `OkapiInference` supports three construction modes: `new`, `with_retry_config`, `with_circuit_breaker`
- `OkapiInference::generate_vision()` is a direct impl method (not on the trait) for multimodal/vision inference
- Token probabilities (`n_probs`) are enabled by default (5 top tokens) for confidence scoring
- Anti-normative generation patterns use `generate_n` for multi-output selection

---

## References

[^ollama-api]: Ollama Contributors. (2024). *Ollama REST API*. https://github.com/ollama/ollama/blob/main/docs/api.md
[^openai-chat-api]: OpenAI. (2024). *Chat Completions API Reference*. https://platform.openai.com/docs/api-reference/chat
[^nygard-release]: Nygard, M. T. (2018). *Release It!: Design and Deploy Production-Ready Software* (2nd ed.). Pragmatic Bookshelf.
[^gguf-spec]: Gerganov, G. (2023). *GGUF: GGML Universal File Format*. https://github.com/ggerganov/ggml/blob/master/docs/gguf.md
[^white-prompt]: White, J., Fu, Q., Schmidt, S., & Sural, S. (2023). A prompt pattern catalog to enhance prompt engineering with ChatGPT. *arXiv preprint arXiv:2302.11382*. https://arxiv.org/abs/2302.11382
[^gamma1994]: Gamma, E., Helm, R., Johnson, R., & Vlissides, J. (1994). *Design Patterns: Elements of Reusable Object-Oriented Software*. Addison-Wesley. Adapter pattern for port compatibility.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
