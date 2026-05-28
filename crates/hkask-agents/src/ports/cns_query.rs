//! CNS Query Types — Domain-native types for CNS observability
//!
//! Domain-native types (`HealthStatus`, `AlertInfo`, `AlertLevel`) are defined
//! here so that consumers do not need to depend on `hkask_cns`.

/// Domain-native alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Domain-native alert information
#[derive(Debug, Clone)]
pub struct AlertInfo {
    pub domain: String,
    pub deficit: u64,
    pub threshold: u64,
    pub severity: AlertLevel,
    pub escalated: bool,
    pub message: String,
}

/// Domain-native CNS health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub overall_deficit: u64,
    pub critical_count: usize,
    pub warning_count: usize,
    pub healthy: bool,
}
