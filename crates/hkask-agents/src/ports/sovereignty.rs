//! Sovereignty — Type definitions for sovereignty checking
//!
//! Defines types for sovereignty verification operations.
//! The concrete SovereigntyChecker implements these operations as inherent methods.

/// Sovereignty operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SovereigntyOperation {
    /// Data read operation
    Read,
    /// Data write operation
    Write,
    /// Data acquisition (passive collection)
    Acquisition,
    /// Data composition (combining multiple sources)
    Composition,
}

/// Sovereignty check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SovereigntyCheckResult {
    /// Whether operation is allowed
    pub allowed: bool,
    /// Reason for denial (if any)
    pub denial_reason: Option<String>,
    /// Data category being accessed
    pub data_category: hkask_types::DataCategory,
    /// Operation type
    pub operation: SovereigntyOperation,
}

impl SovereigntyCheckResult {
    pub fn allowed(
        data_category: hkask_types::DataCategory,
        operation: SovereigntyOperation,
    ) -> Self {
        Self {
            allowed: true,
            denial_reason: None,
            data_category,
            operation,
        }
    }

    pub fn denied(
        data_category: hkask_types::DataCategory,
        operation: SovereigntyOperation,
        reason: &str,
    ) -> Self {
        Self {
            allowed: false,
            denial_reason: Some(reason.to_string()),
            data_category,
            operation,
        }
    }
}
