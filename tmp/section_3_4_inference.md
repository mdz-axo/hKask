### 3.4 Inference (`hkask-inference`)

**Motivating Principles:** P9 (Homeostatic Self-Regulation) + P4 (Clear Boundaries — provider membrane)
**Crate:** `hkask-inference` | **Sources:** `src/*.rs`, `tests/*.rs`

**63 production contracts** + **31 test contracts**.

#### Production Contracts

| FR# | Contract ID | Function | Principle Annotations |
|-----|------------|----------|---------------------|
| FR-I001 | `P9-inf-build-chat-request` | `build_chat_request()` | [P9] Motivating: Homeostatic Self-Regulation — constructs regulated LLM request payload |
| FR-I002 | `P9-inf-map-tool-calls` | `map_tool_calls()` | [P9] Motivating: Homeostatic Self-Regulation — structured tool-call results for routing |
| FR-I003 | `P9-inf-map-token-probs` | `map_token_probs()` | [P9] Motivating: Homeostatic Self-Regulation — token probability metadata for monitoring |
| FR-I004 | `P9-inf-chat-response-to-result` | `chat_response_to_result()` | [P9] Motivating: Homeostatic Self-Regulation — normalizes provider response for monitoring |
| FR-I005 | `P9-inf-parse-sse-stream` | `parse_sse_stream()` | [P9] Motivating: Homeostatic Self-Regulation — parses streaming response chunks for regulated output |
| FR-I006 | `P9-inf-validate-prompt` | `validate_prompt()` | [P9] Motivating: Homeostatic Self-Regulation — input validation prevents token overconsumption |
| FR-I007 | `P9-inf-parse-provider-from-model` | `parse_from_model()` | [P9] Motivating: Homeostatic Self-Regulation — model-name routing to provider boundary |
| FR-I008 | `P9-inf-prefix-model` | `prefix_model()` | [P9] Motivating: Homeostatic Self-Regulation — canonical provider-prefixed model naming |
| FR-I009 | `P9-inf-provider-as-str` | `as_str()` | [P9] Motivating: Homeostatic Self-Regulation — stable provider code for routing |
| FR-I010 | `P9-inf-config-from-env` | `from_env()` | [P9] Motivating: Homeostatic Self-Regulation — inference configuration resolved from environment |
| FR-I011 | `P9-inf-build-http-client` | `build_client()` | [P9] Motivating: Homeostatic Self-Regulation — bounded HTTP client for regulated requests |
| FR-I012 | `P4-inf-deepinfra-backend-new` | `new()` | [P4] Motivating: Clear Boundaries — DeepInfra provider membrane requires valid API key |
| FR-I013 | `P9-inf-deepinfra-generate` | `generate()` | [P9] Motivating: Homeostatic Self-Regulation — regulated text generation |
| FR-I014 | `P9-inf-deepinfra-generate-vision` | `generate_vision()` | [P9] Motivating: Homeostatic Self-Regulation — regulated multimodal generation |
| FR-I015 | `P9-inf-deepinfra-generate-stream` | `generate_stream()` | [P9] Motivating: Homeostatic Self-Regulation — regulated streaming text generation |
| FR-I016 | `P9-inf-deepinfra-list-models` | `list_models()` | [P9] Motivating: Homeostatic Self-Regulation — model variety discovery with freshness filter |
| FR-I017 | `P9-inf-deepinfra-remove-background` | `remove_background()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image transformation |
| FR-I018 | `P9-inf-deepinfra-generate-image` | `generate_image()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image generation |
| FR-I019 | `P9-inf-deepinfra-image-to-image` | `image_to_image()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image editing |
| FR-I020 | `P9-inf-deepinfra-generate-speech` | `generate_speech()` | [P9] Motivating: Homeostatic Self-Regulation — regulated speech synthesis |
| FR-I021 | `P9-inf-deepinfra-transcribe` | `transcribe()` | [P9] Motivating: Homeostatic Self-Regulation — regulated speech transcription |
| FR-I022 | `P4-inf-embedding-router-new` | `new()` | [P4] Motivating: Clear Boundaries — embedding provider membrane gated by API key |
| FR-I023 | `P9-inf-embed-sentences` | `embed_sentences()` | [P9] Motivating: Homeostatic Self-Regulation — regulated batch embedding generation |
| FR-I024 | `P9-inf-embed-sentence` | `embed_sentence()` | [P9] Motivating: Homeostatic Self-Regulation — regulated single embedding generation |
| FR-I025 | `P4-inf-fal-backend-new` | `new()` | [P4] Motivating: Clear Boundaries — fal.ai provider membrane requires valid API key |
| FR-I026 | `P9-inf-fal-generate` | `generate()` | [P9] Motivating: Homeostatic Self-Regulation — regulated text generation |
| FR-I027 | `P9-inf-fal-generate-vision` | `generate_vision()` | [P9] Motivating: Homeostatic Self-Regulation — regulated multimodal generation |
| FR-I028 | `P9-inf-fal-generate-stream` | `generate_stream()` | [P9] Motivating: Homeostatic Self-Regulation — regulated streaming text generation |
| FR-I029 | `P9-inf-fal-list-models` | `list_models()` | [P9] Motivating: Homeostatic Self-Regulation — static model catalog for variety |
| FR-I030 | `P9-inf-fal-generate-image` | `generate_image()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image generation |
| FR-I031 | `P9-inf-fal-image-to-image` | `image_to_image()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image editing |
| FR-I032 | `P9-inf-fal-remove-background` | `remove_background()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image transformation |
| FR-I033 | `P9-inf-fal-upscale` | `upscale()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image upscaling |
| FR-I034 | `P9-inf-fal-generate-video` | `generate_video()` | [P9] Motivating: Homeostatic Self-Regulation — regulated video generation |
| FR-I035 | `P9-inf-fal-image-to-video` | `image_to_video()` | [P9] Motivating: Homeostatic Self-Regulation — regulated video generation |
| FR-I036 | `P9-inf-fal-segment-object` | `segment_object()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image segmentation |
| FR-I037 | `P9-inf-fal-generate-speech` | `generate_speech()` | [P9] Motivating: Homeostatic Self-Regulation — regulated speech synthesis |
| FR-I038 | `P9-inf-fal-transcribe` | `transcribe()` | [P9] Motivating: Homeostatic Self-Regulation — regulated speech transcription |
| FR-I039 | `P4-inf-inference-router-new` | `new()` | [P4] Motivating: Clear Boundaries — multi-provider membrane assembled from configured boundaries |
| FR-I040 | `P9-inf-router-list-models` | `list_models()` | [P9] Motivating: Homeostatic Self-Regulation — aggregated model variety across providers |
| FR-I041 | `P9-inf-router-search-models` | `search_models()` | [P9] Motivating: Homeostatic Self-Regulation — searchable model catalog for routing |
| FR-I042 | `P9-inf-router-list-vision-models` | `list_vision_models()` | [P9] Motivating: Homeostatic Self-Regulation — vision-capable model discovery |
| FR-I043 | `P9-inf-router-generate-vision` | `generate_vision()` | [P9] Motivating: Homeostatic Self-Regulation — regulated multimodal dispatch |
| FR-I044 | `P9-inf-router-generate-image` | `generate_image()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image generation dispatch |
| FR-I045 | `P9-inf-router-image-to-image` | `image_to_image()` | [P9] Motivating: Homeostatic Self-Regulation — regulated image editing dispatch |
| FR-I046 | `P9-inf-router-remove-background` | `remove_background()` | [P9] Motivating: Homeostatic Self-Regulation — regulated background removal dispatch |
| FR-I047 | `P9-inf-router-upscale` | `upscale()` | [P9] Motivating: Homeostatic Self-Regulation — regulated upscaling dispatch |
| FR-I048 | `P9-inf-router-generate-video` | `generate_video()` | [P9] Motivating: Homeostatic Self-Regulation — regulated video generation dispatch |
| FR-I049 | `P9-inf-router-image-to-video` | `image_to_video()` | [P9] Motivating: Homeostatic Self-Regulation — regulated video generation dispatch |
| FR-I050 | `P9-inf-router-generate-speech` | `generate_speech()` | [P9] Motivating: Homeostatic Self-Regulation — regulated speech synthesis dispatch |
| FR-I051 | `P9-inf-router-segment-object` | `segment_object()` | [P9] Motivating: Homeostatic Self-Regulation — regulated segmentation dispatch |
| FR-I052 | `P9-inf-router-transcribe` | `transcribe()` | [P9] Motivating: Homeostatic Self-Regulation — regulated transcription dispatch |
| FR-I053 | `P9-inf-router-embed-text` | `embed_text()` | [P9] Motivating: Homeostatic Self-Regulation — placeholder for regulated embedding dispatch |
| FR-I054 | `P9-inf-infer-vision-support` | `infer_vision_support()` | [P9] Motivating: Homeostatic Self-Regulation — heuristic routing for multimodal models |
| FR-I055 | `P4-inf-ollama-backend-new` | `new()` | [P4] Motivating: Clear Boundaries — local Ollama provider membrane established from config |
| FR-I056 | `P9-inf-ollama-generate` | `generate()` | [P9] Motivating: Homeostatic Self-Regulation — regulated text generation |
| FR-I057 | `P9-inf-ollama-generate-vision` | `generate_vision()` | [P9] Motivating: Homeostatic Self-Regulation — regulated multimodal generation |
| FR-I058 | `P9-inf-ollama-generate-stream` | `generate_stream()` | [P9] Motivating: Homeostatic Self-Regulation — regulated streaming text generation |
| FR-I059 | `P9-inf-ollama-list-models` | `list_models()` | [P9] Motivating: Homeostatic Self-Regulation — model variety discovery |
| FR-I060 | `P4-inf-together-backend-new` | `new()` | [P4] Motivating: Clear Boundaries — Together AI provider membrane requires valid API key |
| FR-I061 | `P9-inf-together-generate` | `generate()` | [P9] Motivating: Homeostatic Self-Regulation — regulated text generation |
| FR-I062 | `P9-inf-together-generate-stream` | `generate_stream()` | [P9] Motivating: Homeostatic Self-Regulation — regulated streaming text generation |
| FR-I063 | `P9-inf-together-list-models` | `list_models()` | [P9] Motivating: Homeostatic Self-Regulation — model variety discovery |

#### Test Contracts

| FR# | Contract ID | Test Name |
|-----|------------|-----------|
| FR-IT001 | `P9-inf-test-chat-response-deserializes` | `chat_response_deserializes_openai_format()` |
| FR-IT002 | `P9-inf-test-build-chat-request-stream-false` | `build_chat_request_stream_false()` |
| FR-IT003 | `P9-inf-test-validate-prompt-rejects` | `validate_prompt_rejects_invalid()` |
| FR-IT004 | `P9-inf-test-disable-thinking-wire` | `disable_thinking_maps_to_wire_format()` |
| FR-IT005 | `P9-inf-test-enable-thinking-omitted` | `enable_thinking_omitted_when_true()` |
| FR-IT006 | `P9-inf-validate-prompt` | `validate_prompt_contract()` |
| FR-IT007 | `P9-inf-test-parse-provider-prefix` | `parse_provider_prefix()` |
| FR-IT008 | `P9-inf-test-unprefixed-model-none` | `parse_no_prefix_returns_none()` |
| FR-IT009 | `P9-inf-test-empty-model-none` | `parse_empty_model_returns_none()` |
| FR-IT010 | `P9-inf-test-too-short-none` | `parse_too_short_returns_none()` |
| FR-IT011 | `P9-inf-test-unknown-prefix-none` | `parse_unknown_prefix_returns_none()` |
| FR-IT012 | `P9-inf-test-prefix-model-format` | `prefix_model_format()` |
| FR-IT013 | `P9-inf-test-fal-prefix` | `parse_fal_prefix()` |
| FR-IT014 | `P9-inf-test-provider-code` | `parse_provider_code_all_codes()` |
| FR-IT015 | `P9-inf-test-provider-code-default` | `parse_provider_code_unknown_defaults_to_ollama()` |
| FR-IT016 | `P9-inf-test-resolve-api-key-primary` | `resolve_api_key_primary_env()` |
| FR-IT017 | `P9-inf-test-resolve-api-key-fallback` | `resolve_api_key_fallback_env()` |
| FR-IT018 | `P9-inf-test-resolve-api-key-empty` | `resolve_api_key_empty_when_missing()` |
| FR-IT019 | `P9-inf-test-resolve-api-key-priority` | `resolve_api_key_primary_wins_over_fallback()` |
| FR-IT020 | `P9-inf-test-fal-backend-new-fails` | `construction_fails_without_api_key()` |
| FR-IT021 | `P9-inf-test-fal-backend-new-succeeds` | `construction_succeeds_with_api_key()` |
| FR-IT022 | `P9-inf-test-fal-static-catalog` | `static_catalog_returns_vision_models()` |
| FR-IT023 | `P9-inf-test-fal-vision-support` | `vision_support_heuristic_recognizes_fal_models()` |
| FR-IT024 | `P9-inf-test-routing-by-provider-prefix` | `routing_by_provider_prefix()` |
| FR-IT025 | `P9-inf-test-unavailable-backend-error` | `unavailable_backend_returns_error()` |
| FR-IT026 | `P9-inf-test-default-provider-routing` | `default_provider_routing()` |
| FR-IT027 | `P9-inf-test-model-override-routing` | `model_override_routing()` |
| FR-IT028 | `P9-inf-test-list-models-degradation` | `list_models_graceful_degradation()` |
| FR-IT029 | `P9-inf-test-thinking-disable-flow` | `disable_thinking_flows_to_wire_format()` |
| FR-IT030 | `P9-inf-test-deepinfra-live-summary` | `deepinfra_summarization()` |
| FR-IT031 | `P9-inf-test-together-live-summary` | `together_summarization()` |
