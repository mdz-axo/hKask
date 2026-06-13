//! API error type with Axum IntoResponse — maps to HTTP status codes per variant.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use hkask_storage::{
    AgentRegistryError, ConsentStoreError, GoalRepositoryError, NuEventError,
    SovereigntyStoreError, StandingSessionError, TripleError, UserStoreError,
};
use hkask_types::InfrastructureError;
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    NotFound { resource: String, id: String },
    Unauthorized { reason: String },
    Forbidden { reason: String },
    BadRequest { message: String },
    Conflict { message: String },
    ServiceUnavailable { reason: String },
    Internal { message: String },
}

impl ApiError {
    fn status_and_message(self) -> (StatusCode, String) {
        match self {
            ApiError::NotFound { resource, id } => {
                (StatusCode::NOT_FOUND, format!("{resource} not found: {id}"))
            }
            ApiError::Unauthorized { reason } => {
                (StatusCode::UNAUTHORIZED, format!("Unauthorized: {reason}"))
            }
            ApiError::Forbidden { reason } => {
                (StatusCode::FORBIDDEN, format!("Forbidden: {reason}"))
            }
            ApiError::BadRequest { message } => {
                (StatusCode::BAD_REQUEST, format!("Bad request: {message}"))
            }
            ApiError::Conflict { message } => {
                (StatusCode::CONFLICT, format!("Conflict: {message}"))
            }
            ApiError::ServiceUnavailable { reason } => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Service unavailable: {reason}"),
            ),
            ApiError::Internal { message } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {message}"),
            ),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound { resource, id } => write!(f, "{resource} not found: {id}"),
            ApiError::Unauthorized { reason } => write!(f, "Unauthorized: {reason}"),
            ApiError::Forbidden { reason } => write!(f, "Forbidden: {reason}"),
            ApiError::BadRequest { message } => write!(f, "Bad request: {message}"),
            ApiError::Conflict { message } => write!(f, "Conflict: {message}"),
            ApiError::ServiceUnavailable { reason } => write!(f, "Service unavailable: {reason}"),
            ApiError::Internal { message } => write!(f, "Internal error: {message}"),
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = self.status_and_message();
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

// ── Store error conversions ──────────────────────────────────────────

impl From<TripleError> for ApiError {
    fn from(e: TripleError) -> Self {
        match e {
            TripleError::NotFound => ApiError::NotFound {
                resource: "triple".into(),
                id: "unknown".into(),
            },
            TripleError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<GoalRepositoryError> for ApiError {
    fn from(e: GoalRepositoryError) -> Self {
        match &e {
            GoalRepositoryError::NotFound(id) => ApiError::NotFound {
                resource: "goal".into(),
                id: id.clone(),
            },
            GoalRepositoryError::VisibilityDenied(reason) => ApiError::Forbidden {
                reason: reason.clone(),
            },
            GoalRepositoryError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
            GoalRepositoryError::InvalidTransition(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            GoalRepositoryError::MaxDepthExceeded(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            GoalRepositoryError::Corrupt(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            GoalRepositoryError::QuarantineFailed(msg) => ApiError::Internal {
                message: msg.clone(),
            },
        }
    }
}

impl From<AgentRegistryError> for ApiError {
    fn from(e: AgentRegistryError) -> Self {
        match &e {
            AgentRegistryError::NotFound(name) => ApiError::NotFound {
                resource: "agent".into(),
                id: name.clone(),
            },
            AgentRegistryError::AlreadyRegistered(name) => ApiError::Conflict {
                message: format!("Agent already registered: {name}"),
            },
            AgentRegistryError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<NuEventError> for ApiError {
    fn from(e: NuEventError) -> Self {
        ApiError::Internal {
            message: e.to_string(),
        }
    }
}

impl From<ConsentStoreError> for ApiError {
    fn from(e: ConsentStoreError) -> Self {
        match &e {
            ConsentStoreError::NotFound(id) => ApiError::NotFound {
                resource: "consent".into(),
                id: id.clone(),
            },
            ConsentStoreError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<SovereigntyStoreError> for ApiError {
    fn from(e: SovereigntyStoreError) -> Self {
        match &e {
            SovereigntyStoreError::UuidParse(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            SovereigntyStoreError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<StandingSessionError> for ApiError {
    fn from(e: StandingSessionError) -> Self {
        match &e {
            StandingSessionError::NotFound(id) => ApiError::NotFound {
                resource: "session".into(),
                id: id.clone(),
            },
            StandingSessionError::Sealed(id) => ApiError::Forbidden {
                reason: format!("Session is sealed (key version mismatch): {id}"),
            },
            StandingSessionError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<UserStoreError> for ApiError {
    fn from(e: UserStoreError) -> Self {
        match &e {
            UserStoreError::NotFound(id) => ApiError::NotFound {
                resource: "user".into(),
                id: id.clone(),
            },
            UserStoreError::ReplicantNameTaken(name) => ApiError::Conflict {
                message: format!("Replicant name already registered: {name}"),
            },
            UserStoreError::InvalidCredentials => ApiError::Unauthorized {
                reason: "Invalid credentials".into(),
            },
            UserStoreError::Encryption(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::Decryption(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::KeyDerivation(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::PasswordHash(msg) => ApiError::Internal {
                message: msg.clone(),
            },
            UserStoreError::PassphraseExpired(days) => ApiError::Unauthorized {
                reason: format!("Passphrase expired {} days ago — must change", days),
            },
            UserStoreError::Infra(e) => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<InfrastructureError> for ApiError {
    fn from(e: InfrastructureError) -> Self {
        ApiError::Internal {
            message: e.to_string(),
        }
    }
}

// ── Agent/crate error conversions ────────────────────────────────────

impl From<hkask_agents::ConsentError> for ApiError {
    fn from(e: hkask_agents::ConsentError) -> Self {
        match &e {
            hkask_agents::ConsentError::ConsentNotFound(id) => ApiError::NotFound {
                resource: "consent".into(),
                id: id.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_storage::EscalationError> for ApiError {
    fn from(e: hkask_storage::EscalationError) -> Self {
        match &e {
            hkask_storage::EscalationError::NotFound(id) => ApiError::NotFound {
                resource: "escalation".into(),
                id: id.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_types::GitError> for ApiError {
    fn from(e: hkask_types::GitError) -> Self {
        match &e {
            hkask_types::GitError::CrateNotFound(name) => ApiError::NotFound {
                resource: "template crate".into(),
                id: name.clone(),
            },
            hkask_types::GitError::Io(_) => ApiError::BadRequest {
                message: e.to_string(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_agents::AcpError> for ApiError {
    fn from(e: hkask_agents::AcpError) -> Self {
        match &e {
            hkask_agents::AcpError::AgentAlreadyRegistered(webid) => ApiError::Conflict {
                message: format!("Agent already registered: {}", webid),
            },
            hkask_agents::AcpError::AgentNotFound(webid) => ApiError::NotFound {
                resource: "agent".into(),
                id: webid.to_string(),
            },
            hkask_agents::AcpError::CapabilityDenied(webid, perm) => ApiError::Forbidden {
                reason: format!("Agent {} lacks permission: {}", webid, perm),
            },
            hkask_agents::AcpError::WildcardCapabilityNotAllowed => ApiError::BadRequest {
                message: "Wildcard capabilities are not allowed".into(),
            },
            hkask_agents::AcpError::MalformedCapability(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_templates::TemplateError> for ApiError {
    fn from(e: hkask_templates::TemplateError) -> Self {
        match &e {
            hkask_templates::TemplateError::NotFound(id) => ApiError::NotFound {
                resource: "template".into(),
                id: id.clone(),
            },
            hkask_templates::TemplateError::CapabilityDenied(msg) => ApiError::Forbidden {
                reason: msg.clone(),
            },
            hkask_templates::TemplateError::PathTraversal(_) => ApiError::BadRequest {
                message: e.to_string(),
            },
            hkask_templates::TemplateError::SandboxViolation(_) => ApiError::Forbidden {
                reason: e.to_string(),
            },
            hkask_templates::TemplateError::Validation(msg) => ApiError::BadRequest {
                message: msg.clone(),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
            },
        }
    }
}

impl From<hkask_types::ports::RegistryError> for ApiError {
    fn from(e: hkask_types::ports::RegistryError) -> Self {
        match &e {
            hkask_types::ports::RegistryError::NotFound(id) => ApiError::NotFound {
                resource: "template".into(),
                id: id.clone(),
            },
            hkask_types::ports::RegistryError::Other(msg) => ApiError::Internal {
                message: msg.clone(),
            },
        }
    }
}

// ── Service layer adapter ────────────────────────────────────────────

impl From<hkask_services::ServiceError> for ApiError {
    fn from(e: hkask_services::ServiceError) -> Self {
        use hkask_services::ServiceError as SE;
        match e {
            SE::EscalationNotFound(id) => ApiError::NotFound {
                resource: "escalation".into(),
                id,
            },
            SE::AgentNotFound(name) => ApiError::NotFound {
                resource: "agent".into(),
                id: name,
            },
            SE::UserNotFound(name) => ApiError::NotFound {
                resource: "user".into(),
                id: name,
            },
            SE::SessionNotFound(id) => ApiError::NotFound {
                resource: "session".into(),
                id,
            },
            SE::PodNotFound(id) => ApiError::NotFound {
                resource: "pod".into(),
                id,
            },
            SE::LoginFailed(_) => ApiError::Unauthorized {
                reason: "Invalid credentials".into(),
            },
            SE::Acp(hkask_agents::acp::AcpError::CapabilityDenied(webid, perm)) => {
                ApiError::Forbidden {
                    reason: format!("Agent {} lacks permission: {}", webid, perm),
                }
            }
            SE::Acp(hkask_agents::acp::AcpError::AgentNotFound(webid)) => ApiError::NotFound {
                resource: "agent".into(),
                id: webid.to_string(),
            },
            SE::Acp(hkask_agents::acp::AcpError::AgentAlreadyRegistered(webid)) => {
                ApiError::Conflict {
                    message: format!("Agent already registered: {}", webid),
                }
            }
            SE::Acp(_) => ApiError::Forbidden {
                reason: "Capability denied".into(),
            },
            SE::SovereigntyStore(hkask_storage::SovereigntyStoreError::UuidParse(msg)) => {
                ApiError::BadRequest { message: msg }
            }
            SE::InvalidAgentType(msg) => ApiError::BadRequest { message: msg },
            SE::InvalidPassphrase(msg) => ApiError::BadRequest {
                message: format!("Invalid passphrase: {}", msg),
            },
            SE::ValidationError(msg) => ApiError::BadRequest { message: msg },
            SE::AgentRegistrationFailed(msg) => ApiError::Conflict { message: msg },
            SE::Escalation(hkask_storage::EscalationError::NotFound(id)) => ApiError::NotFound {
                resource: "escalation".into(),
                id,
            },
            SE::Escalation(_) => ApiError::Internal {
                message: e.to_string(),
            },
            SE::AgentRegistryStore(err) => ApiError::from(err),
            SE::GoalRepo(err) => ApiError::from(err),
            SE::Triple(err) => ApiError::from(err),
            SE::ConsentStore(err) => ApiError::from(err),
            SE::UserStore(err) => ApiError::from(err),
            SE::StandingSessionStore(err) => ApiError::from(err),
            SE::Consent(err) => ApiError::from(err),
            SE::Spec(err) => ApiError::Internal {
                message: err.to_string(),
            },
            SE::Pod(hkask_agents::pod::AgentPodError::PodNotFound(_)) => ApiError::NotFound {
                resource: "pod".into(),
                id: e.to_string(),
            },
            SE::Pod(hkask_agents::pod::AgentPodError::PersonaParseError(msg)) => {
                ApiError::BadRequest {
                    message: format!("Invalid persona: {}", msg),
                }
            }
            SE::Pod(hkask_agents::pod::AgentPodError::InvalidStateTransition(from, to)) => {
                ApiError::Conflict {
                    message: format!("Invalid pod state transition: {} -> {}", from, to),
                }
            }
            SE::Pod(_) => ApiError::Internal {
                message: e.to_string(),
            },
            SE::Template(err) => ApiError::from(err),
            SE::NuEvent(err) => ApiError::Internal {
                message: err.to_string(),
            },
            SE::Keystore(msg) => ApiError::ServiceUnavailable { reason: msg },
            SE::Infra(err) => ApiError::Internal {
                message: err.to_string(),
            },
            other => ApiError::Internal {
                message: other.to_string(),
            },
        }
    }
}
