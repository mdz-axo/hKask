//! CalibratedEnergyEstimator — Self-regulating per-server gas cost estimator.
//!
//! Wraps [`CompositeEnergyEstimator`] and keeps its per-server table in sync with
//! observed `cns.gas.settled` events via [`DynamicGasTable`] and [`GasReport`].
//! A background calibration task can be spawned with [`Self::spawn_calibration`].
//!
//! This closes the Good Regulator feedback loop (P9): estimates are continuously
//! validated against real settlement data and adjusted by exponential moving average.
//!
//! # Design
//!
//! - `DynamicGasTable` and `CompositeEnergyEstimator` are held behind `std::sync::RwLock`
//!   so the synchronous [`EnergyEstimator::estimate_cost`] can read the current
//!   estimator without crossing an async boundary.
//! - Calibration is async and uses `tokio::sync::Mutex` for `last_calibrated_at`.
//! - Calibration is incremental: each pass processes only events since the last
//!   successful calibration, then rebuilds the `CompositeEnergyEstimator` from the
//!   updated table.

use crate::composite_energy_estimator::CompositeEnergyEstimator;
use crate::dynamic_gas_table::DynamicGasTable;
use crate::gas_report::GasReport;
use crate::governed_tool::EnergyEstimator;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use hkask_storage::NuEventStore;
use hkask_types::InfrastructureError;
use hkask_types::WebID;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span};
use serde_json::Value;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use tracing::{info, warn};

/// Default interval between background calibrations.
///
pub const DEFAULT_CALIBRATION_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Default lookback window for the first calibration after construction.
///
pub const DEFAULT_INITIAL_LOOKBACK: ChronoDuration = ChronoDuration::hours(1);

/// Self-regulating energy estimator that refreshes its per-server table from
/// settled CNS gas events.
///
/// # Public Surface (≤7 items — deep-module discipline)
/// - `CalibratedEnergyEstimator` (struct)
/// - `new()` — construct from an event store
/// - `with_initial_lookback()` — configure first-calibration window
/// - `with_event_sink()` — attach a CNS event sink for calibration spans
/// - `calibrate()` — run one calibration pass
/// - `spawn_calibration()` — spawn a background calibration task
/// - `current_table()` — diagnostic snapshot of the calibrated table
pub struct CalibratedEnergyEstimator {
    store: Arc<NuEventStore>,
    table: RwLock<DynamicGasTable>,
    estimator: RwLock<CompositeEnergyEstimator>,
    last_calibrated_at: tokio::sync::Mutex<DateTime<Utc>>,
    event_sink: Option<Arc<dyn NuEventSink>>,
}

impl CalibratedEnergyEstimator {
    /// Create a calibrated estimator backed by the given event store.
    ///
    pub fn new(store: Arc<NuEventStore>) -> Self {
        let table = DynamicGasTable::new();
        let estimator = CompositeEnergyEstimator::from_dynamic_table(&table);
        Self {
            store,
            table: RwLock::new(table),
            estimator: RwLock::new(estimator),
            last_calibrated_at: tokio::sync::Mutex::new(Utc::now() - DEFAULT_INITIAL_LOOKBACK),
            event_sink: None,
        }
    }

    /// Configure how far back the first calibration pass searches for events.
    ///
    #[must_use = "builder methods must be chained or assigned"]
