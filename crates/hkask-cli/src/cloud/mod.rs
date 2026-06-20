//! Cloud provider integrations for pod lifecycle management.
//!
//! Each provider implements operations needed by `kask pod activate`,
//! `kask pod deactivate`, and `kask pod export`.

pub mod fly;
pub mod hetzner;
pub mod tigris;
