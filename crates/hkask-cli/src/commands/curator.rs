//! Curator governance command handlers — escalations, metacognition

use hkask_agents::EscalationEntry;
use std::sync::Arc;

use crate::commands::config::open_registry_db;
use crate::errors::CuratorError;

/// List all pending escalations
pub async fn curator_escalations() -> Result<Vec<EscalationEntry>, CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .list_pending()
        .map_err(|e| CuratorError::EscalationNotFound(e.to_string()))
}

/// Resolve an escalation by ID
pub async fn curator_resolve(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .resolve(id, "cli-administrator")
        .map_err(|e| CuratorError::EscalationResolutionFailed(e.to_string()))
}

/// Dismiss an escalation by ID
pub async fn curator_dismiss(id: &str) -> Result<(), CuratorError> {
    let conn = open_registry_db()?;
    let queue = hkask_agents::EscalationQueue::new(conn)
        .map_err(|e| CuratorError::DatabaseError(e.to_string()))?;

    queue
        .dismiss(id, "cli-administrator")
        .map_err(|e| CuratorError::EscalationResolutionFailed(e.to_string()))
}

/// Run a metacognition cycle and return a summary string
pub async fn curator_metacognition() -> Result<String, CuratorError> {
    use hkask_agents::curator::{
        CuratorContext, MessageDispatch, MetacognitionConfig, MetacognitionLoop,
    };
    use hkask_cns::CnsRuntime;
    use hkask_types::WebID;
    use hkask_types::loops::curation::CuratorHandle;

    let conn = open_registry_db()?;
    let queue = Arc::new(
        hkask_agents::EscalationQueue::new(conn)
            .map_err(|e| CuratorError::DatabaseError(e.to_string()))?,
    );

    let curator_webid = WebID::from_persona(b"Curator");
    let cns = Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD));
    let dispatch = Arc::new(MessageDispatch::new());
    let curator_handle = CuratorHandle::new(curator_webid);
    let context = Arc::new(CuratorContext::new(curator_handle, cns, dispatch, queue));
    let config = MetacognitionConfig::default();
    let loop_instance = MetacognitionLoop::new(context, config);

    let snapshot = loop_instance
        .run_cycle()
        .await
        .map_err(|e| CuratorError::MetacognitionFailed(e.to_string()))?;

    Ok(loop_instance.generate_summary(&snapshot))
}
