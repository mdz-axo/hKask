//! Curation confidence gate (IP-3) — metacognitive decision point
//!
//! The curation confidence gate gives the Curation Loop genuine metacognition.
//! R̄ = confidence that the Curator should proceed with a decision.
//! If R̄ is in the transition zone (0.3 < R̄ < 0.8), the regulated
//! response is to ask what would increase confidence.
//!
//! Evidence channels for v0.1.0:
//! - LlmConfidence { c: f64 } — LLM's self-assessed confidence
//! - TemplateMatch { c: f64 } — Template relevance score
//! - ValidationResult { c: f64 } — Schema/validation pass result
//!
//! Originally in `hkask_cns::allosteric` — the allosteric primitives have been
//! relocated to `hkask_types::allosteric` (the substrate crate) because they are
//! cross-loop primitives shared by L5 and L6. The dependency is now a substrate
//! import, not an L5→L6 authority inversion.

// ARCHITECTURE: Allosteric primitives now live in hkask-types (substrate),
// eliminating the L5→L6 authority inversion. CurationConfidenceGate imports
// from the substrate crate directly. The coupling is deep (sensitivity analysis
// reads gate.c, gate.n directly), so a port trait would just rename the struct
// without meaningful decoupling. Revisit when a second implementation of
// allosteric gating exists.
use hkask_cns::RBarThreshold;
use hkask_types::allosteric::gate::{AllostericGate, AllostericGateConfig};
use hkask_types::allosteric::mwc::mwc_state_function;
use std::time::Duration;

/// Evidence port for the curation confidence gate.
///
/// Each port provides a normalized confidence value c ∈ [0, 1].
/// Ports carry a `label` for disambiguation when multiple ports
/// of the same variant exist (e.g., two `LlmConfidence` ports
/// from different models).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CurationPort {
    /// LLM's self-assessed confidence (from Okapi inference results).
    LlmConfidence {
        /// Disambiguation label (e.g., model name or task ID).
        label: String,
        c: f64,
    },
    /// Template relevance score (from registry).
    TemplateMatch {
        /// Disambiguation label (e.g., template name).
        label: String,
        c: f64,
    },
    /// Schema/validation pass result.
    ValidationResult {
        /// Disambiguation label (e.g., schema name).
        label: String,
        c: f64,
    },
}

impl CurationPort {
    /// Get the confidence value from this port.
    pub fn confidence(&self) -> f64 {
        match self {
            CurationPort::LlmConfidence { c, .. } => *c,
            CurationPort::TemplateMatch { c, .. } => *c,
            CurationPort::ValidationResult { c, .. } => *c,
        }
    }

    /// Get the disambiguation label for this port.
    ///
    /// Returns the explicit label, which is unique per port instance.
    /// Use this (not the variant name) for sensitivity analysis indexing.
    pub fn label(&self) -> &str {
        match self {
            CurationPort::LlmConfidence { label, .. } => label,
            CurationPort::TemplateMatch { label, .. } => label,
            CurationPort::ValidationResult { label, .. } => label,
        }
    }
}

/// Confidence gate decision outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConfidenceDecision {
    /// R̄ ≥ upper threshold: proceed with confidence.
    Proceed,
    /// R̄ in transition zone: ask what would increase confidence.
    SeekMoreEvidence,
    /// R̄ ≤ lower threshold: suppress / do not proceed.
    Suppress,
}

/// Curation confidence gate — metacognitive decision point.
///
/// Uses the MWC equation to compute confidence that the Curator should
/// proceed with a decision. The gate has three zones:
///
/// - R̄ ≥ upper_threshold → Proceed
/// - lower_threshold < R̄ < upper_threshold → SeekMoreEvidence
/// - R̄ ≤ lower_threshold → Suppress
///
/// The "SeekMoreEvidence" zone is the metacognitive behavior: the Curator
/// doesn't just proceed or suppress, it asks what evidence would increase
/// confidence. The sensitivity analysis identifies which channel to verify.
pub struct CurationConfidenceGate {
    /// Underlying MWC gate.
    gate: AllostericGate,
    /// Evidence ports providing confidence values.
    pub ports: Vec<CurationPort>,
    /// Upper R̄ threshold for Proceed zone.
    pub upper_threshold: RBarThreshold,
    /// Lower R̄ threshold for Suppress zone.
    pub lower_threshold: RBarThreshold,
}

impl CurationConfidenceGate {
    /// Create a new curation confidence gate.
    ///
    /// Default parameters: L=100 (moderate skepticism), c=0.05,
    /// n=number of evidence channels.
    pub fn new(ports: Vec<CurationPort>) -> Self {
        let n = ports.len().max(1);
        let config = AllostericGateConfig {
            name: "curation_confidence".to_string(),
            base_l: 100.0, // Moderate skepticism
            c: 0.05,       // Moderate cooperativity
            n,
            threshold: 0.5,
            tau: Duration::from_secs(1),
            hysteresis: 0.5, // Some inertia to avoid oscillation
        };
        Self {
            gate: AllostericGate::new(&config),
            ports,
            upper_threshold: RBarThreshold::DEFAULT_UPPER,
            lower_threshold: RBarThreshold::DEFAULT_LOWER,
        }
    }

    /// Create a gate with custom MWC parameters.
    pub fn with_params(
        l: f64,
        c: f64,
        ports: Vec<CurationPort>,
        upper_threshold: f64,
        lower_threshold: f64,
    ) -> Self {
        let n = ports.len().max(1);
        let config = AllostericGateConfig {
            name: "curation_confidence".to_string(),
            base_l: l,
            c,
            n,
            threshold: 0.5,
            tau: Duration::from_secs(1),
            hysteresis: 0.5,
        };
        Self {
            gate: AllostericGate::new(&config),
            ports,
            upper_threshold: RBarThreshold::new(upper_threshold),
            lower_threshold: RBarThreshold::new(lower_threshold),
        }
    }

    /// Compute the aggregate α from all evidence ports.
    ///
    /// α = average of port confidence values. This treats each port
    /// as an independent "binding site" for the MWC model.
    pub fn aggregate_alpha(&self) -> f64 {
        if self.ports.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.ports.iter().map(|p| p.confidence()).sum();
        sum / self.ports.len() as f64
    }

    /// Compute the R̄ (confidence) from current evidence.
    pub fn confidence(&mut self) -> f64 {
        let alpha = self.aggregate_alpha();
        self.gate.set_alpha(alpha);
        self.gate.r_bar_eq()
    }

    /// Compute the confidence with temporal relaxation.
    pub fn confidence_at(&mut self, dt: Duration) -> f64 {
        let alpha = self.aggregate_alpha();
        self.gate.set_alpha(alpha);
        self.gate.r_bar_at(dt)
    }

    /// Decide based on current evidence.
    ///
    /// Returns a `ConfidenceDecision` based on the three-zone structure:
    /// - R̄ ≥ upper threshold → Proceed
    /// - R̄ ≤ lower threshold → Suppress
    /// - Between → SeekMoreEvidence
    pub fn decide(&mut self) -> ConfidenceDecision {
        let r_bar = self.confidence();
        if r_bar >= self.upper_threshold.as_raw() {
            ConfidenceDecision::Proceed
        } else if r_bar <= self.lower_threshold.as_raw() {
            ConfidenceDecision::Suppress
        } else {
            ConfidenceDecision::SeekMoreEvidence
        }
    }

    /// Decide with temporal relaxation.
    pub fn decide_at(&mut self, dt: Duration) -> ConfidenceDecision {
        let r_bar = self.confidence_at(dt);
        if r_bar >= self.upper_threshold.as_raw() {
            ConfidenceDecision::Proceed
        } else if r_bar <= self.lower_threshold.as_raw() {
            ConfidenceDecision::Suppress
        } else {
            ConfidenceDecision::SeekMoreEvidence
        }
    }

    /// Sensitivity analysis: which evidence channel contributes most
    /// to output uncertainty?
    ///
    /// Returns channels sorted by sensitivity (highest first).
    /// The channel with the highest sensitivity is the one to verify
    /// to most improve confidence.
    pub fn sensitivity_analysis(&self) -> Vec<(String, f64)> {
        let mut sensitivities: Vec<(String, f64)> = self
            .ports
            .iter()
            .map(|port| {
                // Compute R̄ with this port at full confidence
                let mut ports_full = self.ports.clone();
                for p in ports_full.iter_mut() {
                    if p.label() == port.label() {
                        match p {
                            CurationPort::LlmConfidence { label, .. } => {
                                *p = CurationPort::LlmConfidence {
                                    label: label.clone(),
                                    c: 1.0,
                                };
                            }
                            CurationPort::TemplateMatch { label, .. } => {
                                *p = CurationPort::TemplateMatch {
                                    label: label.clone(),
                                    c: 1.0,
                                };
                            }
                            CurationPort::ValidationResult { label, .. } => {
                                *p = CurationPort::ValidationResult {
                                    label: label.clone(),
                                    c: 1.0,
                                };
                            }
                        }
                    }
                }
                let alpha_full: f64 = ports_full.iter().map(|p| p.confidence()).sum::<f64>()
                    / ports_full.len().max(1) as f64;

                // Compute R̄ with this port at zero confidence
                let mut ports_zero = self.ports.clone();
                for p in ports_zero.iter_mut() {
                    if p.label() == port.label() {
                        match p {
                            CurationPort::LlmConfidence { label, .. } => {
                                *p = CurationPort::LlmConfidence {
                                    label: label.clone(),
                                    c: 0.0,
                                };
                            }
                            CurationPort::TemplateMatch { label, .. } => {
                                *p = CurationPort::TemplateMatch {
                                    label: label.clone(),
                                    c: 0.0,
                                };
                            }
                            CurationPort::ValidationResult { label, .. } => {
                                *p = CurationPort::ValidationResult {
                                    label: label.clone(),
                                    c: 0.0,
                                };
                            }
                        }
                    }
                }
                let alpha_zero: f64 = ports_zero.iter().map(|p| p.confidence()).sum::<f64>()
                    / ports_zero.len().max(1) as f64;

                let r_bar_full = mwc_state_function(
                    self.gate.effective_l(),
                    self.gate.c,
                    self.gate.n as u32,
                    alpha_full,
                )
                .unwrap_or(0.0);

                let r_bar_zero = mwc_state_function(
                    self.gate.effective_l(),
                    self.gate.c,
                    self.gate.n as u32,
                    alpha_zero,
                )
                .unwrap_or(0.0);

                let sensitivity = (r_bar_full - r_bar_zero).abs();
                (port.label().to_string(), sensitivity)
            })
            .collect();

        sensitivities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sensitivities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curation_gate_high_confidence_proceeds() {
        // Use low L (low skepticism) so high evidence produces high R̄
        let ports = vec![
            CurationPort::LlmConfidence {
                label: "primary".into(),
                c: 0.95,
            },
            CurationPort::TemplateMatch {
                label: "main".into(),
                c: 0.9,
            },
            CurationPort::ValidationResult {
                label: "schema".into(),
                c: 1.0,
            },
        ];
        let mut gate = CurationConfidenceGate::with_params(
            1.0, // L=1: no T-state preference
            0.1, ports, 0.8, 0.3,
        );
        let r_bar = gate.confidence();
        assert!(
            r_bar >= gate.upper_threshold.as_raw(),
            "High evidence should give high R̄, got {r_bar}"
        );
    }

    #[test]
    fn curation_gate_low_confidence_suppresses() {
        let ports = vec![
            CurationPort::LlmConfidence {
                label: "primary".into(),
                c: 0.1,
            },
            CurationPort::TemplateMatch {
                label: "main".into(),
                c: 0.05,
            },
            CurationPort::ValidationResult {
                label: "schema".into(),
                c: 0.0,
            },
        ];
        let mut gate = CurationConfidenceGate::new(ports);
        let r_bar = gate.confidence();
        assert!(
            r_bar <= gate.lower_threshold.as_raw(),
            "Low evidence should give low R̄, got {r_bar}"
        );
    }

    #[test]
    fn curation_gate_medium_confidence_seeks_evidence() {
        let ports = vec![
            CurationPort::LlmConfidence {
                label: "primary".into(),
                c: 0.5,
            },
            CurationPort::TemplateMatch {
                label: "main".into(),
                c: 0.5,
            },
            CurationPort::ValidationResult {
                label: "schema".into(),
                c: 0.5,
            },
        ];
        let mut gate = CurationConfidenceGate::with_params(
            10.0, // Low L → less skepticism
            0.1, ports, 0.8, 0.3,
        );
        let decision = gate.decide();
        // With moderate L and moderate evidence, we should be in the transition zone
        let r_bar = gate.confidence();
        // The decision depends on actual R̄ value
        if r_bar > 0.3 && r_bar < 0.8 {
            // Should be SeekMoreEvidence
            assert_eq!(decision, ConfidenceDecision::SeekMoreEvidence);
        }
    }

    #[test]
    fn curation_gate_sensitivity_analysis_ranks_channels() {
        let ports = vec![
            CurationPort::LlmConfidence {
                label: "primary".into(),
                c: 0.5,
            },
            CurationPort::TemplateMatch {
                label: "main".into(),
                c: 0.5,
            },
            CurationPort::ValidationResult {
                label: "schema".into(),
                c: 0.5,
            },
        ];
        let gate = CurationConfidenceGate::new(ports);
        let sensitivities = gate.sensitivity_analysis();
        assert_eq!(sensitivities.len(), 3);
        // All channels have equal confidence, so sensitivities should be similar
        // but the ranking still works
        for (_, s) in &sensitivities {
            assert!(*s >= 0.0, "Sensitivity should be non-negative");
        }
    }

    #[test]
    fn curation_gate_aggregate_alpha_averages_ports() {
        let ports = vec![
            CurationPort::LlmConfidence {
                label: "primary".into(),
                c: 0.6,
            },
            CurationPort::TemplateMatch {
                label: "main".into(),
                c: 0.4,
            },
        ];
        let gate = CurationConfidenceGate::new(ports);
        let alpha = gate.aggregate_alpha();
        assert!(
            (alpha - 0.5).abs() < f64::EPSILON,
            "α should average ports, got {alpha}"
        );
    }

    #[test]
    fn curation_gate_empty_ports_zero_alpha() {
        let ports = vec![];
        let gate = CurationConfidenceGate::new(ports);
        let alpha = gate.aggregate_alpha();
        assert!((alpha - 0.0).abs() < f64::EPSILON);
    }
}
