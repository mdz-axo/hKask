//! InferencePort trait implementation — canonical route-to-backend dispatch.

use super::InferenceRouter;
use crate::chat_protocol::validate_prompt;
use hkask_ports::{
    ChatToolDefinition, InferenceError, InferencePort, InferenceResult, InferenceStreamChunk,
};
use hkask_types::template::LLMParameters;
use std::pin::Pin;

impl InferencePort for InferenceRouter {
    // pre:  prompt is non-empty; parameters are valid
    // post: Ok(InferenceResult) when resolved provider backend is configured;
    //       Err(Connection) when resolved provider backend is None
    fn generate(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        // Provider-agnostic fusion orchestration: when fusion is active and not bypassed,
        // send to all panel models in parallel, then have the judge synthesize.
        // Priority: per-call fusion_config override > global config.
        if !parameters.bypass_fusion {
            let fusion = parameters
                .fusion_config
                .clone()
                .or_else(|| self.config.fusion.clone());
            if let Some(fusion) = fusion {
                let prompt = prompt.to_string();
                let parameters = parameters.clone();
                let tools = tools.map(|t| t.to_vec());
                return Box::pin(async move {
                    validate_prompt(&prompt)?;
                    self.orchestrate_fusion(&prompt, &parameters, tools.as_deref(), &fusion)
                        .await
                        .map_err(|e| self.heal_error(e, "generate"))
                });
            }
        }

        // LoRA adapter overrides the model entirely (includes base model).
        // Format: "Qwen3.5-9B#pragmatic-semantics-v1" — adapter IS the full model identifier.
        if let Some(ref adapter) = parameters.adapter {
            let (provider, model) = match self.resolve_chat(adapter) {
                Ok(r) => r,
                Err(e) => return Box::pin(async move { Err(self.heal_error(e, "generate")) }),
            };
            let model = model.to_string();
            let prompt = prompt.to_string();
            let parameters = parameters.clone();
            let tools = tools.map(|t| t.to_vec());
            return Box::pin(async move {
                validate_prompt(&prompt)?;
                self.dispatch_generate(provider, &model, &prompt, &parameters, tools.as_deref())
                    .await
                    .map_err(|e| self.heal_error(e, "generate"))
            });
        }

        let model_name = self.effective_model(None, parameters);
        let (provider, model) = match self.resolve_chat(&model_name) {
            Ok(r) => r,
            Err(e) => return Box::pin(async move { Err(self.heal_error(e, "generate")) }),
        };
        let model = model.to_string();
        let prompt = prompt.to_string();
        let parameters = parameters.clone();
        let tools = tools.map(|t| t.to_vec());

        Box::pin(async move {
            validate_prompt(&prompt)?;
            self.dispatch_generate(provider, &model, &prompt, &parameters, tools.as_deref())
                .await
                .map_err(|e| self.heal_error(e, "generate"))
        })
    }

    // pre:  prompt is non-empty; parameters are valid; model_override may be None
    // post: Ok(InferenceResult) when resolved provider backend is configured;
    //       Err(Connection) when resolved provider backend is None
    fn generate_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        // Provider-agnostic fusion orchestration: when fusion is active, not bypassed,
        // and no explicit model_override is given, use multi-model deliberation.
        // Priority: per-call fusion_config override > global config.
        if !parameters.bypass_fusion && model_override.is_none() {
            let fusion = parameters
                .fusion_config
                .clone()
                .or_else(|| self.config.fusion.clone());
            if let Some(fusion) = fusion {
                let prompt = prompt.to_string();
                let parameters = parameters.clone();
                let tools = tools.map(|t| t.to_vec());
                return Box::pin(async move {
                    validate_prompt(&prompt)?;
                    self.orchestrate_fusion(&prompt, &parameters, tools.as_deref(), &fusion)
                        .await
                        .map_err(|e| self.heal_error(e, "generate_with_model"))
                });
            }
        }

        // LoRA adapter overrides the model entirely (bypasses fusion).
        if let Some(ref adapter) = parameters.adapter {
            let (provider, model) = match self.resolve_chat(adapter) {
                Ok(r) => r,
                Err(e) => {
                    return Box::pin(async move { Err(self.heal_error(e, "generate_with_model")) });
                }
            };
            let model = model.to_string();
            let prompt = prompt.to_string();
            let parameters = parameters.clone();
            let tools = tools.map(|t| t.to_vec());
            return Box::pin(async move {
                validate_prompt(&prompt)?;
                self.dispatch_generate(provider, &model, &prompt, &parameters, tools.as_deref())
                    .await
                    .map_err(|e| self.heal_error(e, "generate_with_model"))
            });
        }

        let model_name = self.effective_model(model_override, parameters);
        let (provider, model) = match self.resolve_chat(&model_name) {
            Ok(r) => r,
            Err(e) => {
                return Box::pin(async move { Err(self.heal_error(e, "generate_with_model")) });
            }
        };
        let model = model.to_string();
        let prompt = prompt.to_string();
        let parameters = parameters.clone();
        let tools = tools.map(|t| t.to_vec());

        Box::pin(async move {
            validate_prompt(&prompt)?;
            self.dispatch_generate(provider, &model, &prompt, &parameters, tools.as_deref())
                .await
                .map_err(|e| self.heal_error(e, "generate_with_model"))
        })
    }

    // pre:  prompt is non-empty; parameters are valid
    // post: Stream of Ok(InferenceStreamChunk) when resolved provider backend is configured;
    //       Stream of Err(Connection) when resolved provider backend is None
    fn generate_stream(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        self.generate_stream_with_model(prompt, parameters, None, tools)
    }

    // pre:  prompt is non-empty; parameters are valid; model_override may be None
    // post: Stream of Ok(InferenceStreamChunk) when resolved provider backend is configured;
    //       Stream of Err(Connection) when resolved provider backend is None
    fn generate_stream_with_model(
        &self,
        prompt: &str,
        parameters: &LLMParameters,
        model_override: Option<&str>,
        tools: Option<&[ChatToolDefinition]>,
    ) -> Pin<
        Box<
            dyn futures_util::Stream<Item = Result<InferenceStreamChunk, InferenceError>>
                + Send
                + '_,
        >,
    > {
        // Fusion (multi-model deliberation) is inherently non-streamable: the
        // orchestrator dispatches a panel in parallel and a judge synthesizes a
        // single InferenceResult — there is no token stream to emit. When fusion
        // is active (and not bypassed) and no LoRA adapter overrides the model,
        // `effective_model` below would resolve to the fusion *judge* model and
        // silently stream the judge alone — a behavioral asymmetry with
        // `generate_with_model` (which runs full fusion). Refuse rather than
        // silently downgrade: the caller must use generate() for full fusion or
        // set bypass_fusion=true to stream a single (non-fused) model.
        if !parameters.bypass_fusion
            && parameters.adapter.is_none()
            && (parameters.fusion_config.is_some() || self.config.fusion.is_some())
        {
            return Box::pin(futures_util::stream::once(async move {
                Err(InferenceError::Generation(
                    "Fusion panel deliberation is non-streamable; use generate() for \
                     full fusion or set bypass_fusion=true to stream a single model."
                        .into(),
                ))
            }));
        }

        // LoRA adapter overrides the model entirely (bypasses fusion).
        if let Some(ref adapter) = parameters.adapter {
            let adapter_str = adapter.to_string();
            let (provider, model) = match self.resolve_chat(&adapter_str) {
                Ok(r) => r,
                Err(e) => {
                    return Box::pin(futures_util::stream::once(async move { Err(e) }));
                }
            };
            let model = model.to_string();
            let prompt = prompt.to_string();
            let parameters = parameters.clone();
            let tools = tools.map(|t| t.to_vec());
            return self.dispatch_generate_stream(
                provider,
                &model,
                &prompt,
                &parameters,
                tools.as_deref(),
            );
        }

        let model_name = self.effective_model(model_override, parameters);
        let (provider, model) = match self.resolve_chat(&model_name) {
            Ok(r) => r,
            Err(e) => {
                return Box::pin(futures_util::stream::once(async move { Err(e) }));
            }
        };
        let model = model.to_string();
        let prompt = prompt.to_string();
        let parameters = parameters.clone();
        let tools = tools.map(|t| t.to_vec());

        self.dispatch_generate_stream(provider, &model, &prompt, &parameters, tools.as_deref())
    }

    fn generate_vision(
        &self,
        prompt: &str,
        images: &[String],
        parameters: &LLMParameters,
        model_override: Option<&str>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<InferenceResult, InferenceError>> + Send + '_>,
    > {
        let prompt = prompt.to_string();
        let images = images.to_vec();
        let parameters = parameters.clone();
        let model_override = model_override.map(|s| s.to_string());
        Box::pin(async move {
            self.generate_vision(&prompt, &images, &parameters, model_override.as_deref())
                .await
        })
    }
}
