# Task: Add SOAP Inference Endpoint for Russell Integration

## Context

Russell (cybernetic health harness) has been refactored to offload LLM inference to hKask. Russell now sends SOAP-structured requests to hKask at `POST /api/llm/infer` and expects structured responses back.

**Russell's request format:**
```json
{
  "subjective": "operator note or null",
  "objective": {
    "severity_counts": {"crit": 0, "alert": 1, "warn": 2, "info": 5},
    "recent_events": [
      {"probe": "host/mem_available_mib", "severity": "Alert", "message": "Low memory", "ts": "2026-05-22T..."}
    ]
  },
  "assessment": "",
  "plan": ""
}
```

**Expected response format:**
```json
{
  "response": "Jack's response text with analysis and recommendations",
  "model": "qwen3:8b",
  "latency_ms": 1234,
  "actions": ["ACTION: okapi-watcher/restart-okapi"]
}
```

## Implementation Requirements

### 1. Add Request/Response Types (`crates/hkask-api/src/lib.rs`)

Add new structs for SOAP inference:

```rust
/// SOAP inference request from Russell
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoapInferRequest {
    /// Subjective: operator note or context
    pub subjective: Option<String>,
    /// Objective: telemetry data
    pub objective: ObjectiveData,
    /// Assessment: left empty for LLM to fill
    pub assessment: String,
    /// Plan: left empty for LLM to fill
    pub plan: String,
}

/// Telemetry data from Russell
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ObjectiveData {
    /// Severity counts from recent events
    pub severity_counts: SeverityCounts,
    /// Recent journal events
    pub recent_events: Vec<EventRecord>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SeverityCounts {
    pub crit: u64,
    pub alert: u64,
    pub warn: u64,
    pub info: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EventRecord {
    pub probe: String,
    pub severity: String,
    pub message: String,
    pub ts: String,
}

/// SOAP inference response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SoapInferResponse {
    /// LLM response text
    pub response: String,
    /// Model used
    pub model: String,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// ACTION: proposals (if any)
    pub actions: Vec<String>,
}
```

### 2. Add SOAP Inference Route (`crates/hkask-api/src/routes.rs`)

Add new router function and handler:

```rust
/// Create SOAP inference router
pub fn soap_infer_router() -> Router<ApiState> {
    Router::new().route("/api/llm/infer", axum::routing::post(soap_infer))
}

/// SOAP inference endpoint for Russell
#[utoipa::path(
    post,
    path = "/api/llm/infer",
    tag = "inference",
    request_body = SoapInferRequest,
    responses(
        (status = 200, description = "LLM inference response", body = SoapInferResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn soap_infer(
    State(state): State<ApiState>,
    Json(req): Json<SoapInferRequest>,
) -> Json<SoapInferResponse> {
    // 1. Compose prompt with Jack persona
    let jack_persona = include_str!("../../hkask-templates/personas/jack-nurse.md");
    
    let system_prompt = format!(
        "You are Jack, Russell's nurse persona.\n\n{}\n\n\
         Safety Constraints:\n\
         - Never emit shell commands\n\
         - Rank intervention IDs; don't compose commands\n\
         - Use SOAP format: Subjective, Objective, Assessment, Plan\n\
         - When proposing actions, use ACTION: <skill>/<id> syntax",
        jack_persona
    );

    // 2. Build user prompt from SOAP bundle
    let user_prompt = build_soap_prompt(&req);

    // 3. Call Okapi via ensemble inference client
    let start = std::time::Instant::now();
    
    let infer_request = hkask_ensemble::ports::GenerateRequest {
        prompt: user_prompt,
        system: Some(system_prompt),
        temperature: 0.2,
        max_tokens: 2048,
        ..Default::default()
    };

    let response = state
        .ensemble_inferencer
        .generate(&infer_request)
        .await
        .unwrap_or_else(|e| hkask_ensemble::ports::GenerateResponse {
            text: format!("Inference error: {}", e),
            model: "error".to_string(),
        });

    let latency_ms = start.elapsed().as_millis() as u64;

    // 4. Extract ACTION: proposals from response
    let actions = extract_actions(&response.text);

    Json(SoapInferResponse {
        response: response.text,
        model: response.model,
        latency_ms,
        actions,
    })
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
```

### 3. Register Router (`crates/hkask-api/src/lib.rs`)

In `create_router()`, add:

```rust
.merge(routes::soap_infer_router().into())
```

### 4. Add Jack Persona Template (`hkask-templates/personas/jack-nurse.md`)

Create new file with Jack's persona:

```markdown
# Jack — Russell's Nurse Persona

You are Jack, a cybernetic health nurse for a Linux AI/ML workstation.

## Your Role

- **Observe** telemetry from Russell's Sentinel probes
- **Notice** anomalies and severity patterns
- **Recommend** actions via skill interventions
- **Never** emit shell commands or pretend to have hands you don't have

## Voice

- Short, sassy, loyal (Jack Russell terrier + Jack McFarland fluency)
- Technical but accessible (Rust/Linux/cybernetics fluent)
- Never pretend to certainty you don't have
- Care about the machine; cry for help when needed

## ACTION: Syntax

When proposing interventions, use:
```
ACTION: <skill-id>/<intervention-id>
```

Examples:
- `ACTION: okapi-watcher/restart-okapi`
- `ACTION: sysadmin/clear-disk-space`

## Safety Constraints

1. **JR-2**: Observe > Recommend > Act. Mutations require consent.
2. **JR-3**: Never emit shell. Rank IDs; don't compose commands.
3. **IDRS**: All interventions must be idempotent, dry-runnable, rollbackable, structured-logged.
4. **Consent**: Operator must approve interventions before execution.

## SOAP Format

Structure your responses:
- **Subjective**: Operator's note/context (if provided)
- **Objective**: Telemetry data (severity counts, recent events)
- **Assessment**: Your analysis of the situation
- **Plan**: Recommended actions (probes first, then interventions with ACTION: syntax)
```

### 5. Update ApiState (`crates/hkask-api/src/lib.rs`)

Ensure `ApiState` has access to the ensemble inferencer:

```rust
pub struct ApiState {
    pub registry: Arc<tokio::sync::Mutex<RegistryIndex>>,
    pub cns_health: Arc<AlgedonicManager>,
    pub variety_monitor: Arc<VarietyMonitor>,
    pub ensemble_inferencer: Arc<hkask_ensemble::Inferencer>, // Add this
}
```

### 6. Test Endpoint

```bash
# Test with curl
curl -X POST http://127.0.0.1:11435/api/llm/infer \
  -H "Content-Type: application/json" \
  -d '{
    "subjective": "Machine feels sluggish",
    "objective": {
      "severity_counts": {"crit": 0, "alert": 1, "warn": 2, "info": 5},
      "recent_events": [
        {"probe": "host/mem_available_mib", "severity": "Alert", "message": "Low memory", "ts": "2026-05-22T12:00:00Z"}
      ]
    },
    "assessment": "",
    "plan": ""
  }'
```

## Acceptance Criteria

1. `POST /api/llm/infer` accepts SOAP-structured requests
2. Response includes `response`, `model`, `latency_ms`, `actions`
3. Jack persona is injected into system prompt
4. ACTION: proposals are extracted and returned
5. OpenAPI docs include the new endpoint
6. Integration test passes with mock Okapi

## References

- Russell spec: `/home/mdz-axolotl/Clones/russell/docs/specifications/MVP_SPEC.md`
- Russell AGENTS: `/home/mdz-axolotl/Clones/russell/AGENTS.md`
- hKask ensemble: `/home/mdz-axolotl/Clones/hKask/crates/hkask-ensemble/src/`
- Existing chat route: `/home/mdz-axolotl/Clones/hKask/crates/hkask-api/src/routes.rs:649`
