//! Capability token manager — OCAP enforcement with ed25519 signing

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::types::{
    CapabilityToken, CreateCapabilityRequest, GmlError, TokenVerification, VerifyCapabilityRequest,
};

#[derive(Debug)]
pub struct CapabilityManager {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl CapabilityManager {
    pub fn new() -> Result<Self, GmlError> {
        let mut csprng = rand::thread_rng();
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = VerifyingKey::from(&signing_key);

        Ok(Self {
            signing_key,
            verifying_key,
        })
    }

    fn generate_token_id(issuer: &str, subject: &str, issued_at: DateTime<Utc>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(issuer.as_bytes());
        hasher.update(subject.as_bytes());
        hasher.update(issued_at.to_rfc3339().as_bytes());
        let hash = hasher.finalize();
        format!("gml_{}", hex::encode(&hash[..8]))
    }

    fn sign_token(&self, token_data: &str) -> Result<String, GmlError> {
        let mut hasher = Sha256::new();
        hasher.update(token_data.as_bytes());
        let message_hash = hasher.finalize();

        let signature = self.signing_key.sign(&message_hash);
        Ok(hex::encode(signature.to_bytes()))
    }

    fn verify_signature(&self, token_data: &str, signature_hex: &str) -> Result<bool, GmlError> {
        let signature_bytes = hex::decode(signature_hex)?;
        let signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());

        let mut hasher = Sha256::new();
        hasher.update(token_data.as_bytes());
        let message_hash = hasher.finalize();

        match self.verifying_key.verify(&message_hash, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn create_capability(
        &self,
        request: CreateCapabilityRequest,
    ) -> Result<CapabilityToken, GmlError> {
        let now = Utc::now();
        let expires_at = request
            .expires_in_seconds
            .map(|secs| now + chrono::Duration::seconds(secs));

        let token_id = Self::generate_token_id(&request.issuer, &request.subject, now);

        let token_data = format!(
            "{}:{}:{}:{}:{}:{}",
            token_id,
            request.issuer,
            request.subject,
            request.operations.join(","),
            now.to_rfc3339(),
            expires_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "never".to_string())
        );

        let signature = self.sign_token(&token_data)?;

        Ok(CapabilityToken {
            id: token_id,
            issuer: request.issuer,
            subject: request.subject,
            operations: request.operations,
            scope: request.scope,
            effector_budget: request.effector_budget,
            issued_at: now,
            expires_at,
            signature,
        })
    }

    pub fn verify_capability(
        &self,
        request: VerifyCapabilityRequest,
    ) -> Result<TokenVerification, GmlError> {
        let token = &request.token;

        if let Some(expires) = token.expires_at
            && Utc::now() > expires
        {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some("Token expired".into()),
            });
        }

        let token_data = format!(
            "{}:{}:{}:{}:{}:{}",
            token.id,
            token.issuer,
            token.subject,
            token.operations.join(","),
            token.issued_at.to_rfc3339(),
            token
                .expires_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "never".to_string())
        );

        let signature_valid = self.verify_signature(&token_data, &token.signature)?;

        if !signature_valid {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some("Invalid signature".into()),
            });
        }

        if !token.operations.contains(&request.operation) {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some(format!("Operation '{}' not allowed", request.operation)),
            });
        }

        if let Some(scope) = request.scope
            && let Some(token_scope) = &token.scope
            && !token_scope.contains(&scope)
        {
            return Ok(TokenVerification {
                valid: false,
                token_id: token.id.clone(),
                subject: token.subject.clone(),
                operations: token.operations.clone(),
                error: Some(format!("Scope '{}' not allowed", scope)),
            });
        }

        Ok(TokenVerification {
            valid: true,
            token_id: token.id.clone(),
            subject: token.subject.clone(),
            operations: token.operations.clone(),
            error: None,
        })
    }

    pub fn check_effector_budget(
        &self,
        token: &CapabilityToken,
        concentration: f64,
    ) -> Result<bool, GmlError> {
        if let Some(budget) = token.effector_budget {
            Ok(concentration <= budget)
        } else {
            Ok(true)
        }
    }
}
