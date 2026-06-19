//! WalletGasCalibrator — Runtime calibration of the wallet gas→rJoule rate.
//!
//! Observes aggregate `cns.gas.settled` events via `GasReport`, computes the
//! global actual/estimated gas ratio, and feeds it to `WalletEnergyEstimator`.
//! The resulting `gas_per_rjoule` is pushed into `WalletManager` so that
//! `WalletBackedBudget` uses a live, calibrated conversion rate.
//!
//! This closes the wallet-energy feedback loop (P9): the system's estimate of
//! how much gas a rJoule buys is continuously validated against real settlements.

use crate::gas_report::GasReport;
use crate::wallet_energy_estimator::WalletEnergyEstimator;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use hkask_storage::NuEventStore;
use hkask_types::InfrastructureError;
use hkask_types::WebID;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span};
use hkask_wallet::WalletManager;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Default interval between background wallet-gas calibrations.
///
pub const DEFAULT_WALLET_CALIBRATION_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Default lookback window for the first calibration pass after construction.
///
pub const DEFAULT_WALLET_INITIAL_LOOKBACK: ChronoDuration = ChronoDuration::hours(1);

/// Calibrator for the wallet gas→rJoule conversion rate.
///
/// # Public Surface (≤7 items — deep-module discipline)
/// - `WalletGasCalibrator` (struct)
/// - `new()` — construct from event store and wallet manager
/// - `with_initial_lookback()` — configure first-calibration window
/// - `with_event_sink()` — attach a CNS event sink for calibration spans
/// - `calibrate()` — run one calibration pass
/// - `spawn_calibration()` — spawn a background calibration task
pub struct WalletGasCalibrator {
    store: Arc<NuEventStore>,
    wallet_manager: Arc<WalletManager>,
    estimator: std::sync::Mutex<WalletEnergyEstimator>,
    last_calibrated_at: tokio::sync::Mutex<DateTime<Utc>>,
    event_sink: Option<Arc<dyn NuEventSink>>,
}

impl WalletGasCalibrator {
    /// Create a wallet gas calibrator backed by the given event store and wallet manager.
    ///
    pub fn new(store: Arc<NuEventStore>, wallet_manager: Arc<WalletManager>) -> Self {
        let initial_rate = wallet_manager.gas_per_rjoule();
        Self {
            store,
            wallet_manager,
            estimator: std::sync::Mutex::new(WalletEnergyEstimator::new(initial_rate)),
            last_calibrated_at: tokio::sync::Mutex::new(
                Utc::now() - DEFAULT_WALLET_INITIAL_LOOKBACK,
            ),
            event_sink: None,
        }
    }

    /// Configure how far back the first calibration pass searches for events.
    ///
    /// \[P4\] Constraining: Clear Boundaries — lookback limits calibration scope
    #[must_use = "builder methods must be chained or assigned"]
