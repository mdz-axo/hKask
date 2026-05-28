pub use crate::security::ValidationError;

pub trait RateLimitPort: Send + Sync {
    fn acquire(&self, key: &str, tokens: f64) -> Result<(), ValidationError>;
    fn available(&self, key: &str) -> f64;
    fn reset(&self, key: &str);
}
