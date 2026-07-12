//! Formatting helpers for metacognition output.

use hkask_types::cns::CnsHealth;

pub(super) fn format_health_status(h: &CnsHealth) -> String {
    if h.healthy {
        format!(
            "Healthy (deficit={}, warnings={})",
            h.overall_deficit, h.warning_count
        )
    } else {
        format!(
            "Degraded (deficit={}, critical={}, warnings={})",
            h.overall_deficit, h.critical_count, h.warning_count
        )
    }
}
