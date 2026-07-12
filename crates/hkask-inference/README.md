# hkask-inference

Multi-provider inference router for hKask — DeepInfra, Together AI, fal.ai, OpenRouter, KiloCode.

## Features

- **Provider dispatch** — route inference requests to the best available provider
- **Model selection** — fuzzy search, prefix-based routing (`DI/`, `FA/`, `TG/`, `OR/`, `KC/`)
- **Provider ID parsing** — `ProviderId` with model name resolution
- **Prompt validation** — never-panics input validation

## Configuration

| Variable | Description |
|----------|-------------|
| `DI_API_KEY` | DeepInfra API key |
| `FA_API_KEY` | Fal.ai API key |
| `TG_API_KEY` | Together AI API key |
| `OR_API_KEY` | OpenRouter API key |
| `KC_API_KEY` | KiloCode API key |
| `INFERENCE_MODEL` | Default model (e.g., `qwen/qwen3-235b-a22b-2507`) |
