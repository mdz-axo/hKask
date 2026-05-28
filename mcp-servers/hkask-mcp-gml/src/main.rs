//! hKask MCP GML — Allosteric Thinking with MWC model and OCAP enforcement

mod capability;
mod engine;
mod server;
mod types;

pub use capability::CapabilityManager;
pub use engine::MwcEngine;
pub use server::GmlServer;
pub use types::*;

use hkask_mcp::server::{ServerContext, run_stdio_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-gml",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| Ok(GmlServer::new(ctx.webid)),
        vec![],
    )
    .await
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_r_bar_l_100_alpha_0() {
        let r_bar = MwcEngine::compute_r_bar(100.0, 0.1, 4, 0.0).unwrap();
        assert!(
            (r_bar - 0.01).abs() < 0.001,
            "Expected R̄ ≈ 0.01, got {}",
            r_bar
        );
    }

    #[test]
    fn test_compute_r_bar_l_1_alpha_0() {
        let r_bar = MwcEngine::compute_r_bar(1.0, 0.1, 4, 0.0).unwrap();
        assert!(
            (r_bar - 0.5).abs() < 0.001,
            "Expected R̄ = 0.5, got {}",
            r_bar
        );
    }

    #[test]
    fn test_compute_r_bar_invalid_l() {
        assert!(MwcEngine::compute_r_bar(0.0, 0.1, 4, 1.0).is_err());
        assert!(MwcEngine::compute_r_bar(-1.0, 0.1, 4, 1.0).is_err());
    }

    #[test]
    fn test_compute_delta_g() {
        let delta_g = MwcEngine::compute_delta_g(0.5, 298.0);
        assert!((delta_g - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_create_capability_token() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec!["bind_effector".to_string()],
            scope: None,
            effector_budget: Some(50.0),
            expires_in_seconds: Some(86400),
        };
        let token = manager.create_capability(request).unwrap();
        assert_eq!(token.issuer, "did:webid:curator");
        assert!(!token.signature.is_empty());
    }

    #[test]
    fn test_verify_capability_valid() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec!["bind_effector".to_string()],
            scope: None,
            effector_budget: None,
            expires_in_seconds: None,
        };
        let token = manager.create_capability(request).unwrap();
        let verification = manager
            .verify_capability(VerifyCapabilityRequest {
                token: token.clone(),
                operation: "bind_effector".to_string(),
                scope: None,
            })
            .unwrap();
        assert!(verification.valid);
    }

    #[test]
    fn test_verify_capability_wrong_operation() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec!["bind_effector".to_string()],
            scope: None,
            effector_budget: None,
            expires_in_seconds: None,
        };
        let token = manager.create_capability(request).unwrap();
        let verification = manager
            .verify_capability(VerifyCapabilityRequest {
                token,
                operation: "compute_equilibrium".to_string(),
                scope: None,
            })
            .unwrap();
        assert!(!verification.valid);
    }

    #[test]
    fn test_check_effector_budget() {
        let manager = CapabilityManager::new().unwrap();
        let request = CreateCapabilityRequest {
            issuer: "did:webid:curator".to_string(),
            subject: "did:webid:researcher".to_string(),
            operations: vec![],
            scope: None,
            effector_budget: Some(50.0),
            expires_in_seconds: None,
        };
        let token = manager.create_capability(request).unwrap();
        assert!(manager.check_effector_budget(&token, 30.0).unwrap());
        assert!(!manager.check_effector_budget(&token, 100.0).unwrap());
    }
}
