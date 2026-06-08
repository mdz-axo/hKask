//! Ensemble session bundle (P2.2).
//!
//! Extracted from `ApiState::new()`. Composes the gas governance adapter
//! (which lets CNS sense ensemble gas usage) with the `SessionManager`
//! that the `/api/chat` route consumes.

use std::sync::Arc;

use crate::gas::{API_ENSEMBLE_GAS_CAP, ApiGasGovernanceAdapter};

/// Ensemble session bundle (P2.2).
///
/// Extracted from `ApiState::new()`. Composes the gas governance adapter
/// (which lets CNS sense ensemble gas usage) with the `SessionManager`
/// that the `/api/chat` route consumes. Also returns the inference port
/// extracted from the optional `ensemble_inferencer`, and the
/// `ensemble_inferencer` itself (for `ensemble_inferencer_with_breaker`).
pub(crate) struct EnsembleSession {
    pub session_manager: Arc<tokio::sync::RwLock<hkask_ensemble::SessionManager>>,
    pub gas_governance: Arc<dyn hkask_ensemble::GasGovernancePort>,
    pub inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
    /// Returned to the caller to be stored on `ApiState`; consumed by
    /// `ensemble_inferencer_with_breaker` for SOAP and ensemble routes.
    pub ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
}

/// Wire the ensemble session manager with CNS gas governance so ensemble
/// sessions in API mode respect the L6 budget.
///
/// P2.2 extraction: the inference port is also extracted here because
/// the caller needs it both on the returned bundle and on the final
/// `ApiState` literal — extracting once avoids a second clone.
pub(crate) fn build_ensemble_session(
    ensemble_inferencer: Option<Arc<hkask_ensemble::adapters::InferencePortAdapter>>,
    cybernetics_loop: Arc<tokio::sync::RwLock<hkask_cns::CyberneticsLoop>>,
    system_webid: hkask_types::WebID,
) -> EnsembleSession {
    let inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>> =
        ensemble_inferencer.as_ref().map(|ei| Arc::clone(ei.port()));
    let gas_governance: Arc<dyn hkask_ensemble::GasGovernancePort> = Arc::new(
        ApiGasGovernanceAdapter::new(cybernetics_loop, system_webid, API_ENSEMBLE_GAS_CAP),
    );
    let session_manager = Arc::new(tokio::sync::RwLock::new(
        hkask_ensemble::SessionManager::new(system_webid)
            .with_gas_governance(Arc::clone(&gas_governance)),
    ));
    EnsembleSession {
        session_manager,
        gas_governance,
        inference_port,
        ensemble_inferencer,
    }
}
