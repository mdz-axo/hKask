//! Pod lifecycle NuEvent emission — removed per Prohibition #2 (stubs are debt).
//!
//! The four `emit_pod_*` functions (emit_pod_event, emit_pod_registered,
//! emit_pod_activated, emit_pod_deactivated) were declared `#[allow(dead_code)]`
//! with zero callers anywhere in the codebase. Deleted 2026-06-21.
//!
//! If pod lifecycle CNS observability is needed, restore with actual call sites
//! wired into PodDeployment, ActivePods::create_pod, and ActivePods::activate_pod.
