//! OCR Pipeline — Typed, multi-backend, self-verifying document processing.
//!
//! Architecture:
//! ```text
//! PDF → [Decimate] → PageQueue → [Score → Route → OCR] → ResultBuffer → [Assembly] → VerifiedDocument
//!                                                                             ↓
//!                                                                      [Verification]
//!                                                                             ↓
//!                                                                      PipelineOutcome
//! ```

pub mod complexity;
pub mod cross_validation;
pub mod decimation;
pub mod pipeline;
pub mod routing;
pub mod semantic;
pub mod verification;
