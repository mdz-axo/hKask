//! CuratorSync — Lazy one-way semantic sync loop
//!
//! Polls source pods' triples tables on each tick, inserts new public
//! triples into the CuratorPod's SemanticIndex. Cursor-based incremental
//! sync — only fetches triples published since last poll.
//!
//! ## Protocol
//!
//! Push-then-pull: pod writes local → fires CNS event →
//! Curator polls pod's table (this module is the poll side).
//!
//! ## Consistency
//!
//! Eventual, bounded by polling interval (~1 second).
//! On CuratorPod restart: cursor-based catch-up replays all triples
//! published since last cursor. On source pod deletion: skip, advance cursor.
//!
//! ## Principles
//!
//! \[P1\] User Sovereignty — Curator opens pods read-only, never writes
//! \[P4\] Clear Boundaries — deterministic passphrase, OCAP gating
//! \[P5\] Essentialism — 1 struct, 1 loop, no new crates
//! \[P9\] Homeostasis — polling loop is the regulation cycle
//! \[P11\] Digital Sphere — only Public triples are synced

use crate::PodID;
use crate::PodKind;
use crate::PodRegistry;
use crate::curator::SemanticIndex;
use hkask_storage::Database;
use hkask_types::{Visibility, WebID};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;
use tracing;

/// Derive the SQLCipher passphrase for a pod from its .webid sidecar file.
/// The .webid file is the bootstrapping mechanism — you can't read the database
/// without the passphrase, and you can't derive the passphrase without the webid.
/// Same derivation as PodFactory::create_pod_storage (HKDF-SHA256 from master key).
fn derive_passphrase(db_path: &Path) -> Result<String, String> {
    let webid_path = db_path.with_extension("webid");
    let webid_str = std::fs::read_to_string(&webid_path)
        .map_err(|e| format!("Failed to read webid file {:?}: {e}", webid_path))?;
    let webid: WebID = webid_str
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse WebID from {:?}: {e}", webid_path))?;
    let context = format!(
        "{}:{}",
        hkask_types::secret::derivation_contexts::OCAP_SECRET,
        webid
    );
    let secret_ref = hkask_types::secret::SecretRef::derived(
        hkask_types::secret::derivation_contexts::MASTER_KEY_ENV,
        &context,
    );
    let bytes =
        hkask_keystore::resolve(&secret_ref).map_err(|e| format!("Key derivation failed: {e}"))?;
    Ok(hex::encode(&*bytes))
}

/// The Curator's sync engine.
///
/// Owns a reference to the shared SemanticIndex (the same Arc that
/// all PodContexts read from). Each tick: scans pods, opens each
/// source pod's database read-only, queries new public triples since
/// cursor, inserts into index, advances cursor.
pub struct CuratorSync {
    /// Shared SemanticIndex — writes here, PodContext reads from here
    index: Arc<std::sync::RwLock<SemanticIndex>>,
    /// Pod registry for scanning active pods
    registry: Arc<PodRegistry>,
    /// Polling interval
    interval: Duration,
    /// Consecutive tick failures — escalates to CNS alert after threshold
    consecutive_failures: std::sync::atomic::AtomicU64,
}

impl CuratorSync {
    /// Create a new CuratorSync.
    ///
    /// `index` must be the same Arc that ActivePods.curator_index points to.
    pub fn new(index: Arc<std::sync::RwLock<SemanticIndex>>, registry: Arc<PodRegistry>) -> Self {
        Self {
            index,
            registry,
            interval: Duration::from_secs(1),
            consecutive_failures: AtomicU64::new(0),
        }
    }

    /// Run the sync loop — polls source pods' triples tables on each tick.
    /// Returns when the provided cancellation token fires.
    pub async fn run(&self, mut cancel: tokio::sync::watch::Receiver<bool>) {
        tracing::info!(
            target: "hkask.curator.sync",
            "Curator sync loop started — polling every {:?}",
            self.interval
        );

        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.interval) => {
                    if let Err(e) = self.tick().await {
                        let failures = self.consecutive_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        tracing::warn!(
                            target: "hkask.curator.sync",
                            error = %e,
                            consecutive_failures = failures,
                            "Curator sync tick failed"
                        );
                        // Escalate after 10 consecutive failures (~10s)
                        if failures >= 10 {
                            tracing::error!(
                                target: "cns.curator.sync.degraded",
                                consecutive_failures = failures,
                                "CURATOR_SYNC_DEGRADED: {} consecutive sync failures — check passphrase derivation and pod availability",
                                failures
                            );
                        }
                    }
                }
                _ = cancel.changed() => {
                    tracing::info!(target: "hkask.curator.sync", "Curator sync loop stopped");
                    return;
                }
            }
        }
    }

    /// Single sync tick — polls all source pods for new public triples.
    async fn tick(&self) -> Result<(), String> {
        let pods = self.registry.scan_by_kind().map_err(|e| e.to_string())?;

        for (kind, stem, db_path) in &pods {
            // Skip the CuratorPod itself — it IS the index
            if *kind == PodKind::Curator {
                continue;
            }

            // Derive deterministic PodID matching PodFactory (kind:name format).
            // Filename uses dots (replicant.alice.db), factory uses colon (replicant:alice).
            let pod_id = if let Some(dot_pos) = stem.find('.') {
                PodID::from_name(&format!("{}:{}", &stem[..dot_pos], &stem[dot_pos + 1..]))
            } else {
                PodID::from_name(stem)
            };

            match self.sync_pod(pod_id, db_path).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::debug!(
                            target: "hkask.curator.sync",
                            pod_id = %pod_id,
                            new_triples = count,
                            "Synced triples from source pod"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.curator.sync",
                        pod_id = %pod_id,
                        error = %e,
                        "Failed to sync pod — will retry next tick"
                    );
                }
            }
        }

        // Reset failure counter on successful tick
        self.consecutive_failures
            .store(0, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    /// Open a source pod's database read-only, query new public triples
    /// since last cursor, insert into SemanticIndex, advance cursor.
    /// Uses spawn_blocking for database I/O to avoid blocking the tokio worker.
    async fn sync_pod(&self, pod_id: PodID, db_path: &Path) -> Result<usize, String> {
        // Get current cursor for this pod
        let cursor = {
            let index = self.index.read().unwrap();
            index.cursor_for(&pod_id)
        };

        let db_path = db_path.to_path_buf();
        let index = Arc::clone(&self.index);
        tokio::task::spawn_blocking(move || {
            let db = open_source_db(&db_path)?;

            let query = "SELECT rowid, entity, attribute, value, confidence FROM triples WHERE rowid > ?1 AND visibility = 'public' ORDER BY rowid ASC";
            let rows: Vec<(i64, String, String, String, f64)> = {
                let conn_arc = db.conn_arc();
                let conn = conn_arc.lock().map_err(|e| format!("Failed to lock pod DB: {e}"))?;
                let mut stmt = conn.prepare(query).map_err(|e| format!("Failed to prepare query: {e}"))?;
                stmt.query_map(rusqlite::params![cursor as i64], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                })
                .map_err(|e| format!("Failed to query triples: {e}"))?
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>()
            };

            if rows.is_empty() {
                return Ok(0);
            }

            let mut new_cursor = cursor;
            let mut count = 0;
            let mut idx = index.write().unwrap();

            for (rowid, entity, attribute, value_str, confidence) in &rows {
                let value: serde_json::Value = serde_json::from_str(value_str)
                    .unwrap_or(serde_json::Value::String(value_str.to_string()));
                let conf: hkask_types::Confidence = (*confidence).into();
                let triple = hkask_storage::Triple::new(entity, attribute, value, hkask_types::WebID::default())
                    .with_confidence(conf)
                    .with_visibility(Visibility::Public);
                idx.insert(&triple, pod_id).map_err(|e| format!("Failed to insert triple: {e}"))?;
                new_cursor = (*rowid) as u64;
                count += 1;
            }

            idx.advance_cursor(pod_id, new_cursor);

            if count > 0 {
                tracing::info!(
                    target: "hkask.curator.sync",
                    pod_id = %pod_id,
                    new_triples = count,
                    cursor = new_cursor,
                    "Curator synced semantic triples"
                );
            }

            Ok(count)
        })
        .await
        .map_err(|e| format!("spawn_blocking join error: {e}"))?
    }
}

/// Open a pod's SQLCipher database. Free function so it can be called from spawn_blocking.
fn open_source_db(db_path: &Path) -> Result<Database, String> {
    let passphrase = derive_passphrase(db_path)?;
    let path_str = db_path.to_string_lossy().to_string();
    Database::open(&path_str, &passphrase).map_err(|e| format!("Failed to open pod DB: {e}"))
}

impl CuratorSync {
    /// Open a pod's SQLCipher database (kept for backward compatibility).
    #[allow(dead_code)]
    fn open_read_only(&self, db_path: &Path) -> Result<Database, String> {
        open_source_db(db_path)
    }
}
