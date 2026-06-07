//! HTTP routes — per-domain modules

mod acp;
mod bots;
mod bundles;
mod chat;
mod cns;
mod consolidation;
mod curator;
mod ensemble;
mod episodic;
mod git;
mod goal;
mod mcp;
mod models;
mod pods;
mod soap_infer;
mod sovereignty;
mod spec;
mod templates;

// Re-export router functions
pub use acp::acp_router;
pub use bots::bots_router;
pub use bundles::bundles_router;
pub use chat::chat_router;
pub use cns::cns_router;
pub use consolidation::consolidation_router;
pub use curator::curator_router;
pub use ensemble::ensemble_router;
pub use episodic::episodic_router;
pub use git::git_router;
pub use goal::goal_router;
pub use mcp::mcp_router;
pub use models::models_router;
pub use pods::pods_router;
pub use soap_infer::soap_infer_router;
pub use sovereignty::sovereignty_router;
pub use spec::spec_router;
pub use templates::templates_router;

// Re-export domain-local types that may be used externally
pub use acp::{AcpAgentResponse, AcpRegisterRequest, AcpRegisterResponse, AgentListResponse};
pub use bundles::{
    ApplyBundleResponse, BundleListResponse, BundleSummary, ComposeBundleRequest,
    ComposeBundleResponse, DeactivateBundleResponse, EvolveBundleResponse,
};
pub use chat::{ChatRequest, ChatResponse};
pub use cns::{CnsHealthResponse, CnsVarietyResponse, VarietyCounterResponse};
pub use curator::{
    DismissEscalationRequest, DismissEscalationResponse, EscalationEntryResponse,
    EscalationStatsResponse, ListEscalationsResponse, MetacognitionStatusResponse,
    ResolveEscalationRequest, ResolveEscalationResponse,
};
pub use ensemble::{
    CreateChatRequest, EnsembleResponse, RecordResponseRequest, RegisterBotRequest,
    SendMessageRequest, StandingStartRequest, StandingStartResponse, StandingStatusResponse,
};
pub use git::{ArchiveRequest, ArchiveResponse, ResolveShaResponse};
pub use goal::{CreateGoalRequest, GoalListResponse, GoalResponse, SetGoalStateRequest};
pub use models::{ModelEntry, ModelListResponse, ModelSearchQuery};
pub use pods::{CreatePodRequest, CreatePodResponse, ListPodsResponse, PodStatusResponse};
pub use soap_infer::{
    EventRecord, ObjectiveData, SeverityCounts, SoapInferAuthRequest, SoapInferRequest,
    SoapInferResponse, ValidationErrorType,
};
pub use sovereignty::{
    AccessCheckResponse, KillZoneResponse, SovereigntyConsentResponse, SovereigntyStatusResponse,
};
pub use spec::{
    SpecCaptureRequest, SpecCaptureResponse, SpecCultivateResponse, SpecListResponse,
    SpecValidateRequest, SpecValidateResponse, TestInvariantRequest, TestInvariantResponse,
    TestVerifyRequest, TestVerifyResponse,
};
pub use templates::{GrantCapabilityRequest, TemplateResponse};
