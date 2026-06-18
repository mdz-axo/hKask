---
title: "Inference Router — API Contract"
version: "0.27.0"
status: "Active"
last_updated: 2026-06-17
audience: [architects, developers]
domain: "Application"
mds_categories: [domain]
---

# Inference Router — API Contract

> **Note:** This reference document provides implementation detail supplementary to the authoritative specification in [`MDS.md`](../core/MDS.md).

**Source:** `crates/hkask-inference/src/`
**Config:** `crates/hkask-inference/src/config.rs` (`InferenceConfig`)

---

## Overview

hKask uses a multi-provider inference router that dispatches LLM requests based on a 2-letter provider prefix in the model name. All providers speak OpenAI-compatible `/v1/chat/completions`, enabling a single wire format across backends.

### Provider Map

| Prefix | Provider | Type | API Key Env |
|--------|----------|------|-------------|
| `DI/` | DeepInfra | Cloud | `DI_API_KEY` |
| `TG/` | Together AI | Cloud | `TOGETHER_API_KEY` |
| `FA/` | fal.ai | Cloud | `FA_API_KEY` |
| `RP/` | RunPod | Cloud | `RUNPOD_API_KEY` |
| `BT/` | Baseten | Cloud | `BASETEN_API_KEY` |
| `OM/` | Ollama | Local | (none — local instance) |
| (none) | Default (DI) | Configurable | `HKASK_DEFAULT_PROVIDER` |

---

## API Endpoint

### POST /v1/chat/completions

All providers use the OpenAI-compatible chat completions endpoint. The router constructs identical requests regardless of backend.

**Base URLs (from config defaults):**
- DeepInfra: `https://api.deepinfra.com/v1/chat/completions`
- Together AI: `https://api.together.xyz/v1/chat/completions`
- fal.ai: `https://api.fal.ai/v1/chat/completions`
- RunPod: `https://api.runpod.io/v1/chat/completions`
- Baseten: `https://api.baseten.co/v1/chat/completions`
- Ollama: `http://127.0.0.1:11434/v1/chat/completions`

#### Request Schema

```json
{
  "model": "string — Model identifier (provider prefix stripped before dispatch)",
  "messages": [
    {
      "role": "string — 'user' | 'system' | 'assistant'",
      "content": "string — Prompt text",
      "images": ["base64-encoded image data (optional, for vision models)"]
    }
  ],
  "temperature": "float32 — Sampling temperature (0.0–2.0)",
  "top_p": "float32 — Nucleus sampling threshold (0.0–1.0)",
  "max_tokens": "int32 — Maximum tokens to generate",
  "stream": "bool|null — Enable SSE streaming"
}
```

#### Authentication

All cloud providers use Bearer token authentication:
`Authorization: Bearer {API_KEY}`

Ollama (local) requires no authentication.

---

## Configuration

### InferenceConfig Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_provider` | ProviderId | `DI` | Default provider for unprefixed models |
| `deepinfra_base_url` | String | `https://api.deepinfra.com` | DeepInfra API endpoint |
| `deepinfra_api_key` | String | (empty) | DeepInfra Bearer token |
| `together_base_url` | String | `https://api.together.xyz` | Together AI API endpoint |
| `together_api_key` | String | (empty) | Together AI Bearer token |
| `fal_base_url` | String | `https://api.fal.ai` | fal.ai API endpoint |
| `fal_api_key` | String | (empty) | fal.ai Bearer token |
| `ollama_base_url` | String | `http://127.0.0.1:11434` | Ollama API endpoint |
| `timeout_secs` | u64 | `120` | HTTP request timeout |
| `pool_max_idle` | usize | `5` | Max idle connections per host |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DI_API_KEY` | (none) | DeepInfra API key |
| `TOGETHER_API_KEY` | (none) | Together AI API key |
| `FA_API_KEY` | (none) | fal.ai API key |
| `RUNPOD_API_KEY` | (none) | RunPod API key |
| `BASETEN_API_KEY` | (none) | Baseten API key |
| `OM_BASE_URL` | `http://127.0.0.1:11434` | Ollama base URL |
| `DI_BASE_URL` | `https://api.deepinfra.com` | DeepInfra base URL |
| `TG_BASE_URL` | `https://api.together.xyz` | Together AI base URL |
| `FA_BASE_URL` | `https://api.fal.ai` | fal.ai base URL |
| `HKASK_DEFAULT_PROVIDER` | `DI` | Default provider for unprefixed models |

---

## Model Catalog

### Discovering Models

Models are discovered from each provider's native listing endpoint:

| Provider | Endpoint | Notes |
|----------|----------|-------|
| Ollama | `GET /api/tags` | All local models |
| DeepInfra | `GET /v1/models` | Cloud models |
| Together AI | `GET /v1/models` | Cloud models |
| fal.ai | `GET /v1/models` | Cloud models |
| RunPod | `GET /v1/models` | Cloud models |
| Baseten | `GET /v1/models` | Cloud models |

### Switching Models

| Interface | How |
|-----------|-----|
| REPL slash | `/model DI/meta-llama/Llama-3.3-70B-Instruct` inside the terminal |
| API request | `{ "input": "...", "model": "DI/meta-llama/Llama-3.3-70B-Instruct" }` in `POST /api/chat` |
| API search | `GET /api/models/search?q=llama` to find matching models across providers |

---

## Embedding Router

### `EmbeddingRouter`

Generates embedding vectors for semantic search and memory operations. Currently supports Ollama and DeepInfra.

| Provider | Supported | Endpoint | Wire Format |
|----------|-----------|----------|-------------|
| Ollama | ✅ | `POST /api/embed` | `{model, input: [...]}` |
| DeepInfra | ✅ | `POST /v1/embeddings` | `{model, input: [...]}` (OpenAI) |
| Together AI | ❌ (not yet implemented) | — | — |
| fal.ai | ❌ (no embedding endpoint) | — | — |
| RunPod | ❌ (adapter-composition only) | — | — |
| Baseten | ❌ (adapter-composition only) | — | — |

**Source:** `crates/hkask-inference/src/embedding_router.rs`

---

## Architecture Notes

- `InferencePort` is the single async inference trait in `hkask-types`; `InferenceRouter` is its primary implementation.
- `EmbeddingRouter` provides embedding generation across supported providers (Ollama, DeepInfra).
- Each backend owns its own HTTP client, auth, and model listing endpoint — no shared abstraction.
- Shared chat protocol types and helpers live in `chat_protocol.rs` as free functions.
- The router is a pure dispatcher — no response transformation, no automatic failover between providers.

---

## References

[^ollama-api]: Ollama Contributors. (2024). *Ollama REST API*. https://github.com/ollama/ollama/blob/main/docs/api.md
[^openai-chat-api]: OpenAI. (2024). *Chat Completions API Reference*. https://platform.openai.com/docs/api-reference/chat
