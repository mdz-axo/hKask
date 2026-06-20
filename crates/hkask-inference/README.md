# hkask-inference

Multi-provider inference router for hKask — DeepInfra, Together AI, fal.ai, OpenRouter.

## Features

- **Provider dispatch** — route inference requests to the best available provider
- **Model selection** — fuzzy search, prefix-based routing (`FW/`, `DI/`, `TOG/`, `OR/`)
- **Provider ID parsing** — `ProviderId` with model name resolution
- **Prompt validation** — never-panics input validation

## Configuration

| Variable | Description |
|----------|-------------|
| `FW_API_KEY` | Fireworks / DeepInfra API key |
| `DI_API_KEY` | DeepInfra API key |
| `TOGETHER_API_KEY` | Together AI API key |
| `OR_API_KEY` | OpenRouter API key |
| `INFERENCE_MODEL` | Default model (e.g., `google/gemma-4-26B-A4B-it`) |
