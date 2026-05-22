//! Security Hardening — Input Validation and Rate Limiting
//!
//! This module provides:
//! - **Input Validation**: Schema-based validation for pod operations
//! - **Rate Limiting**: Token bucket algorithm for abuse prevention
//! - **OCAP Enhancement**: Attenuation history tracking and expiry enforcement

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Input validation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Field too long: {field} (max {max} chars)")]
    FieldTooLong { field: String, max: usize },
    #[error("Invalid format: {field}")]
    InvalidFormat { field: String },
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Input validator trait for pod operations
pub trait InputValidator<T> {
    fn validate(&self, input: &T) -> ValidationResult<()>;
}

/// Agent persona input for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersonaInput {
    pub name: String,
    pub agent_type: String,
    pub version: String,
    pub description: String,
    pub editor: String,
    pub capabilities: Vec<String>,
}

impl InputValidator<AgentPersonaInput> for AgentPersonaInput {
    fn validate(&self, input: &AgentPersonaInput) -> ValidationResult<()> {
        // Name validation: 1-64 chars, alphanumeric + hyphens
        if input.name.is_empty() {
            return Err(ValidationError::MissingField("name".to_string()));
        }
        if input.name.len() > 64 {
            return Err(ValidationError::FieldTooLong {
                field: "name".to_string(),
                max: 64,
            });
        }
        if !input
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ValidationError::InvalidFormat {
                field: "name".to_string(),
            });
        }

        // Agent type validation
        if !["bot", "replicant"].contains(&input.agent_type.as_str()) {
            return Err(ValidationError::InvalidFormat {
                field: "agent_type".to_string(),
            });
        }

        // Version validation: semver-like
        if input.version.is_empty() || input.version.len() > 32 {
            return Err(ValidationError::InvalidFormat {
                field: "version".to_string(),
            });
        }

        // Description validation: optional but max 1000 chars
        if input.description.len() > 1000 {
            return Err(ValidationError::FieldTooLong {
                field: "description".to_string(),
                max: 1000,
            });
        }

        // Editor validation
        if input.editor.is_empty() || input.editor.len() > 256 {
            return Err(ValidationError::InvalidFormat {
                field: "editor".to_string(),
            });
        }

        // Capabilities validation: max 20 capabilities, each max 128 chars
        if input.capabilities.len() > 20 {
            return Err(ValidationError::InvalidFormat {
                field: "capabilities".to_string(),
            });
        }
        for cap in &input.capabilities {
            if cap.len() > 128 {
                return Err(ValidationError::FieldTooLong {
                    field: "capability".to_string(),
                    max: 128,
                });
            }
        }

        Ok(())
    }
}

/// Token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    pub fn consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    pub fn available(&self) -> f64 {
        self.tokens
    }
}

/// Rate limiter for pod operations
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    default_max_tokens: f64,
    default_refill_rate: f64,
}

impl RateLimiter {
    pub fn new(default_max_tokens: f64, default_refill_rate: f64) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            default_max_tokens,
            default_refill_rate,
        }
    }

    pub async fn acquire(&self, key: &str, tokens: f64) -> Result<(), ValidationError> {
        let mut buckets = self.buckets.write().await;
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(self.default_max_tokens, self.default_refill_rate));

        if bucket.consume(tokens) {
            Ok(())
        } else {
            Err(ValidationError::RateLimitExceeded)
        }
    }

    pub async fn get_available(&self, key: &str) -> f64 {
        let buckets = self.buckets.read().await;
        buckets
            .get(key)
            .map(|b| b.available())
            .unwrap_or(self.default_max_tokens)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(10.0, 1.0) // 10 requests burst, 1 request/second refill
    }
}

/// OCAP attenuation history tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttenuationHistory {
    pub root_nonce: String,
    pub chain: Vec<AttenuationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttenuationEntry {
    pub delegated_from: String,
    pub delegated_to: String,
    pub timestamp: i64,
    pub attenuation_level: u8,
}

impl AttenuationHistory {
    pub fn new(root_nonce: String) -> Self {
        Self {
            root_nonce,
            chain: vec![],
        }
    }

    pub fn add_entry(
        &mut self,
        delegated_from: String,
        delegated_to: String,
        timestamp: i64,
        attenuation_level: u8,
    ) {
        self.chain.push(AttenuationEntry {
            delegated_from,
            delegated_to,
            timestamp,
            attenuation_level,
        });
    }

    pub fn verify_chain(&self) -> bool {
        // Verify attenuation levels are sequential
        for i in 1..self.chain.len() {
            if self.chain[i].attenuation_level != self.chain[i - 1].attenuation_level + 1 {
                return false;
            }
        }
        true
    }

    pub fn chain_length(&self) -> usize {
        self.chain.len()
    }
}

/// Capability expiry enforcer
pub struct ExpiryEnforcer {
    max_lifetime: Duration,
}

impl ExpiryEnforcer {
    pub fn new(max_lifetime: Duration) -> Self {
        Self { max_lifetime }
    }

    pub fn calculate_expiry(&self, creation_time: i64) -> i64 {
        creation_time + self.max_lifetime.as_secs() as i64
    }

    pub fn is_expired(&self, expires_at: i64, current_time: i64) -> bool {
        current_time > expires_at
    }

    pub fn validate_expiry(&self, expires_at: i64, current_time: i64) -> ValidationResult<()> {
        if self.is_expired(expires_at, current_time) {
            Err(ValidationError::InvalidInput(
                "Capability token has expired".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn max_lifetime_secs(&self) -> u64 {
        self.max_lifetime.as_secs()
    }
}

impl Default for ExpiryEnforcer {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600)) // 1 hour default
    }
}

/// Security context for pod operations
#[derive(Default)]
pub struct SecurityContext {
    pub rate_limiter: RateLimiter,
    pub expiry_enforcer: ExpiryEnforcer,
}

impl SecurityContext {
    pub fn new(rate_limiter: RateLimiter, expiry_enforcer: ExpiryEnforcer) -> Self {
        Self {
            rate_limiter,
            expiry_enforcer,
        }
    }
}
