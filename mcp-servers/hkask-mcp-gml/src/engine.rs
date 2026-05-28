//! MWC computation engine — Monod-Wyman-Changeux model calculations

use crate::types::{Concept, Effector, GmlError};

#[derive(Debug, Default, Clone)]
pub struct MwcEngine;

impl MwcEngine {
    pub fn compute_r_bar(l: f64, c: f64, n: u32, alpha: f64) -> Result<f64, GmlError> {
        if l <= 0.0 {
            return Err(GmlError::InvalidMwcParameters("L must be > 0".into()));
        }
        if c <= 0.0 {
            return Err(GmlError::InvalidMwcParameters("c must be > 0".into()));
        }

        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;

        let numerator = one_plus_alpha.powi(n as i32);
        let denominator = numerator + l * one_plus_c_alpha.powi(n as i32);

        if denominator == 0.0 {
            return Err(GmlError::InvalidMwcParameters("Denominator is zero".into()));
        }

        Ok(numerator / denominator)
    }

    pub fn compute_hill(l: f64, c: f64, n: u32, alpha: f64, _r_bar: f64) -> f64 {
        if alpha == 0.0 || c == 1.0 {
            return 0.0;
        }

        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;

        let numerator = (n as f64)
            * one_plus_alpha.powi(n as i32)
            * l
            * one_plus_c_alpha.powi(n as i32)
            * (c - 1.0)
            * alpha;

        let denominator =
            (one_plus_alpha.powi(n as i32) + l * one_plus_c_alpha.powi(n as i32)).powi(2);

        if denominator == 0.0 {
            return 0.0;
        }

        let hill = numerator / denominator;
        hill.abs()
    }

    pub fn compute_delta_g(r_bar: f64, temperature: f64) -> f64 {
        const R: f64 = 8.314;

        if r_bar <= 0.0 || r_bar >= 1.0 {
            return 0.0;
        }

        let ratio = r_bar / (1.0 - r_bar);
        -R * temperature * ratio.ln()
    }

    pub fn apply_effectors(
        concept: &Concept,
        effectors: &[Effector],
    ) -> Result<(f64, f64, f64), GmlError> {
        let n = concept.ports.len() as u32;
        if n == 0 {
            return Err(GmlError::InvalidInput("No allosteric ports".into()));
        }

        let avg_c = concept.ports.iter().map(|p| p.affinity_c).sum::<f64>() / (n as f64);

        let old_alpha = concept.current_alpha;
        let new_alpha = old_alpha + effectors.iter().map(|e| e.concentration).sum::<f64>();

        let old_r_bar = Self::compute_r_bar(concept.l, avg_c, n, old_alpha)?;
        let new_r_bar = Self::compute_r_bar(concept.l, avg_c, n, new_alpha)?;

        let _old_hill = Self::compute_hill(concept.l, avg_c, n, old_alpha, old_r_bar);
        let new_hill = Self::compute_hill(concept.l, avg_c, n, new_alpha, new_r_bar);

        Ok((new_r_bar, new_hill, new_alpha))
    }
}
