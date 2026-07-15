//! CliHost — implements hkask_repl::ReplHost by delegating to CLI subsystems.

use hkask_repl::host::{OnboardingError, OnboardingOutcome, ReplHost};

/// Host implementation bridging the REPL crate to CLI subsystems.
pub struct CliHost;

impl ReplHost for CliHost {
    fn resolve_user_webid(&self) -> hkask_types::WebID {
        crate::commands::helpers::resolve_user_webid()
    }

    fn run_onboarding(
        &self,
        rt: &tokio::runtime::Handle,
    ) -> Result<OnboardingOutcome, OnboardingError> {
        match rt.block_on(crate::onboarding::run_onboarding()) {
            Ok(outcome) => Ok(OnboardingOutcome {
                signed_in_agent: outcome.signed_in_agent,
                resolved_secrets: outcome.resolved_secrets,
                selected_model: outcome.selected_model,
                is_first_run: outcome.is_first_run,
            }),
            Err(crate::onboarding::OnboardingError::Cancelled) => Err(OnboardingError::Cancelled),
            Err(e) => Err(OnboardingError::Failed(e.to_string())),
        }
    }

    fn list_templates_local(&self) -> Vec<hkask_ports::RegistryEntry> {
        crate::commands::template::list_templates_local()
    }

    #[cfg(feature = "tui")]
    fn open_transcript_viewer(&self, path: &std::path::Path) -> anyhow::Result<()> {
        crate::transcript_viewer::TranscriptViewer::from_file(path).and_then(|mut v| v.run())
    }

    fn run_sovereignty_status(&self) {
        crate::commands::sovereignty::run(crate::cli::SovereigntyAction::Status);
    }
}
