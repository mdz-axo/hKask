//! Cloud provider integrations.
//!
//! Re-exports from hkask-services-cloud for CLI access.
//! The canonical implementations live in the service layer.

pub use hkask_services_cloud::fly::{
    self, FlyClient, MachineConfig, MachineGuest, MachineMount, MachinePort, MachineService,
    MachineSpec,
};
pub use hkask_services_cloud::tigris;
pub use hkask_services_cloud::hetzner;
