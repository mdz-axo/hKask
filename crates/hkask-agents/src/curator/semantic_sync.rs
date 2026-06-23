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
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;
use tracing;

/// Cross-agent artifact index — maps agent names to their published artifacts.
/// Built by CuratorSync from manifest.json files in agent directories.
#[derive(Debug, Clone, Default)]
pub struct ArtifactIndex {
    pub artifacts: HashMap<String, Vec<ArtifactEntry>>,
}

/// A single published artifact entry from an agent's manifest.
#[derive(Debug, Clone)]
pub struct ArtifactEntry {
    pub artifact_type: String,
    pub name: String,
    pub hash: String,
    pub published_at: String,
}

/// Derive the SQLCipher passphrase for a pod database.
///
/// Tries the OCAP secret derivation first (same as PodFactory), then falls
/// back to the DB passphrase (same as MCP servers use for memory.db). This
/// dual-path resolution handles both pod.db (OCAP-encrypted) and memory.db
/// (DB-passphrase-encrypted) from the same webid sidecar.
fn derive_passphrase(db_path: &Path) -> Result<String, String> {
    // Resolve the webid — try pod.webid first, then the file's own .webid
    let webid_str = resolve_webid_for_db(db_path)?;
    let webid: WebID = webid_str
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse WebID: {e}"))?;

    // Try OCAP secret derivation (used by pod.db)
    let context = format!(
        "{}:{}",
        hkask_types::secret::derivation_contexts::OCAP_SECRET,
        webid
    );
    let secret_ref = hkask_types::secret::SecretRef::derived(
        hkask_types::secret::derivation_contexts::MASTER_KEY_ENV,
        &context,
    );
    if let Ok(bytes) = hkask_keystore::resolve(&secret_ref) {
        return Ok(hex::encode(&*bytes));
    }

    // Fall back to DB passphrase (used by memory.db via MCP server credential chain)
    match hkask_keystore::keychain::resolve_db_passphrase() {
        Ok(bytes) => Ok(hex::encode(&*bytes)),
        Err(e) => Err(format!("Key derivation failed (OCAP + DB passphrase): {e}")),
    }
}

/// Resolve the webid string for a database file.
///
/// Tries the file's own .webid sidecar first, then falls back to the sibling
/// pod.webid in the same agent directory (used by memory.db which shares the
/// pod's webid).
fn resolve_webid_for_db(db_path: &Path) -> Result<String, String> {
    let own_webid = db_path.with_extension("webid");
    if own_webid.exists() {
        return std::fs::read_to_string(&own_webid)
            .map_err(|e| format!("Failed to read webid file {:?}: {e}", own_webid));
    }
    if let Some(parent) = db_path.parent() {
        let pod_webid = parent.join("pod.webid");
        if pod_webid.exists() {
            return std::fs::read_to_string(&pod_webid)
                .map_err(|e| format!("Failed to read pod webid {:?}: {e}", pod_webid));
        }
    }
    Err(format!(
        "No .webid sidecar found for {:?} or sibling pod.webid",
        db_path
    ))
}

/// The Curator's sync engine.
///
/// Owns a reference to the shared SemanticIndex (the same Arc that
/// all PodContexts read from). Each tick: scans pods, opens each
/// source pod's database read-only, queries new public triples since
/// cursor, inserts into index, advances cursor. Also scans agent
/// manifest.json files for cross-agent artifact discovery.
pub struct CuratorSync {
    /// Shared SemanticIndex — writes here, PodContext reads from here
    index: Arc<std::sync::RwLock<SemanticIndex>>,
    /// Pod registry for scanning active pods
    registry: Arc<PodRegistry>,
    /// Polling interval
    interval: Duration,
    /// Consecutive tick failures — escalates to CNS alert after threshold
    consecutive_failures: std::sync::atomic::AtomicU64,
    /// Cross-agent artifact index — agent_name → published artifacts
    artifact_index: Arc<std::sync::RwLock<ArtifactIndex>>,
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
            artifact_index: Arc::new(std::sync::RwLock::new(ArtifactIndex::default())),
        }
    }

    /// Get a reference to the cross-agent artifact index.
    pub fn artifact_index(&self) -> Arc<std::sync::RwLock<ArtifactIndex>> {
        Arc::clone(&self.artifact_index)
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

    /// Single sync tick — polls all source pods for new public triples
    /// from both pod.db (episodic/semantic store) and memory.db (MCP tool store).
    async fn tick(&self) -> Result<(), String> {
        let pods = self.registry.scan_by_kind().map_err(|e| e.to_string())?;

        for (kind, stem, db_path) in &pods {
            // Skip the CuratorPod itself — it IS the index
            if *kind == PodKind::Curator {
                continue;
            }

            // Derive deterministic PodID from kind + original agent name.
            // This matches PodFactory which uses format!("{}:{}", pod_kind, persona.agent.name).
            let pod_id = PodID::from_name(&format!("{}:{}", kind, stem));

            match self.sync_pod(pod_id, db_path).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::debug!(
                            target: "hkask.curator.sync",
                            pod_id = %pod_id,
                            new_triples = count,
                            "Synced triples from pod.db"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.curator.sync",
                        pod_id = %pod_id,
                        error = %e,
                        "Failed to sync pod.db — will retry next tick"
                    );
                }
            }

            // Phase 1: Also sync public semantic triples from memory.db.
            // MCP tools (memory, condenser, research, etc.) write experiences
            // to the agent's memory database, and public semantic triples
            // there need to reach the Curator's index just like pod.db triples.
            let memory_db = db_path.parent().map(|p| p.join("memory.db"));
            if let Some(ref mem_path) = memory_db
                && mem_path.exists()
            {
                // Use a shifted PodID namespace for memory.db triples so
                // cursors don't collide with pod.db cursors for the same agent.
                let mem_pod_id = PodID::from_name(&format!("memory:{}", pod_id));
                match self.sync_pod(mem_pod_id, mem_path).await {
                    Ok(count) => {
                        if count > 0 {
                            tracing::debug!(
                                target: "hkask.curator.sync",
                                pod_id = %pod_id,
                                new_triples = count,
                                "Synced triples from memory.db"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "hkask.curator.sync",
                            pod_id = %pod_id,
                            error = %e,
                            "Failed to sync memory.db — will retry next tick"
                        );
                    }
                }
            }
        }

        // Reset failure counter on successful tick
        self.consecutive_failures
            .store(0, std::sync::atomic::Ordering::Relaxed);
        // CNS: curator sync completed — variety signal per agent count
        tracing::info!(target: "cns.curator.sync", pod_count = pods.len(), "CNS");

        // Phase 2: Sync artifact manifests from agent directories.
        // Reads manifest.json files to build the cross-agent artifact index.
        self.sync_artifacts();

        Ok(())
    }

    /// Scan agent directories for manifest.json files and rebuild the
    /// cross-agent artifact index. Called at the end of each sync tick.
    fn sync_artifacts(&self) {
        let agents_dir = std::path::Path::new(hkask_types::agent_paths::AGENTS_DIR);
        if !agents_dir.exists() {
            return;
        }
        let mut new_index: HashMap<String, Vec<ArtifactEntry>> = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(agents_dir) {
            for entry in entries.flatten() {
                let agent_dir = entry.path();
                if !agent_dir.is_dir() {
                    continue;
                }
                let manifest_path = agent_dir.join("manifest.json");
                if !manifest_path.exists() {
                    continue;
                }
                let agent_name = agent_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                if let Ok(content) = std::fs::read_to_string(&manifest_path)
                    && let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&content)
                    && let Some(artifact_list) =
                        manifest.get("artifacts").and_then(|a| a.as_array())
                {
                    let entries: Vec<ArtifactEntry> = artifact_list
                        .iter()
                        .filter_map(|a| {
                            Some(ArtifactEntry {
                                artifact_type: a.get("type")?.as_str()?.to_string(),
                                name: a.get("name")?.as_str()?.to_string(),
                                hash: a.get("hash")?.as_str()?.to_string(),
                                published_at: a.get("published_at")?.as_str()?.to_string(),
                            })
                        })
                        .collect();
                    if !entries.is_empty() {
                        tracing::debug!(
                            target: "hkask.curator.artifacts",
                            agent = %agent_name,
                            count = entries.len(),
                            "Indexing agent artifacts"
                        );
                        new_index.insert(agent_name, entries);
                    }
                }
            }
        }

        // Swap in the new index atomically
        if let Ok(mut idx) = self.artifact_index.write() {
            *idx = ArtifactIndex {
                artifacts: new_index,
            };
        }
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
    /// Open a pod's SQLCipher database.
    #[allow(dead_code)]
    fn open_read_only(&self, db_path: &Path) -> Result<Database, String> {
        open_source_db(db_path)
    }
}
