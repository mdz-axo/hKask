//! rSolidity — runtime contract vocabulary for hKask.
//!
//! Provides macros and attributes that make the declarative source `REQ:` contracts
//! in the spec executable: preconditions, postconditions, invariants, OCAP gates,
//! and CNS span emission. The detailed contract text remains in source comments
//! (lines that begin with the REQ: contract tag) so `scripts/contract-audit.sh` can
//! continue to use them as the authoritative audit signal during the
//! strangler-fig migration.

pub use hkask_rsolidity_macros::{contract, ocap};

/// Capability-verification trait for `#[ocap]` gates.
///
/// Implement this for the receiver type of an OCAP-gated method. The
/// `#[ocap(resource = "...", operation = "...")]` attribute injects a call to
/// `verify_ocap` at the start of the annotated method.
pub trait Ocap {
    type Error;
    fn verify_ocap(&self, resource: &str, operation: &str) -> Result<(), Self::Error>;
}

/// REQ: P9-rsolidity-emit-helper
    /// pre:  arguments are valid
    /// post: returns expected result
/// Private helper used by the `emit!` macro.
#[doc(hidden)]
pub fn __private_emit<S, V, P, T>(span: S, verb: V, phase: P, payload: T)
where
    S: std::fmt::Display,
    V: std::fmt::Display,
    P: serde::Serialize + std::fmt::Debug,
    T: serde::Serialize + std::fmt::Debug,
{
    let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| format!("{:?}", payload));
    tracing::info!(
        target: "rsolidity.emit",
        span = %span,
        verb = %verb,
        phase = ?phase,
        payload = %payload_json,
        "emit"
    );
}

/// Precondition gate. Panics with the contract id and message on violation.
#[macro_export]
macro_rules! require {
    ($cond:expr, $id:literal, $msg:literal) => {
        if !($cond) {
            ::core::panic!(::core::concat!("require violated [", $id, "]: ", $msg));
        }
    };
}

/// Postcondition / invariant gate. Panics in debug builds on violation.
#[macro_export]
macro_rules! assert {
    ($cond:expr, $id:literal, $msg:literal) => {
        ::core::assert!(
            $cond,
            ::core::concat!("assert violated [", $id, "]: ", $msg)
        );
    };
}

/// Explicit failure path. Returns `Err(err)` from the current function.
#[macro_export]
macro_rules! revert {
    ($id:literal, $err:expr) => {{
        let _ = $id; // contract id reserved for tracing/auditing
        return ::core::result::Result::Err($err);
    }};
}

/// CNS span emission. Logs the event via `tracing` (the CNS sink integration is
/// layered on top of the same `tracing` targets).
#[macro_export]
macro_rules! emit {
    ($span:expr, $verb:expr, $phase:expr, $payload:expr) => {
        $crate::__private_emit($span, $verb, $phase, $payload)
    };
}
