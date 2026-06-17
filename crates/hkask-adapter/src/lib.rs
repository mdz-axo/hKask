//! hKask Adapter — trained adapter lifecycle & inference composition
//!
//! # Architecture
//!
//! ```text
//! AdapterRouter
//!   ├── AdapterStore      — CRUD for trained LoRA adapters
//!   ├── Expertise         — semantic capability descriptor
//!   ├── EndpointLifecycle — state machine (provisioning → running → draining → terminated)
//!   └── ProviderCost      — cost model per inference provider
//! ```
//!
//! # Design
//!
//! An `Expertise` (a named, provenance-tracked capability descriptor) links a
//! `TrainedLoRAAdapter` (content-addressed, owner-scoped artifact) to an
//! `InferenceEndpoint` (a provider-provisioned, lifecycle-governed, cost-tracked resource).
//! Every operation is OCAP-gated. Every state transition emits a CNS span.
//! Every endpoint drains on session completion or budget exhaustion.

pub mod adapter_store;
pub mod endpoint_lifecycle;
pub mod expertise;

// Re-exports — public API
pub use adapter_store::{AdapterStore, AdapterStoreError, TrainedLoRAAdapter};
pub use endpoint_lifecycle::{EndpointLifecycle, EndpointPhase, EndpointPhaseError};
pub use expertise::{Expertise, MdsDomain, TrainingProvenance};
