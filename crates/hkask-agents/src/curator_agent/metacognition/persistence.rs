//! Escalation persistence with exponential-backoff retry.

use std::sync::Arc;
use std::time::Duration;

use hkask_storage::EscalationQueue;
use hkask_types::{BotID, InfrastructureError, TemplateID};

/// Maximum retries for persisting an escalation to the queue before
/// the CNS feedback loop is declared broken (P9 — Homeostatic Self-Regulation).
const MAX_ESCALATION_PERSIST_RETRIES: u32 = 3;

/// Base delay between escalation persist retries (exponential backoff).
const ESCALATION_PERSIST_BASE_DELAY_MS: u64 = 100;

/// Persist a single escalation entry with exponential-backoff retry.
///
/// Retries up to `MAX_ESCALATION_PERSIST_RETRIES` times before
/// declaring the CNS feedback loop broken. On final failure, emits a
/// critical-level tracing event so the operator is alerted.
///
/// pre:  queue is a valid EscalationQueue handle
/// post: escalation persisted (Ok) or all retries exhausted (Err)
pub(super) async fn persist_escalation_with_retry(
    queue: &Arc<EscalationQueue>,
    template_id: TemplateID,
    bot_id: BotID,
    output: &str,
    confidence: f64,
    retry_count: u32,
    error_context: &str,
) -> Result<(), hkask_storage::EscalationError> {
    let mut last_error = String::new();
    for attempt in 0..=MAX_ESCALATION_PERSIST_RETRIES {
        match queue.add(
            template_id,
            bot_id,
            output.to_string(),
            confidence,
            retry_count,
            error_context.to_string(),
        ) {
            Ok(_escalation_id) => return Ok(()),
            Err(e) => {
                last_error = e.to_string();
                if attempt < MAX_ESCALATION_PERSIST_RETRIES {
                    let delay_ms = ESCALATION_PERSIST_BASE_DELAY_MS * 2u64.pow(attempt);
                    tracing::warn!(
                        target: "cns.curation.escalation",
                        attempt = attempt + 1,
                        max_retries = MAX_ESCALATION_PERSIST_RETRIES,
                        delay_ms,
                        "Escalation persist failed, retrying..."
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    // All retries exhausted — CNS feedback loop is broken (P9 violation)
    tracing::error!(
        target: "cns.curation.escalation.critical",
        template_id = %template_id,
        error = %last_error,
        max_retries = MAX_ESCALATION_PERSIST_RETRIES,
        "Escalation persistence exhausted all retries — CNS feedback loop broken. Manual intervention required."
    );
    Err(hkask_storage::EscalationError::Infra(
        InfrastructureError::Database(last_error),
    ))
}
