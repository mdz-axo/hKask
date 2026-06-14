//! Wallet API routes — balance, deposits, withdrawals, API keys.
//!
//! All endpoints require `ApiState` with an attached `WalletService`.
//! Routes return 503 Service Unavailable if the wallet service is not configured.

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use crate::ApiState;
use hkask_types::wallet::{ApiKeyId, ChainId, PrivacyMode, RJoule, WalletId};
use std::str::FromStr;
/// Create wallet router.
pub fn wallet_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(get_balance))
        .routes(routes!(get_deposit_address))
        .routes(routes!(create_deposit_reference))
        .routes(routes!(get_transactions))
        .routes(routes!(create_key, list_keys, revoke_key))
        .routes(routes!(withdraw))
        .routes(routes!(request_key, fund_key, delete_key))
}
// ── Response types ───────────────────────────────────────────────────────────
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WalletBalanceResponse {
    pub wallet_id: String,
    pub rjoules: u64,
    pub usdc_equivalent: f64,
    pub gas_equivalent: u64,
pub struct DepositAddressResponse {
    pub address: String,
    pub chain: String,
    pub privacy: String,
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DepositReferenceRequest {
pub struct DepositReferenceResponse {
    pub reference: String,
    pub expires_at: String,
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TransactionQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
pub struct TransactionResponse {
    pub rjoules_delta: i64,
    pub balance_after: u64,
    pub timestamp: String,
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionResponse>,
pub struct CreateKeyRequest {
    pub limit_rj: u64,
    pub expiry_days: Option<u32>,
    pub private: Option<bool>,
    pub chain: Option<String>,
    #[serde(default)]
    pub scope: Vec<String>,
    pub purpose: String,
    pub rate_limit: Option<RateLimitConfig>,
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub tokens_per_day: u64,
pub struct ApiKeyCreatedResponse {
    pub key_id: String,
    pub private_key_hex: String,
    pub spending_limit_rj: u64,
    pub expires_at: Option<String>,
    pub privacy_mode: String,
    pub preferred_chain: Option<String>,
pub struct ApiKeyEntry {
    pub spent_rj: u64,
    pub status: String,
pub struct ApiKeyListResponse {
    pub keys: Vec<ApiKeyEntry>,
pub struct ApiKeyRevokedResponse {
    pub revoked: bool,
pub struct WithdrawRequest {
    pub amount_rj: u64,
    pub to_address: String,
pub struct WithdrawalResponse {
    pub tx_hash: String,
// ── Helpers ──────────────────────────────────────────────────────────────────
fn wallet_err(status: StatusCode, msg: &str) -> Response<Body> {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
fn get_wallet(state: &ApiState) -> Result<&hkask_services::WalletService, StatusCode> {
    state
        .wallet_service
        .as_ref()
        .map(|arc| arc.as_ref())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
fn parse_chain(s: Option<&str>) -> ChainId {
    match s {
        Some("hedera") => ChainId::Hedera,
        Some("hinkal") => ChainId::Hinkal,
        _ => ChainId::Solana,
    }
// ── GET /api/wallet/balance ─────────────────────────────────────────────────
/// Get current wallet balance.
#[utoipa::path(
    get,
    path = "/api/wallet/balance",
    responses(
        (status = 200, body = WalletBalanceResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn get_balance(State(state): State<ApiState>) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();
    match svc.get_balance(wallet_id) {
        Ok(balance) => (
            StatusCode::OK,
            Json(WalletBalanceResponse {
                wallet_id: wallet_id.to_string(),
                rjoules: balance.rjoules,
                usdc_equivalent: balance.usdc_equivalent_micro as f64 / 1_000_000.0,
                gas_equivalent: balance.gas_equivalent,
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
// ── GET /api/wallet/deposit-address ──────────────────────────────────────────
pub struct DepositAddressQuery {
/// Get a deposit address for receiving USDC.
    path = "/api/wallet/deposit-address",
    params(DepositAddressQuery),
        (status = 200, body = DepositAddressResponse),
async fn get_deposit_address(
    State(state): State<ApiState>,
    Query(q): Query<DepositAddressQuery>,
) -> impl IntoResponse {
    let chain = parse_chain(q.chain.as_deref());
    let privacy = if q.private.unwrap_or(false) {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    match svc.get_deposit_address(wallet_id, chain, privacy) {
        Ok(addr) => (
            Json(DepositAddressResponse {
                address: addr.address,
                chain: chain.to_string(),
                privacy: privacy.to_string(),
// ── POST /api/wallet/deposit-reference ──────────────────────────────────────
/// Generate a one-time deposit reference for shielded deposits.
    post,
    path = "/api/wallet/deposit-reference",
    request_body = DepositReferenceRequest,
        (status = 200, body = DepositReferenceResponse),
async fn create_deposit_reference(
    Json(req): Json<DepositReferenceRequest>,
    let chain = parse_chain(Some(&req.chain));
    match svc.generate_deposit_reference(wallet_id, chain, 24) {
        Ok(dep_ref) => (
            Json(DepositReferenceResponse {
                reference: dep_ref.reference,
                expires_at: dep_ref.expires_at.to_rfc3339(),
// ── GET /api/wallet/transactions ────────────────────────────────────────────
/// Get paginated transaction history.
    path = "/api/wallet/transactions",
    params(TransactionQuery),
        (status = 200, body = TransactionListResponse),
async fn get_transactions(
    Query(q): Query<TransactionQuery>,
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);
    match svc.get_transactions(wallet_id, limit, offset) {
        Ok(txs) => (
            Json(TransactionListResponse {
                transactions: txs
                    .iter()
                    .map(|tx| TransactionResponse {
                        rjoules_delta: tx.rjoules_delta,
                        balance_after: tx.balance_after,
                        timestamp: tx.timestamp.to_rfc3339(),
                    })
                    .collect(),
// ── POST /api/wallet/keys ───────────────────────────────────────────────────
/// Create a new API key.
    path = "/api/wallet/keys",
    request_body = CreateKeyRequest,
        (status = 201, body = ApiKeyCreatedResponse),
async fn create_key(
    Json(req): Json<CreateKeyRequest>,
    let privacy = if req.private.unwrap_or(false) {
    let preferred_chain = req.chain.as_deref().map(|c| parse_chain(Some(c)));
    // Ensure wallet exists
    if let Err(e) = svc.ensure_wallet(wallet_id) {
        return wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response();
    match svc.create_key(
        wallet_id,
        RJoule::new(req.limit_rj),
        req.expiry_days,
        privacy,
        preferred_chain,
        req.scope,
        req.purpose,
        req.rate_limit
            .map(|rl| hkask_types::wallet::RateLimitConfig {
                requests_per_minute: rl.requests_per_minute,
                tokens_per_day: rl.tokens_per_day,
    ) {
        Ok(material) => (
            StatusCode::CREATED,
            Json(ApiKeyCreatedResponse {
                key_id: material.key_id.to_string(),
                private_key_hex: material.private_key_hex,
                spending_limit_rj: material.capability.spending_limit_rj.as_u64(),
                expires_at: material.capability.expiry.map(|e| e.to_rfc3339()),
                privacy_mode: material.capability.privacy_mode.to_string(),
                preferred_chain: material.capability.preferred_chain.map(|c| c.to_string()),
                scope: material.capability.scope,
                purpose: material.capability.purpose,
                rate_limit: material.capability.rate_limit.map(|rl| RateLimitConfig {
                    requests_per_minute: rl.requests_per_minute,
                    tokens_per_day: rl.tokens_per_day,
                }),
// ── GET /api/wallet/keys ────────────────────────────────────────────────────
/// List active API keys.
        (status = 200, body = ApiKeyListResponse),
async fn list_keys(State(state): State<ApiState>) -> impl IntoResponse {
    match svc.list_keys(wallet_id) {
        Ok(keys) => (
            Json(ApiKeyListResponse {
                keys: keys
                    .map(|key| {
                        let status = if key.spent_rj.as_u64() >= key.spending_limit_rj.as_u64() {
                            "exhausted"
                        } else if key.expiry.is_some_and(|exp| chrono::Utc::now() > exp) {
                            "expired"
                        } else {
                            "active"
                        };
                        ApiKeyEntry {
                            key_id: key.key_id.to_string(),
                            spent_rj: key.spent_rj.as_u64(),
                            limit_rj: key.spending_limit_rj.as_u64(),
                            status: status.to_string(),
                            privacy_mode: key.privacy_mode.to_string(),
                            expires_at: key.expiry.map(|e| e.to_rfc3339()),
                            preferred_chain: key.preferred_chain.map(|c| c.to_string()),
                        }
// ── DELETE /api/wallet/keys/{key_id} ────────────────────────────────────────
/// Revoke an API key.
    delete,
    path = "/api/wallet/keys/{key_id}",
        (status = 200, body = ApiKeyRevokedResponse),
async fn revoke_key(
    Path(key_id_str): Path<String>,
    let key_id = match key_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return wallet_err(StatusCode::BAD_REQUEST, "Invalid key ID format").into_response();
        }
    match svc.revoke_key(key_id) {
        Ok(()) => (
            Json(ApiKeyRevokedResponse {
                key_id: key_id_str,
                revoked: true,
// ── POST /api/wallet/withdraw ───────────────────────────────────────────────
/// Withdraw rJoules as USDC to an external address.
    path = "/api/wallet/withdraw",
    request_body = WithdrawRequest,
        (status = 200, body = WithdrawalResponse),
async fn withdraw(
    Json(req): Json<WithdrawRequest>,
    let chain = parse_chain(req.chain.as_deref());
    match svc
        .withdraw(
            wallet_id,
            RJoule::new(req.amount_rj),
            &req.to_address,
            chain,
            privacy,
        .await
    {
        Ok(tx_hash) => (
            Json(WithdrawalResponse {
                tx_hash: tx_hash.0,
                amount_rj: req.amount_rj,
// ── 7R7-governed API key issuance ───────────────────────────────────────────
/// Request body for 7R7-governed API key issuance.
pub struct KeyRequestRequest {
    /// Replicant name funding this key.
    pub replicant: String,
    /// Allowed endpoint scopes.
    /// Stated purpose (≥20 chars, gate 4).
    /// rJoule allocation to encumber from the replicant's wallet.
    pub allocation_rj: u64,
    /// Optional rate limit configuration.
/// Response for a successfully issued API key.
pub struct KeyRequestResponse {
    /// Shown once — the key secret (Ed25519 private key hex).
    pub key_secret: String,
/// Request body for funding (replenishing) an API key's encumbrance.
pub struct FundKeyRequest {
    /// Replicant name providing the funds.
    /// Additional rJoules to encumber.
/// Response after funding a key.
pub struct FundKeyResponse {
    pub new_allocation_rj: u64,
    pub remaining_rj: u64,
/// 6-gate approval for API key issuance (single function, per essentialist G2).
///
/// Gates:
/// 1. Replicant authenticated (UserStore session)
/// 2. Clean CNS history (no abuse flags, 90 days)
/// 3. Valid scope (endpoints exist in registry)
/// 4. Purpose stated (≥20 chars)
/// 5. Rate limit feasible (≤ scope maximum)
/// 6. Wallet balance ≥ allocation_rj
async fn approve_key_request(
    state: &ApiState,
    req: &KeyRequestRequest,
) -> Result<(WalletId, hkask_types::WebID), (StatusCode, String)> {
    // Gate 1: Replicant authenticated
    let user_store = state.agent_service.user_store();
    let identity = {
        let store = user_store.lock().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("UserStore lock failed: {e}"),
            )
        })?;
        store
            .get_replicant(&req.replicant)
            .map_err(|e| {
                (
                    StatusCode::UNAUTHORIZED,
                    format!("Replicant lookup failed: {e}"),
                )
            })?
            .ok_or_else(|| {
                    format!("Replicant '{}' not registered", req.replicant),
    let webid = identity.replicant_webid;
    // Gate 2: Clean CNS history (no abuse flags in last 90 days)
    // Stub: CNS alert query not yet exposed via service layer.
    // Full implementation deferred to Gap 2 (CNS API spans).
    // Gate 3: Valid scope — endpoints exist in registry
    // Stub: registry endpoint validation deferred.
    // In production, this checks each scope entry against registered MCP tools.
    // Gate 4: Purpose stated (≥20 chars)
    if req.purpose.chars().count() < 20 {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Purpose must be at least 20 characters (got {})",
                req.purpose.chars().count()
            ),
        ));
    // Gate 5: Rate limit feasible
    if let Some(ref rl) = req.rate_limit {
        if rl.requests_per_minute == 0 || rl.tokens_per_day == 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Rate limit values must be positive".to_string(),
            ));
        // Max defaults: 60 req/min, 1M tokens/day
        if rl.requests_per_minute > 60 {
                format!(
                    "requests_per_minute {} exceeds maximum 60",
                    rl.requests_per_minute
                ),
        if rl.tokens_per_day > 1_000_000 {
                    "tokens_per_day {} exceeds maximum 1,000,000",
                    rl.tokens_per_day
    // Gate 6: Wallet balance ≥ allocation_rj
    let wallet_id = WalletId::default(); // replicant wallet = default wallet for now
    let svc = state.wallet_service.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Wallet service not configured".to_string(),
    })?;
    let allocation = RJoule::new(req.allocation_rj);
    if !svc
        .can_afford(wallet_id, allocation)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        let balance = svc
            .get_balance(wallet_id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            StatusCode::PAYMENT_REQUIRED,
                "Insufficient wallet balance: have {} rJ, need {} rJ",
                balance.rjoules, req.allocation_rj
    Ok((wallet_id, webid))
/// POST /api/keys/request
/// 7R7-governed API key issuance with 6-gate approval.
/// Returns the key secret exactly once.
    path = "/api/keys/request",
    request_body = KeyRequestRequest,
        (status = 201, description = "Key issued", body = KeyRequestResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Replicant not authenticated"),
        (status = 402, description = "Insufficient wallet balance"),
async fn request_key(
    Json(req): Json<KeyRequestRequest>,
    // 6-gate approval
    let (wallet_id, _webid) = match approve_key_request(&state, &req).await {
        Ok(v) => v,
        Err((status, msg)) => return wallet_err(status, &msg).into_response(),
        Err(status) => return wallet_err(status, "Wallet service not configured").into_response(),
    let rate_limit = req
        .rate_limit
        .map(|rl| hkask_types::wallet::RateLimitConfig {
            requests_per_minute: rl.requests_per_minute,
            tokens_per_day: rl.tokens_per_day,
        });
    // Create the key
    let material = match svc.create_key(
        allocation,
        Some(90), // 90-day expiry per architecture doc §5.4
        PrivacyMode::Transparent,
        None,
        req.scope.clone(),
        req.purpose.clone(),
        rate_limit,
        Ok(m) => m,
        Err(e) => {
            return wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response();
    // Encumber the allocation from the wallet
    if let Err(e) = svc.encumber_key(wallet_id, material.key_id, allocation) {
        // Rollback: revoke the key if encumbrance fails
        let _ = svc.revoke_key(material.key_id);
    (
        StatusCode::CREATED,
        Json(KeyRequestResponse {
            key_id: material.key_id.to_string(),
            key_secret: material.private_key_hex,
            scope: material.capability.scope,
            allocation_rj: material.capability.spending_limit_rj.as_u64(),
            expires_at: material.capability.expiry.map(|e| e.to_rfc3339()),
            rate_limit: material.capability.rate_limit.map(|rl| RateLimitConfig {
        }),
        .into_response()
/// POST /api/keys/{key_id}/fund
/// Replenish an API key's encumbrance with additional rJoules from the
/// funding replicant's wallet.
    path = "/api/keys/{key_id}/fund",
    params(
        ("key_id" = String, Path, description = "API key ID to fund")
    ),
    request_body = FundKeyRequest,
        (status = 200, description = "Key funded", body = FundKeyResponse),
        (status = 404, description = "Key not found"),
async fn fund_key(
    Json(req): Json<FundKeyRequest>,
    let key_id = match ApiKeyId::from_str(&key_id_str) {
        Err(e) => return wallet_err(StatusCode::BAD_REQUEST, &e.to_string()).into_response(),
    let amount = RJoule::new(req.amount_rj);
    // Check wallet balance
    if !svc.can_afford(wallet_id, amount).unwrap_or(false) {
        return wallet_err(
            &format!("Insufficient wallet balance for {} rJ", req.amount_rj),
        .into_response();
    // Encumber additional rJoules
    if let Err(e) = svc.encumber_key(wallet_id, key_id, amount) {
    // Read updated encumbrance
    let enc = match svc.get_encumbrance(key_id) {
        Ok(Some(e)) => e,
        Ok(None) => return wallet_err(StatusCode::NOT_FOUND, "Key not found").into_response(),
        StatusCode::OK,
        Json(FundKeyResponse {
            key_id: key_id_str,
            new_allocation_rj: enc.amount_rj,
            remaining_rj: enc.remaining_rj(),
/// DELETE /api/keys/{key_id}
/// Revoke an API key and release its encumbrance.
    path = "/api/keys/{key_id}",
        ("key_id" = String, Path, description = "API key ID to revoke")
        (status = 200, description = "Key revoked"),
async fn delete_key(
    // Release encumbrance first (returns unspent rJ to wallet)
    if let Err(e) = svc.release_encumbrance(key_id) {
    // Revoke the key
    if let Err(e) = svc.revoke_key(key_id) {
        Json(serde_json::json!({
            "key_id": key_id_str,
            "revoked": true
        })),
