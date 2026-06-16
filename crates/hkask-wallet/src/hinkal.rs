//! HinkalPort — privacy-preserving deposits and withdrawals via Hinkal REST API.
//!
//! # Feature gate
//! This module is only compiled when the `hinkal` feature is enabled.
//! Default builds have zero Hinkal dependencies.
//!
//! # Hinkal protocol `[IS-DECL]`
//! Hinkal provides shielded/private transactions across Ethereum, Solana, Tron,
//! Polygon, Base, Arbitrum, Optimism, and Arc. The protocol uses a Shielded Pool
//! with zkSNARKs (Groth16), stealth addresses, and relayers.
//!
//! # REST API integration `[IS-DECL]`
//! Hinkal exposes a REST API at `https://api.hinkal.io` running inside a
//! GCP Confidential VM (AMD SEV). The API handles all cryptography internally:
//! UTXO decryption, zero-knowledge proof generation, and transaction building.
//! Callers authenticate with wallet-signed messages — no SDK required.

use async_trait::async_trait;
use chrono::Utc;
use hkask_types::WebID;
use hkask_types::cns::CnsSpan;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span, SpanNamespace};
use hkask_types::wallet::{ChainId, TxHash, WalletError, WalletId};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering},
};
use std::time::Duration;

use crate::chain::{ChainPort, DepositEvent};
use crate::privacy::{PrivacyPort, ShieldedTransfer};
use crate::signing;

/// HTTP request timeout for Hinkal API calls.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Hinkal API base URL.
const HINKAL_API_BASE: &str = "https://api.hinkal.io";

/// Solana mainnet chain ID in Hinkal API.
const HINKAL_SOLANA_CHAIN_ID: u64 = 501;

/// Solana USDC mint.
const SOLANA_USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Circuit breaker: maximum consecutive health check failures before
/// the port is considered unhealthy and fails open to transparent mode.
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// Circuit breaker: cooldown duration after max failures before retrying.
const CIRCUIT_BREAKER_COOLDOWN_SECS: u64 = 60;

/// Hinkal session TTL (server-side validity window).
const SESSION_TTL_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest {
    address: String,
    chain_id: u64,
    nonce: String,
    signature: String,
    write_access: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionResponse {
    success: Option<bool>,
    error: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone)]
struct SessionMaterial {
    nonce: String,
    signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawUnsignedPayload {
    nonce: String,
    to_public: String,
    amount_usdc_micro: u64,
    chain_id: u64,
    token: String,
}

/// Payload for shielding (depositing) assets into the Hinkal pool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShieldDepositPayload {
    nonce: String,
    amount_usdc_micro: u64,
    chain_id: u64,
    token: String,
}

/// Unified payload enum for dispatching in submit_signed_tx.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum HinkalPayload {
    Withdraw(WithdrawUnsignedPayload),
    Shield(ShieldDepositPayload),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawRequest {
    address: String,
    chain_id: u64,
    recipient: String,
    nonce: String,
    token_amounts: Vec<TokenAmountRequest>,
    message: String,
    signature: String,
    session_nonce: String,
    session_signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenAmountRequest {
    token: String,
    amount: String,
}

/// Request body for POST /deposit (shield assets into Hinkal pool).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DepositRequest {
    address: String,
    chain_id: u64,
    nonce: String,
    token_amounts: Vec<TokenAmountRequest>,
    message: String,
    signature: String,
    session_nonce: String,
    session_signature: String,
}

#[derive(Debug, Clone)]
struct BalanceTokenAmount {
    token: String,
    amount_usdc_micro: u64,
    memo: Option<String>,
    commitment: Option<String>,
}

/// Hinkal chain port — privacy-preserving deposits via Hinkal REST API.
pub struct HinkalPort {
    /// HTTP client for Hinkal API (rustls, no openssl).
    client: Client,
    /// Hinkal API base URL.
    api_base_url: String,
    /// Treasury public key for session authentication.
    treasury_pubkey: String,
    /// Circuit breaker: count of consecutive health check failures.
    consecutive_failures: AtomicU32,
    /// Circuit breaker: timestamp of when cooldown started (Unix seconds).
    cooldown_start: AtomicI64,
    /// Last observed balance per token (micro units) for delta-based transfer detection.
    last_balances: Mutex<HashMap<String, u64>>,
    /// Cached Hinkal session nonce (read/write auth context).
    session_nonce: Mutex<Option<String>>,
    /// Cached Hinkal session signature paired with `session_nonce`.
    session_signature: Mutex<Option<String>>,
    /// Whether the cached session has write access.
    session_write_access: AtomicBool,
    /// Cached session expiration timestamp (Unix seconds).
    session_expires_at: AtomicI64,
    /// Optional CNS event sink for chain error span emission.
    event_sink: Option<Arc<dyn NuEventSink>>,
}

impl HinkalPort {
    /// Create a new HinkalPort connected to the Hinkal API.
    ///
    /// REQ: P9-wallet-hinkal-port-new
    /// [P9] Motivating: Homeostatic Self-Regulation — privacy port is part of the energy loop
    /// [P4] Constraining: Clear Boundaries — HTTPS-only and non-empty treasury pubkey
    /// pre:  api_base_url is a valid absolute URL
    /// pre:  treasury_pubkey is a non-empty account/public key string
    /// post: HTTP client initialized with rustls TLS
    /// post: circuit breaker initialized with zero failures
    pub fn new(api_base_url: &str, treasury_pubkey: &str) -> Result<Self, WalletError> {
        if api_base_url.trim().is_empty() {
            return Err(Self::chain_error("Hinkal API base URL must not be empty"));
        }
        if treasury_pubkey.trim().is_empty() {
            return Err(Self::chain_error(
                "Hinkal treasury account must not be empty",
            ));
        }

        let trimmed = api_base_url.trim();
        let is_insecure_nonlocal = trimmed.starts_with("http://")
            && !(trimmed.starts_with("http://127.0.0.1")
                || trimmed.starts_with("http://localhost")
                || trimmed.starts_with("http://[::1]"));
        if is_insecure_nonlocal {
            return Err(Self::chain_error(
                "Hinkal API base URL must use https (http allowed only for localhost tests)",
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                WalletError::Infra(hkask_types::InfrastructureError::Database(format!(
                    "failed to build HTTP client for Hinkal API: {e}"
                )))
            })?;

        Ok(HinkalPort {
            client,
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            treasury_pubkey: treasury_pubkey.to_string(),
            consecutive_failures: AtomicU32::new(0),
            cooldown_start: AtomicI64::new(0),
            last_balances: Mutex::new(HashMap::new()),
            session_nonce: Mutex::new(None),
            session_signature: Mutex::new(None),
            session_write_access: AtomicBool::new(false),
            session_expires_at: AtomicI64::new(0),
            event_sink: None,
        })
    }

    /// Create a HinkalPort with default production base URL.
    pub fn with_default_base(treasury_pubkey: &str) -> Result<Self, WalletError> {
        Self::new(HINKAL_API_BASE, treasury_pubkey)
    }

    /// Attach a CNS event sink for chain error span emission.
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    #[allow(dead_code)]
    fn default_actor(&self) -> WebID {
        WebID::from_persona_with_namespace(self.treasury_pubkey.as_bytes(), "wallet-hinkal")
    }

    /// Emit a CNS chain_error span if an event sink is configured.
    fn emit_cns_chain_error_for_actor(&self, actor: &WebID, operation: &str, error_msg: &str) {
        if let Some(ref sink) = self.event_sink {
            let span_obj = Span::new(SpanNamespace::from(CnsSpan::WalletChainError), "error");
            let event = NuEvent::new(
                actor.clone(),
                span_obj,
                Phase::Sense,
                serde_json::json!({
                    "actor": actor.to_string(),
                    "chain": "hinkal",
                    "operation": operation,
                    "error": error_msg,
                }),
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::warn!(target: "hkask.wallet.hinkal", error = %e, "Failed to persist CNS chain_error span");
            }
        }
    }

    #[allow(dead_code)]
    fn emit_cns_chain_error(&self, operation: &str, error_msg: &str) {
        let actor = self.default_actor();
        self.emit_cns_chain_error_for_actor(&actor, operation, error_msg);
    }

    fn chain_error(message: impl Into<String>) -> WalletError {
        WalletError::ChainError {
            chain: ChainId::Hinkal,
            message: message.into(),
        }
    }

    fn parse_amount_field(v: &serde_json::Value) -> Option<u64> {
        match v {
            serde_json::Value::Number(n) => n.as_u64(),
            serde_json::Value::String(s) => s.parse::<u64>().ok(),
            _ => None,
        }
    }

    fn generate_nonce() -> String {
        let bytes: [u8; 16] = rand::random();
        hex::encode(bytes)
    }

    fn build_session_message(nonce: &str, write_access: bool) -> String {
        let mut lines = vec![
            "Authorize Hinkal session".to_string(),
            format!("Session ID: {nonce}"),
        ];
        if write_access {
            lines.push("This signature can also be used to submit transactions.".to_string());
        }
        lines.join("\n")
    }

    fn build_withdraw_message(
        nonce: &str,
        chain_id: u64,
        token_address: &str,
        amount: u64,
        recipient: &str,
    ) -> String {
        format!(
            "Hinkal Enclave\n\nPrimary Type: Withdraw\nNonce: {nonce}\nChain ID: {chain_id}\n\
             Token Amounts:\n  0:\n    Token: {token_address}\n    Amount: {amount}\n\
             Recipient: {recipient}"
        )
    }

    fn build_shield_message(
        nonce: &str,
        chain_id: u64,
        token_address: &str,
        amount: u64,
    ) -> String {
        format!(
            "Hinkal Enclave\n\nPrimary Type: Shield\nNonce: {nonce}\nChain ID: {chain_id}\n\
             Token Amounts:\n  0:\n    Token: {token_address}\n    Amount: {amount}"
        )
    }

    async fn read_response_body(resp: reqwest::Response) -> String {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable-body>".to_string());
        format!("HTTP {}: {}", status.as_u16(), text)
    }

    fn now_unix_seconds() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    fn cached_session(&self, write_access: bool) -> Result<Option<SessionMaterial>, WalletError> {
        let now = Self::now_unix_seconds();
        let expires_at = self.session_expires_at.load(Ordering::Relaxed);
        if expires_at <= now {
            self.session_write_access.store(false, Ordering::Relaxed);
            self.session_expires_at.store(0, Ordering::Relaxed);
            let mut nonce = self
                .session_nonce
                .lock()
                .map_err(|_| Self::chain_error("session nonce lock poisoned"))?;
            let mut signature = self
                .session_signature
                .lock()
                .map_err(|_| Self::chain_error("session signature lock poisoned"))?;
            *nonce = None;
            *signature = None;
            return Ok(None);
        }

        let cached_write = self.session_write_access.load(Ordering::Relaxed);
        if write_access && !cached_write {
            return Ok(None);
        }

        let nonce = self
            .session_nonce
            .lock()
            .map_err(|_| Self::chain_error("session nonce lock poisoned"))?
            .clone();
        let signature = self
            .session_signature
            .lock()
            .map_err(|_| Self::chain_error("session signature lock poisoned"))?
            .clone();

        match (nonce, signature) {
            (Some(nonce), Some(signature)) => Ok(Some(SessionMaterial { nonce, signature })),
            _ => Ok(None),
        }
    }

    fn store_cached_session(
        &self,
        nonce: String,
        signature: String,
        write_access: bool,
    ) -> Result<(), WalletError> {
        *self
            .session_nonce
            .lock()
            .map_err(|_| Self::chain_error("session nonce lock poisoned"))? = Some(nonce);
        *self
            .session_signature
            .lock()
            .map_err(|_| Self::chain_error("session signature lock poisoned"))? = Some(signature);
        self.session_write_access
            .store(write_access, Ordering::Relaxed);
        self.session_expires_at.store(
            Self::now_unix_seconds() + SESSION_TTL_SECS,
            Ordering::Relaxed,
        );
        Ok(())
    }

    async fn create_session(
        &self,
        actor: &WebID,
        write_access: bool,
    ) -> Result<SessionMaterial, WalletError> {
        if let Some(session) = self.cached_session(write_access)? {
            return Ok(session);
        }

        let nonce = Self::generate_nonce();
        let message = Self::build_session_message(&nonce, write_access);
        let signature = signing::sign_message(message.as_bytes())?;
        let signature_hex = hex::encode(signature);

        let req = CreateSessionRequest {
            address: self.treasury_pubkey.clone(),
            chain_id: HINKAL_SOLANA_CHAIN_ID,
            nonce: nonce.clone(),
            signature: signature_hex.clone(),
            write_access,
        };

        let url = format!("{}/create-session", self.api_base_url);
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("create-session request failed: {e}");
                self.emit_cns_chain_error_for_actor(actor, "create_session", &msg);
                Self::chain_error(msg)
            })?;

        if !resp.status().is_success() {
            let body = Self::read_response_body(resp).await;
            let msg = format!(
                "create-session rejected (write_access={write_access}, nonce={nonce}): {body}"
            );
            self.emit_cns_chain_error_for_actor(actor, "create_session", &msg);
            return Err(Self::chain_error(msg));
        }

        let body: CreateSessionResponse = resp.json().await.map_err(|e| {
            let msg = format!("create-session returned invalid JSON response: {e}");
            self.emit_cns_chain_error_for_actor(actor, "create_session", &msg);
            Self::chain_error(msg)
        })?;

        if body.success == Some(false) {
            let msg = format!(
                "create-session failed: {}",
                body.error
                    .or(body.message)
                    .unwrap_or_else(|| "unknown error".to_string())
            );
            self.emit_cns_chain_error_for_actor(actor, "create_session", &msg);
            return Err(Self::chain_error(msg));
        }

        self.store_cached_session(nonce.clone(), signature_hex.clone(), write_access)?;

        Ok(SessionMaterial {
            nonce,
            signature: signature_hex,
        })
    }

    fn parse_balance_entries(
        &self,
        root: &serde_json::Value,
    ) -> Result<Vec<BalanceTokenAmount>, WalletError> {
        fn select_entries(value: &serde_json::Value) -> Option<&Vec<serde_json::Value>> {
            value
                .get("balance")
                .and_then(|v| v.as_array())
                .or_else(|| value.get("balances").and_then(|v| v.as_array()))
                .or_else(|| {
                    value
                        .get("data")
                        .and_then(|d| d.get("balance"))
                        .and_then(|v| v.as_array())
                })
                .or_else(|| {
                    value
                        .get("data")
                        .and_then(|d| d.get("balances"))
                        .and_then(|v| v.as_array())
                })
        }

        let entries = select_entries(root)
            .ok_or_else(|| Self::chain_error("balance response missing balance array"))?;

        let mut out = Vec::with_capacity(entries.len());
        for row in entries {
            let token = row
                .get("token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Self::chain_error("balance row missing token"))?
                .to_string();

            let amount_value = row
                .get("amount")
                .ok_or_else(|| Self::chain_error("balance row missing amount"))?;

            let amount_usdc_micro = Self::parse_amount_field(amount_value)
                .ok_or_else(|| Self::chain_error("balance row amount must be u64/string-u64"))?;

            let memo = row
                .get("memo")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let commitment = row
                .get("commitment")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            out.push(BalanceTokenAmount {
                token,
                amount_usdc_micro,
                memo,
                commitment,
            });
        }

        Ok(out)
    }

    async fn fetch_balance(
        &self,
        actor: &WebID,
        session: &SessionMaterial,
    ) -> Result<Vec<BalanceTokenAmount>, WalletError> {
        let url = format!("{}/balance", self.api_base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("address", self.treasury_pubkey.as_str()),
                ("chainId", "501"),
                ("nonce", session.nonce.as_str()),
                ("signature", session.signature.as_str()),
            ])
            .send()
            .await
            .map_err(|e| {
                let msg = format!("balance request failed: {e}");
                self.emit_cns_chain_error_for_actor(actor, "fetch_balance", &msg);
                Self::chain_error(msg)
            })?;

        if !resp.status().is_success() {
            let body = Self::read_response_body(resp).await;
            let msg = format!(
                "balance request rejected (nonce={}): {}",
                session.nonce, body
            );
            self.emit_cns_chain_error_for_actor(actor, "fetch_balance", &msg);
            return Err(Self::chain_error(msg));
        }

        let value: serde_json::Value = resp.json().await.map_err(|e| {
            let msg = format!("balance returned invalid JSON: {e}");
            self.emit_cns_chain_error_for_actor(actor, "fetch_balance", &msg);
            Self::chain_error(msg)
        })?;
        self.parse_balance_entries(&value)
    }

    /// Poll Hinkal balance endpoint using session authentication.
    async fn monitor_deposits(
        &self,
        actor: &WebID,
    ) -> Result<Vec<BalanceTokenAmount>, WalletError> {
        let session = self.create_session(actor, false).await?;
        self.fetch_balance(actor, &session).await
    }

    fn transfer_commitment(
        &self,
        token: &str,
        prev: u64,
        current: u64,
        explicit: Option<&str>,
    ) -> String {
        if let Some(c) = explicit {
            return c.to_string();
        }
        let mut hasher = Sha256::new();
        hasher.update(self.treasury_pubkey.as_bytes());
        hasher.update(token.as_bytes());
        hasher.update(prev.to_le_bytes());
        hasher.update(current.to_le_bytes());
        let digest = hasher.finalize();
        hex::encode(digest)
    }

    // ── Circuit breaker ──────────────────────────────────────────────────────

    /// Check Hinkal relayer health via `GET /ping`.
    pub async fn check_relayer_health(&self) -> bool {
        let cooldown = self.cooldown_start.load(Ordering::Relaxed);
        if cooldown > 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            if now - cooldown < CIRCUIT_BREAKER_COOLDOWN_SECS as i64 {
                return false;
            }
            self.cooldown_start.store(0, Ordering::Relaxed);
            self.consecutive_failures.store(0, Ordering::Relaxed);
        }

        let url = format!("{}/ping", self.api_base_url);
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
                true
            }
            _ => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= MAX_CONSECUTIVE_FAILURES {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;
                    self.cooldown_start.store(now, Ordering::Relaxed);
                    tracing::warn!(
                        target: "hkask.wallet.hinkal",
                        failures = failures,
                        cooldown_secs = CIRCUIT_BREAKER_COOLDOWN_SECS,
                        "Hinkal relay unhealthy — circuit breaker engaged"
                    );
                }
                false
            }
        }
    }

    /// Whether the relay is currently in cooldown (circuit breaker open).
    pub fn in_cooldown(&self) -> bool {
        let cooldown = self.cooldown_start.load(Ordering::Relaxed);
        if cooldown == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now - cooldown < CIRCUIT_BREAKER_COOLDOWN_SECS as i64
    }

    /// Submit a withdraw (unshield) transaction to POST /withdraw.
    async fn submit_withdraw(
        &self,
        actor: &WebID,
        payload: &WithdrawUnsignedPayload,
    ) -> Result<TxHash, WalletError> {
        if payload.chain_id != HINKAL_SOLANA_CHAIN_ID {
            return Err(Self::chain_error(format!(
                "unsupported chain_id={} for Hinkal withdraw",
                payload.chain_id
            )));
        }
        if payload.amount_usdc_micro == 0 {
            return Err(Self::chain_error("withdraw amount must be > 0"));
        }
        if payload.to_public.trim().is_empty() {
            return Err(Self::chain_error("withdraw recipient must not be empty"));
        }

        let session = self.create_session(actor, true).await?;
        let message = Self::build_withdraw_message(
            &payload.nonce,
            payload.chain_id,
            &payload.token,
            payload.amount_usdc_micro,
            &payload.to_public,
        );
        let signature = signing::sign_message(message.as_bytes())?;

        let req = WithdrawRequest {
            address: self.treasury_pubkey.clone(),
            chain_id: payload.chain_id,
            recipient: payload.to_public.clone(),
            nonce: payload.nonce.clone(),
            token_amounts: vec![TokenAmountRequest {
                token: payload.token.clone(),
                amount: payload.amount_usdc_micro.to_string(),
            }],
            message,
            signature: hex::encode(signature),
            session_nonce: session.nonce,
            session_signature: session.signature,
        };

        let url = format!("{}/withdraw", self.api_base_url);
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("withdraw request failed: {e}");
                self.emit_cns_chain_error_for_actor(actor, "submit_withdraw", &msg);
                Self::chain_error(msg)
            })?;

        if !resp.status().is_success() {
            let body = Self::read_response_body(resp).await;
            let msg = format!("withdraw rejected: {body}");
            self.emit_cns_chain_error_for_actor(actor, "submit_withdraw", &msg);
            return Err(Self::chain_error(msg));
        }

        let v: serde_json::Value = resp.json().await.map_err(|e| {
            let msg = format!("withdraw returned invalid JSON: {e}");
            self.emit_cns_chain_error_for_actor(actor, "submit_withdraw", &msg);
            Self::chain_error(msg)
        })?;

        let tx_hash = v
            .get("txHash")
            .and_then(|x| x.as_str())
            .or_else(|| v.get("tx_hash").and_then(|x| x.as_str()))
            .or_else(|| v.get("hash").and_then(|x| x.as_str()))
            .or_else(|| {
                v.get("data")
                    .and_then(|d| d.get("txHash"))
                    .and_then(|x| x.as_str())
            })
            .ok_or_else(|| {
                let msg = "withdraw response missing tx hash".to_string();
                self.emit_cns_chain_error_for_actor(actor, "submit_withdraw", &msg);
                Self::chain_error(msg)
            })?;

        Ok(TxHash(tx_hash.to_string()))
    }

    /// Submit a shield (deposit) transaction to POST /deposit.
    async fn submit_shield(
        &self,
        actor: &WebID,
        payload: &ShieldDepositPayload,
    ) -> Result<TxHash, WalletError> {
        if payload.chain_id != HINKAL_SOLANA_CHAIN_ID {
            return Err(Self::chain_error(format!(
                "unsupported chain_id={} for Hinkal shield",
                payload.chain_id
            )));
        }
        if payload.amount_usdc_micro == 0 {
            return Err(Self::chain_error("shield amount must be > 0"));
        }

        let session = self.create_session(actor, true).await?;
        let message = Self::build_shield_message(
            &payload.nonce,
            payload.chain_id,
            &payload.token,
            payload.amount_usdc_micro,
        );
        let signature = signing::sign_message(message.as_bytes())?;

        let req = DepositRequest {
            address: self.treasury_pubkey.clone(),
            chain_id: payload.chain_id,
            nonce: payload.nonce.clone(),
            token_amounts: vec![TokenAmountRequest {
                token: payload.token.clone(),
                amount: payload.amount_usdc_micro.to_string(),
            }],
            message,
            signature: hex::encode(signature),
            session_nonce: session.nonce,
            session_signature: session.signature,
        };

        let url = format!("{}/deposit", self.api_base_url);
        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                let msg = format!("deposit request failed: {e}");
                self.emit_cns_chain_error_for_actor(actor, "submit_shield", &msg);
                Self::chain_error(msg)
            })?;

        if !resp.status().is_success() {
            let body = Self::read_response_body(resp).await;
            let msg = format!("deposit rejected: {body}");
            self.emit_cns_chain_error_for_actor(actor, "submit_shield", &msg);
            return Err(Self::chain_error(msg));
        }

        let v: serde_json::Value = resp.json().await.map_err(|e| {
            let msg = format!("deposit returned invalid JSON: {e}");
            self.emit_cns_chain_error_for_actor(actor, "submit_shield", &msg);
            Self::chain_error(msg)
        })?;

        let tx_hash = v
            .get("txHash")
            .and_then(|x| x.as_str())
            .or_else(|| v.get("tx_hash").and_then(|x| x.as_str()))
            .or_else(|| v.get("hash").and_then(|x| x.as_str()))
            .or_else(|| {
                v.get("data")
                    .and_then(|d| d.get("txHash"))
                    .and_then(|x| x.as_str())
            })
            .ok_or_else(|| {
                let msg = "deposit response missing tx hash".to_string();
                self.emit_cns_chain_error_for_actor(actor, "submit_shield", &msg);
                Self::chain_error(msg)
            })?;

        Ok(TxHash(tx_hash.to_string()))
    }
}

#[async_trait]
impl ChainPort for HinkalPort {
    fn chain_id(&self) -> ChainId {
        ChainId::Hinkal
    }

    fn derive_deposit_address(&self, _index: u64) -> Result<String, WalletError> {
        Ok(self.treasury_pubkey.clone())
    }

    async fn monitor_deposits(
        &self,
        _actor: &WebID,
        _addresses: &[String],
    ) -> Result<Vec<DepositEvent>, WalletError> {
        Err(Self::chain_error(
            "Hinkal transparent deposit monitoring is not supported; use monitor_shielded_transfers via PrivacyPort",
        ))
    }

    fn build_withdrawal_tx(
        &self,
        _to_address: &str,
        _amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError> {
        Err(Self::chain_error(
            "Hinkal chain adapter supports shielded withdrawal path only; use PrivacyMode::Shielded",
        ))
    }

    async fn submit_signed_tx(
        &self,
        _actor: &WebID,
        _signed_tx_bytes: &[u8],
    ) -> Result<TxHash, WalletError> {
        Err(Self::chain_error(
            "Hinkal chain submit path is unavailable; use PrivacyPort::submit_signed_tx for shielded flow",
        ))
    }

    async fn confirmations(&self, _actor: &WebID, _tx_hash: &TxHash) -> Result<u64, WalletError> {
        Err(Self::chain_error(
            "confirmation checking requires underlying chain RPC integration",
        ))
    }
}

#[async_trait]
impl PrivacyPort for HinkalPort {
    fn our_shielded_address(&self) -> Result<String, WalletError> {
        Ok(self.treasury_pubkey.clone())
    }

    fn shielded_deposit_address(&self, _wallet_id: WalletId) -> Result<String, WalletError> {
        Ok(self.treasury_pubkey.clone())
    }

    async fn monitor_shielded_transfers(
        &self,
        actor: &WebID,
    ) -> Result<Vec<ShieldedTransfer>, WalletError> {
        let balances = self.monitor_deposits(actor).await?;

        let mut transfers = Vec::new();
        let mut last = self
            .last_balances
            .lock()
            .map_err(|_| Self::chain_error("balance state lock poisoned"))?;

        for row in balances {
            let prev = last.get(&row.token).copied().unwrap_or(0);
            if row.amount_usdc_micro > prev {
                let commitment = self.transfer_commitment(
                    &row.token,
                    prev,
                    row.amount_usdc_micro,
                    row.commitment.as_deref(),
                );
                transfers.push(ShieldedTransfer {
                    commitment,
                    from_shielded: "hinkal-pool".to_string(),
                    to_shielded: self.treasury_pubkey.clone(),
                    amount_usdc_micro: row.amount_usdc_micro - prev,
                    chain: ChainId::Solana, // Hinkal currently settles on Solana
                    memo: row.memo,
                    block_time: Utc::now(),
                });
            }
            last.insert(row.token, row.amount_usdc_micro);
        }

        Ok(transfers)
    }

    fn build_shield_tx(
        &self,
        amount_usdc_micro: u64,
        chain: ChainId,
    ) -> Result<Vec<u8>, WalletError> {
        if amount_usdc_micro == 0 {
            return Err(Self::chain_error("shield amount must be > 0"));
        }
        // Only Solana settlement is currently supported
        if chain != ChainId::Hinkal && chain != ChainId::Solana {
            return Err(WalletError::ChainError {
                chain,
                message: format!(
                    "Hinkal shielding only supports Solana settlement layer (got {chain:?})"
                ),
            });
        }

        let payload = ShieldDepositPayload {
            nonce: Self::generate_nonce(),
            amount_usdc_micro,
            chain_id: HINKAL_SOLANA_CHAIN_ID,
            token: SOLANA_USDC_MINT.to_string(),
        };

        serde_json::to_vec(&payload)
            .map_err(|e| Self::chain_error(format!("failed to encode shield payload: {e}")))
    }

    fn build_unshield_tx(
        &self,
        to_public: &str,
        amount_usdc_micro: u64,
    ) -> Result<Vec<u8>, WalletError> {
        if to_public.trim().is_empty() {
            return Err(Self::chain_error("withdraw recipient must not be empty"));
        }
        if amount_usdc_micro == 0 {
            return Err(Self::chain_error("withdraw amount must be > 0"));
        }

        let payload = WithdrawUnsignedPayload {
            nonce: Self::generate_nonce(),
            to_public: to_public.to_string(),
            amount_usdc_micro,
            chain_id: HINKAL_SOLANA_CHAIN_ID,
            token: SOLANA_USDC_MINT.to_string(),
        };

        serde_json::to_vec(&payload)
            .map_err(|e| Self::chain_error(format!("failed to encode unshield payload: {e}")))
    }

    async fn submit_signed_tx(
        &self,
        actor: &WebID,
        signed_tx_bytes: &[u8],
    ) -> Result<TxHash, WalletError> {
        // Try raw payload first (from build_unshield_tx / build_shield_tx).
        if let Ok(payload) = serde_json::from_slice::<HinkalPayload>(signed_tx_bytes) {
            return match payload {
                HinkalPayload::Withdraw(p) => self.submit_withdraw(actor, &p).await,
                HinkalPayload::Shield(p) => self.submit_shield(actor, &p).await,
            };
        }

        // Fallback: legacy format with appended 64-byte Ed25519 signature.
        if signed_tx_bytes.len() <= 64 {
            let msg = "invalid payload: too short for legacy signature format".to_string();
            self.emit_cns_chain_error_for_actor(actor, "submit_signed_tx", &msg);
            return Err(Self::chain_error(msg));
        }
        let payload_bytes = &signed_tx_bytes[..signed_tx_bytes.len() - 64];
        let hinkal_payload: HinkalPayload = serde_json::from_slice(payload_bytes).map_err(|e| {
            let msg = format!("failed to decode Hinkal payload: {e}");
            self.emit_cns_chain_error_for_actor(actor, "submit_signed_tx", &msg);
            Self::chain_error(msg)
        })?;

        match hinkal_payload {
            HinkalPayload::Withdraw(payload) => self.submit_withdraw(actor, &payload).await,
            HinkalPayload::Shield(payload) => self.submit_shield(actor, &payload).await,
        }
    }

    fn available_for_chain(&self, chain: ChainId) -> bool {
        if chain != ChainId::Hinkal {
            return false;
        }
        !self.in_cooldown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Default)]
    struct CaptureSink {
        last_event: Mutex<Option<NuEvent>>,
    }

    impl NuEventSink for CaptureSink {
        fn persist(&self, event: &NuEvent) -> Result<(), hkask_types::InfrastructureError> {
            *self.last_event.lock().expect("lock") = Some(event.clone());
            Ok(())
        }
    }

    fn test_port(base: &str) -> HinkalPort {
        HinkalPort::new(base, "treasury_pubkey_test").expect("port")
    }

    // REQ: P9-wallet-hinkal-chain-error-actor-test — chain_error emission uses caller-provided actor identity
    #[tokio::test]
    async fn emit_chain_error_uses_provided_actor() {
        let actor = WebID::from_persona(b"actor-hinkal-test");
        let sink = Arc::new(CaptureSink::default());
        let port = HinkalPort::new("https://api.hinkal.io", "test_treasury_pubkey")
            .expect("port")
            .with_event_sink(sink.clone());

        let err = PrivacyPort::submit_signed_tx(&port, &actor, b"short")
            .await
            .expect_err("short payload should fail");
        assert!(matches!(err, WalletError::ChainError { .. }));

        let event = sink
            .last_event
            .lock()
            .expect("lock")
            .clone()
            .expect("event persisted");
        assert_eq!(event.observer_webid.to_string(), actor.to_string());
        assert_eq!(event.observation["operation"], "submit_signed_tx");
    }

    // REQ: P9-wallet-hinkal-session-read-format-test — session message format matches Hinkal API spec
    #[test]
    fn session_message_read_format() {
        let msg = HinkalPort::build_session_message("test-nonce-123", false);
        assert!(msg.contains("Authorize Hinkal session"));
        assert!(msg.contains("Session ID: test-nonce-123"));
        assert!(!msg.contains("submit transactions"));
    }

    // REQ: P9-wallet-hinkal-session-write-format-test — write session message includes transaction authorization
    #[test]
    fn session_message_write_format() {
        let msg = HinkalPort::build_session_message("test-nonce-456", true);
        assert!(msg.contains("Authorize Hinkal session"));
        assert!(msg.contains("Session ID: test-nonce-456"));
        assert!(msg.contains("This signature can also be used to submit transactions."));
    }

    // REQ: P9-wallet-hinkal-withdraw-message-format-test — Solana withdraw message format matches Hinkal API spec
    #[test]
    fn withdraw_message_format() {
        let msg = HinkalPort::build_withdraw_message(
            "nonce-789",
            501,
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            1_000_000,
            "recipient_solana_address",
        );
        assert!(msg.contains("Hinkal Enclave"));
        assert!(msg.contains("Primary Type: Withdraw"));
        assert!(msg.contains("Nonce: nonce-789"));
        assert!(msg.contains("Chain ID: 501"));
        assert!(msg.contains("Token: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        assert!(msg.contains("Amount: 1000000"));
        assert!(msg.contains("Recipient: recipient_solana_address"));
    }

    // REQ: P9-wallet-hinkal-circuit-breaker-healthy-test — circuit breaker initial state is healthy
    #[test]
    fn circuit_breaker_initial_state() {
        let port = HinkalPort::new("https://api.hinkal.io", "test_treasury_pubkey").unwrap();
        assert!(!port.in_cooldown());
        assert!(port.available_for_chain(ChainId::Hinkal));
    }

    // REQ: P9-wallet-hinkal-circuit-breaker-chain-test — circuit breaker denies non-Hinkal chains
    #[test]
    fn available_for_chain_rejects_non_hinkal() {
        let port = HinkalPort::new("https://api.hinkal.io", "test_treasury_pubkey").unwrap();
        assert!(!port.available_for_chain(ChainId::Solana));
        assert!(!port.available_for_chain(ChainId::Hedera));
    }

    // REQ: P9-wallet-hinkal-session-create-test — session bootstrap success path maps request/response correctly
    #[tokio::test]
    async fn create_session_success() {
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create-session"))
            .and(body_partial_json(serde_json::json!({
                "address": "treasury_pubkey_test",
                "chainId": 501,
                "writeAccess": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true
            })))
            .mount(&server)
            .await;

        let port = test_port(&server.uri());
        let actor = WebID::from_persona(b"hinkal-test");
        let session = port.create_session(&actor, false).await.expect("session");
        assert!(!session.nonce.is_empty());
        assert!(!session.signature.is_empty());
    }

    // REQ: P9-wallet-hinkal-session-cache-ttl-test — cached session is reused while unexpired
    #[test]
    fn cached_session_reused_within_ttl() {
        let port = HinkalPort::new("https://api.hinkal.io", "treasury_pubkey_test").unwrap();
        port.store_cached_session("nonce-1".to_string(), "sig-1".to_string(), false)
            .expect("cache");

        let session = port.cached_session(false).expect("cached").expect("some");
        assert_eq!(session.nonce, "nonce-1");
        assert_eq!(session.signature, "sig-1");
    }

    // REQ: P9-wallet-hinkal-session-cache-write-test — write-access lookup does not reuse read-only cached session
    #[test]
    fn cached_read_session_not_reused_for_write() {
        let port = HinkalPort::new("https://api.hinkal.io", "treasury_pubkey_test").unwrap();
        port.store_cached_session("nonce-1".to_string(), "sig-1".to_string(), false)
            .expect("cache");

        let read = port.cached_session(false).expect("read cached");
        let write = port.cached_session(true).expect("write cached");

        assert!(read.is_some());
        assert!(write.is_none());
    }

    // REQ: P9-wallet-hinkal-nonce-reuse-test — nonce reuse/server rejection is propagated fail-closed
    #[tokio::test]
    async fn create_session_nonce_reuse_propagates_error() {
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/create-session"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({ "error": "nonce already used" })),
            )
            .mount(&server)
            .await;

        let port = test_port(&server.uri());
        let actor = WebID::from_persona(b"hinkal-test");
        let err = port
            .create_session(&actor, false)
            .await
            .expect_err("must fail");
        match err {
            WalletError::ChainError { chain, message } => {
                assert_eq!(chain, ChainId::Hinkal);
                assert!(message.contains("create-session rejected"));
                assert!(message.contains("nonce="));
            }
            other => panic!("expected ChainError, got {other:?}"),
        }
    }

    // REQ: P9-wallet-hinkal-invalid-balance-test — invalid/partial balance payload fails closed
    #[tokio::test]
    async fn monitor_shielded_transfers_rejects_invalid_balance_payload() {
        unsafe {
            std::env::set_var(
                "HKASK_MASTER_KEY",
                "xXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxXxX",
            );
        }

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/create-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/balance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "balance": [
                    { "token": "USDC" }
                ]
            })))
            .mount(&server)
            .await;

        let port = test_port(&server.uri());
        let actor = WebID::from_persona(b"hinkal-monitor-test");
        let err = port
            .monitor_shielded_transfers(&actor)
            .await
            .expect_err("must fail closed");

        match err {
            WalletError::ChainError { chain, message } => {
                assert_eq!(chain, ChainId::Hinkal);
                assert!(message.contains("missing amount"));
            }
            other => panic!("expected ChainError, got {other:?}"),
        }
    }

    // REQ: P9-wallet-hinkal-unshield-payload-test — build_unshield_tx encodes deterministic request payload fields
    #[test]
    fn build_unshield_tx_encodes_payload() {
        let port = HinkalPort::new("https://api.hinkal.io", "treasury_pubkey_test").unwrap();
        let bytes = port
            .build_unshield_tx("recipient_pubkey", 1_500_000)
            .expect("payload");
        let payload: WithdrawUnsignedPayload = serde_json::from_slice(&bytes).expect("json");

        assert_eq!(payload.to_public, "recipient_pubkey");
        assert_eq!(payload.amount_usdc_micro, 1_500_000);
        assert_eq!(payload.chain_id, 501);
        assert_eq!(payload.token, SOLANA_USDC_MINT);
        assert_eq!(payload.nonce.len(), 32);
    }

    // REQ: P9-wallet-hinkal-shielded-withdraw-delta-test — monitor_shielded_transfers emits only positive balance deltas
    #[tokio::test]
    async fn monitor_shielded_transfers_emits_balance_deltas() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/balance"))
            .and(wiremock::matchers::query_param("nonce", "nonce-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "balance": [{ "token": "USDC", "amount": "1000000" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/balance"))
            .and(wiremock::matchers::query_param("nonce", "nonce-2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "balance": [{ "token": "USDC", "amount": "1500000" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let port = test_port(&server.uri());
        port.store_cached_session("nonce-1".to_string(), "sig-1".to_string(), false)
            .expect("cache-1");
        let actor = WebID::from_persona(b"hinkal-monitor-test");
        let first = port
            .monitor_shielded_transfers(&actor)
            .await
            .expect("first poll");

        port.store_cached_session("nonce-2".to_string(), "sig-2".to_string(), false)
            .expect("cache-2");
        let second = port
            .monitor_shielded_transfers(&actor)
            .await
            .expect("second poll");

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].amount_usdc_micro, 1_000_000);
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].amount_usdc_micro, 500_000);
    }

    // REQ: P9-wallet-hinkal-suppress-nonincreasing-test — monitor_shielded_transfers suppresses non-increasing balances
    #[tokio::test]
    async fn monitor_shielded_transfers_suppresses_non_increasing_balances() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/balance"))
            .and(wiremock::matchers::query_param("nonce", "nonce-a"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "balance": [{ "token": "USDC", "amount": "1000000" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/balance"))
            .and(wiremock::matchers::query_param("nonce", "nonce-b"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "balance": [{ "token": "USDC", "amount": "1000000" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let port = test_port(&server.uri());
        port.store_cached_session("nonce-a".to_string(), "sig-a".to_string(), false)
            .expect("cache-a");
        let actor = WebID::from_persona(b"hinkal-monitor-test");
        let first = port
            .monitor_shielded_transfers(&actor)
            .await
            .expect("first poll");

        port.store_cached_session("nonce-b".to_string(), "sig-b".to_string(), false)
            .expect("cache-b");
        let second = port
            .monitor_shielded_transfers(&actor)
            .await
            .expect("second poll");

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    // REQ: P9-wallet-hinkal-shield-message-format-test — shield message format matches Hinkal API spec
    #[test]
    fn shield_message_format() {
        let msg = HinkalPort::build_shield_message(
            "nonce-shield-001",
            501,
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            2_000_000,
        );
        assert!(msg.contains("Hinkal Enclave"));
        assert!(msg.contains("Primary Type: Shield"));
        assert!(msg.contains("Nonce: nonce-shield-001"));
        assert!(msg.contains("Chain ID: 501"));
        assert!(msg.contains("Token: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        assert!(msg.contains("Amount: 2000000"));
        // Shield message should NOT contain a Recipient field (unlike Withdraw)
        assert!(!msg.contains("Recipient"));
    }

    // REQ: P9-wallet-hinkal-shield-payload-test — build_shield_tx encodes deterministic payload fields
    #[test]
    fn build_shield_tx_encodes_payload() {
        let port = HinkalPort::new("https://api.hinkal.io", "treasury_pubkey_test").unwrap();
        let bytes = port
            .build_shield_tx(2_500_000, ChainId::Hinkal)
            .expect("payload");
        let payload: ShieldDepositPayload = serde_json::from_slice(&bytes).expect("json");

        assert_eq!(payload.amount_usdc_micro, 2_500_000);
        assert_eq!(payload.chain_id, 501);
        assert_eq!(payload.token, SOLANA_USDC_MINT);
        assert_eq!(payload.nonce.len(), 32);
    }

    // REQ: P9-wallet-hinkal-shield-zero-amount-test — build_shield_tx rejects zero amount
    #[test]
    fn build_shield_tx_rejects_zero_amount() {
        let port = HinkalPort::new("https://api.hinkal.io", "treasury_pubkey_test").unwrap();
        let err = port.build_shield_tx(0, ChainId::Hinkal).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("must be > 0"));
    }

    // REQ: P9-wallet-hinkal-shield-unsupported-chain-test — build_shield_tx rejects unsupported chain
    #[test]
    fn build_shield_tx_rejects_unsupported_chain() {
        let port = HinkalPort::new("https://api.hinkal.io", "treasury_pubkey_test").unwrap();
        let err = port.build_shield_tx(1_000, ChainId::Hedera).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("only supports Solana"));
    }

    // REQ: P9-wallet-hinkal-payload-deser-test — HinkalPayload untagged deserialization dispatches correctly
    #[test]
    fn hinkal_payload_deserialization_dispatches() {
        // Withdraw payload (has to_public field)
        let withdraw_json = serde_json::json!({
            "nonce": "abc123",
            "toPublic": "recipient_addr",
            "amountUsdcMicro": 1_000_000,
            "chainId": 501,
            "token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        });
        let payload: HinkalPayload =
            serde_json::from_value(withdraw_json).expect("withdraw deserialize");
        assert!(matches!(payload, HinkalPayload::Withdraw(_)));

        // Shield payload (no to_public field)
        let shield_json = serde_json::json!({
            "nonce": "def456",
            "amountUsdcMicro": 2_000_000,
            "chainId": 501,
            "token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        });
        let payload: HinkalPayload =
            serde_json::from_value(shield_json).expect("shield deserialize");
        assert!(matches!(payload, HinkalPayload::Shield(_)));
    }
}
