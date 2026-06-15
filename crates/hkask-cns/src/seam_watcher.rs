//! Public Seam Watcher — R7.3's "watcher on the wall" for API contract health.
//!
//! Loads the machine-readable public seam inventory (JSON) at startup,
//! registers per-crate coverage as CNS variety dimensions, detects drift
//! from previous snapshots, and emits CNS spans for observability.
//!
//! Architecture:
//!   inventory.json → SeamWatcher::load() → VarietyMonitor domains
//!       → check_drift() → emit_spans() → CNS → Curator → CLI/REPL
//!
//! Non-fatal by design: if the JSON file is missing, seam watching is
//! silently disabled. The system runs normally without it.

use hkask_types::cns::{SeamCoverage, SeamInventory};
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use hkask_types::id::WebID;
use std::path::PathBuf;
use tracing;

// ── Seam Drift ───────────────────────────────────────────────────────────────

/// Per-crate coverage delta since last snapshot.
///
/// Positive `delta_pct` = coverage improved (more items got REQ tests).
/// Negative `delta_pct` = coverage degraded (items added without tests,
/// or tests removed).
#[derive(Debug, Clone)]
pub struct SeamDrift {
    pub crate_name: String,
    pub previous_coverage_pct: f64,
    pub current_coverage_pct: f64,
    /// Positive = improvement, negative = degradation
    pub delta_pct: f64,
    /// Change in total public item count (positive = items added)
    pub items_added: i64,
    /// Change in covered item count (positive = more tests)
    pub items_covered_delta: i64,
}

// ── Seam Watcher ────────────────────────────────────────────────────────────

/// Watches the public seam — loads inventory, tracks coverage, emits CNS spans.
///
/// R7.3 (the CNS bot) owns this concern. The watcher is initialized during
/// bootstrap Phase 7 (CNS Active) and runs for the lifetime of the daemon.
pub struct SeamWatcher {
    /// Current inventory snapshot
    inventory: SeamInventory,
    /// Previous snapshot for drift detection (None on first load)
    previous: Option<SeamInventory>,
    /// Path to the JSON inventory file
    inventory_path: PathBuf,
}

impl SeamWatcher {
    /// Load inventory from the JSON file at `path`.
    ///
    /// Returns `None` if the file doesn't exist or can't be parsed —
    /// seam watching is non-fatal. The system runs normally without it.
    pub fn load(path: impl Into<PathBuf>) -> Option<Self> {
        let inventory_path: PathBuf = path.into();
        let contents = match std::fs::read_to_string(&inventory_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::info!(
                    target: "cns.architecture.seam",
                    path = %inventory_path.display(),
                    error = %e,
                    "Seam inventory JSON not found — seam watching disabled (non-fatal)"
                );
                return None;
            }
        };

        let inventory: SeamInventory = match serde_json::from_str(&contents) {
            Ok(inv) => inv,
            Err(e) => {
                tracing::warn!(
                    target: "cns.architecture.seam",
                    path = %inventory_path.display(),
                    error = %e,
                    "Seam inventory JSON parse failed — seam watching disabled"
                );
                return None;
            }
        };

        tracing::info!(
            target: "cns.architecture.seam",
            path = %inventory_path.display(),
            crates = %inventory.crates.len(),
            total_items = %inventory.totals.total_items,
            coverage_pct = %inventory.totals.coverage_pct,
            "Seam inventory loaded — R7.3 watching the public seam"
        );

        Some(Self {
            inventory,
            previous: None,
            inventory_path,
        })
    }

    /// Register all per-crate seam domains in the CNS runtime's VarietyMonitor.
    ///
    /// Each crate gets a domain `seam:{crate_name}` with expected variety
    /// set to its current covered item count. The CNS will alert when
    /// covered items drop below this baseline.
    ///
    /// Called once at startup after load. Requires a `CnsRuntime` reference
    /// to access the algedonic manager for set_expected_variety.
    pub fn register_domains(&self, runtime: &crate::runtime::CnsRuntime) {
        for (crate_name, coverage) in &self.inventory.crates {
            let domain = format!("seam:{}", crate_name);
            let expected = coverage.covered_items;

            // Register expected variety — this is the baseline.
            // The algedonic manager will alert when observed variety
            // (covered items in refreshed inventory) drops below this.
            // We use a blocking call since this runs at startup.
            runtime.calibrate_threshold_blocking(&domain, expected);

            tracing::debug!(
                target: "cns.architecture.seam",
                crate_name = %crate_name,
                domain = %domain,
                expected_variety = %expected,
                coverage_pct = %coverage.coverage_pct,
                "Registered seam variety domain"
            );
        }

        tracing::info!(
            target: "cns.architecture.seam",
            domains = %self.inventory.crates.len(),
            "All seam variety domains registered"
        );
    }

    /// Check for drift: compare current inventory against previous snapshot.
    ///
    /// Returns a list of per-crate coverage deltas. On first call (no previous
    /// snapshot), returns an empty vec — no drift to report.
    ///
    /// After calling this, the current inventory becomes the previous snapshot
    /// for the next check.
    pub fn check_drift(&mut self) -> Vec<SeamDrift> {
        let previous = match &self.previous {
            Some(p) => p,
            None => {
                // First check — no previous snapshot to compare against.
                // Store current as baseline for next check.
                self.previous = Some(self.inventory.clone());
                return Vec::new();
            }
        };

        let mut drifts = Vec::new();

        for (crate_name, current) in &self.inventory.crates {
            let prev = match previous.crates.get(crate_name) {
                Some(p) => p,
                None => {
                    // New crate — no previous baseline. Report as informational.
                    drifts.push(SeamDrift {
                        crate_name: crate_name.clone(),
                        previous_coverage_pct: 0.0,
                        current_coverage_pct: current.coverage_pct,
                        delta_pct: current.coverage_pct,
                        items_added: current.total_items as i64,
                        items_covered_delta: current.covered_items as i64,
                    });
                    continue;
                }
            };

            let delta_pct = current.coverage_pct - prev.coverage_pct;
            let items_added = current.total_items as i64 - prev.total_items as i64;
            let items_covered_delta = current.covered_items as i64 - prev.covered_items as i64;

            // Only report if something changed
            if delta_pct != 0.0 || items_added != 0 || items_covered_delta != 0 {
                drifts.push(SeamDrift {
                    crate_name: crate_name.clone(),
                    previous_coverage_pct: prev.coverage_pct,
                    current_coverage_pct: current.coverage_pct,
                    delta_pct,
                    items_added,
                    items_covered_delta,
                });
            }
        }

        // Also detect removed crates
        for (crate_name, prev) in &previous.crates {
            if !self.inventory.crates.contains_key(crate_name) {
                drifts.push(SeamDrift {
                    crate_name: crate_name.clone(),
                    previous_coverage_pct: prev.coverage_pct,
                    current_coverage_pct: 0.0,
                    delta_pct: -prev.coverage_pct,
                    items_added: -(prev.total_items as i64),
                    items_covered_delta: -(prev.covered_items as i64),
                });
            }
        }

        // Store current as new baseline
        self.previous = Some(self.inventory.clone());

        if !drifts.is_empty() {
            tracing::info!(
                target: "cns.architecture.seam.drift",
                drift_count = %drifts.len(),
                "Seam drift detected"
            );
        }

        drifts
    }

    /// Emit CNS spans for the current coverage state.
    ///
    /// - `cns.architecture.seam.coverage` — per-crate coverage ratio
    /// - `cns.architecture.seam.drift` — per-crate delta since last check
    ///
    /// These spans flow through the CNS observer system to the Curator
    /// and appear in `/status` output.
    pub async fn emit_spans(&self, sink: &dyn NuEventSink, drifts: &[SeamDrift]) {
        let webid = WebID::default();

        // Emit coverage spans for each crate
        for (crate_name, coverage) in &self.inventory.crates {
            let span = Span::new(
                SpanNamespace::new("cns.architecture.seam.coverage"),
                crate_name.as_str(),
            );
            let event = NuEvent::new(
                webid,
                span,
                Phase::Compare,
                serde_json::json!({
                    "crate": crate_name,
                    "total_items": coverage.total_items,
                    "covered_items": coverage.covered_items,
                    "uncovered_items": coverage.uncovered_items,
                    "coverage_pct": coverage.coverage_pct,
                    "req_tests": coverage.req_tests,
                    "high_risk_uncovered": coverage.high_risk_uncovered,
                }),
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::debug!(
                    target: "cns.architecture.seam",
                    error = %e,
                    "Failed to persist seam coverage span"
                );
            }
        }

        // Emit drift spans for crates that changed
        for drift in drifts {
            let span = Span::new(
                SpanNamespace::new("cns.architecture.seam.drift"),
                drift.crate_name.as_str(),
            );
            let severity = if drift.delta_pct < -5.0 {
                "critical"
            } else if drift.delta_pct < 0.0 {
                "warning"
            } else {
                "improvement"
            };
            let event = NuEvent::new(
                webid,
                span,
                Phase::Compare,
                serde_json::json!({
                    "crate": drift.crate_name,
                    "previous_coverage_pct": drift.previous_coverage_pct,
                    "current_coverage_pct": drift.current_coverage_pct,
                    "delta_pct": drift.delta_pct,
                    "items_added": drift.items_added,
                    "items_covered_delta": drift.items_covered_delta,
                    "severity": severity,
                }),
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::debug!(
                    target: "cns.architecture.seam",
                    error = %e,
                    "Failed to persist seam drift span"
                );
            }
        }
    }

    /// Refresh the inventory from disk (e.g., after a code update regenerates the JSON).
    ///
    /// Moves current → previous, loads new from disk. Returns the new inventory
    /// if successful, `None` if the file is missing or unparseable.
    pub fn refresh(&mut self) -> bool {
        let contents = match std::fs::read_to_string(&self.inventory_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    target: "cns.architecture.seam",
                    path = %self.inventory_path.display(),
                    error = %e,
                    "Seam inventory refresh failed — file not readable"
                );
                return false;
            }
        };

        let new_inventory: SeamInventory = match serde_json::from_str(&contents) {
            Ok(inv) => inv,
            Err(e) => {
                tracing::warn!(
                    target: "cns.architecture.seam",
                    path = %self.inventory_path.display(),
                    error = %e,
                    "Seam inventory refresh failed — JSON parse error"
                );
                return false;
            }
        };

        // Check if anything actually changed
        let changed = new_inventory.totals.total_items != self.inventory.totals.total_items
            || new_inventory.totals.covered_items != self.inventory.totals.covered_items
            || (new_inventory.totals.coverage_pct - self.inventory.totals.coverage_pct).abs()
                > 0.01;

        self.previous = Some(std::mem::replace(&mut self.inventory, new_inventory));

        if changed {
            tracing::info!(
                target: "cns.architecture.seam",
                total_items = %self.inventory.totals.total_items,
                coverage_pct = %self.inventory.totals.coverage_pct,
                "Seam inventory refreshed — changes detected"
            );
        } else {
            tracing::debug!(
                target: "cns.architecture.seam",
                "Seam inventory refreshed — no changes"
            );
        }

        true
    }

    /// Get a reference to the current inventory.
    pub fn inventory(&self) -> &SeamInventory {
        &self.inventory
    }

    /// Get workspace-wide coverage percentage.
    pub fn overall_coverage(&self) -> f64 {
        self.inventory.totals.coverage_pct
    }

    /// Get per-crate coverage data.
    pub fn crate_coverage(&self, crate_name: &str) -> Option<&SeamCoverage> {
        self.inventory.crates.get(crate_name)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::cns::SeamCoverage;
    use std::collections::HashMap;

    fn make_test_inventory(crates: Vec<(&str, u64, u64, u64)>) -> SeamInventory {
        let mut crate_map = HashMap::new();
        let mut total_items = 0u64;
        let mut total_covered = 0u64;
        let mut total_uncovered = 0u64;

        for (name, total, covered, high_risk) in &crates {
            let uncovered = total - covered;
            let pct = if *total > 0 {
                (*covered as f64 / *total as f64) * 100.0
            } else {
                0.0
            };
            crate_map.insert(
                name.to_string(),
                SeamCoverage {
                    crate_name: name.to_string(),
                    total_items: *total,
                    covered_items: *covered,
                    uncovered_items: uncovered,
                    coverage_pct: pct,
                    req_tests: *covered, // simplified: one REQ per covered item
                    high_risk_uncovered: *high_risk,
                },
            );
            total_items += total;
            total_covered += covered;
            total_uncovered += uncovered;
        }

        let overall_pct = if total_items > 0 {
            (total_covered as f64 / total_items as f64) * 100.0
        } else {
            0.0
        };

        SeamInventory {
            generated: "2026-01-01T00:00:00Z".into(),
            totals: SeamCoverage {
                crate_name: "workspace".into(),
                total_items,
                covered_items: total_covered,
                uncovered_items: total_uncovered,
                coverage_pct: overall_pct,
                req_tests: total_covered,
                high_risk_uncovered: 0,
            },
            crates: crate_map,
        }
    }

    // REQ: svc-cns-seam-001 — check_drift_detects_coverage_drop
    //
    // When covered items decrease, check_drift must report a negative delta_pct.
    #[test]
    fn check_drift_detects_coverage_drop() {
        let inv1 = make_test_inventory(vec![("test-crate", 100, 50, 10)]);
        let inv2 = make_test_inventory(vec![("test-crate", 100, 45, 15)]);

        let mut watcher = SeamWatcher {
            inventory: inv2,
            previous: Some(inv1),
            inventory_path: PathBuf::from("/nonexistent/test.json"),
        };

        let drifts = watcher.check_drift();
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].crate_name, "test-crate");
        assert!(
            drifts[0].delta_pct < 0.0,
            "Coverage drop must have negative delta"
        );
        assert_eq!(drifts[0].items_covered_delta, -5);
    }

    // REQ: svc-cns-seam-002 — check_drift_detects_coverage_improvement
    //
    // When covered items increase, check_drift must report a positive delta_pct.
    #[test]
    fn check_drift_detects_coverage_improvement() {
        let inv1 = make_test_inventory(vec![("test-crate", 100, 40, 20)]);
        let inv2 = make_test_inventory(vec![("test-crate", 100, 55, 10)]);

        let mut watcher = SeamWatcher {
            inventory: inv2,
            previous: Some(inv1),
            inventory_path: PathBuf::from("/nonexistent/test.json"),
        };

        let drifts = watcher.check_drift();
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].crate_name, "test-crate");
        assert!(
            drifts[0].delta_pct > 0.0,
            "Coverage improvement must have positive delta"
        );
        assert_eq!(drifts[0].items_covered_delta, 15);
    }

    // REQ: svc-cns-seam-003 — check_drift_returns_empty_when_no_change
    //
    // When inventory is identical, check_drift must return an empty vec.
    #[test]
    fn check_drift_returns_empty_when_no_change() {
        let inv = make_test_inventory(vec![("test-crate", 100, 50, 10)]);

        let mut watcher = SeamWatcher {
            inventory: inv.clone(),
            previous: Some(inv),
            inventory_path: PathBuf::from("/nonexistent/test.json"),
        };

        let drifts = watcher.check_drift();
        assert!(drifts.is_empty(), "No drift when inventories are identical");
    }

    // REQ: svc-cns-seam-004 — check_drift_first_call_returns_empty
    //
    // On first call (no previous snapshot), check_drift returns empty vec
    // and stores current as baseline.
    #[test]
    fn check_drift_first_call_returns_empty() {
        let inv = make_test_inventory(vec![("test-crate", 100, 50, 10)]);

        let mut watcher = SeamWatcher {
            inventory: inv,
            previous: None,
            inventory_path: PathBuf::from("/nonexistent/test.json"),
        };

        let drifts = watcher.check_drift();
        assert!(
            drifts.is_empty(),
            "First check with no previous must return empty"
        );
        assert!(
            watcher.previous.is_some(),
            "First check must store baseline"
        );
    }

    // REQ: svc-cns-seam-005 — check_drift_detects_new_crate
    //
    // When a new crate appears, it's reported with previous_coverage_pct = 0.0.
    #[test]
    fn check_drift_detects_new_crate() {
        let inv1 = make_test_inventory(vec![("crate-a", 100, 50, 10)]);
        let inv2 = make_test_inventory(vec![("crate-a", 100, 50, 10), ("crate-b", 50, 25, 5)]);

        let mut watcher = SeamWatcher {
            inventory: inv2,
            previous: Some(inv1),
            inventory_path: PathBuf::from("/nonexistent/test.json"),
        };

        let drifts = watcher.check_drift();
        let new_crate_drift: Vec<_> = drifts
            .iter()
            .filter(|d| d.crate_name == "crate-b")
            .collect();
        assert_eq!(new_crate_drift.len(), 1, "New crate must be reported");
        assert_eq!(new_crate_drift[0].previous_coverage_pct, 0.0);
        assert!(new_crate_drift[0].delta_pct > 0.0);
    }

    // REQ: svc-cns-seam-006 — check_drift_detects_removed_crate
    //
    // When a crate is removed, it's reported with current_coverage_pct = 0.0.
    #[test]
    fn check_drift_detects_removed_crate() {
        let inv1 = make_test_inventory(vec![("crate-a", 100, 50, 10), ("crate-b", 50, 25, 5)]);
        let inv2 = make_test_inventory(vec![("crate-a", 100, 50, 10)]);

        let mut watcher = SeamWatcher {
            inventory: inv2,
            previous: Some(inv1),
            inventory_path: PathBuf::from("/nonexistent/test.json"),
        };

        let drifts = watcher.check_drift();
        let removed: Vec<_> = drifts
            .iter()
            .filter(|d| d.crate_name == "crate-b")
            .collect();
        assert_eq!(removed.len(), 1, "Removed crate must be reported");
        assert_eq!(removed[0].current_coverage_pct, 0.0);
        assert!(removed[0].delta_pct < 0.0);
    }

    // REQ: svc-cns-seam-007 — load_returns_none_for_missing_file
    //
    // SeamWatcher::load must return None (not panic) when the JSON file
    // doesn't exist — seam watching is non-fatal.
    #[test]
    fn load_returns_none_for_missing_file() {
        let result = SeamWatcher::load("/nonexistent/path/to/inventory.json");
        assert!(result.is_none(), "Missing file must return None, not panic");
    }

    // REQ: svc-cns-seam-008 — load_parses_valid_json
    //
    // SeamWatcher::load must successfully parse a valid inventory JSON file.
    #[test]
    fn load_parses_valid_json() {
        // Write a minimal valid inventory to a temp file
        let dir = std::env::temp_dir();
        let path = dir.join("test-seam-inventory.json");
        let json = serde_json::json!({
            "generated": "2026-01-01T00:00:00Z",
            "source": "test",
            "purpose": "test",
            "totals": {
                "total_items": 10,
                "covered": 5,
                "uncovered": 5,
                "coverage_pct": 50.0,
                "req_tests": 5,
                "crates_analyzed": 1
            },
            "crates": {
                "test-crate": {
                    "total_items": 10,
                    "covered": 5,
                    "uncovered": 5,
                    "coverage_pct": 50,
                    "req_tests": 5,
                    "high_risk_uncovered": 2,
                    "items": []
                }
            },
            "priority": {
                "total_high_risk_uncovered": 2,
                "top_100": [],
                "per_crate_counts": { "test-crate": 2 }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();

        let result = SeamWatcher::load(&path);
        assert!(result.is_some(), "Valid JSON must parse successfully");
        let watcher = result.unwrap();
        assert_eq!(watcher.overall_coverage(), 50.0);
        assert!(watcher.crate_coverage("test-crate").is_some());

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }
}
