//! Metacognition Store Adapter — Bridges hkask_storage::MetacognitionStore to MetacognitionPort

use crate::ports::{MetacognitionPort, MetacognitionPortError, StoredHealthSnapshot};
use hkask_storage::MetacognitionStore;
use std::sync::Arc;

pub struct MetacognitionStoreAdapter {
    store: Arc<MetacognitionStore>,
}

impl MetacognitionStoreAdapter {
    pub fn new(store: Arc<MetacognitionStore>) -> Self {
        Self { store }
    }
}

impl MetacognitionPort for MetacognitionStoreAdapter {
    fn save_snapshot(&self, snapshot: &StoredHealthSnapshot) -> Result<i64, MetacognitionPortError> {
        let stored = hkask_storage::StoredSnapshot {
            id: 0,
            timestamp: snapshot.timestamp.clone(),
            cns_health: snapshot.cns_health.clone(),
            critical_alerts: snapshot.critical_alerts,
            total_alerts: snapshot.total_alerts,
            variety_counters_json: snapshot.variety_counters_json.clone(),
            bot_reports_json: snapshot.bot_reports_json.clone(),
        };
        self.store
            .save_snapshot(&stored)
            .map_err(|e| MetacognitionPortError::Storage(e.to_string()))
    }

    fn list_snapshots(&self, limit: usize) -> Result<Vec<StoredHealthSnapshot>, MetacognitionPortError> {
        self.store
            .list_snapshots(limit)
            .map(|v| {
                v.into_iter()
                    .map(|s| StoredHealthSnapshot {
                        timestamp: s.timestamp,
                        cns_health: s.cns_health,
                        critical_alerts: s.critical_alerts,
                        total_alerts: s.total_alerts,
                        variety_counters_json: s.variety_counters_json,
                        bot_reports_json: s.bot_reports_json,
                    })
                    .collect()
            })
            .map_err(|e| MetacognitionPortError::Storage(e.to_string()))
    }
}
