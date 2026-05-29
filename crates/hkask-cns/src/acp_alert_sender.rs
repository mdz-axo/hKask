use crate::algedonic_escalation::AcpSender;
use tracing::{info, warn};

/// Sends algedonic alerts to the Curator via ACP protocol.
///
/// This implementation formats alerts as ACP messages and dispatches
/// them through the provided ACP transport channel.
pub struct AcpAlertSender {
    /// Target Curator WebID for alert delivery
    curator_webid: hkask_types::WebID,
    /// Optional callback for actual ACP delivery (injected at wiring time)
    transport: Option<Box<dyn Fn(hkask_types::WebID, String) + Send + Sync>>,
}

impl AcpAlertSender {
    /// Create a new ACP alert sender targeting the given Curator WebID.
    pub fn new(curator_webid: hkask_types::WebID) -> Self {
        Self {
            curator_webid,
            transport: None,
        }
    }

    /// Wire in a transport callback for actual ACP message delivery.
    /// The callback receives (target_webid, message_body) and should
    /// dispatch the message via the ACP runtime.
    pub fn with_transport<F>(mut self, transport: F) -> Self
    where
        F: Fn(hkask_types::WebID, String) + Send + Sync + 'static,
    {
        self.transport = Some(Box::new(transport));
        self
    }
}

impl AcpSender for AcpAlertSender {
    fn send_alert(
        &self,
        domain: &str,
        severity: &str,
        deficit: u64,
        drift_magnitude: f64,
        message: &str,
    ) {
        let alert_message = serde_json::json!({
            "type": "algedonic_alert",
            "domain": domain,
            "severity": severity,
            "deficit": deficit,
            "drift_magnitude": drift_magnitude,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
        .to_string();

        match &self.transport {
            Some(transport) => {
                info!(
                    target: "cns.algedonic.acp",
                    domain = %domain,
                    severity = %severity,
                    curator = %self.curator_webid,
                    "Dispatching algedonic alert to Curator via ACP"
                );
                transport(self.curator_webid, alert_message);
            }
            None => {
                warn!(
                    target: "cns.algedonic.acp",
                    domain = %domain,
                    "ACP transport not wired — alert logged but not delivered to Curator. \
                     Call with_transport() to enable ACP delivery."
                );
            }
        }
    }
}
