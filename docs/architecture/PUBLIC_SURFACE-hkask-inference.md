---
title: "Public Surface Justification — hkask-inference"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-inference

**Crate:** `hkask-inference`  
**Public items in lib.rs:** 18  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-inference` is the **multi-provider inference router** — dispatches to Ollama, Fireworks, DeepInfra, Together AI, and fal.ai. Its surface is large because it abstracts multiple backends:

1. **Provider adapters** — Each provider (Ollama, Fireworks, DeepInfra, Together, fal.ai) has its own adapter implementing `InferencePort`.
2. **InferenceRouter** — The central dispatcher that routes model names (with provider prefixes) to the correct backend.
3. **InferenceConfig** — Environment-based configuration for all providers.
4. **Embedding support** — `embed_text`, `embed_sentences` for vectorization across providers.

## Mitigations

- **Trait-based dispatch:** `InferencePort` trait enables uniform provider interface.
- **Provider prefix routing:** `OM/`, `DI/`, `FW/`, `TG/`, `FA/` prefixes eliminate ambiguity.

## Deletion Test

Delete `hkask-inference` and the multi-provider routing, provider adapters, and embedding dispatch reappear in every crate that needs LLM access. The crate earns its existence.
