//! SOAP inference routes for Russell integration

use axum::{Json, extract::State, http::StatusCode, routing::Router};

use hkask_ensemble::ports::InferenceClient;

use crate::{
    ApiState, SoapInferAuthRequest, SoapInferRequest, SoapInferResponse, SoapInferenceConfig,
    ValidationErrorType,
};

/// Create SOAP inference router
pub fn soap_infer_router() -> Router<ApiState> {
    Router::new().route("/api/llm/infer", axum::routing::post(soap_infer))
}

/// SOAP inference endpoint for Russell
#[utoipa::path(
    post,
    path = "/api/llm/infer",
    tag = "inference",
    request_body = SoapInferAuthRequest,
    responses(
        (status = 200, description = "LLM inference response", body = SoapInferResponse),
        (status = 400, description = "Validation failed"),
        (status = 403, description = "Capability verification failed"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error"),
        (status = 504, description = "Inference timeout"),
    ),
)]
async fn soap_infer(
    State(state): State<ApiState>,
    Json(req): Json<SoapInferAuthRequest>,
) -> Result<Json<SoapInferResponse>, StatusCode> {
    use std::time::Instant;
    use tokio::time::{Duration, timeout};

    let config = SoapInferenceConfig::from_env().map_err(|e| {
        tracing::error!("SOAP inference config error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let start = Instant::now();

    // Validate request size (DoS prevention)
    if let Err(err) = validate_soap_request(&req.request, &config) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify capability token (OCAP security boundary)
    // Parse token to extract holder WebID for proper authority tracking
    let token = match hkask_types::capability::CapabilityToken::from_base64(&req.capability_token) {
        Ok(t) => t,
        Err(_) => {
            return Err(StatusCode::FORBIDDEN);
        }
    };

    // Verify token signature
    if !token.verify(&config.capability_secret) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Load Jack persona from file (runtime loading)
    let jack_persona = match config.load_jack_persona() {
        Ok(content) => content,
        Err(e) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let system_prompt = format!(
        "You are Jack, Russell's nurse persona.\n\n{}\n\n\
         Safety Constraints:\n\
         - Never emit shell commands\n\
         - Rank intervention IDs; don't compose commands\n\
         - Use SOAP format: Subjective, Objective, Assessment, Plan\n\
         - When proposing actions, use ACTION: <skill>/<id> syntax",
        jack_persona
    );

    let user_prompt = build_soap_prompt(&req.request);
    let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

    let infer_request = hkask_ensemble::ports::GenerateRequest {
        model: config.model.clone(),
        prompt: full_prompt.clone(),
        options: Some(hkask_ensemble::ports::GenerateOptions {
            n_probs: None,
            temperature: Some(config.temperature),
            max_tokens: Some(config.max_tokens as i32),
        }),
    };

    // Inference with timeout (resilience pattern)
    let response_text = if let Some(ref inferencer) = state.ensemble_inferencer {
        match timeout(
            Duration::from_secs(config.timeout_secs),
            inferencer.generate(&infer_request),
        )
        .await
        {
            Ok(Ok(resp)) => resp.response,
            Ok(Err(e)) => {
                format!("Inference error: {}", e)
            }
            Err(_) => {
                return Err(StatusCode::GATEWAY_TIMEOUT);
            }
        }
    } else {
        format!(
            "Mock response: Received SOAP request with {} events, {} crit, {} alert, {} warn, {} info",
            req.request.objective.recent_events.len(),
            req.request.objective.severity_counts.crit,
            req.request.objective.severity_counts.alert,
            req.request.objective.severity_counts.warn,
            req.request.objective.severity_counts.info,
        )
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    let actions = extract_actions(&response_text);

    Ok(Json(SoapInferResponse {
        response: response_text,
        model: config.model,
        latency_ms,
        actions,
    }))
}

/// Build SOAP prompt from request
fn build_soap_prompt(req: &SoapInferRequest) -> String {
    let mut prompt = String::new();

    if let Some(subj) = &req.subjective {
        prompt.push_str(&format!("**Subjective:** {}\n\n", subj));
    }

    prompt.push_str("**Objective:**\n");
    prompt.push_str(&format!(
        "Severity: {} crit, {} alert, {} warn, {} info\n\n",
        req.objective.severity_counts.crit,
        req.objective.severity_counts.alert,
        req.objective.severity_counts.warn,
        req.objective.severity_counts.info,
    ));

    if !req.objective.recent_events.is_empty() {
        prompt.push_str("Recent Events:\n");
        for event in &req.objective.recent_events {
            prompt.push_str(&format!(
                "- [{}] {}: {}\n",
                event.severity, event.probe, event.message
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("**Assessment:**\n(Awaiting your analysis)\n\n");
    prompt.push_str("**Plan:**\n(Awaiting your recommendations)\n");

    prompt
}

/// Extract ACTION: proposals from response text
fn extract_actions(response: &str) -> Vec<String> {
    let mut actions = Vec::new();
    for line in response.lines() {
        if let Some(action) = line.trim().strip_prefix("ACTION:") {
            actions.push(action.trim().to_string());
        }
    }
    actions
}

/// Validate SOAP request size and content (DoS prevention)
pub fn validate_soap_request(
    req: &SoapInferRequest,
    config: &SoapInferenceConfig,
) -> Result<(), ValidationErrorType> {
    // Check event count
    if req.objective.recent_events.len() > config.max_events {
        return Err(ValidationErrorType::TooManyEvents);
    }

    // Check subjective length
    if let Some(subj) = &req.subjective
        && subj.len() > config.max_subjective_len
    {
        return Err(ValidationErrorType::SubjectiveTooLong);
    }

    // Check event message lengths
    for event in &req.objective.recent_events {
        if event.message.len() > config.max_message_len {
            return Err(ValidationErrorType::MessageTooLong);
        }
    }

    Ok(())
}
