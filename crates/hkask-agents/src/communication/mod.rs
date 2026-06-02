//! Communication — Loop 4: dumb transport pipe
//!
//! Routes `LoopMessage`s between the 6 loops. Does NOT dampen,
//! throttle, or circuit-break — those are Cybernetics regulation
//! actions applied TO communication channels.

pub mod communication_loop;
pub mod dispatch;
pub mod escalation;

pub use communication_loop::CommunicationLoop;
pub use dispatch::MessageDispatch;
pub use escalation::{
    EscalationEntry, EscalationError, EscalationQueue, EscalationStats, EscalationStatus,
};
