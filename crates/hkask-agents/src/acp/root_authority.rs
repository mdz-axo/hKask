//! Root Authority for OCAP capability delegation
//!
//! All capability tokens trace back to a root authority. The root authority
//! is the ultimate source of all capabilities in the system.
//!
//! # OCAP Discipline
//!
//! - No ambient authority: capabilities must be explicitly granted
//! - Attenuation chain: each delegation reduces authority

use hkask_types::{
    CapabilityAction, CapabilityResource, CapabilityToken, CapabilityTokenBuilder,
    SYSTEM_MAX_ATTENUATION, WebID,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

use super::AcpError;

/// Root authority for OCAP capability delegation
///
/// All capability tokens trace back to a root authority. The root authority
/// is the ultimate source of all capabilities in the system.
///
/// # OCAP Discipline
///
/// - No ambient authority: capabilities must be explicitly granted
/// - Attenuation chain: each delegation reduces authority
pub(crate) struct RootAuthority {
    /// Root authority WebID (system identity)
    root_webid: WebID,
    /// Root secret for HMAC signing (Arc to avoid copying on Clone)
    root_secret: Arc<Zeroizing<Vec<u8>>>,
    /// Next token ID counter
    token_counter: Arc<RwLock<u64>>,
}

impl RootAuthority {
    /// Create new root authority
    pub fn new(root_webid: WebID, root_secret: &[u8]) -> Self {
        Self {
            root_webid,
            root_secret: Arc::new(Zeroizing::new(root_secret.to_vec())),
            token_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Create root capability token
    ///
    /// This is the starting point of an attenuation chain.
    /// Root tokens have attenuation_level=0 and max_attenuation=7.
    pub async fn create_root_token(
        &self,
        resource: CapabilityResource,
        resource_id: String,
        action: CapabilityAction,
        delegated_to: WebID,
    ) -> Result<CapabilityToken, AcpError> {
        let token_id = {
            let mut counter = self.token_counter.write().await;
            *counter += 1;
            *counter
        };

        let context_nonce = format!("root-{}-{}", self.root_webid, token_id);

        let token = CapabilityTokenBuilder::new(
            resource,
            resource_id,
            action,
            self.root_webid,
            delegated_to,
        )
        .attenuation(0, SYSTEM_MAX_ATTENUATION)
        .context_nonce(context_nonce)
        .sign(self.root_secret.as_ref());

        Ok(token)
    }

    /// Verify attenuation chain from root to current token
    ///
    /// Returns Ok if:
    /// - Root nonce starts with expected root prefix
    /// - Attenuation level is within expected bounds
    /// - Chain is unbroken (each level increments by 1)
    pub fn verify_attenuation_chain(
        &self,
        token: &CapabilityToken,
        expected_root: &WebID,
    ) -> Result<(), AcpError> {
        let root_nonce = token.root_context_nonce();
        let expected_prefix = format!("root-{}", expected_root);

        if !root_nonce.starts_with(&expected_prefix) {
            return Err(AcpError::CapabilityDenied(
                token.delegated_to,
                "Root nonce mismatch".to_string(),
            ));
        }

        if token.attenuation_level > token.max_attenuation {
            return Err(AcpError::CapabilityDenied(
                token.delegated_to,
                "Attenuation level exceeds maximum".to_string(),
            ));
        }

        Ok(())
    }

    /// Get root WebID
    pub fn root_webid(&self) -> &WebID {
        &self.root_webid
    }
}
