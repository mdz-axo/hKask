use crate::inference_types::{InferenceError, InferenceResult, InferenceUsage, StructuredToolCall};
use futures_util::Stream;
use hkask_types::template::LLMParameters;
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
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>>;

    /// Falls back to `generate()` when `model_override` is `None`.
    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        _model_override: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.generate(prompt, parameters)
    }

    fn generate_n(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        n: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<InferenceResult>, InferenceError>> + Send + '_>>
    {
        use futures_util::future::join_all;
        let futures: Vec<_> = (0..n).map(|_| self.generate(prompt, parameters)).collect();
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
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        let future = self.generate(prompt, parameters);
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
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        if model_override.is_some() {
            let future = self.generate_with_model(prompt, parameters, model_override);
            Box::pin(futures_util::stream::once(async move {
                Ok(InferenceStreamChunk::from(future.await?))
            }))
        } else {
            self.generate_stream(prompt, parameters)
        }
    }

    /// Vision inference — send base64-encoded images to a multimodal model.
    /// Default: falls back to `generate_with_model()` (text-only). Override for vision-capable backends.
    fn generate_vision(
        &self,
        prompt: &str,
        _images: &[String],
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.generate_with_model(prompt, parameters, model_override)
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
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate(p, pa)
    }
    fn generate_with_model(
        &self,
        p: &str,
        pa: &LLMParameters,
        m: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>> {
        self.as_ref().generate_with_model(p, pa, m)
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
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref().generate_stream(p, pa)
    }
    fn generate_stream_with_model(
        &self,
        p: &str,
        pa: &LLMParameters,
        m: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<InferenceStreamChunk, InferenceError>> + Send + '_>> {
        self.as_ref().generate_stream_with_model(p, pa, m)
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
