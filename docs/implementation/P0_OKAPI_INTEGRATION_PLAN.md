# hKask Phase 0 Implementation Plan — Okapi Integration

**Date:** 2026-05-19  
**Version:** 1.0.0  
**Status:** Ready for Implementation  
**MVP Target:** Pre-alpha MVP v0.21.0

---

## Executive Summary

This document specifies Phase 0 (P0) MVP-blocking implementation work for hKask's integration with Okapi inference engine. All P0 work belongs in hKask crates — Okapi already provides required capabilities.

**Three P0 Deliverables:**
1. **CNS Span Translator** (`hkask-mcp-inference`) — Okapi metrics → CNS spans
2. **Confidence-Based Router** (`hkask-ensemble`) — Token probabilities → escalation decisions
3. **Template Contract Validator** (`hkask-templates`) — YAML frontmatter → register-time validation

**Line Budget Impact:** ~800 LOC across 3 crates (within ≤30,000 LOC budget)

---

## Part I: Okapi Context for hKask Developers

### I.1: What is Okapi?

**Okapi** is a thin fork of Ollama that exposes llama.cpp capabilities hidden by Ollama's mass-market UX. It serves as hKask's inference engine.

**Key Characteristics:**
| Property | Value |
|----------|-------|
| **Port** | `127.0.0.1:11435` (Ollama runs on 11434) |
| **Line Budget** | ≤50,000 lines Go (excluding vendored llama.cpp) |
| **Philosophy** | "Integrator, not inventor" — exposes llama.cpp via CGo |
| **Backend** | llama.cpp (vendored C++ inference library) |
| **Primary Runner** | `ollamarunner` (pure Go, supports all Okapi features) |
| **Fallback Runner** | `llamarunner` (C++ subprocess, basic inference only) |

**Okapi ≠ hKask:**
- Okapi is the **inference engine** (Go + llama.cpp)
- hKask is the **orchestration layer** (Rust + MCP + ACP)
- Okapi exposes capabilities; hKask orchestrates them via templates

### I.2: Okapi Architecture (Relevant to hKask)

```
┌─────────────────────────────────────────────────────────────────┐
│                         Okapi                                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ HTTP Server (server/routes.go)                            │  │
│  │  - /api/generate, /api/chat (standard Ollama API)         │  │
│  │  - /api/adapters/* (LoRA hot-swap) ← hKask uses this      │  │
│  │  - /api/engine/status (capabilities) ← hKask uses this    │  │
│  │  - /api/metrics/stream (SSE) ← hKask uses this            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                             │                                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Engine (server/engine.go)                                 │  │
│  │  - Single-model manager (OKAPI_SIMPLE_ENGINE=1)           │  │
│  │  - Load/unload semantics                                  │  │
│  │  - Context utilization tracking                           │  │
│  └───────────────────────────────────────────────────────────┘  │
│                             │                                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Runner (runner/ollamarunner/)                             │  │
│  │  - LoRA adapter injection                                 │  │
│  │  - Token probability calculation ← hKask uses this        │  │
│  │  - Advanced samplers (mirostat, DRY, XTC)                 │  │
│  └───────────────────────────────────────────────────────────┘  │
│                             │                                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ llama.cpp CGo Bindings (llama/)                           │  │
│  │  - Direct C++ inference                                   │  │
│  │  - All primitives exposed                                 │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### I.3: Okapi API Surface (hKask Integration Points)

| Endpoint | Method | Purpose | hKask Consumer |
|----------|--------|---------|----------------|
| `/api/generate` | POST | Text generation | `hkask-mcp-inference` |
| `/api/chat` | POST | Chat completion | `hkask-mcp-inference` |
| `/api/embed` | POST | Batch embeddings | `hkask-mcp-embedding` |
| `/api/engine/status` | GET | Capabilities + context utilization | `hkask-templates` (validation) |
| `/api/metrics/stream` | GET | SSE metrics stream | `hkask-mcp-inference` (CNS translator) |
| `/api/adapters/load` | POST | Load LoRA adapter | `hkask-mcp-inference` |
| `/api/adapters/unload` | POST | Unload LoRA adapter | `hkask-mcp-inference` |
| `/api/adapters/swap` | POST | Atomic adapter swap | `hkask-mcp-inference` |

### I.4: Okapi Response Schemas (Critical for hKask)

#### I.4.1: Token Probabilities (Confidence Routing)

```json
POST /api/generate
{
  "model": "qwen3:8b",
  "prompt": "What is the capital of France?",
  "options": {
    "n_probs": 5
  }
}

Response:
{
  "response": "Paris",
  "completion_probabilities": [
    {
      "token": "Paris",
      "prob": 0.95,
      "top_k": [
        {"token": "Paris", "prob": 0.95},
        {"token": "Lyon", "prob": 0.03},
        {"token": "Marseille", "prob": 0.01},
        {"token": "Nice", "prob": 0.005},
        {"token": "Toulouse", "prob": 0.005}
      ]
    },
    {
      "token": " is",
      "prob": 0.92,
      "top_k": [...]
    }
  ]
}
```

**Key Fields:**
- `completion_probabilities[]` — Array of per-token probabilities
- `prob` — Selected token's probability (0.0 - 1.0)
- `top_k[]` — Top-N most probable tokens (for confidence calculation)

**Implementation Note:** Okapi calculates these from llama.cpp's sampler softmax output. The `n_probs` parameter controls how many top tokens to return (default: 0, max: 20).

#### I.4.2: Engine Status (Capability Discovery)

```json
GET /api/engine/status

Response:
{
  "mode": "engine",
  "engine_available": true,
  "model_loaded": true,
  "model_name": "qwen3:8b",
  "capabilities": {
    "runner_type": "ollamarunner",
    "lora_hot_swap": true,
    "token_probs": true,
    "full_metrics": true,
    "advanced_sampling": true,
    "grammar_native": true,
    "speculative_decoding": true,
    "dry_sampler": true,
    "xtc_sampler": true,
    "min_keep": true,
    "chunked_prefill": true,
    "moe_observability": true
  },
  "context_length": 8192,
  "context_used": 1024,
  "context_free": 7168,
  "context_utilization_pct": 12.5
}
```

**Key Fields:**
- `runner_type` — "ollamarunner" (full features) or "llamarunner" (basic only)
- `token_probs` — Required for confidence-based routing
- `grammar_native` — Required for Process templates with GBNF
- `lora_hot_swap` — Required for adapter-based task routing
- `context_utilization_pct` — Used by hKask for context-window routing

#### I.4.3: SSE Metrics Stream (CNS Span Source)

```
GET /api/metrics/stream?interval=5

Response (Server-Sent Events):
event: metrics
data: {"tokens_generated_total": 1234, "kv_cache_tokens": 456, "context_length": 8192, "adapter_swap_latency_ms": 45, "gpu_memory_used_bytes": 4294967296, "prompt_cache_hit_ratio": 0.75}

event: metrics
data: {"tokens_generated_total": 1289, "kv_cache_tokens": 501, "context_length": 8192, "adapter_swap_latency_ms": 45, "gpu_memory_used_bytes": 4294967296, "prompt_cache_hit_ratio": 0.76}
```

**Key Fields:**
- `tokens_generated_total` — Cumulative token count
- `kv_cache_tokens` — Current KV-cache utilization
- `context_length` — Configured context window
- `adapter_swap_latency_ms` — Last adapter swap duration
- `gpu_memory_used_bytes` — VRAM consumption
- `prompt_cache_hit_ratio` — Cache efficiency (0.0 - 1.0)

**Implementation Note:** Okapi emits SSE events at the specified interval (5 seconds recommended). hKask should subscribe once at startup and emit CNS spans on delta (when values change).

### I.5: Okapi Design Philosophy (Why It Matters for hKask)

**"Integrator, Not Inventor":**
- Okapi does NOT implement inference algorithms
- Okapi DOES expose llama.cpp primitives via CGo
- All Okapi features map to llama.cpp functions

**Implications for hKask:**
1. Okapi's API is stable (depends on llama.cpp, not experimental features)
2. Okapi's capabilities are well-defined (12 feature flags in `/api/engine/status`)
3. Okapi's metrics are factual observations (confidence=1.0 for CNS spans)

**Graceful Degradation:**
- If model falls back to `llamarunner`, Okapi features degrade gracefully
- `llamarunner` = basic inference only (no LoRA, no token probs, no advanced sampling)
- hKask should check `capabilities.runner_type` before using advanced features

### I.6: Okapi Configuration (Environment Variables)

| Variable | Default | hKask Relevance |
|----------|---------|-----------------|
| `OLLAMA_HOST` | `127.0.0.1:11434` | Set to `127.0.0.1:11435` for Okapi |
| `OKAPI_SIMPLE_ENGINE` | `0` | Set to `1` for single-model Engine (required for hKask lifecycle API) |
| `OLLAMA_KEEP_ALIVE` | `5m` | Model idle timeout (hKask can override via `/api/load`) |
| `OLLAMA_CONTEXT_LENGTH` | Auto | Override for context-window routing |
| `OLLAMA_GPU_LAYERS` | `-1` (auto) | Cap GPU layer count for multi-model deployments |

---

## Part II: P0-1 — CNS Span Translator

### II.1: Specification

**Crate:** `hkask-mcp-inference`  
**Module:** `src/metrics_translator.rs`  
**Dependencies:** `hkask-cns`, `reqwest`, `tokio-stream`, `serde_json`

**Purpose:** Subscribe to Okapi's SSE metrics stream and emit CNS spans on delta (when values change).

**Design Decisions:**
| Decision | Value | Rationale |
|----------|-------|-----------|
| Translation location | `hkask-mcp-inference` | Okapi is inference-agnostic; hKask owns span semantics |
| Scrape method | SSE stream | Okapi already implements; more efficient than polling |
| Span batching | Delta-only | Reduces CNS noise; only emit when values change |
| Confidence | Fixed 1.0 | Metrics are observational facts, not inferences |

### II.2: CNS Span Namespaces

| Okapi Metric | CNS Span Namespace | Trigger | Data Fields |
|-------------|-------------------|---------|-------------|
| `tokens_generated_total` | `cns.connector.llm.tokens` | Delta | `tokens_generated`, `delta` |
| `kv_cache_tokens` + `context_length` | `cns.connector.llm.context` | Delta | `kv_cache_tokens`, `context_length`, `utilization_pct` |
| `adapter_swap_latency_ms` | `cns.tool.adapter_swap` | Event (when > 0 and changed) | `latency_ms` |
| `gpu_memory_used_bytes` | `cns.connector.llm.gpu_memory` | Delta | `used_bytes`, `delta` |
| `prompt_cache_hit_ratio` | `cns.connector.llm.cache_hit` | Delta | `hit_ratio` |

### II.3: Implementation

```rust
// hkask-mcp-inference/src/metrics_translator.rs
use tokio::sync::mpsc;
use serde::Deserialize;
use chrono::{DateTime, Utc};

/// Okapi metrics as received from SSE stream
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OkapiMetrics {
    pub tokens_generated_total: i64,
    pub kv_cache_tokens: i64,
    pub context_length: i64,
    pub adapter_swap_latency_ms: i64,
    pub gpu_memory_used_bytes: u64,
    pub prompt_cache_hit_ratio: Option<f64>,
}

/// CNS span for external I/O observation
#[derive(Debug, Clone, Serialize)]
pub struct CnsSpan {
    pub namespace: String,
    pub timestamp: DateTime<Utc>,
    pub outcome: Outcome,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
pub enum Outcome {
    Success { data: serde_json::Value },
    Failure { error: String, error_code: Option<String> },
}

/// Subscribes to Okapi SSE stream and emits CNS spans on delta
pub struct MetricsTranslator {
    sse_url: String,
    cns_tx: mpsc::Sender<CnsSpan>,
    last_metrics: Option<OkapiMetrics>,
}

impl MetricsTranslator {
    pub fn new(okapi_base_url: &str, cns_tx: mpsc::Sender<CnsSpan>) -> Self {
        Self {
            sse_url: format!("{}/api/metrics/stream?interval=5", okapi_base_url),
            cns_tx,
            last_metrics: None,
        }
    }
    
    /// Subscribe to SSE stream and translate metrics to CNS spans
    pub async fn subscribe_and_translate(&mut self) -> Result<(), MetricsError> {
        tracing::info!("Subscribing to Okapi SSE stream: {}", self.sse_url);
        
        let mut stream = reqwest_sse::get(&self.sse_url).await?;
        
        while let Some(event) = stream.next().await {
            let metrics: OkapiMetrics = serde_json::from_str(&event.data)
                .map_err(|e| MetricsError::ParseError(e.to_string()))?;
            
            // Emit delta spans (skip first event - no baseline for comparison)
            if let Some(last) = &self.last_metrics {
                self.emit_delta_spans(&metrics, last).await?;
            }
            
            self.last_metrics = Some(metrics);
        }
        
        Ok(())
    }
    
    /// Emit CNS spans for changed metrics only
    async fn emit_delta_spans(&self, current: &OkapiMetrics, last: &OkapiMetrics) -> Result<(), MetricsError> {
        // Token throughput change
        if current.tokens_generated_total != last.tokens_generated_total {
            self.emit_span(CnsSpan {
                namespace: "cns.connector.llm.tokens".to_string(),
                timestamp: Utc::now(),
                outcome: Outcome::Success {
                    data: json!({
                        "tokens_generated": current.tokens_generated_total,
                        "delta": current.tokens_generated_total - last.tokens_generated_total,
                    }),
                },
                confidence: 1.0,
            }).await?;
        }
        
        // Context utilization change
        if current.kv_cache_tokens != last.kv_cache_tokens {
            let utilization_pct = if current.context_length > 0 {
                (current.kv_cache_tokens as f64 / current.context_length as f64) * 100.0
            } else {
                0.0
            };
            
            self.emit_span(CnsSpan {
                namespace: "cns.connector.llm.context".to_string(),
                timestamp: Utc::now(),
                outcome: Outcome::Success {
                    data: json!({
                        "kv_cache_tokens": current.kv_cache_tokens,
                        "context_length": current.context_length,
                        "utilization_pct": utilization_pct,
                    }),
                },
                confidence: 1.0,
            }).await?;
        }
        
        // Adapter swap (event-based, not delta)
        if current.adapter_swap_latency_ms > 0 && current.adapter_swap_latency_ms != last.adapter_swap_latency_ms {
            self.emit_span(CnsSpan {
                namespace: "cns.tool.adapter_swap".to_string(),
                timestamp: Utc::now(),
                outcome: Outcome::Success {
                    data: json!({
                        "latency_ms": current.adapter_swap_latency_ms,
                    }),
                },
                confidence: 1.0,
            }).await?;
        }
        
        // GPU memory change
        if current.gpu_memory_used_bytes != last.gpu_memory_used_bytes {
            self.emit_span(CnsSpan {
                namespace: "cns.connector.llm.gpu_memory".to_string(),
                timestamp: Utc::now(),
                outcome: Outcome::Success {
                    data: json!({
                        "used_bytes": current.gpu_memory_used_bytes,
                        "delta": (current.gpu_memory_used_bytes as i64 - last.gpu_memory_used_bytes as i64).abs(),
                    }),
                },
                confidence: 1.0,
            }).await?;
        }
        
        // Prompt cache hit ratio change
        if current.prompt_cache_hit_ratio != last.prompt_cache_hit_ratio {
            if let Some(ratio) = current.prompt_cache_hit_ratio {
                self.emit_span(CnsSpan {
                    namespace: "cns.connector.llm.cache_hit".to_string(),
                    timestamp: Utc::now(),
                    outcome: Outcome::Success {
                        data: json!({
                            "hit_ratio": ratio,
                        }),
                    },
                    confidence: 1.0,
                }).await?;
            }
        }
        
        Ok(())
    }
    
    async fn emit_span(&self, span: CnsSpan) -> Result<(), MetricsError> {
        self.cns_tx.send(span)
            .await
            .map_err(|e| MetricsError::CnsEmissionError(e.to_string()))
    }
}

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("SSE stream error: {0}")]
    SseError(String),
    
    #[error("JSON parse error: {0}")]
    ParseError(String),
    
    #[error("CNS emission error: {0}")]
    CnsEmissionError(String),
}
```

### II.4: Integration with hkask-cns

```rust
// hkask-mcp-inference/src/lib.rs
mod metrics_translator;

use metrics_translator::{MetricsTranslator, CnsSpan};
use tokio::sync::mpsc;

pub struct InferenceMcpServer {
    cns_tx: mpsc::Sender<CnsSpan>,
    // ... other fields
}

impl InferenceMcpServer {
    pub async fn new(okapi_base_url: &str) -> Result<Self, Error> {
        let (cns_tx, cns_rx) = mpsc::channel(100);
        
        // Start CNS span translator
        let mut translator = MetricsTranslator::new(okapi_base_url, cns_tx.clone());
        tokio::spawn(async move {
            if let Err(e) = translator.subscribe_and_translate().await {
                tracing::error!("CNS translator error: {}", e);
            }
        });
        
        // Start CNS span consumer (from hkask-cns)
        hkask_cns::spawn_span_consumer(cns_rx);
        
        Ok(Self { cns_tx, /* ... */ })
    }
}
```

### II.5: Testing

```rust
// hkask-mcp-inference/tests/metrics_translator_test.rs
#[tokio::test]
async fn test_delta_only_emission() {
    let (tx, mut rx) = mpsc::channel(100);
    
    let mut translator = MetricsTranslator::new("http://localhost:11435", tx);
    
    // First event - no spans emitted (baseline)
    let first_metrics = OkapiMetrics {
        tokens_generated_total: 1000,
        kv_cache_tokens: 500,
        context_length: 8192,
        adapter_swap_latency_ms: 0,
        gpu_memory_used_bytes: 4294967296,
        prompt_cache_hit_ratio: Some(0.75),
    };
    translator.last_metrics = Some(first_metrics.clone());
    
    // Second event - same values, no spans
    translator.emit_delta_spans(&first_metrics, &first_metrics).await.unwrap();
    assert!(rx.try_recv().is_err()); // No spans emitted
    
    // Third event - changed values, spans emitted
    let changed_metrics = OkapiMetrics {
        tokens_generated_total: 1050, // Changed
        kv_cache_tokens: 500, // Unchanged
        context_length: 8192, // Unchanged
        adapter_swap_latency_ms: 0, // Unchanged
        gpu_memory_used_bytes: 4294967296, // Unchanged
        prompt_cache_hit_ratio: Some(0.75), // Unchanged
    };
    translator.emit_delta_spans(&changed_metrics, &first_metrics).await.unwrap();
    
    // Should emit exactly one span (tokens changed)
    let span = rx.recv().await.unwrap();
    assert_eq!(span.namespace, "cns.connector.llm.tokens");
    assert!(rx.try_recv().is_err()); // No more spans
}
```

---

## Part III: P0-2 — Confidence-Based Router

### III.1: Specification

**Crate:** `hkask-ensemble`  
**Module:** `src/confidence_router.rs`  
**Dependencies:** `hkask-mcp-inference`, `serde`, `serde_json`

**Purpose:** Calculate confidence from Okapi token probabilities and escalate to larger models when confidence is below threshold.

**Design Decisions:**
| Decision | Value | Rationale |
|----------|-------|-----------|
| Confidence formula | `avg(prob) × (1 - sqrt(variance))` | Penalizes high-variance (uncertain) outputs |
| Threshold default | 0.75 (75%) | Balanced between escalation frequency and quality |
| Threshold config | Per-template in YAML frontmatter | Templates declare their own confidence requirements |
| Escalation target | Pre-configured larger model | Simple; template declares `escalate_to_model` |
| n_probs value | Template-configurable | Trade-off: accuracy vs. latency |

### III.2: Confidence Calculation

**Formula:**
```
confidence = avg(prob) × (1 - sqrt(variance))

where:
  avg(prob) = Σ(prob_i) / n
  variance = Σ(prob_i - avg)^2 / n
```

**Rationale:**
- `avg(prob)` — Average token probability (raw confidence)
- `variance` — Measures consistency across tokens
- `sqrt(variance)` — Standard deviation (penalty factor)
- High variance = uncertain output = lower confidence

**Example Calculations:**

| Token Probs | Avg | Variance | Sqrt(Var) | Confidence |
|-------------|-----|----------|-----------|------------|
| [0.95, 0.92, 0.94] | 0.937 | 0.0002 | 0.014 | **0.924** (high) |
| [0.60, 0.35, 0.50] | 0.483 | 0.010 | 0.100 | **0.435** (low) |
| [0.80, 0.50, 0.90] | 0.733 | 0.030 | 0.173 | **0.607** (medium) |

### III.3: Implementation

```rust
// hkask-ensemble/src/confidence_router.rs
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Token probability from Okapi response
#[derive(Debug, Clone, Deserialize)]
pub struct TokenProbability {
    pub token: String,
    pub prob: f64,
    pub top_k: Vec<TokenProb>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenProb {
    pub token: String,
    pub prob: f64,
}

/// Okapi generate/chat response
#[derive(Debug, Clone, Deserialize)]
pub struct OkapiResponse {
    pub response: String,
    pub completion_probabilities: Option<Vec<TokenProbability>>,
}

/// Confidence configuration (from template frontmatter or default)
#[derive(Debug, Clone)]
pub struct ConfidenceConfig {
    pub threshold: f64,  // Default: 0.75
    pub escalate_to_model: String,
    pub n_probs: i32,
}

impl Default for ConfidenceConfig {
    fn default() -> Self {
        Self {
            threshold: 0.75,
            escalate_to_model: "qwen3:70b".to_string(),
            n_probs: 5,
        }
    }
}

/// Confidence-based router with escalation
pub struct ConfidenceRouter {
    config: ConfidenceConfig,
    okapi_client: OkapiClient,
}

impl ConfidenceRouter {
    pub fn new(config: ConfidenceConfig, okapi_client: OkapiClient) -> Self {
        Self { config, okapi_client }
    }
    
    /// Generate response with confidence-based escalation
    pub async fn generate_with_escalation(
        &self,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, RouterError> {
        tracing::debug!(
            "Generating with confidence threshold: {:.2}, escalate to: {}",
            self.config.threshold,
            self.config.escalate_to_model
        );
        
        // First attempt with primary model
        let response = self.okapi_client.generate(request).await?;
        
        // Calculate confidence if probabilities available
        if let Some(probs) = &response.completion_probabilities {
            let confidence = compute_confidence(probs);
            
            tracing::debug!(
                "Calculated confidence: {:.3} (threshold: {:.3})",
                confidence,
                self.config.threshold
            );
            
            if confidence < self.config.threshold {
                tracing::info!(
                    "Low confidence ({:.3} < {:.3}), escalating to {}",
                    confidence,
                    self.config.threshold,
                    self.config.escalate_to_model
                );
                
                // Emit CNS span for escalation
                emit_cns_escalation_span(confidence, self.config.threshold).await;
                
                // Escalate to larger model
                let mut escalate_request = request.clone();
                escalate_request.model = self.config.escalate_to_model.clone();
                
                return self.okapi_client.generate(&escalate_request).await
                    .map_err(|e| RouterError::EscalationFailed(e.to_string()));
            }
        }
        
        Ok(response)
    }
}

/// Compute confidence score from token probabilities
/// Formula: avg(prob) × (1 - sqrt(variance))
pub fn compute_confidence(probs: &[TokenProbability]) -> f64 {
    if probs.is_empty() {
        return 0.0;
    }
    
    let avg_prob: f64 = probs.iter()
        .map(|p| p.prob)
        .sum::<f64>() / probs.len() as f64;
    
    let variance: f64 = probs.iter()
        .map(|p| (p.prob - avg_prob).powi(2))
        .sum::<f64>() / probs.len() as f64;
    
    // Confidence = average probability penalized by variance
    avg_prob * (1.0 - variance.sqrt())
}

/// Emit CNS span for escalation event
async fn emit_cns_escalation_span(confidence: f64, threshold: f64) {
    // Implementation depends on hkask-cns integration
    tracing::info!(
        target: "cns.prompt.escalation",
        confidence = %confidence,
        threshold = %threshold,
        "Confidence below threshold, escalating to larger model"
    );
}

#[derive(Debug, Error)]
pub enum RouterError {
    #[error("Okapi client error: {0}")]
    OkapiError(String),
    
    #[error("Escalation failed: {0}")]
    EscalationFailed(String),
}
```

### III.4: Template Integration

```yaml
# Example: Template frontmatter with confidence config
---
template_type: Prompt
domain: WordAct
requires_okapi:
  n_probs: 5  # Required for confidence calculation
confidence:
  threshold: 0.80  # Override default 0.75 (higher confidence required)
  escalate_to_model: "qwen3:70b"
lexicon_terms:
  - classify
  - answer
contract:
  input:
    type: object
    properties:
      question: {type: string}
  output:
    type: object
    properties:
      answer: {type: string}
      confidence: {type: number}
---

[inference]
You are answering factual questions. Return concise, accurate responses.
```

### III.5: Testing

```rust
// hkask-ensemble/tests/confidence_router_test.rs
#[tokio::test]
async fn test_high_confidence_no_escalation() {
    let config = ConfidenceConfig {
        threshold: 0.75,
        escalate_to_model: "qwen3:70b".to_string(),
        n_probs: 5,
    };
    
    let mock_client = MockOkapiClient::new();
    mock_client.expect_generate().returning(|_| {
        Ok(OkapiResponse {
            response: "Paris".to_string(),
            completion_probabilities: Some(vec![
                TokenProbability { token: "Paris".to_string(), prob: 0.95, top_k: vec![] },
                TokenProbability { token: " is".to_string(), prob: 0.92, top_k: vec![] },
                TokenProbability { token: " the".to_string(), prob: 0.94, top_k: vec![] },
            ]),
        })
    });
    
    let router = ConfidenceRouter::new(config, mock_client);
    let response = router.generate_with_escalation(&request).await.unwrap();
    
    assert_eq!(response.response, "Paris");
    // Should NOT escalate (confidence > 0.75)
}

#[tokio::test]
async fn test_low_confidence_escalation() {
    let config = ConfidenceConfig {
        threshold: 0.75,
        escalate_to_model: "qwen3:70b".to_string(),
        n_probs: 5,
    };
    
    let mut mock_client = MockOkapiClient::new();
    
    // First call (low confidence)
    mock_client.expect_generate()
        .times(1)
        .returning(|_| {
            Ok(OkapiResponse {
                response: "Maybe Paris".to_string(),
                completion_probabilities: Some(vec![
                    TokenProbability { token: "Maybe".to_string(), prob: 0.60, top_k: vec![] },
                    TokenProbability { token: " Paris".to_string(), prob: 0.50, top_k: vec![] },
                ]),
            })
        });
    
    // Second call (escalation)
    mock_client.expect_generate()
        .times(1)
        .returning(|_| {
            Ok(OkapiResponse {
                response: "Paris".to_string(),
                completion_probabilities: Some(vec![
                    TokenProbability { token: "Paris".to_string(), prob: 0.98, top_k: vec![] },
                ]),
            })
        });
    
    let router = ConfidenceRouter::new(config, mock_client);
    let response = router.generate_with_escalation(&request).await.unwrap();
    
    assert_eq!(response.response, "Paris");
    // Should escalate (confidence < 0.75)
}

#[test]
fn test_confidence_formula() {
    // High confidence, low variance
    let probs = vec![
        TokenProbability { token: "a".to_string(), prob: 0.95, top_k: vec![] },
        TokenProbability { token: "b".to_string(), prob: 0.92, top_k: vec![] },
        TokenProbability { token: "c".to_string(), prob: 0.94, top_k: vec![] },
    ];
    let confidence = compute_confidence(&probs);
    assert!(confidence > 0.85);
    
    // Low confidence, high variance
    let probs = vec![
        TokenProbability { token: "a".to_string(), prob: 0.60, top_k: vec![] },
        TokenProbability { token: "b".to_string(), prob: 0.35, top_k: vec![] },
        TokenProbability { token: "c".to_string(), prob: 0.50, top_k: vec![] },
    ];
    let confidence = compute_confidence(&probs);
    assert!(confidence < 0.60);
}
```

---

## Part IV: P0-3 — Template Contract Validator

### IV.1: Specification

**Crate:** `hkask-templates`  
**Module:** `src/contract_validator.rs`  
**Dependencies:** `hkask-types` (hLexicon), `serde`, `serde_yaml`, `thiserror`

**Purpose:** Validate template contracts at registration time with actionable error messages.

**Design Decisions:**
| Decision | Value | Rationale |
|----------|-------|-----------|
| Contract schema | YAML frontmatter in template | Jinja2-compatible; keeps contract with template |
| Validation timing | Register-time + optional CI | Fail fast; invalid templates never enter registry |
| Failure mode | Reject with detailed errors | Actionable messages guide template authors |
| Capability check | `GET /api/engine/status` at startup | Cache capabilities; refresh on demand |

### IV.2: Template Frontmatter Schema

```yaml
---
template_type: Prompt  # Prompt | Process | Cognition
domain: WordAct
requires_okapi:
  n_probs: 5           # Required for Prompt templates
  grammar: null        # Required for Process templates (GBNF path)
  adapter: null        # Optional: requires LoRA hot-swap
confidence:
  threshold: 0.75      # Override default
  escalate_to_model: "qwen3:70b"
lexicon_terms:
  - classify
  - discriminate
  - route
contract:
  input:
    type: object
    properties:
      raw_prompt: {type: string}
  output:
    type: object
    properties:
      result: {type: string}
---
```

### IV.3: Okapi Capabilities Schema

```json
GET /api/engine/status

{
  "capabilities": {
    "runner_type": "ollamarunner",
    "lora_hot_swap": true,
    "token_probs": true,
    "grammar_native": true,
    "advanced_sampling": true,
    "speculative_decoding": true,
    "moe_observability": true
  }
}
```

### IV.4: Implementation

```rust
// hkask-templates/src/contract_validator.rs
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashSet;

/// Template frontmatter (YAML)
#[derive(Debug, Deserialize)]
pub struct TemplateFrontmatter {
    pub template_type: TemplateType,
    pub domain: String,
    pub requires_okapi: Option<OkapiRequirements>,
    pub confidence: Option<ConfidenceConfig>,
    pub lexicon_terms: Vec<String>,
    pub contract: Option<ContractSchema>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TemplateType {
    Prompt,    // WordAct - what to say
    Process,   // FlowDef - what to do
    Cognition, // KnowAct - how to think
}

/// Okapi requirements from frontmatter
#[derive(Debug, Deserialize)]
pub struct OkapiRequirements {
    pub n_probs: Option<i32>,
    pub grammar: Option<String>,
    pub adapter: Option<String>,
}

/// Okapi capabilities (from /api/engine/status)
#[derive(Debug, Deserialize, Clone)]
pub struct OkapiCapabilities {
    pub runner_type: String,
    pub lora_hot_swap: bool,
    pub token_probs: bool,
    pub grammar_native: bool,
    pub advanced_sampling: bool,
}

/// Validation error with actionable message
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Template type '{template_type}' requires 'n_probs' in requires_okapi, but it was not specified. Add 'n_probs: 5' to enable token probability-based confidence routing.")]
    MissingNProbs { template_type: String },
    
    #[error("Process template requires 'grammar' constraint in requires_okapi, but it was not specified. Add 'grammar: \"path/to/constraint.gbnf\"' to enable grammar-constrained decoding.")]
    MissingGrammar,
    
    #[error("Template requires LoRA adapter '{adapter}', but Okapi's lora_hot_swap capability is disabled. Okapi runner type: {runner_type}. Use an ollamarunner-compatible model or remove the adapter requirement.")]
    AdapterNotSupported { adapter: String, runner_type: String },
    
    #[error("Template requires 'n_probs' but Okapi's token_probs capability is disabled. Okapi runner type: {runner_type}. Token probabilities are only available with ollamarunner.")]
    TokenProbsNotSupported { runner_type: String },
    
    #[error("Template requires 'grammar' but Okapi's grammar_native capability is disabled. Okapi runner type: {runner_type}. Grammar constraints are only available with ollamarunner.")]
    GrammarNotSupported { runner_type: String },
    
    #[error("Invalid lexicon term '{term}' - not found in hLexicon. Available terms: {available_terms:?}. Use only canonical hLexicon terms to ensure consistent LLM interpretation.")]
    UnknownLexiconTerm { term: String, available_terms: Vec<String> },
    
    #[error("Confidence threshold {threshold} is outside valid range [0.0, 1.0]. Use a value between 0.0 and 1.0 inclusive.")]
    InvalidConfidenceThreshold { threshold: f64 },
}

/// Contract validator for template registration
pub struct ContractValidator {
    okapi_capabilities: OkapiCapabilities,
    hlexicon_terms: HashSet<String>,
}

impl ContractValidator {
    pub fn new(okapi_capabilities: OkapiCapabilities, hlexicon_terms: Vec<String>) -> Self {
        Self {
            okapi_capabilities,
            hlexicon_terms: hlexicon_terms.into_iter().collect(),
        }
    }
    
    /// Validate template frontmatter at registration time
    pub fn validate(&self, frontmatter: &TemplateFrontmatter) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Validate Okapi requirements based on template type
        if let Some(reqs) = &frontmatter.requires_okapi {
            // Prompt templates require n_probs for confidence routing
            if frontmatter.template_type == TemplateType::Prompt && reqs.n_probs.is_none() {
                errors.push(ValidationError::MissingNProbs {
                    template_type: "Prompt".to_string(),
                });
            }
            
            // Process templates require grammar constraints
            if frontmatter.template_type == TemplateType::Process && reqs.grammar.is_none() {
                errors.push(ValidationError::MissingGrammar);
            }
            
            // Check capability compatibility
            if reqs.n_probs.is_some() && !self.okapi_capabilities.token_probs {
                errors.push(ValidationError::TokenProbsNotSupported {
                    runner_type: self.okapi_capabilities.runner_type.clone(),
                });
            }
            
            if reqs.grammar.is_some() && !self.okapi_capabilities.grammar_native {
                errors.push(ValidationError::GrammarNotSupported {
                    runner_type: self.okapi_capabilities.runner_type.clone(),
                });
            }
            
            if reqs.adapter.is_some() && !self.okapi_capabilities.lora_hot_swap {
                errors.push(ValidationError::AdapterNotSupported {
                    adapter: reqs.adapter.clone().unwrap(),
                    runner_type: self.okapi_capabilities.runner_type.clone(),
                });
            }
        }
        
        // Validate lexicon terms
        for term in &frontmatter.lexicon_terms {
            if !self.hlexicon_terms.contains(term) {
                errors.push(ValidationError::UnknownLexiconTerm {
                    term: term.clone(),
                    available_terms: self.hlexicon_terms.iter().cloned().collect(),
                });
            }
        }
        
        // Validate confidence config
        if let Some(conf) = &frontmatter.confidence {
            if conf.threshold < 0.0 || conf.threshold > 1.0 {
                errors.push(ValidationError::InvalidConfidenceThreshold {
                    threshold: conf.threshold,
                });
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Fetch Okapi capabilities at hKask startup
pub async fn fetch_okapi_capabilities(okapi_base_url: &str) -> Result<OkapiCapabilities, ValidatorError> {
    let response = reqwest::get(&format!("{}/api/engine/status", okapi_base_url))
        .await?
        .json()
        .await?;
    
    Ok(response)
}

#[derive(Debug, Error)]
pub enum ValidatorError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON parse error: {0}")]
    ParseError(String),
}
```

### IV.5: Registration Integration

```rust
// hkask-templates/src/registry.rs
impl TemplateRegistry {
    pub fn register_template(
        &mut self,
        template: &Template,
        validator: &ContractValidator,
    ) -> Result<(), RegistrationError> {
        // Parse frontmatter
        let frontmatter: TemplateFrontmatter = template.parse_frontmatter()
            .map_err(|e| RegistrationError::ParseError(e.to_string()))?;
        
        // Validate contract
        if let Err(errors) = validator.validate(&frontmatter) {
            return Err(RegistrationError::ValidationFailed {
                template_id: template.id.clone(),
                errors,
            });
        }
        
        // Store validated template
        self.templates.insert(template.id.clone(), template.clone());
        
        tracing::info!("Template registered: {}", template.id);
        
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Validation failed for template '{template_id}': {errors:?}")]
    ValidationFailed {
        template_id: String,
        errors: Vec<ValidationError>,
    },
}

// Example error output:
// Error: Template registration failed for "factual-qa-v1":
//   - Template type 'Prompt' requires 'n_probs' in requires_okapi, but it was not specified. Add 'n_probs: 5' to enable token probability-based confidence routing.
//   - Invalid lexicon term 'answer' - not found in hLexicon. Available terms: ["classify", "discriminate", "route", "recognize"]. Use only canonical hLexicon terms to ensure consistent LLM interpretation.
```

---

## Part V: Implementation Checklist

### V.1: P0-1 — CNS Span Translator

- [ ] Create `hkask-mcp-inference/src/metrics_translator.rs`
- [ ] Implement SSE stream subscription
- [ ] Implement delta-only span emission
- [ ] Wire to `hkask-cns` span consumer
- [ ] Add unit tests for delta detection
- [ ] Add integration test with mock Okapi SSE server

### V.2: P0-2 — Confidence-Based Router

- [ ] Create `hkask-ensemble/src/confidence_router.rs`
- [ ] Implement confidence formula (avg × (1 - sqrt(variance)))
- [ ] Implement escalation logic
- [ ] Add template frontmatter parsing for confidence config
- [ ] Add CNS span emission for escalation events
- [ ] Add unit tests for confidence calculation
- [ ] Add integration test with mock Okapi client

### V.3: P0-3 — Template Contract Validator

- [ ] Create `hkask-templates/src/contract_validator.rs`
- [ ] Implement frontmatter schema parsing
- [ ] Implement Okapi capabilities fetcher
- [ ] Implement validation rules with actionable errors
- [ ] Wire to template registration flow
- [ ] Add unit tests for each validation rule
- [ ] Add integration test with mock Okapi server

### V.4: Documentation

- [ ] Update `hkask-mcp-inference/README.md` with CNS translator docs
- [ ] Update `hkask-ensemble/README.md` with confidence router docs
- [ ] Update `hkask-templates/README.md` with contract validator docs
- [ ] Add example template frontmatter to docs/
- [ ] Document Okapi integration in hKask architecture docs

---

## Part VI: Testing Strategy

### VI.1: Unit Tests

| Component | Test Cases |
|-----------|------------|
| `compute_confidence()` | High confidence (low variance), low confidence (high variance), empty probs |
| `emit_delta_spans()` | No change (no spans), single metric change, multiple metric changes |
| `validate()` | Missing n_probs, missing grammar, capability mismatch, unknown lexicon term |

### VI.2: Integration Tests

| Test | Mock | Assertion |
|------|------|-----------|
| CNS translator | Mock Okapi SSE server | Spans emitted on delta only |
| Confidence router | Mock Okapi client | Escalation on low confidence |
| Contract validator | Mock Okapi `/api/engine/status` | Validation errors for mismatched capabilities |

### VI.3: End-to-End Tests

| Test | Setup | Assertion |
|------|-------|-----------|
| Template registration | Valid frontmatter | Template registered successfully |
| Template registration | Invalid frontmatter | Registration rejected with actionable errors |
| Confidence routing | Okapi with token probs | Escalation to larger model when confidence < threshold |

---

## Part VII: Okapi Configuration for hKask

### VII.1: Required Environment Variables

```bash
# Set these when running Okapi for hKask integration
export OLLAMA_HOST=127.0.0.1:11435  # Okapi port (not Ollama's 11434)
export OKAPI_SIMPLE_ENGINE=1        # Required for hKask lifecycle API
export OLLAMA_KEEP_ALIVE=5m         # Model idle timeout (hKask can override)
```

### VII.2: hKask Configuration

```yaml
# hKask config.yaml
inference:
  okapi_base_url: "http://127.0.0.1:11435"
  default_confidence_threshold: 0.75
  default_escalate_to_model: "qwen3:70b"

cns:
  span_buffer_size: 100
  variety_counter_window: 60s  # Algedonic alert if variety deficit > 100
```

---

## Part VIII: Future Work (Post-P0)

### P1: RDF Embedding with LLM-Calculated Confidence

- Implement confidence rubric in RDF extraction template
- Parse multi-dimensional confidence (accuracy, completeness, ambiguity)
- Store confidence in `hkask-storage` triples table

### P2: MoE Expert Dynamic Tracking

- Wait for llama.cpp instrumentation (per-token expert activation)
- Implement Okapi metrics endpoint for expert activations
- Implement hKask CNS monitor for hot expert detection

### P2: Speculative Decoding Curator

- Implement template-declared latency budget
- Implement draft model selection logic
- Implement adaptive n_draft tuning

---

## Appendix A: Okapi File Reference

| File | Purpose | hKask Relevance |
|------|---------|-----------------|
| `server/routes_kask.go` | hKask lifecycle API | Reference for `/api/load`, `/api/unload` |
| `server/capabilities.go` | Runner capability reporting | Source of `/api/engine/status` response |
| `server/metrics.go` | Prometheus + SSE metrics | Source of `/api/metrics/stream` |
| `runner/ollamarunner/probs.go` | Token probability calculation | Source of `completion_probabilities` |
| `api/types.go` | Options, Runner, Request/Response types | Reference for request/response schemas |

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| **Okapi** | Inference engine (Go + llama.cpp), hKask's backend |
| **ollamarunner** | Pure Go runner (full Okapi features) |
| **llamarunner** | C++ subprocess runner (basic inference only) |
| **LoRA** | Low-Rank Adaptation (adapter fine-tuning) |
| **CNS** | Cybernetic Nervous System (hKask observability) |
| **ν-event** | Cybernetic audit trail event |
| **Algedonic alert** | Variety deficit → escalation signal |
| **Matroshka depth** | Template recursion limit (default: 7) |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Phase 0 Implementation Plan — Okapi Integration*
*Rust = loom, YAML/Jinja2 = thread, Okapi = inference*
*MVP in progress.*
