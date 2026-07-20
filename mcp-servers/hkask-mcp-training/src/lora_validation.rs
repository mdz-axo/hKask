//! LoRA/QLoRA training-config validation — math-contract gates.
//!
//! Implements the static subset of the `lora-training` skill's quality gates
//! as Rust assertions. Called by `training_submit` before pod creation to
//! catch config errors that would silently degrade model quality or waste
//! GPU time.
//!
//! Gates implemented (static subset — runtime gates like gradient flow are
//! documented but not enforced here):
//!
//! - G-M1: No-op-at-init invariant (init_lora_weights produces ΔW=0 at step 0)
//! - G-M2: Merge equivalence (bias='none' required for must-merge inference)
//! - G-M3: Scaling form (α/r or α/√r, never raw α or 1)
//! - G-M4: Rank budget (r < min(d_in, d_out), warn if r ≥ 0.5×min)
//! - G-Q1: Frozen base quantized (QLoRA mode: load_in_4bit + nf4)
//! - G-Q2: Adapter dtype (compute dtype is bf16/fp16, not fp32)
//! - G-Q4: No silent upcast (QLoRA mode: bf16=true, not fp16-only)
//!
//! Gates NOT enforced here (require runtime instrumentation):
//! - G-M5: Trainable param count (needs model loaded)
//! - G-Q3: Gradient flow (needs backward pass)
//! - G-Q5: Paged optimizer (conditional on peak memory)
//! - G-Q6: NF4 optimality (needs weight distribution analysis)
//! - G-D1..G-D3: Data/eval gates (need dataset stats)
//! - G-F1..G-F2: Forgetting gates (post-training)
//!
//! Anchored to: LoRA (arXiv:2106.09685), QLoRA (arXiv:2305.14314),
//! rsLoRA (arXiv:2312.03732), PEFT v0.19.0.

use crate::providers::types::{LoraParams, QuantizationParams, TrainingParams};

/// Severity of a validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Hard refusal — do not submit the job.
    Refuse,
    /// Soft warning — submit but flag in telemetry.
    Warn,
    /// Informational — no action needed.
    Info,
}

/// A single validation finding from a math-contract gate.
#[derive(Debug, Clone)]
pub struct ValidationFinding {
    /// Gate ID (e.g., "G-M1", "G-Q1").
    pub gate_id: &'static str,
    /// Severity: refuse, warn, or info.
    pub severity: ValidationSeverity,
    /// Human-readable message with the specific violation.
    pub message: String,
    /// Source citation (arXiv paper section or PEFT docs section).
    pub source: &'static str,
    /// Concrete remediation recommendation.
    pub remediation: String,
}

/// Validate training params against the LoRA/QLoRA math-contract gates.
///
/// Returns a list of findings. If any finding has `Refuse` severity, the
/// caller must not submit the job. `Warn` findings should be logged but
/// do not block submission.
pub fn validate_training_params(params: &TrainingParams) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();

    // G-M1: No-op-at-init invariant.
    // PEFT default init (B=0, A~Gaussian) makes ΔW=0 at step 0.
    // Our LoraParams doesn't expose init_lora_weights yet — the default
    // (r, alpha, dropout, target_modules, modules_to_save, use_rslora)
    // matches PEFT's default init. If we add init_lora_weights later,
    // this gate should check it.
    // Source: LoRA paper §4.1; PEFT v0.19.0 LoraConfig.init_lora_weights docstring.
    // (No finding emitted — default init is safe. Documented for completeness.)

    // G-M2: Merge equivalence.
    // bias='none' is the only safe setting for must-merge inference.
    // Our LoraParams doesn't expose bias yet — default is 'none' (PEFT default).
    // If we add bias later, this gate should refuse bias='all'/'lora_only'
    // when inference requires merging.
    // Source: LoRA paper §4.2; PEFT v0.19.0 LoraConfig.bias docstring.
    // (No finding emitted — default bias is safe. Documented for completeness.)

    // G-M3: Scaling form.
    // scaling = α/r (or α/√r if use_rslora).
    // We check that alpha and r are non-zero and that the ratio is reasonable.
    validate_scaling_form(&params.lora, &mut findings);

    // G-M4: Rank budget.
    // r should be < min(d_in, d_out). We don't know d_in/d_out without the
    // model loaded, but we can warn on absurdly high r.
    validate_rank_budget(&params.lora, &mut findings);

    // G-Q1: Frozen base quantized (QLoRA mode only).
    // If load_in_4bit is true, bnb_4bit_quant_type must be 'nf4'.
    validate_qlora_quantization(&params.quantization, &mut findings);

    // G-Q2: Adapter dtype (compute dtype).
    // If QLoRA mode, bnb_4bit_compute_dtype should be bf16 or fp16, not fp32.
    validate_compute_dtype(&params.quantization, &mut findings);

    // G-Q4: No silent upcast.
    // If QLoRA mode, bf16 should be true (not fp16-only, which can upcast).
    validate_no_silent_upcast(params, &mut findings);

    findings
}

/// G-M3: Scaling form validation.
///
/// scaling = α/r (default) or α/√r (if use_rslora).
/// Refuse if r=0 or alpha=0 (division by zero).
/// Warn if r > 64 and use_rslora is false (should use rsLoRA for high rank).
fn validate_scaling_form(lora: &LoraParams, findings: &mut Vec<ValidationFinding>) {
    if lora.r == 0 {
        findings.push(ValidationFinding {
            gate_id: "G-M3",
            severity: ValidationSeverity::Refuse,
            message: format!("LoRA rank r=0 — division by zero in scaling α/r"),
            source: "LoRA paper §4.1 (α/r scaling); rsLoRA arXiv:2312.03732",
            remediation: "Set r to a positive integer (typical: 8–64)".to_string(),
        });
    }
    if lora.alpha == 0 {
        findings.push(ValidationFinding {
            gate_id: "G-M3",
            severity: ValidationSeverity::Refuse,
            message: format!("LoRA alpha=0 — scaling factor is zero, adapter has no effect"),
            source: "LoRA paper §4.1 (α/r scaling)",
            remediation: "Set alpha to a positive integer (typical: 2×r)".to_string(),
        });
    }
    // rsLoRA recommendation for high rank.
    if lora.r > 64 && !lora.use_rslora {
        findings.push(ValidationFinding {
            gate_id: "G-M3",
            severity: ValidationSeverity::Warn,
            message: format!(
                "LoRA rank r={} > 64 without use_rslora — scaling α/r underperforms α/√r at high rank",
                lora.r
            ),
            source: "rsLoRA paper arXiv:2312.03732 (Rank-Stabilized LoRA)",
            remediation: format!(
                "Set use_rslora=true, or reduce r to ≤64 (current scaling: {}/{})",
                lora.alpha, lora.r
            ),
        });
    }
}

/// G-M4: Rank budget validation.
///
/// r should be < min(d_in, d_out). Without the model loaded we can't check
/// the exact bound, but we warn on absurdly high r that defeats the low-rank
/// premise.
fn validate_rank_budget(lora: &LoraParams, findings: &mut Vec<ValidationFinding>) {
    if lora.r > 128 {
        findings.push(ValidationFinding {
            gate_id: "G-M4",
            severity: ValidationSeverity::Warn,
            message: format!(
                "LoRA rank r={} > 128 — defeats low-rank premise; consider full fine-tuning",
                lora.r
            ),
            source: "LoRA paper §4.3 (rank sufficiency experiments)",
            remediation: "Reduce r to ≤128, or use full fine-tuning if the task requires high rank"
                .to_string(),
        });
    }
    if lora.r > 256 {
        findings.push(ValidationFinding {
            gate_id: "G-M4",
            severity: ValidationSeverity::Refuse,
            message: format!(
                "LoRA rank r={} > 256 — not low-rank; LoRA provides no benefit at this rank",
                lora.r
            ),
            source: "LoRA paper §4.3 (rank sufficiency experiments)",
            remediation: "Use full fine-tuning, or reduce r significantly".to_string(),
        });
    }
}

/// G-Q1: QLoRA quantization validation.
///
/// If load_in_4bit is true, bnb_4bit_quant_type must be 'nf4' (not 'fp4').
/// NF4 is information-theoretically optimal for normally-distributed weights.
fn validate_qlora_quantization(quant: &QuantizationParams, findings: &mut Vec<ValidationFinding>) {
    if quant.load_in_4bit {
        match &quant.bnb_4bit_quant_type {
            None => {
                findings.push(ValidationFinding {
                    gate_id: "G-Q1",
                    severity: ValidationSeverity::Warn,
                    message: "QLoRA mode (load_in_4bit=true) without bnb_4bit_quant_type — defaults to fp4, but nf4 is optimal".to_string(),
                    source: "QLoRA paper §3 (NF4 — 4-bit NormalFloat)",
                    remediation: "Set bnb_4bit_quant_type=\"nf4\"".to_string(),
                });
            }
            Some(t) if t != "nf4" => {
                findings.push(ValidationFinding {
                    gate_id: "G-Q1",
                    severity: ValidationSeverity::Warn,
                    message: format!(
                        "QLoRA mode with bnb_4bit_quant_type=\"{}\" — nf4 is information-theoretically optimal for normally-distributed weights",
                        t
                    ),
                    source: "QLoRA paper §3 (NF4 derivation)",
                    remediation: "Set bnb_4bit_quant_type=\"nf4\"".to_string(),
                });
            }
            _ => {} // nf4 — pass
        }
        if !quant.bnb_4bit_use_double_quant {
            findings.push(ValidationFinding {
                gate_id: "G-Q1",
                severity: ValidationSeverity::Info,
                message: "QLoRA mode without bnb_4bit_use_double_quant — double quantization saves ~0.37 bits/param".to_string(),
                source: "QLoRA paper §3 (double quantization)",
                remediation: "Set bnb_4bit_use_double_quant=true for additional memory savings".to_string(),
            });
        }
    }
}

/// G-Q2: Compute dtype validation.
///
/// If QLoRA mode, bnb_4bit_compute_dtype should be bf16 or fp16, not fp32.
/// fp32 compute through a 4-bit base wastes the memory savings.
fn validate_compute_dtype(quant: &QuantizationParams, findings: &mut Vec<ValidationFinding>) {
    if quant.load_in_4bit {
        match &quant.bnb_4bit_compute_dtype {
            None => {
                // Default is fp16 in bitsandbytes — acceptable.
            }
            Some(dt) if dt == "fp32" => {
                findings.push(ValidationFinding {
                    gate_id: "G-Q2",
                    severity: ValidationSeverity::Refuse,
                    message: "QLoRA mode with bnb_4bit_compute_dtype=\"fp32\" — fp32 compute through 4-bit base wastes memory (silent 2× upcast)".to_string(),
                    source: "QLoRA paper §3 (compute in bf16 through frozen base)",
                    remediation: "Set bnb_4bit_compute_dtype=\"bf16\" or \"fp16\"".to_string(),
                });
            }
            Some(dt) if dt != "bf16" && dt != "fp16" => {
                findings.push(ValidationFinding {
                    gate_id: "G-Q2",
                    severity: ValidationSeverity::Warn,
                    message: format!(
                        "QLoRA mode with bnb_4bit_compute_dtype=\"{}\" — expected \"bf16\" or \"fp16\"",
                        dt
                    ),
                    source: "QLoRA paper §3 (compute dtype)",
                    remediation: "Set bnb_4bit_compute_dtype=\"bf16\" (preferred) or \"fp16\"".to_string(),
                });
            }
            _ => {} // bf16 or fp16 — pass
        }
    }
}

/// G-Q4: No silent upcast.
///
/// If QLoRA mode, bf16 should be true (not fp16-only). fp16 can cause
/// silent upcast to fp32 in some operations, doubling memory.
fn validate_no_silent_upcast(params: &TrainingParams, findings: &mut Vec<ValidationFinding>) {
    if params.quantization.load_in_4bit {
        if !params.advanced.bf16 && params.advanced.fp16 {
            findings.push(ValidationFinding {
                gate_id: "G-Q4",
                severity: ValidationSeverity::Warn,
                message: "QLoRA mode with fp16=true and bf16=false — fp16 can cause silent upcast to fp32 in some operations".to_string(),
                source: "QLoRA paper §3 (bf16 compute); PEFT prepare_model_for_kbit_training docstring",
                remediation: "Set bf16=true (preferred over fp16 for QLoRA)".to_string(),
            });
        }
    }
}

/// Returns true if any finding has `Refuse` severity — the job must not be submitted.
pub fn has_refusals(findings: &[ValidationFinding]) -> bool {
    findings
        .iter()
        .any(|f| f.severity == ValidationSeverity::Refuse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::{AdvancedParams, OptimizationParams, SequenceParams};

    fn default_params() -> TrainingParams {
        TrainingParams {
            num_epochs: 3,
            batch_size: 4,
            learning_rate: 2e-4,
            lora: LoraParams::default(),
            quantization: QuantizationParams::default(),
            optimization: OptimizationParams::default(),
            sequence: SequenceParams::default(),
            advanced: AdvancedParams::default(),
        }
    }

    #[test]
    fn default_params_pass_all_gates() {
        let findings = validate_training_params(&default_params());
        let refusals: Vec<_> = findings
            .iter()
            .filter(|f| f.severity == ValidationSeverity::Refuse)
            .collect();
        assert!(
            refusals.is_empty(),
            "Default params should not refuse: {:?}",
            refusals
        );
    }

    #[test]
    fn rank_zero_refuses() {
        let mut params = default_params();
        params.lora.r = 0;
        let findings = validate_training_params(&params);
        assert!(has_refusals(&findings));
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-M3" && f.severity == ValidationSeverity::Refuse)
        );
    }

    #[test]
    fn alpha_zero_refuses() {
        let mut params = default_params();
        params.lora.alpha = 0;
        let findings = validate_training_params(&params);
        assert!(has_refusals(&findings));
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-M3" && f.severity == ValidationSeverity::Refuse)
        );
    }

    #[test]
    fn high_rank_without_rslora_warns() {
        let mut params = default_params();
        params.lora.r = 128;
        params.lora.use_rslora = false;
        let findings = validate_training_params(&params);
        assert!(!has_refusals(&findings));
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-M3" && f.severity == ValidationSeverity::Warn)
        );
    }

    #[test]
    fn rank_over_256_refuses() {
        let mut params = default_params();
        params.lora.r = 512;
        let findings = validate_training_params(&params);
        assert!(has_refusals(&findings));
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-M4" && f.severity == ValidationSeverity::Refuse)
        );
    }

    #[test]
    fn qlora_without_nf4_warns() {
        let mut params = default_params();
        params.quantization.load_in_4bit = true;
        params.quantization.bnb_4bit_quant_type = None;
        let findings = validate_training_params(&params);
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-Q1" && f.severity == ValidationSeverity::Warn)
        );
    }

    #[test]
    fn qlora_with_fp4_warns() {
        let mut params = default_params();
        params.quantization.load_in_4bit = true;
        params.quantization.bnb_4bit_quant_type = Some("fp4".to_string());
        let findings = validate_training_params(&params);
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-Q1" && f.severity == ValidationSeverity::Warn)
        );
    }

    #[test]
    fn qlora_with_nf4_passes() {
        let mut params = default_params();
        params.quantization.load_in_4bit = true;
        params.quantization.bnb_4bit_quant_type = Some("nf4".to_string());
        params.quantization.bnb_4bit_use_double_quant = true;
        let findings = validate_training_params(&params);
        assert!(findings.iter().all(|f| f.gate_id != "G-Q1"));
    }

    #[test]
    fn qlora_with_fp32_compute_refuses() {
        let mut params = default_params();
        params.quantization.load_in_4bit = true;
        params.quantization.bnb_4bit_quant_type = Some("nf4".to_string());
        params.quantization.bnb_4bit_compute_dtype = Some("fp32".to_string());
        let findings = validate_training_params(&params);
        assert!(has_refusals(&findings));
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-Q2" && f.severity == ValidationSeverity::Refuse)
        );
    }

    #[test]
    fn qlora_with_bf16_compute_passes() {
        let mut params = default_params();
        params.quantization.load_in_4bit = true;
        params.quantization.bnb_4bit_quant_type = Some("nf4".to_string());
        params.quantization.bnb_4bit_compute_dtype = Some("bf16".to_string());
        let findings = validate_training_params(&params);
        assert!(findings.iter().all(|f| f.gate_id != "G-Q2"));
    }

    #[test]
    fn qlora_fp16_only_warns_silent_upcast() {
        let mut params = default_params();
        params.quantization.load_in_4bit = true;
        params.advanced.fp16 = true;
        params.advanced.bf16 = false;
        let findings = validate_training_params(&params);
        assert!(
            findings
                .iter()
                .any(|f| f.gate_id == "G-Q4" && f.severity == ValidationSeverity::Warn)
        );
    }
}
