//! Federation sync loop and link lifecycle management.

pub mod federation_sync;
pub mod health_model;
pub mod invitation_policy;
pub mod link;
pub mod link_manager;
pub mod payload_store;
pub mod transport;

pub use federation_sync::FederationSync;
pub use health_model::FederationHealthModel;
pub use invitation_policy::{
    AllowListInvitationPolicy, InvitationDecision, InvitationPolicy, ManualInvitationPolicy,
};
pub use link::{FederationLink, LinkError, LinkState, RevocationScope};
pub use link_manager::FederationLinkManager;
