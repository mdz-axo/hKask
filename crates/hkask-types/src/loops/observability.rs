//! Loop 4: Observability (CNS)
//!
//! Observability is the sensing half of the Cybernetic loop (Loop 7),
//! which manages the Observability→Governance feedback cycle.
//! Observability detects anomalies and generates alerts — it does not
//! decide what to do about them. Governance (Loop 3) acts on them.
//!
//! No separate handle types — CnsRuntime is the single entry point.
