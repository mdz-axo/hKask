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
use hkask_types::wallet::{ChainId, PrivacyMode, RJoule, WalletId};

/// Create wallet router.
pub fn wallet_router() -> OpenApiRouter<ApiState> {
    OpenApiRouter::new()
        .routes(routes!(get_balance))
        .routes(routes!(get_deposit_address))
        .routes(routes!(create_deposit_reference))
        .routes(routes!(get_transactions))
        .routes(routes!(create_key, list_keys, revoke_key))
        .routes(routes!(withdraw))
}

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WalletBalanceResponse {
    pub wallet_id: String,
    pub rjoules: u64,
    pub usdc_equivalent: f64,
    pub gas_equivalent: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DepositAddressResponse {
    pub address: String,
    pub chain: String,
    pub privacy: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DepositReferenceRequest {
    pub chain: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DepositReferenceResponse {
    pub reference: String,
    pub chain: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TransactionQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TransactionResponse {
    pub rjoules_delta: i64,
    pub balance_after: u64,
    pub timestamp: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionResponse>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateKeyRequest {
    pub limit_rj: u64,
    pub expiry_days: Option<u32>,
    pub private: Option<bool>,
    pub chain: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiKeyCreatedResponse {
    pub key_id: String,
    pub private_key_hex: String,
    pub spending_limit_rj: u64,
    pub expires_at: Option<String>,
    pub privacy_mode: String,
    pub preferred_chain: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiKeyEntry {
    pub key_id: String,
    pub spent_rj: u64,
    pub limit_rj: u64,
    pub status: String,
    pub privacy_mode: String,
    pub expires_at: Option<String>,
    pub preferred_chain: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiKeyListResponse {
    pub keys: Vec<ApiKeyEntry>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiKeyRevokedResponse {
    pub key_id: String,
    pub revoked: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct WithdrawRequest {
    pub amount_rj: u64,
    pub to_address: String,
    pub chain: Option<String>,
    pub private: Option<bool>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WithdrawalResponse {
    pub tx_hash: String,
    pub amount_rj: u64,
    pub chain: String,
    pub privacy: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn wallet_err(status: StatusCode, msg: &str) -> Response<Body> {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

fn get_wallet(state: &ApiState) -> Result<&hkask_services::WalletService, StatusCode> {
    state
        .wallet_service
        .as_ref()
        .map(|arc| arc.as_ref())
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

fn parse_chain(s: Option<&str>) -> ChainId {
    match s {
        Some("hedera") => ChainId::Hedera,
        _ => ChainId::Solana,
    }
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
    }
}

// ── GET /api/wallet/deposit-address ──────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DepositAddressQuery {
    pub chain: Option<String>,
    pub private: Option<bool>,
}

/// Get a deposit address for receiving USDC.
#[utoipa::path(
    get,
    path = "/api/wallet/deposit-address",
    params(DepositAddressQuery),
    responses(
        (status = 200, body = DepositAddressResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn get_deposit_address(
    State(state): State<ApiState>,
    Query(q): Query<DepositAddressQuery>,
) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();
    let chain = parse_chain(q.chain.as_deref());
    let privacy = if q.private.unwrap_or(false) {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    };

    match svc.get_deposit_address(wallet_id, chain, privacy) {
        Ok(addr) => (
            StatusCode::OK,
            Json(DepositAddressResponse {
                address: addr.address,
                chain: chain.to_string(),
                privacy: privacy.to_string(),
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// ── POST /api/wallet/deposit-reference ──────────────────────────────────────

/// Generate a one-time deposit reference for shielded deposits.
#[utoipa::path(
    post,
    path = "/api/wallet/deposit-reference",
    request_body = DepositReferenceRequest,
    responses(
        (status = 200, body = DepositReferenceResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn create_deposit_reference(
    State(state): State<ApiState>,
    Json(req): Json<DepositReferenceRequest>,
) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();
    let chain = parse_chain(Some(&req.chain));

    match svc.generate_deposit_reference(wallet_id, chain, 24) {
        Ok(dep_ref) => (
            StatusCode::OK,
            Json(DepositReferenceResponse {
                reference: dep_ref.reference,
                chain: chain.to_string(),
                expires_at: dep_ref.expires_at.to_rfc3339(),
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// ── GET /api/wallet/transactions ────────────────────────────────────────────

/// Get paginated transaction history.
#[utoipa::path(
    get,
    path = "/api/wallet/transactions",
    params(TransactionQuery),
    responses(
        (status = 200, body = TransactionListResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn get_transactions(
    State(state): State<ApiState>,
    Query(q): Query<TransactionQuery>,
) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();
    let limit = q.limit.unwrap_or(50);
    let offset = q.offset.unwrap_or(0);

    match svc.get_transactions(wallet_id, limit, offset) {
        Ok(txs) => (
            StatusCode::OK,
            Json(TransactionListResponse {
                transactions: txs
                    .iter()
                    .map(|tx| TransactionResponse {
                        rjoules_delta: tx.rjoules_delta,
                        balance_after: tx.balance_after,
                        timestamp: tx.timestamp.to_rfc3339(),
                    })
                    .collect(),
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// ── POST /api/wallet/keys ───────────────────────────────────────────────────

/// Create a new API key.
#[utoipa::path(
    post,
    path = "/api/wallet/keys",
    request_body = CreateKeyRequest,
    responses(
        (status = 201, body = ApiKeyCreatedResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn create_key(
    State(state): State<ApiState>,
    Json(req): Json<CreateKeyRequest>,
) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();
    let privacy = if req.private.unwrap_or(false) {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    };
    let preferred_chain = req.chain.as_deref().map(|c| parse_chain(Some(c)));

    // Ensure wallet exists
    if let Err(e) = svc.ensure_wallet(wallet_id) {
        return wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response();
    }

    match svc.create_key(
        wallet_id,
        RJoule::new(req.limit_rj),
        req.expiry_days,
        privacy,
        preferred_chain,
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
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// ── GET /api/wallet/keys ────────────────────────────────────────────────────

/// List active API keys.
#[utoipa::path(
    get,
    path = "/api/wallet/keys",
    responses(
        (status = 200, body = ApiKeyListResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn list_keys(State(state): State<ApiState>) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();

    match svc.list_keys(wallet_id) {
        Ok(keys) => (
            StatusCode::OK,
            Json(ApiKeyListResponse {
                keys: keys
                    .iter()
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
                    })
                    .collect(),
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// ── DELETE /api/wallet/keys/{key_id} ────────────────────────────────────────

/// Revoke an API key.
#[utoipa::path(
    delete,
    path = "/api/wallet/keys/{key_id}",
    responses(
        (status = 200, body = ApiKeyRevokedResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn revoke_key(
    State(state): State<ApiState>,
    Path(key_id_str): Path<String>,
) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };

    let key_id = match key_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            return wallet_err(StatusCode::BAD_REQUEST, "Invalid key ID format").into_response();
        }
    };

    match svc.revoke_key(key_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiKeyRevokedResponse {
                key_id: key_id_str,
                revoked: true,
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}

// ── POST /api/wallet/withdraw ───────────────────────────────────────────────

/// Withdraw rJoules as USDC to an external address.
#[utoipa::path(
    post,
    path = "/api/wallet/withdraw",
    request_body = WithdrawRequest,
    responses(
        (status = 200, body = WithdrawalResponse),
        (status = 503, description = "Wallet service not configured")
    )
)]
async fn withdraw(
    State(state): State<ApiState>,
    Json(req): Json<WithdrawRequest>,
) -> impl IntoResponse {
    let svc = match get_wallet(&state) {
        Ok(s) => s,
        Err(status) => return wallet_err(status, "Wallet service not configured"),
    };
    let wallet_id = WalletId::default();
    let chain = parse_chain(req.chain.as_deref());
    let privacy = if req.private.unwrap_or(false) {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    };

    match svc
        .withdraw(
            wallet_id,
            RJoule::new(req.amount_rj),
            &req.to_address,
            chain,
            privacy,
        )
        .await
    {
        Ok(tx_hash) => (
            StatusCode::OK,
            Json(WithdrawalResponse {
                tx_hash: tx_hash.0,
                amount_rj: req.amount_rj,
                chain: chain.to_string(),
                privacy: privacy.to_string(),
            }),
        )
            .into_response(),
        Err(e) => wallet_err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()).into_response(),
    }
}
