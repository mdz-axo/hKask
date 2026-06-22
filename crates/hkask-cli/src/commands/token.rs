//! Token issuance and management — DelegationToken lifecycle.
//!
//! `kask token issue` — issue an Ed25519-signed DelegationToken for a replicant
//! `kask token list` — list tokens issued for a replicant
//! `kask token revoke` — revoke a token by ID

use crate::cli::TokenAction;
use hkask_services::ServiceError;
use hkask_types::{AgentKind, WebID};

/// Parse a human-readable TTL string into seconds.
/// Supports: "30s", "5m", "24h", "7d".
fn parse_ttl(ttl: &str) -> Result<i64, String> {
    let (value_str, unit) = ttl.split_at(ttl.len().saturating_sub(1));
    let value: i64 = value_str
        .parse()
        .map_err(|_| format!("Invalid TTL value: {}", value_str))?;
    let multiplier = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        _ => return Err(format!("Unknown TTL unit: {unit}. Use s, m, h, or d.")),
    };
    Ok(value * multiplier)
}

/// Issue a new DelegationToken for a replicant.
///
/// Returns the token serialized as JSON (for use as HKASK_DELEGATION_TOKEN).
pub async fn token_issue(
    replicant: &str,
    capabilities: Vec<String>,
    ttl: &str,
) -> Result<String, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    let webid = WebID::from_persona(replicant.as_bytes());
    let ttl_secs = parse_ttl(ttl).map_err(|e| ServiceError::Config {
        source: None,
        message: e,
    })?;
    let expires_at = chrono::Utc::now().timestamp() + ttl_secs;

    let (_system_webid, a2a) = ctx.identity();
    let mut token = a2a
        .register_agent(webid, AgentKind::Replicant, capabilities.clone())
        .await
        .map_err(|e| ServiceError::A2A {
            message: e.to_string(),
        })?;

    // Set expiry on the token
    token.expires_at = Some(expires_at);

    // Persist to agent registry for listing
    let def = hkask_storage::AgentDefinition {
        name: replicant.to_string(),
        agent_kind: AgentKind::Replicant,
        charter: None,
        capabilities,
        rights: vec![],
        responsibilities: vec![],
    };
    let reg = hkask_storage::RegisteredAgent {
        definition: def,
        token_hash: hex::encode(token.signature_bytes()),
        registered_at: hkask_types::time::now_rfc3339(),
        source_yaml: String::new(),
    };
    ctx.agent_registry_store()
        .insert(&reg)
        .map_err(|e| ServiceError::AgentRegistryStore {
            message: e.to_string(),
        })?;

    serde_json::to_string_pretty(&token).map_err(|e| ServiceError::Config {
        source: Some(Box::new(e)),
        message: "Failed to serialize token".into(),
    })
}

/// List tokens for a replicant (or all replicants if None).
pub fn token_list(replicant: Option<&str>) -> Result<Vec<TokenEntry>, ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    let agents =
        ctx.agent_registry_store()
            .list()
            .map_err(|e| ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })?;
    let entries: Vec<TokenEntry> = agents
        .into_iter()
        .filter(|a| replicant.is_none_or(|r| a.definition.name == r))
        .map(|a| TokenEntry {
            name: a.definition.name,
            token_hash: a.token_hash,
            capabilities: a.definition.capabilities,
            registered_at: a.registered_at,
        })
        .collect();
    Ok(entries)
}

/// Revoke a token by ID.
pub async fn token_revoke(token_id: &str) -> Result<(), ServiceError> {
    let ctx = crate::commands::helpers::build_service_context();
    let (_system_webid, a2a) = ctx.identity();
    a2a.revoke_capability(token_id).await;
    Ok(())
}

/// Display-friendly token entry for listing.
#[derive(Debug)]
pub struct TokenEntry {
    pub name: String,
    pub token_hash: String,
    pub capabilities: Vec<String>,
    pub registered_at: String,
}

/// Dispatch a token subcommand.
pub fn run_token(rt: &tokio::runtime::Runtime, action: TokenAction) {
    match action {
        TokenAction::Issue {
            replicant,
            capabilities,
            ttl,
        } => match rt.block_on(token_issue(&replicant, capabilities, &ttl)) {
            Ok(token_json) => {
                println!("{}", token_json);
                eprintln!("Token issued for {replicant} (TTL: {ttl}). Store this securely.");
                eprintln!("Use in IDE:  HKASK_DELEGATION_TOKEN='{token_json}'");
            }
            Err(e) => eprintln!("Token issue failed: {e}"),
        },
        TokenAction::List { replicant } => match token_list(replicant.as_deref()) {
            Ok(entries) if entries.is_empty() => {
                println!("No tokens found.");
            }
            Ok(entries) => {
                for e in &entries {
                    println!(
                        "{} — {} — {}",
                        e.name,
                        e.capabilities.join(", "),
                        e.registered_at
                    );
                }
            }
            Err(e) => eprintln!("Token list failed: {e}"),
        },
        TokenAction::Revoke { token_id } => match rt.block_on(token_revoke(&token_id)) {
            Ok(()) => println!("Token {token_id} revoked."),
            Err(e) => eprintln!("Token revoke failed: {e}"),
        },
    }
}
