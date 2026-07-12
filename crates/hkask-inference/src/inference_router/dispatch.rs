//! Provider dispatch — routes generate calls to resolved backends.

use super::InferenceRouter;
use crate::config::ProviderId;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use std::pin::Pin;

impl InferenceRouter {
    /// Dispatch a generate call to the resolved backend.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — shared dispatch for text generation
    /// pre:  provider is a resolved ProviderId with available backend
    /// pre:  model, prompt, params are validated and cloned
    /// post: returns Ok(InferenceResult) on success
    /// post: returns Err(Connection) if backend is None or provider is unsupported
    pub(crate) async fn dispatch_generate(
        &self,
        provider: ProviderId,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Result<InferenceResult, InferenceError> {
        match provider {
            ProviderId::DeepInfra => {
                self.deepinfra
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("DeepInfra backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
            ProviderId::Fal => {
                self.fal
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("fal.ai backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
            ProviderId::Together => {
                self.together
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Together backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
            ProviderId::OpenRouter => {
                self.openrouter
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("OpenRouter backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
            ProviderId::KiloCode => {
                self.kilocode
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("KiloCode backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
            ProviderId::Runpod => Err(InferenceError::Connection(
                "Runpod is an adapter provider".to_string(),
            )),
            ProviderId::Ollama => {
                self.ollama
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Ollama backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
            ProviderId::Cline => {
                self.cline
                    .as_ref()
                    .ok_or_else(|| {
                        InferenceError::Connection("Cline backend unavailable".to_string())
                    })?
                    .generate(model, prompt, params, tools)
                    .await
            }
        }
    }

    /// Dispatch a streaming generate call to the resolved backend.
    pub(crate) fn dispatch_generate_stream(
        &self,
        provider: ProviderId,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        let model = model.to_string();
        let prompt = prompt.to_string();
        let params = params.clone();
        let tools = tools.map(|t| t.to_vec());
        match provider {
            ProviderId::DeepInfra => {
                match self.deepinfra.as_ref().ok_or_else(|| {
                    InferenceError::Connection("DeepInfra backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Fal => {
                match self.fal.as_ref().ok_or_else(|| {
                    InferenceError::Connection("fal.ai backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Together => {
                match self.together.as_ref().ok_or_else(|| {
                    InferenceError::Connection("Together backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::OpenRouter => {
                match self.openrouter.as_ref().ok_or_else(|| {
                    InferenceError::Connection("OpenRouter backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::KiloCode => {
                match self.kilocode.as_ref().ok_or_else(|| {
                    InferenceError::Connection("KiloCode backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Runpod => Box::pin(futures_util::stream::once(async move {
                Err(InferenceError::Connection(
                    "Runpod is an adapter provider".to_string(),
                ))
            })),
            ProviderId::Ollama => {
                match self.ollama.as_ref().ok_or_else(|| {
                    InferenceError::Connection("Ollama backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
            ProviderId::Cline => {
                match self.cline.as_ref().ok_or_else(|| {
                    InferenceError::Connection("Cline backend unavailable".to_string())
                }) {
                    Ok(b) => b.generate_stream(&model, &prompt, &params, tools.as_deref()),
                    Err(e) => Box::pin(futures_util::stream::once(async move { Err(e) })),
                }
            }
        }
    }
}
