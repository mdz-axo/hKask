use super::inference_types::{
    ChatMessage, ChatToolDefinition, InferenceError, InferenceResult, InferenceUsage,
    StructuredToolCall,
};
use crate::template::LLMParameters;
use futures_util::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// LLM invocation boundary. Uses ``Pin<Box<dyn Future>>`` (not `async_trait`) for object-safety.
/// Impls: `InferenceRouter` (hkask-inference), `Arc<dyn InferencePort>` (blanket).
pub trait InferencePort: Send + Sync {
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>;

    /// Falls back to `generate()` when `model_override` is `None`.
    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.generate(prompt, parameters, tools)
    }

    /// Multi-turn inference with an explicit message array.
    ///
    /// This is the correct path for chat/REPL: each message carries its own
    /// `role` ("system", "user", "assistant"), so the provider sees the
    /// conversation as `[system, user, assistant, user, ...]` — not a single
    /// flattened string. This eliminates the "you responding to yourself"
    /// defect where previous assistant responses were embedded inside a
    /// `user` role message.
    ///
    /// Default: flattens messages to a string and delegates to
    /// `generate_with_model`. Backends that speak the OpenAI wire format
    /// override this to pass the message array directly.
    fn generate_with_messages(
        &self,
        messages: &[ChatMessage],
        parameters: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        let prompt = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        self.generate_with_model(&prompt, parameters, model_override, tools)
    }

    /// Streaming variant of `generate_with_messages`.
    ///
    /// Default: yields a single chunk from `generate_with_messages`.
    fn generate_stream_with_messages(
        &self,
        messages: &[ChatMessage],
        parameters: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        let future = self.generate_with_messages(messages, parameters, model_override, tools);
        Box::pin(futures_util::stream::once(async move {
            Ok(InferenceStreamChunk::from(future.await?))
        }))
    }

    fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<InferenceResult>, InferenceError>> + Send + '_>>
    {
        use futures_util::future::join_all;
        let futures: Vec<_> = (0..n)
            .map(|_| self.generate(prompt, parameters, None))
            .collect();
        Box::pin(async move {
            let results = join_all(futures).await;
            results.into_iter().collect()
        })
    }

    /// Stream inference chunks. Default: yields single chunk from `generate()`. Override for SSE/streaming backends.
    fn generate_stream(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        let future = self.generate(prompt, parameters, tools);
        Box::pin(futures_util::stream::once(async move {
            Ok(InferenceStreamChunk::from(future.await?))
        }))
    }

    /// Stream with optional model override. Falls back to `generate_stream()` when `model_override` is `None`.
    fn generate_stream_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        if model_override.is_some() {
            let future = self.generate_with_model(prompt, parameters, model_override, tools);
            Box::pin(futures_util::stream::once(async move {
                Ok(InferenceStreamChunk::from(future.await?))
            }))
        } else {
            self.generate_stream(prompt, parameters, tools)
        }
    }

    /// Vision inference — send base64-encoded images to a multimodal model.
    ///
    /// The default rejects the request so an implementation cannot silently drop images.
    fn generate_vision(
        &self,
        _prompt: &str,
        _images: &[String],
        _parameters: &LLMParameters,
        _model_override: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        Box::pin(async {
            Err(InferenceError::VisionUnsupported(
                "backend does not implement vision inference".to_string(),
            ))
        })
    }
}

/// A single chunk of streaming inference output. Final chunk has `finish_reason` + `usage`.
#[derive(Debug, Clone)]
pub struct InferenceStreamChunk {
    pub text_delta: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<InferenceUsage>,
    pub tool_calls: Vec<StructuredToolCall>,
}

impl From<InferenceResult> for InferenceStreamChunk {
    fn from(r: InferenceResult) -> Self {
        Self {
            text_delta: r.text,
            model: r.model,
            finish_reason: Some(r.finish_reason),
            usage: Some(r.usage),
            tool_calls: r.tool_calls,
        }
    }
}

/// Blanket impl — enables `InferenceLoop<Arc<dyn InferencePort>>` default type param.
/// Vtable dispatch only at construction; hot path uses static dispatch.
impl InferencePort for Arc<dyn InferencePort> {
    fn generate(
        &self,
        p: &str,
        pa: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate(p, pa, tools)
    }
    fn generate_with_model(
        &self,
        p: &str,
        pa: &LLMParameters,
        m: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate_with_model(p, pa, m, tools)
    }
    fn generate_with_messages(
        &self,
        messages: &[ChatMessage],
        pa: &LLMParameters,
        m: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate_with_messages(messages, pa, m, tools)
    }
    fn generate_stream_with_messages(
        &self,
        messages: &[ChatMessage],
        pa: &LLMParameters,
        m: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref()
            .generate_stream_with_messages(messages, pa, m, tools)
    }
    fn generate_n(
        &self,
        p: &str,
        pa: &LLMParameters,
        n: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<InferenceResult>, InferenceError>> + Send + '_>>
    {
        self.as_ref().generate_n(p, pa, n)
    }
    fn generate_stream(
        &self,
        p: &str,
        pa: &LLMParameters,
        t: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref().generate_stream(p, pa, t)
    }
    fn generate_stream_with_model(
        &self,
        p: &str,
        pa: &LLMParameters,
        m: Option<&str>,
        t: Option<&[ChatToolDefinition]>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref().generate_stream_with_model(p, pa, m, t)
    }
    fn generate_vision(
        &self,
        p: &str,
        imgs: &[String],
        pa: &LLMParameters,
        m: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate_vision(p, imgs, pa, m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TextOnlyInference;

    impl InferencePort for TextOnlyInference {
        fn generate(
            &self,
            _prompt: &str,
            _parameters: &LLMParameters,
            _tools: Option<&[ChatToolDefinition]>,
        ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>
        {
            Box::pin(async {
                Err(InferenceError::Generation(
                    "text generation must not be used for vision".to_string(),
                ))
            })
        }
    }

    #[tokio::test]
    async fn default_vision_inference_rejects_images() {
        let result = TextOnlyInference
            .generate_vision(
                "describe",
                &["image-data".to_string()],
                &LLMParameters::default(),
                None,
            )
            .await;

        assert!(matches!(result, Err(InferenceError::VisionUnsupported(_))));
    }
}
