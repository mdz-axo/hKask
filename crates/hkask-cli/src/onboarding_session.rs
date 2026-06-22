//! Onboarding state machine — resumable, self-healing replicant creation.
//!
//! Converts the linear `create_first_replicant_flow` into explicit states.
//! Each step is independently callable and carries its own recovery logic.
//! If any step fails, the session can be resumed from the failed state.

use hkask_services::{
    MatrixRegistrationResult, OnboardingService, RegistryHandle, ResolvedSecrets, ServiceConfig,
    ServiceError,
};
use hkask_storage::UserProfile;

use crate::onboarding::OnboardingError;

/// Accumulated state across onboarding steps.
/// Each field is populated as its corresponding step completes.
pub struct OnboardingSession {
    // ── Collected before the state machine starts ──
    pub user_profile: UserProfile,
    pub replicant_name: String,
    pub display_name: String,
    pub description: String,

    // ── Accumulated during state machine execution ──
    selected_model: Option<String>,
    passphrase: Option<String>,
    resolved_secrets: Option<ResolvedSecrets>,
    registry_handle: Option<RegistryHandle>,
    matrix_result: Option<MatrixRegistrationResult>,

    homeserver_url: String,
}

impl OnboardingSession {
    /// Create a new session with identity already collected.
    pub fn new(user_profile: UserProfile, replicant_name: String, description: String) -> Self {
        let display_name = user_profile.replicant_display_name(&replicant_name);
        let homeserver_url = std::env::var("HKASK_MATRIX_URL")
            .unwrap_or_else(|_| "http://localhost:8008".to_string());
        Self {
            user_profile,
            replicant_name,
            display_name,
            description,
            selected_model: None,
            passphrase: None,
            resolved_secrets: None,
            registry_handle: None,
            matrix_result: None,
            homeserver_url,
        }
    }

    /// Run all remaining steps to completion. Returns the completed session.
    /// Interactive callbacks (`get_model`, `get_passphrase`) are injected so
    /// this state machine has no stdio dependencies.
    pub async fn run(
        mut self,
        get_model: impl FnOnce() -> Result<String, OnboardingError>,
        get_passphrase: impl FnOnce() -> Result<String, OnboardingError>,
    ) -> Result<CompletedSession, (Self, OnboardingError)> {
        // Advance through provider (no callback needed).
        if let Err(e) = self.advance_provider().await {
            return Err((self, e));
        }
        // Model selection uses the injected callback.
        if let Err(e) = self.advance_model(get_model()) {
            return Err((self, e));
        }
        // Passphrase uses the injected callback.
        if let Err(e) = self.advance_passphrase(get_passphrase()) {
            return Err((self, e));
        }
        // Remaining steps are pure service calls.
        if let Err(e) = self.advance_registry().await {
            return Err((self, e));
        }
        if let Err(e) = self.advance_profile().await {
            return Err((self, e));
        }
        if let Err(e) = self.advance_replicant().await {
            return Err((self, e));
        }
        if let Err(e) = self.advance_matrix().await {
            return Err((self, e));
        }
        Ok(CompletedSession {
            display_name: self.display_name,
            description: self.description,
            selected_model: self.selected_model.unwrap_or_default(),
            resolved_secrets: self.resolved_secrets,
            registry_handle: self.registry_handle,
            matrix_result: self.matrix_result,
        })
    }

    // ── Step implementations ─────────────────────────────────────────────

    async fn advance_provider(&mut self) -> Result<(), OnboardingError> {
        // Provider setup is idempotent — skip if already configured.
        let config = hkask_inference::InferenceConfig::from_env();
        if config.deepinfra_api_key.is_empty()
            && config.together_api_key.is_empty()
            && config.fal_api_key.is_empty()
        {
            return Err(OnboardingError::Service(ServiceError::Config {
                source: None,
                message: "No inference provider configured. Set DEEPINFRA_API_KEY.".into(),
            }));
        }
        Ok(())
    }

    fn advance_model(
        &mut self,
        model_result: Result<String, OnboardingError>,
    ) -> Result<(), OnboardingError> {
        let model = model_result?;
        self.selected_model = Some(model);
        Ok(())
    }

    fn advance_passphrase(
        &mut self,
        passphrase_result: Result<String, OnboardingError>,
    ) -> Result<(), OnboardingError> {
        let passphrase = passphrase_result?;
        self.passphrase = Some(passphrase);
        Ok(())
    }

    async fn advance_registry(&mut self) -> Result<(), OnboardingError> {
        let passphrase = self.passphrase.as_ref().ok_or_else(|| {
            OnboardingError::Service(ServiceError::Config {
                source: None,
                message: "Passphrase not set before registry init".into(),
            })
        })?;

        // Remove orphaned DB from previous failed attempt.
        if let Ok(pre_config) = ServiceConfig::from_env()
            && OnboardingService::has_orphaned_db(&pre_config)
        {
            eprintln!("  A database from a previous failed setup was found.");
            eprint!("  Remove it? [y/N] ");
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let confirm = crate::onboarding::read_line().unwrap_or_default();
            if confirm.trim().to_lowercase().starts_with('y') {
                OnboardingService::remove_orphaned_db(&pre_config);
                eprintln!("  Removed orphaned database.");
            } else {
                eprintln!("  Keeping existing database. Setup will use it if compatible.");
            }
        }

        // Derive secrets and store in keychain
        let resolved = OnboardingService::derive_secrets(passphrase, true).map_err(|e| {
            eprintln!("  \x1b[31m✗\x1b[0m Failed to derive security keys: {}", e);
            OnboardingError::Service(e)
        })?;

        // Initialize registry.
        let config = ServiceConfig::from_secrets(
            resolved.a2a_secret.clone(),
            resolved.db_passphrase.clone(),
            resolved.mcp_secret.clone(),
            self.display_name.clone(),
        );
        let handle = OnboardingService::init_registry(&config)
            .await
            .map_err(|e| {
                eprintln!("  \x1b[31m✗\x1b[0m Failed to initialize database: {}", e);
                OnboardingError::Service(e)
            })?;

        self.resolved_secrets = Some(resolved);
        self.registry_handle = Some(handle);
        Ok(())
    }

    async fn advance_profile(&mut self) -> Result<(), OnboardingError> {
        let handle = self.registry_handle.as_ref().ok_or_else(|| {
            OnboardingError::Service(ServiceError::Config {
                source: None,
                message: "Registry not initialized before profile store".into(),
            })
        })?;
        OnboardingService::store_user_profile(&handle.store, &self.user_profile).map_err(|e| {
            eprintln!("  \x1b[31m✗\x1b[0m Failed to store user profile: {}", e);
            OnboardingError::Service(e)
        })?;
        Ok(())
    }

    async fn advance_replicant(&mut self) -> Result<(), OnboardingError> {
        let handle = self.registry_handle.as_ref().ok_or_else(|| {
            OnboardingError::Service(ServiceError::Config {
                source: None,
                message: "Registry not initialized before replicant registration".into(),
            })
        })?;
        OnboardingService::register_replicant(
            &handle.a2a,
            &handle.store,
            &self.replicant_name,
            &self.description,
            Some(&self.user_profile),
            None,
            None,
        )
        .await
        .map_err(|e| {
            eprintln!("  \x1b[31m✗\x1b[0m Failed to register replicant: {}", e);
            OnboardingError::Service(e)
        })?;
        Ok(())
    }

    async fn advance_matrix(&mut self) -> Result<(), OnboardingError> {
        // Matrix registration with auto-recovery (Conduit health check + container restart).
        let result = OnboardingService::register_matrix_accounts(
            &self.user_profile,
            &self.display_name,
            self.passphrase.as_deref().unwrap_or(""),
            &self.homeserver_url,
        )
        .await;
        // Matrix is non-blocking — store result even on failure.
        self.matrix_result = result.ok();
        if self.matrix_result.is_none() {
            eprintln!();
            eprintln!("  \x1b[33m⚠\x1b[0m  Matrix chat accounts could not be registered.");
            eprintln!("  Automatic Conduit recovery was attempted but failed.");
            eprintln!(
                "  Matrix registration will be retried on next \x1b[36mkask chat\x1b[0m session."
            );
            eprintln!();
        }
        Ok(())
    }
}

/// The completed session, ready for post-onboarding summary.
pub struct CompletedSession {
    pub display_name: String,
    pub description: String,
    pub selected_model: String,
    pub resolved_secrets: Option<ResolvedSecrets>,
    pub registry_handle: Option<RegistryHandle>,
    pub matrix_result: Option<MatrixRegistrationResult>,
}
