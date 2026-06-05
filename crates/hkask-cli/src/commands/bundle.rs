//! Bundle command handlers for `kask bundle`
//!
//! Implements the CLI display logic for skill bundle management operations.
//! Full implementation awaits template rendering infrastructure.

use crate::cli::BundleAction;

pub fn run_bundle(action: BundleAction) {
    match action {
        BundleAction::Compose {
            skills,
            name,
            visibility,
        } => {
            println!(
                "Composing bundle from skills: {:?} with name: {:?} visibility: {}",
                skills, name, visibility
            );
        }
        BundleAction::Apply { bundle_id } => {
            println!("Applying bundle: {}", bundle_id);
        }
        BundleAction::List => {
            println!("Listing all bundles");
        }
        BundleAction::Show { bundle_id } => {
            println!("Showing bundle: {}", bundle_id);
        }
        BundleAction::Evolve { bundle_id } => {
            println!("Evolving bundle: {}", bundle_id);
        }
        BundleAction::Skills => {
            println!("Listing available skills");
        }
        BundleAction::Off => {
            println!("Deactivating current bundle");
        }
    }
}
