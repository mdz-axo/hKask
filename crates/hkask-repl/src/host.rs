//! Host trait — implemented by the CLI binary to provide REPL dependencies.

/// Outcome of the onboarding flow.
///
/// NOTE: mirrors `crate::onboarding::OnboardingOutcome` in hkask-cli.
/// Duplicated here to avoid a circular dependency between hkask-repl
/// and hkask-cli. Keep field names synchronized.
#[derive(Debug, Clone)]
pub struct OnboardingOutcome {
    pub signed_in_agent: String,
    pub resolved_secrets: Option<hkask_services_onboarding::ResolvedSecrets>,
    pub selected_model: Option<String>,
    pub is_first_run: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum OnboardingError {
    #[error("Onboarding cancelled by user")]
    Cancelled,
    #[error("Onboarding failed: {0}")]
    Failed(String),
}

pub trait ReplHost: Send + Sync {
    fn resolve_user_webid(&self) -> hkask_types::WebID;
    fn run_onboarding(
        &self,
        rt: &tokio::runtime::Handle,
    ) -> Result<OnboardingOutcome, OnboardingError>;
    fn list_templates_local(&self) -> Vec<hkask_ports::RegistryEntry>;
    #[cfg(feature = "tui")]
    fn open_transcript_viewer(&self, path: &std::path::Path) -> Result<(), String>;
    fn run_sovereignty_status(&self);
}
