//! Matrix transport builder — Conduit connection, 7R7 listener, and receptor startup.

use std::sync::Arc;

use hkask_communication::listener::{
    AlgedonicReceptor, ComposerReceptor, ConsolidatorReceptor, CuratorReceptor,
    CyberneticsReceptor, Receptor, ReceptorStore, ReceptorSupervisor, VarietyReceptor,
};
use hkask_types::event::NuEvent;

// ── ReceptorStore adapter ───────────────────────────────────────────────────

/// Adapts `hkask_storage::NuEventStore` to the `ReceptorStore` trait
/// so that 7R7 receptors (r7-2 through r7-7) can query CNS events
/// without depending on the storage crate directly.
pub(crate) struct NuEventStoreReceptorAdapter {
    store: Arc<hkask_storage::nu_event_store::NuEventStore>,
}

impl NuEventStoreReceptorAdapter {
    pub fn new(store: Arc<hkask_storage::nu_event_store::NuEventStore>) -> Self {
        Self { store }
    }
}

impl ReceptorStore for NuEventStoreReceptorAdapter {
    fn query_spans(
        &self,
        since: chrono::DateTime<chrono::Utc>,
        span_prefix: &str,
        limit: u64,
    ) -> Result<Vec<NuEvent>, Box<dyn std::error::Error + Send + Sync>> {
        self.store
            .query_by_prefix(span_prefix, since, limit)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

// ── Matrix transport builder ────────────────────────────────────────────────

pub(crate) async fn build_matrix(
    event_sink: Option<Arc<dyn hkask_types::event::NuEventSink>>,
) -> Option<Arc<tokio::sync::Mutex<hkask_communication::matrix::MatrixTransport>>> {
    let homeserver_url =
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string());
    let keychain = hkask_keystore::Keychain::default();

    let credentials = {
        if let Ok(password) =
            keychain.retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_BOT_CURATOR)
        {
            Some(("@hkask-curator:localhost".to_string(), password))
        } else if let (Ok(username), Ok(password)) = (
            keychain.retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_USERNAME),
            keychain.retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_PASSWORD),
        ) {
            Some((username, password))
        } else if let (Ok(username), Ok(password)) = (
            std::env::var("HKASK_MATRIX_AGENT_USERNAME"),
            std::env::var("HKASK_MATRIX_AGENT_PASSWORD"),
        ) {
            Some((username, password))
        } else {
            None
        }
    };

    match credentials {
        Some((username, password)) => {
            let mut transport = hkask_communication::matrix::MatrixTransport::new(&homeserver_url);
            match transport.login(&username, &password).await {
                Ok(()) => {
                    let transport = Arc::new(tokio::sync::Mutex::new(transport));
                    let mut listener =
                        hkask_communication::listener::SevenR7Listener::new(transport.clone(), 30);
                    if let Some(sink) = event_sink {
                        listener = listener.with_event_sink(sink);
                    }
                    listener.start().await;
                    tracing::info!(
                        target: "cns.communication.matrix.daemon",
                        username = %username,
                        homeserver = %homeserver_url,
                        "Matrix transport connected and 7R7 listener started"
                    );
                    Some(transport)
                }
                Err(e) => {
                    tracing::warn!(
                        target: "cns.communication.matrix.daemon",
                        username = %username,
                        error = %e,
                        "Matrix login failed — Conduit may not be running. Continuing without Matrix."
                    );
                    None
                }
            }
        }
        None => {
            tracing::info!(
                target: "cns.communication.matrix.daemon",
                "No Matrix credentials found in keychain or environment. Continuing without Matrix."
            );
            None
        }
    }
}

// ── 7R7 CNS receptor builder ────────────────────────────────────────────────

/// Build and start all 7R7 CNS-observing receptors (r7-2 through r7-7).
///
/// Each receptor is a passive observer that polls the NuEventStore for
/// domain-specific CNS spans and emits its own observation spans. Zero
/// receptors classify, escalate, moderate, or judge — they are dumb pipes.
///
/// Receptors are independent of Matrix — they observe CNS state directly.
/// r7-1 (Matrix observer) is started separately by `build_matrix()`.
///
/// Interval mapping:
/// - r7-2 Variety: 60s (alert patterns change quickly)
/// - r7-3 Algedonic: 60s (alert patterns)
/// - r7-4 Composer: 300s (composition changes are slow)
/// - r7-5 Consolidator: 300s (memory patterns are slow)
/// - r7-6 Cybernetics: 120s (infra health)
/// - r7-7 Curator: 120s (curation activity)
///
/// expect: "The system provides cybernetic observability through CNS spans"
/// pre:  nu_event_store is a live NuEventStore with the nu_events schema
/// pre:  event_sink is the same sink used by r7-1 for CNS span persistence
/// post: all six CNS-observing receptors started; Err if store or sink is missing
pub(crate) async fn build_and_start_receptors(
    nu_event_store: Arc<hkask_storage::nu_event_store::NuEventStore>,
    event_sink: Arc<dyn hkask_types::event::NuEventSink>,
) -> hkask_communication::listener::ReceptorSupervisor {
    let store: Arc<dyn ReceptorStore> = Arc::new(NuEventStoreReceptorAdapter::new(nu_event_store));
    let sink = Arc::clone(&event_sink);

    // Build receptors as Arc<dyn Receptor> for uniform supervisor management
    let receptors: Vec<Arc<dyn hkask_communication::listener::Receptor>> = vec![
        Arc::new(
            VarietyReceptor::new(60)
                .with_store(Arc::clone(&store))
                .with_event_sink(Arc::clone(&sink)),
        ),
        Arc::new(
            AlgedonicReceptor::new(60)
                .with_store(Arc::clone(&store))
                .with_event_sink(Arc::clone(&sink)),
        ),
        Arc::new(
            ComposerReceptor::new(300)
                .with_store(Arc::clone(&store))
                .with_event_sink(Arc::clone(&sink)),
        ),
        Arc::new(
            ConsolidatorReceptor::new(300)
                .with_store(Arc::clone(&store))
                .with_event_sink(Arc::clone(&sink)),
        ),
        Arc::new(
            CyberneticsReceptor::new(120)
                .with_store(Arc::clone(&store))
                .with_event_sink(Arc::clone(&sink)),
        ),
        Arc::new(
            CuratorReceptor::new(120)
                .with_store(Arc::clone(&store))
                .with_event_sink(Arc::clone(&sink)),
        ),
    ];

    let supervisor = ReceptorSupervisor::new(receptors, 30)
        .with_event_sink(event_sink);
    supervisor.start_all().await;
    supervisor
}
