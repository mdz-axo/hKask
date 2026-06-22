//! Federation sync loop and link lifecycle management.

pub mod federation_sync;
pub mod link;
pub mod link_manager;
pub mod payload_store;
pub mod transport;

pub use federation_sync::FederationSync;
pub use link::{FederationLink, LinkError, LinkState, RevocationScope};
pub use link_manager::FederationLinkManager;
