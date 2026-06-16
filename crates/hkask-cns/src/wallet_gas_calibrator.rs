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
use hkask_wallet::WalletManager;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Default interval between background wallet-gas calibrations.
pub const DEFAULT_WALLET_CALIBRATION_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Default lookback window for the first calibration pass after construction.
pub const DEFAULT_WALLET_INITIAL_LOOKBACK: ChronoDuration = ChronoDuration::hours(1);

/// Calibrator for the wallet gas→rJoule conversion rate.
///
/// # Public Surface (≤7 items — deep-module discipline)
/// - `WalletGasCalibrator` (struct)
/// - `new()` — construct from event store and wallet manager
/// - `with_initial_lookback()` — configure first-calibration window
/// - `calibrate()` — run one calibration pass
/// - `spawn_calibration()` — spawn a background calibration task
pub struct WalletGasCalibrator {
    store: Arc<NuEventStore>,
    wallet_manager: Arc<WalletManager>,
    estimator: std::sync::Mutex<WalletEnergyEstimator>,
    last_calibrated_at: tokio::sync::Mutex<DateTime<Utc>>,
}

impl WalletGasCalibrator {
    /// Create a wallet gas calibrator backed by the given event store and wallet manager.
    ///
    /// REQ: GAS-CALIB-005 — runtime calibration of wallet gas conversion rate
    /// post: returns WalletGasCalibrator with the manager's current gas_per_rjoule rate
    pub fn new(store: Arc<NuEventStore>, wallet_manager: Arc<WalletManager>) -> Self {
        let initial_rate = wallet_manager.gas_per_rjoule();
        Self {
            store,
            wallet_manager,
            estimator: std::sync::Mutex::new(WalletEnergyEstimator::new(initial_rate)),
            last_calibrated_at: tokio::sync::Mutex::new(
                Utc::now() - DEFAULT_WALLET_INITIAL_LOOKBACK,
            ),
        }
    }

    /// Configure how far back the first calibration pass searches for events.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_initial_lookback(mut self, lookback: ChronoDuration) -> Self {
        let now = Utc::now();
        self.last_calibrated_at = tokio::sync::Mutex::new(now - lookback);
        self
    }

    /// Run one incremental calibration pass.
    ///
    /// Queries aggregate settled gas over the window since the last calibration,
    /// computes `total_actual / total_reserved`, feeds the ratio to the internal
    /// `WalletEnergyEstimator`, and pushes the resulting `gas_per_rjoule` to the
    /// shared `WalletManager`.
    ///
    /// REQ: GAS-CALIB-005
    /// pre:  `self.store` is a valid NuEventStore; `self.wallet_manager` is valid
    /// post: if settled events exist and the aggregate ratio exceeds tolerance,
    ///       `wallet_manager.gas_per_rjoule()` is updated
    /// post: returns true if the rate was adjusted
    pub async fn calibrate(&self) -> Result<bool, InfrastructureError> {
        let until = Utc::now();
        let since = {
            let mut last = self.last_calibrated_at.lock().await;
            let s = *last;
            *last = until;
            s
        };

        let report = GasReport::new(Arc::clone(&self.store));
        let totals = report.query_total(since, until)?;

        // Nothing to calibrate if no gas was reserved.
        if totals.total_reserved == 0 {
            return Ok(false);
        }

        let ratio = totals.total_consumed as f64 / totals.total_reserved as f64;
        let adjusted = self
            .estimator
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?
            .calibrate(ratio);

        if adjusted {
            let new_rate = self
                .estimator
                .lock()
                .map_err(|_| InfrastructureError::LockPoisoned)?
                .gas_per_rjoule;
            self.wallet_manager.set_gas_per_rjoule(new_rate);
            info!(
                target: "cns.wallet.calibration",
                since = %since,
                until = %until,
                ratio = %ratio,
                new_rate = new_rate,
                "Calibrated wallet gas_per_rjoule"
            );
        }

        Ok(adjusted)
    }

    /// Spawn a background task that calls `calibrate()` at the given interval.
    ///
    /// The task runs until the runtime shuts down. Calibration errors are logged
    /// but do not crash the task.
    pub fn spawn_calibration(self: Arc<Self>, interval: Duration) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if let Err(e) = self.calibrate().await {
                    warn!(
                        target: "cns.wallet.calibration",
                        error = %e,
                        "Background wallet gas calibration failed"
                    );
                }
            }
        });
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use hkask_storage::in_memory_db;
    use hkask_types::NuEventSink;
    use hkask_types::WebID;
    use hkask_types::event::{NuEvent, Phase, Span, SpanKind};
    use hkask_wallet::WalletManager;
    use std::collections::HashMap;

    fn make_wallet_manager() -> Arc<WalletManager> {
        // SAFETY: test-only env var set in single-threaded test context.
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }
        let db = in_memory_db();
        let store = Arc::new(hkask_storage::WalletStore::new(db.conn_arc()));
        let manager = WalletManager::build(
            hkask_types::wallet::WalletConfig::default(),
            store,
            HashMap::new(),
            None,
            Arc::new(hkask_wallet::price_feed::StaticPriceFeed::new()),
        )
        .unwrap();
        Arc::new(manager)
    }

    fn settled_event(agent: WebID, reserved: u64, actual: u64) -> NuEvent {
        NuEvent::new(
            agent,
            Span::from_kind(SpanKind::GasSettled),
            Phase::Act,
            serde_json::json!({
                "server": "hkask-mcp-test",
                "tool": "test_tool",
                "reserved": reserved,
                "actual": actual,
                "refunded": reserved.saturating_sub(actual),
            }),
            0,
        )
    }

    // REQ: GAS-CALIB-005 — calibrate updates WalletManager gas_per_rjoule
    #[tokio::test]
    async fn calibrate_updates_wallet_manager_rate() {
        let agent = WebID::new();
        let wallet_manager = make_wallet_manager();
        assert_eq!(wallet_manager.gas_per_rjoule(), 1000);

        let db = in_memory_db();
        let store: Arc<NuEventStore> = Arc::new(NuEventStore::new(db.conn_arc()));
        store
            .persist(&settled_event(agent, 100, 200))
            .expect("persist settled event");

        let calibrator = Arc::new(WalletGasCalibrator::new(
            Arc::clone(&store),
            Arc::clone(&wallet_manager),
        ));
        let adjusted = calibrator.calibrate().await.unwrap();
        assert!(adjusted, "ratio 2.0 should adjust rate");
        assert_eq!(
            wallet_manager.gas_per_rjoule(),
            2000,
            "rate should double from 1000 to 2000"
        );
    }

    // REQ: GAS-CALIB-005 — no settled events leaves rate unchanged
    #[tokio::test]
    async fn calibrate_no_events_leaves_rate_unchanged() {
        let wallet_manager = make_wallet_manager();
        let db = in_memory_db();
        let store: Arc<NuEventStore> = Arc::new(NuEventStore::new(db.conn_arc()));

        let calibrator = Arc::new(WalletGasCalibrator::new(
            Arc::clone(&store),
            Arc::clone(&wallet_manager),
        ));
        let adjusted = calibrator.calibrate().await.unwrap();
        assert!(!adjusted);
        assert_eq!(wallet_manager.gas_per_rjoule(), 1000);
    }

    // REQ: GAS-CALIB-005 — custom initial lookback is accepted
    #[test]
    fn with_initial_lookback_changes_first_window() {
        let wallet_manager = make_wallet_manager();
        let db = in_memory_db();
        let store: Arc<NuEventStore> = Arc::new(NuEventStore::new(db.conn_arc()));

        let _calibrator = WalletGasCalibrator::new(Arc::clone(&store), Arc::clone(&wallet_manager))
            .with_initial_lookback(ChronoDuration::minutes(30));
        // Construction succeeds; internal state is not directly observable.
        assert_eq!(wallet_manager.gas_per_rjoule(), 1000);
    }
}
