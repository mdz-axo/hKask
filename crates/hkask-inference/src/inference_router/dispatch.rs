//! Provider dispatch — routes generate calls to resolved backends via trait-object maps.

use super::InferenceRouter;
use crate::config::ProviderId;
use hkask_ports::{ChatToolDefinition, InferenceError, InferenceResult, InferenceStreamChunk};
use hkask_types::template::LLMParameters;
use std::pin::Pin;

impl InferenceRouter {
    /// Dispatch a generate call to the resolved chat backend.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — shared dispatch for text generation
    /// pre:  provider is a resolved ProviderId with an available chat backend
    /// pre:  model, prompt, params are validated
    /// post: returns Ok(InferenceResult) on success
    /// post: returns Err(Connection) if no chat backend is configured for provider
    pub(crate) async fn dispatch_generate(
        &self,
        provider: ProviderId,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Result<InferenceResult, InferenceError> {
        let backend = self.chat_backends.get(&provider).ok_or_else(|| {
            InferenceError::Connection(format!(
                "Provider {} is not available (check configuration)",
                provider.as_str()
            ))
        })?;
        backend.generate(model, prompt, params, tools).await
    }

    /// Dispatch a streaming generate call to the resolved chat backend.
    ///
    /// expect: "The system dispatches regulated inference to the correct provider"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — shared dispatch for streaming
    /// pre:  provider is a resolved ProviderId with an available chat backend
    /// post: returns a stream of Ok(InferenceStreamChunk) on success
    /// post: returns a stream yielding Err(Connection) if no chat backend is configured
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
        match self.chat_backends.get(&provider) {
            Some(backend) => backend.generate_stream(&model, &prompt, &params, tools.as_deref()),
            None => {
                let provider_str = provider.as_str().to_string();
                Box::pin(futures_util::stream::once(async move {
                    Err(InferenceError::Connection(format!(
                        "Provider {} is not available (check configuration)",
                        provider_str
                    )))
                }))
            }
        }
    }
}
