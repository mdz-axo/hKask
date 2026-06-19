//! `kask onboard` — add a new replicant to an existing hKask installation.
//!
//! Runs the same guided flow as first-run setup but skips re-entering the
//! passphrase when secrets are already in the OS keychain.

/// Run the `kask onboard` command synchronously.
/// Run the onboarding workflow.
///
pub fn run(rt: &tokio::runtime::Runtime) {
    match rt.block_on(crate::onboarding::run_add_replicant()) {
        Ok(()) => {}
        Err(e) => {
            // Cancelled is a deliberate user action — exit cleanly.
            if matches!(e, crate::onboarding::OnboardingError::Cancelled) {
                std::process::exit(0);
            }
            eprintln!("Onboarding failed: {}", e);
            std::process::exit(1);
        }
    }
}
