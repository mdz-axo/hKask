---
title: "Inference Router — API Contract"
version: "0.27.0"
status: "Active"
last_updated: 2026-06-15
audience: [architects, developers]
domain: "Application"
mds_categories: [domain]
---

# Inference Router — API Contract

> **Note:** This reference document provides implementation detail supplementary to the authoritative specification in [`MDS.md`](../core/MDS.md) §7 §2.5.

**Version:** 0.28.0
**Last Updated:** 2026-06-11
**Status:** Active

---

## Overview

hKask uses a multi-provider inference router (`hkask-inference` crate) that dispatches LLM requests to Fireworks.ai (cloud) or DeepInfra (cloud) based on a 2-letter provider prefix in the model name. Both providers speak OpenAI-compatible `/v1/chat/completions`, enabling a single wire format across backends.

**Source:** `crates/hkask-inference/src/`
**Config:** `crates/hkask-inference/src/config.rs` (`InferenceConfig`)

### Provider Map

| Prefix | Provider | Type | Base URL |
|--------|----------|------|----------|
| `FW/` | Fireworks.ai | Cloud | `https://api.fireworks.ai/inference` |
| `DI/` | DeepInfra | Cloud | `https://api.deepinfra.com/v1/openai` |

---

## API Endpoint

### POST /v1/chat/completions

All three providers use the OpenAI-compatible chat completions endpoint. The router constructs identical requests regardless of backend.

**Base URLs:**
- Fireworks: `{FW_BASE_URL}/v1/chat/completions`
- DeepInfra: `{DI_BASE_URL}/v1/chat/completions`

#### Request Schema

```json
{
  "model": "string (required) — Model identifier (provider prefix stripped before dispatch)",
  "messages": [
    {
      "role": "string — 'user' | 'system' | 'assistant'",
      "content": "string — Prompt text",
      "images": ["base64-encoded image data (optional, for vision models)"]
    }
  ],
  "temperature": "float32 — Sampling temperature (0.0–2.0)",
  "top_p": "float32 — Nucleus sampling threshold (0.0–1.0)",
  "top_k": "int32 — Top-k sampling parameter",
  "min_p": "float32 — Min-p threshold (0.0–1.0)",
  "typical_p": "float32 — Locally typical sampling (0.0–1.0)",
  "frequency_penalty": "float32 — Frequency penalty (0.0–2.0)",
  "presence_penalty": "float32 — Presence penalty (0.0–2.0)",
  "max_tokens": "int32 — Maximum tokens to generate",
  "seed": "uint64|null — Deterministic seed for reproducibility",
  "n_probs": "int32|null — Number of top token probabilities to return (default: 5)",
  "stream": "bool|null — Enable SSE streaming"
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
      "finish_reason": "string — 'stop' | 'length' | 'content_filter' | 'tool_calls'",
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
      ],
      "tool_calls": [
        {
          "id": "string",
          "function": {
            "name": "string — server/tool convention",
            "arguments": "object"
          }
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

- **Fireworks:** `Authorization: Bearer {FW_API_KEY}`
- **DeepInfra:** `Authorization: Bearer {DI_API_KEY}`

---

## Error Handling

### Error Types (InferenceError)

| Variant | Description |
|---------|-------------|
| `Connection` | Network/connection failure, provider unavailable |
| `Model` | Invalid model identifier |
| `Generation` | Empty response, prompt validation failure |
| `Json` | Response parse error |
| `CircuitOpen` | Circuit breaker tripped |

---

## Configuration

### InferenceConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fireworks_base_url` | String | `https://api.fireworks.ai/inference` | Fireworks API endpoint |
| `fireworks_api_key` | String | (empty) | Fireworks Bearer token |
| `deepinfra_base_url` | String | `https://api.deepinfra.com/v1/openai` | DeepInfra API endpoint |
| `deepinfra_api_key` | String | (empty) | DeepInfra Bearer token |
| `timeout_secs` | u64 | `120` | HTTP request timeout |
| `pool_max_idle` | usize | `5` | Max idle connections per host |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FW_BASE_URL` | `https://api.fireworks.ai/inference` | Fireworks base URL |
| `FW_API_KEY` | (none) | Fireworks API key |
| `DI_BASE_URL` | `https://api.deepinfra.com/v1/openai` | DeepInfra base URL |
| `DI_API_KEY` | (none) | DeepInfra API key |

---

## Model Catalog

### Discovering Models

Models are discovered from each provider's native listing endpoint:

| Provider | Endpoint | Filter |
|----------|----------|--------|
| Fireworks | `GET /v1/models` | Updated ≤ 6 months ago |
| DeepInfra | `GET /v1/models` | Updated ≤ 6 months ago |

Results are merged and returned with provider prefixes applied.

- **CLI:** `/model` shows the current model; `/model <query>` performs fuzzy search across all providers
- **API:** `GET /api/models` lists all available models; `GET /api/models/search?q=<query>` performs fuzzy search
- **Source:** `crates/hkask-inference/src/inference_router.rs` (`list_models`, `search_models`)

### Switching Models

| Interface | How |
|-----------|-----|
| CLI flag | `kask chat -m OM/qwen3:8b` |
| CLI slash | `/model FW/llama-v3p1-70b-instruct` inside `kask chat` |
| API request | `{ "input": "...", "model": "DI/meta-llama/Llama-3.3-70B-Instruct" }` in `POST /api/chat` |
| API search | `GET /api/models/search?q=llama` to find matching models across providers |

---

## Prompt Validation

Prompts are validated before API calls:
- Must be non-empty
- Must not exceed 1,000,000 characters

---

## Vision Inference

### `InferenceRouter::generate_vision()`

Sends base64-encoded images along with a text prompt to a vision-capable model. Dispatches to the appropriate backend based on the model's provider prefix. All three providers support vision models.

**Source:** `crates/hkask-inference/src/inference_router.rs`

#### Method Signature

```rust
pub async fn generate_vision(
    &self,
    prompt: &str,
    images: &[String],          // base64-encoded
    params: &LLMParameters,
    model_override: Option<&str>,
) -> Result<InferenceResult, InferenceError>
```

#### Usage: OCR Pipeline

The `hkask-mcp-markitdown` server uses `generate_vision` for OCR fallback:

1. `markitdown_convert` extracts text from PDF/MD/HTML/TXT
2. If text extraction yields < 50 words (likely a scanned PDF), falls back to OCR
3. OCR sends the file bytes (base64) to a vision model via `generate_vision`
4. Vision model returns extracted text

**Environment variable:** `HKASK_OCR_MODEL` — must be set to a vision-capable model.

---

## Embedding Router

### `EmbeddingRouter`

Generates embedding vectors for semantic search and style composition. Routes to the appropriate provider based on model prefix.

| Provider | Endpoint | Wire Format |
|----------|----------|-------------|
| Fireworks | `POST /v1/embeddings` | `{model, input: [...]}` (OpenAI) |
| DeepInfra | `POST /v1/embeddings` | `{model, input: [...]}` (OpenAI) |

**Source:** `crates/hkask-inference/src/embedding_router.rs`

---

## Architecture Notes

- `InferencePort` is the single async inference trait in `hkask-types`; `InferenceRouter` is its primary implementation.
- `EmbeddingRouter` provides embedding generation across providers.
- Each backend owns its own HTTP client, auth, and model listing endpoint — no shared abstraction.
- Shared chat protocol types and helpers live in `chat_protocol.rs` as free functions.
- The router is a pure dispatcher — no response transformation, no automatic failover between providers.

---

## References

[^openai-chat-api]: OpenAI. (2024). *Chat Completions API Reference*. https://platform.openai.com/docs/api-reference/chat
[^nygard-release]: Nygard, M. T. (2018). *Release It!: Design and Deploy Production-Ready Software* (2nd ed.). Pragmatic Bookshelf.

---

*ℏKask - A Minimal Viable Container for Agents — v0.28.0*
