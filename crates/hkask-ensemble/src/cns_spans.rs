//! Unified CNS Span Schema for Okapi Integration
//!
//! Defines a consistent ontology for CNS spans emitted by Okapi integration components.

use hkask_types::{NuEvent, Span, WebID};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Unified CNS span types for Okapi integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OkapiCnsSpan {
    /// Token throughput observation
    TokenThroughput { total: i64, delta: i64 },
    /// Context utilization observation
    ContextUtilization {
        kv_cache_tokens: i64,
        context_length: i64,
        utilization_pct: f64,
    },
    /// Adapter swap event
    AdapterSwap { latency_ms: i64 },
    /// GPU memory observation
    GpuMemory { used_bytes: u64, delta: i64 },
    /// Prompt cache hit ratio
    CacheHitRatio { hit_ratio: f64 },
    /// Confidence escalation event
    ConfidenceEscalation {
        initial_confidence: f64,
        threshold: f64,
        primary_model: String,
        escalated_model: String,
    },
    /// Template capability validation
    CapabilityValidation {
        template_id: String,
        validation_result: ValidationResult,
    },
    /// MoE expert placement observation
    MoEExpertPlacement {
        expert_id: u32,
        gpu_id: Option<u32>,
        memory_bytes: u64,
    },
    /// MoE expert activation frequency
    MoEExpertActivation {
        expert_id: u32,
        activation_count: u64,
        window_seconds: u64,
    },
    /// MoE expert co-activation pair
    MoEExpertCoactivation {
        expert_a: u32,
        expert_b: u32,
        coactivation_count: u64,
    },
    /// MoE offload ratio (experts on CPU / total experts)
    MoEOffloadRatio {
        offloaded: u32,
        total: u32,
        ratio: f64,
    },
}

/// Validation result for capability checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub success: bool,
    pub errors: Vec<String>,
}

impl OkapiCnsSpan {
    /// Convert to CNS span namespace string
    pub fn namespace(&self) -> &'static str {
        match self {
            OkapiCnsSpan::TokenThroughput { .. } => "cns.connector.llm.tokens",
            OkapiCnsSpan::ContextUtilization { .. } => "cns.connector.llm.context",
            OkapiCnsSpan::AdapterSwap { .. } => "cns.tool.adapter_swap",
            OkapiCnsSpan::GpuMemory { .. } => "cns.connector.llm.gpu_memory",
            OkapiCnsSpan::CacheHitRatio { .. } => "cns.connector.llm.cache_hit",
            OkapiCnsSpan::ConfidenceEscalation { .. } => "cns.prompt.escalation",
            OkapiCnsSpan::CapabilityValidation { .. } => "cns.prompt.validation",
            OkapiCnsSpan::MoEExpertPlacement { .. } => "cns.connector.llm.moe_experts",
            OkapiCnsSpan::MoEExpertActivation { .. } => "cns.connector.llm.moe_experts",
            OkapiCnsSpan::MoEExpertCoactivation { .. } => "cns.connector.llm.moe_experts",
            OkapiCnsSpan::MoEOffloadRatio { .. } => "cns.connector.llm.moe_experts",
        }
    }

    /// Convert to NuEvent
    pub fn to_nu_event(&self, observer: WebID) -> NuEvent {
        let span = Span::Connector(self.namespace().to_string());
        let observation = json!(self);

        NuEvent::new(observer, span, hkask_types::Phase::Observe, observation, 0)
    }

    /// Create token throughput span
    pub fn token_throughput(total: i64, delta: i64) -> Self {
        OkapiCnsSpan::TokenThroughput { total, delta }
    }

    /// Create context utilization span
    pub fn context_utilization(kv_cache_tokens: i64, context_length: i64) -> Self {
        let utilization_pct = if context_length > 0 {
            (kv_cache_tokens as f64 / context_length as f64) * 100.0
        } else {
            0.0
        };
        OkapiCnsSpan::ContextUtilization {
            kv_cache_tokens,
            context_length,
            utilization_pct,
        }
    }

    /// Create adapter swap span
    pub fn adapter_swap(latency_ms: i64) -> Self {
        OkapiCnsSpan::AdapterSwap { latency_ms }
    }

    /// Create GPU memory span
    pub fn gpu_memory(used_bytes: u64, delta: i64) -> Self {
        OkapiCnsSpan::GpuMemory { used_bytes, delta }
    }

    /// Create cache hit ratio span
    pub fn cache_hit_ratio(hit_ratio: f64) -> Self {
        OkapiCnsSpan::CacheHitRatio { hit_ratio }
    }

    /// Create confidence escalation span
    pub fn confidence_escalation(
        initial_confidence: f64,
        threshold: f64,
        primary_model: String,
        escalated_model: String,
    ) -> Self {
        OkapiCnsSpan::ConfidenceEscalation {
            initial_confidence,
            threshold,
            primary_model,
            escalated_model,
        }
    }

    /// Create capability validation span
    pub fn capability_validation(template_id: String, success: bool, errors: Vec<String>) -> Self {
        OkapiCnsSpan::CapabilityValidation {
            template_id,
            validation_result: ValidationResult { success, errors },
        }
    }

    /// Create MoE expert placement span
    pub fn moe_expert_placement(expert_id: u32, gpu_id: Option<u32>, memory_bytes: u64) -> Self {
        OkapiCnsSpan::MoEExpertPlacement {
            expert_id,
            gpu_id,
            memory_bytes,
        }
    }

    /// Create MoE expert activation span
    pub fn moe_expert_activation(
        expert_id: u32,
        activation_count: u64,
        window_seconds: u64,
    ) -> Self {
        OkapiCnsSpan::MoEExpertActivation {
            expert_id,
            activation_count,
            window_seconds,
        }
    }

    /// Create MoE expert co-activation span
    pub fn moe_expert_coactivation(expert_a: u32, expert_b: u32, coactivation_count: u64) -> Self {
        OkapiCnsSpan::MoEExpertCoactivation {
            expert_a,
            expert_b,
            coactivation_count,
        }
    }

    /// Create MoE offload ratio span with algedonic alert threshold
    pub fn moe_offload_ratio(offloaded: u32, total: u32) -> Self {
        let ratio = if total > 0 {
            offloaded as f64 / total as f64
        } else {
            0.0
        };
        OkapiCnsSpan::MoEOffloadRatio {
            offloaded,
            total,
            ratio,
        }
    }
}
