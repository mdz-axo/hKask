//! WebID Registry Integration for Okapi Capabilities
//!
//! Integrates Okapi capability management with hKask agent WebID registry.
//! This allows capability-based authorization to be tied to specific agent identities.

use hkask_types::{TemplateID, WebID};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::capability::{OkapiCapability, OkapiOperation};
use crate::ocap_enforcement::CapabilityQueryPort;

/// WebID-to-capability mapping entry
#[derive(Debug, Clone)]
pub struct WebIDCapabilityEntry {
    /// Agent WebID
    pub webid: WebID,
    /// Capabilities granted to this WebID
    pub capabilities: Vec<OkapiCapability>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last used timestamp
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether entry is active
    pub active: bool,
}

impl WebIDCapabilityEntry {
    /// Create new entry
    pub fn new(webid: WebID, capabilities: Vec<OkapiCapability>) -> Self {
        Self {
            webid,
            capabilities,
            created_at: chrono::Utc::now(),
            last_used_at: None,
            active: true,
        }
    }

    /// Mark as used
    pub fn mark_used(&mut self) {
        self.last_used_at = Some(chrono::Utc::now());
    }

    /// Check if entry has capability for operation
    pub fn has_capability(&self, operation: OkapiOperation) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.has_operation(operation) && !cap.is_expired())
    }

    /// Get best capability for operation
    pub fn get_capability(&self, operation: OkapiOperation) -> Option<&OkapiCapability> {
        self.capabilities
            .iter()
            .find(|cap| cap.has_operation(operation) && !cap.is_expired())
    }
}

/// WebID capability registry
pub struct WebIDCapabilityRegistry {
    entries: RwLock<HashMap<WebID, WebIDCapabilityEntry>>,
    template_scoped: RwLock<HashMap<TemplateID, Vec<WebID>>>,
}

impl WebIDCapabilityRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            template_scoped: RwLock::new(HashMap::new()),
        }
    }

    /// Register capabilities for a WebID
    pub async fn register(
        &self,
        webid: WebID,
        capabilities: Vec<OkapiCapability>,
    ) -> Result<(), RegistryError> {
        let mut entries = self.entries.write().await;

        let entry = WebIDCapabilityEntry::new(webid, capabilities);
        entries.insert(webid, entry);

        Ok(())
    }

    /// Register template-scoped capabilities
    pub async fn register_template_scoped(
        &self,
        webid: WebID,
        template_id: TemplateID,
        capabilities: Vec<OkapiCapability>,
    ) -> Result<(), RegistryError> {
        // Register the capabilities
        self.register(webid, capabilities).await?;

        // Add to template mapping
        let mut template_scoped = self.template_scoped.write().await;
        template_scoped
            .entry(template_id)
            .or_insert_with(Vec::new)
            .push(webid);

        Ok(())
    }

    /// Get capabilities for a WebID
    pub async fn get_capabilities(&self, webid: WebID) -> Option<Vec<OkapiCapability>> {
        let entries = self.entries.read().await;
        entries.get(&webid).map(|entry| {
            let mut entry = entry.clone();
            entry.mark_used();
            entry.capabilities.clone()
        })
    }

    /// Check if WebID has capability for operation
    pub async fn has_capability(&self, webid: WebID, operation: OkapiOperation) -> bool {
        let entries = self.entries.read().await;
        entries
            .get(&webid)
            .map(|entry| entry.has_capability(operation))
            .unwrap_or(false)
    }

    /// Get all WebIDs with template scope
    pub async fn get_template_scoped_webids(&self, template_id: TemplateID) -> Vec<WebID> {
        let template_scoped = self.template_scoped.read().await;
        template_scoped
            .get(&template_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Revoke capabilities for a WebID
    pub async fn revoke(&self, webid: WebID) -> Result<(), RegistryError> {
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(&webid) {
            entry.active = false;
            Ok(())
        } else {
            Err(RegistryError::WebIDNotFound)
        }
    }

    /// Remove capabilities for a WebID
    pub async fn remove(&self, webid: WebID) -> Result<(), RegistryError> {
        let mut entries = self.entries.write().await;
        entries.remove(&webid).ok_or(RegistryError::WebIDNotFound)?;
        Ok(())
    }

    /// Get all active entries
    pub async fn get_active_entries(&self) -> Vec<WebIDCapabilityEntry> {
        let entries = self.entries.read().await;
        entries.values().filter(|e| e.active).cloned().collect()
    }

    /// Get registry statistics
    pub async fn stats(&self) -> RegistryStats {
        let entries = self.entries.read().await;
        let template_scoped = self.template_scoped.read().await;

        let total_entries = entries.len();
        let active_entries = entries.values().filter(|e| e.active).count();
        let total_templates = template_scoped.len();

        RegistryStats {
            total_entries,
            active_entries,
            total_templates,
        }
    }
}

impl Default for WebIDCapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry error
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("WebID not found in registry")]
    WebIDNotFound,

    #[error("Template not found")]
    TemplateNotFound,

    #[error("Capability not found")]
    CapabilityNotFound,

    #[error("Registry error: {0}")]
    Other(String),
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub total_templates: usize,
}

/// Authorize Okapi operation for WebID
pub async fn authorize_operation(
    registry: Arc<WebIDCapabilityRegistry>,
    webid: WebID,
    operation: OkapiOperation,
) -> Result<OkapiCapability, AuthorizationError> {
    let has_cap = registry.has_capability(webid, operation).await;

    if !has_cap {
        return Err(AuthorizationError::CapabilityNotFound);
    }

    let capabilities = registry
        .get_capabilities(webid)
        .await
        .ok_or(AuthorizationError::WebIDNotFound)?;

    capabilities
        .into_iter()
        .find(|cap| cap.has_operation(operation) && !cap.is_expired())
        .ok_or(AuthorizationError::CapabilityNotFound)
}

/// Authorization error
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("WebID not found")]
    WebIDNotFound,

    #[error("Capability not found for operation")]
    CapabilityNotFound,

    #[error("Capability expired")]
    CapabilityExpired,

    #[error("Registry error: {0}")]
    RegistryError(#[from] RegistryError),
}

#[async_trait::async_trait]
impl CapabilityQueryPort for WebIDCapabilityRegistry {
    async fn has_capability(&self, webid: WebID, operation: OkapiOperation) -> bool {
        // Call inherent method explicitly
        let entries = self.entries.read().await;
        entries
            .get(&webid)
            .map(|entry| entry.has_capability(operation))
            .unwrap_or(false)
    }

    async fn get_capabilities(&self, webid: WebID) -> Option<Vec<OkapiCapability>> {
        // Call inherent method explicitly
        let entries = self.entries.read().await;
        entries.get(&webid).map(|entry| entry.capabilities.clone())
    }
}
