//! Wallet command handlers for `kask wallet`
//!
//! Implements CLI display logic for wallet operations: balance, deposits,
//! withdrawals, API key management, and transaction history.

use crate::cli::{KeyAction, WalletAction};
use hkask_services::WalletService;
use hkask_storage::WalletStore;
use hkask_storage::database::in_memory_db;
use hkask_types::wallet::{ChainId, PrivacyMode, RJoule, WalletConfig, WalletId};
use hkask_wallet::{ApiKeyIssuer, WalletManager};
use std::sync::Arc;

/// Run a wallet subcommand. Builds a standalone WalletService for CLI use.
pub fn run(action: WalletAction) {
    let svc = build_wallet_service();
    match action {
        WalletAction::Balance => handle_balance(&svc),
        WalletAction::DepositAddress { chain, private } => {
            handle_deposit_address(&svc, chain, private)
        }
        WalletAction::DepositReference { chain } => handle_deposit_reference(&svc, chain),
        WalletAction::History { limit } => handle_history(&svc, limit),
        WalletAction::Key { action } => handle_key(&svc, action),
        WalletAction::Withdraw {
            amount_rj,
            to,
            chain,
            private,
        } => handle_withdraw(&svc, amount_rj, to, chain, private),
    }
}

fn build_wallet_service() -> WalletService {
    let db = in_memory_db();
    let store = Arc::new(WalletStore::new(db.conn_arc()));
    let config = WalletConfig::default();
    let manager = Arc::new(
        WalletManager::build(config, Arc::clone(&store), Default::default(), None)
            .expect("Failed to build WalletManager"),
    );
    let issuer =
        Arc::new(ApiKeyIssuer::new(Arc::clone(&store)).expect("Failed to build ApiKeyIssuer"));
    WalletService::new(manager, issuer)
}

// ── Balance ──────────────────────────────────────────────────────────────────

fn handle_balance(svc: &WalletService) {
    let wallet_id = WalletId::default();
    match svc.get_balance(wallet_id) {
        Ok(balance) => {
            println!("Wallet Balance");
            println!("==============");
            println!();
            println!("  rJoules:  {}", balance.rjoules);
            println!(
                "  USDC:     ~{:.4}",
                balance.usdc_equivalent_micro as f64 / 1_000_000.0
            );
            println!("  Gas:      {}", balance.gas_equivalent);
        }
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

// ── Deposit address ─────────────────────────────────────────────────────────

fn handle_deposit_address(svc: &WalletService, chain: Option<String>, private: bool) {
    let wallet_id = WalletId::default();
    let chain = parse_chain(chain.as_deref());
    let privacy = if private {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    };

    match svc.get_deposit_address(wallet_id, chain, privacy) {
        Ok(addr) => {
            println!("Deposit Address");
            println!("===============");
            println!();
            println!("  Chain:    {chain}");
            println!("  Privacy:  {privacy}");
            println!("  Address:  {}", addr.address);
            if private {
                println!();
                println!("  For shielded deposits, generate a one-time reference:");
                println!("    kask wallet deposit-reference --chain {chain}");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

// ── Deposit reference ────────────────────────────────────────────────────────

fn handle_deposit_reference(svc: &WalletService, chain: String) {
    let wallet_id = WalletId::default();
    let chain = parse_chain(Some(&chain));

    match svc.generate_deposit_reference(wallet_id, chain, 24) {
        Ok(dep_ref) => {
            println!("Deposit Reference");
            println!("=================");
            println!();
            println!("  Reference:  {}", dep_ref.reference);
            println!("  Chain:      {chain}");
            println!(
                "  Expires:    {}",
                dep_ref.expires_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!();
            println!("  Include this reference in the memo field of your shielded deposit.");
        }
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

// ── History ──────────────────────────────────────────────────────────────────

fn handle_history(svc: &WalletService, limit: Option<u32>) {
    let wallet_id = WalletId::default();
    let limit = limit.unwrap_or(20);

    match svc.get_transactions(wallet_id, limit, 0) {
        Ok(txs) => {
            println!("Transaction History (last {})", txs.len());
            println!("==============================");
            println!();
            if txs.is_empty() {
                println!("  (no transactions)");
            } else {
                for tx in &txs {
                    let direction = if tx.rjoules_delta >= 0 { "+" } else { "" };
                    println!(
                        "  {} {} rJ  → balance: {} rJ",
                        direction, tx.rjoules_delta, tx.balance_after
                    );
                    println!("    {}", tx.timestamp.format("%Y-%m-%d %H:%M:%S"));
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

// ── API Keys ─────────────────────────────────────────────────────────────────

fn handle_key(svc: &WalletService, action: KeyAction) {
    match action {
        KeyAction::Create {
            limit,
            expiry,
            private,
            chain,
        } => handle_key_create(svc, limit, expiry, private, chain),
        KeyAction::List => handle_key_list(svc),
        KeyAction::Revoke { key_id } => handle_key_revoke(svc, key_id),
    }
}

fn handle_key_create(
    svc: &WalletService,
    limit: u64,
    expiry: Option<u32>,
    private: bool,
    chain: Option<String>,
) {
    let wallet_id = WalletId::default();
    let privacy = if private {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    };
    let preferred_chain = chain.as_deref().map(|c| parse_chain(Some(c)));

    // Ensure wallet exists
    if let Err(e) = svc.ensure_wallet(wallet_id) {
        eprintln!("Error ensuring wallet: {e}");
        return;
    }

    match svc.create_key(
        wallet_id,
        RJoule::new(limit),
        expiry,
        privacy,
        preferred_chain,
        vec![],
        String::new(),
        None,
    ) {
        Ok(material) => {
            println!("API Key Created");
            println!("==============");
            println!();
            println!("  Key ID:       {}", material.key_id);
            println!("  Private Key:  {}", material.private_key_hex);
            println!();
            println!("  ⚠  Store this private key securely. It will not be shown again.");
            println!();
            println!(
                "  Spending Limit:  {} rJ",
                material.capability.spending_limit_rj
            );
            if let Some(exp) = material.capability.expiry {
                println!("  Expires:         {}", exp.format("%Y-%m-%d"));
            }
            println!("  Privacy Mode:    {}", material.capability.privacy_mode);
            if let Some(c) = material.capability.preferred_chain {
                println!("  Preferred Chain:  {c}");
            }
        }
        Err(e) => {
            eprintln!("Error creating key: {e}");
        }
    }
}

fn handle_key_list(svc: &WalletService) {
    let wallet_id = WalletId::default();
    match svc.list_keys(wallet_id) {
        Ok(keys) => {
            println!("API Keys");
            println!("========");
            println!();
            if keys.is_empty() {
                println!("  (no active keys)");
            } else {
                for key in &keys {
                    let status = if key.spent_rj.as_u64() >= key.spending_limit_rj.as_u64() {
                        "EXHAUSTED"
                    } else if key.expiry.is_some_and(|exp| chrono::Utc::now() > exp) {
                        "EXPIRED"
                    } else {
                        "active"
                    };
                    println!("  • {}  [{}]", key.key_id, status);
                    println!(
                        "    Spent: {}/{} rJ  |  Privacy: {}",
                        key.spent_rj, key.spending_limit_rj, key.privacy_mode
                    );
                    if let Some(exp) = key.expiry {
                        println!("    Expires: {}", exp.format("%Y-%m-%d"));
                    }
                    if let Some(c) = key.preferred_chain {
                        println!("    Chain: {c}");
                    }
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing keys: {e}");
        }
    }
}

fn handle_key_revoke(svc: &WalletService, key_id_str: String) {
    use std::str::FromStr;
    let key_id = match hkask_types::wallet::ApiKeyId::from_str(&key_id_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid key ID: {e}");
            return;
        }
    };

    match svc.revoke_key(key_id) {
        Ok(()) => {
            println!("Key revoked: {key_id_str}");
            println!("Unspent rJoules returned to wallet.");
        }
        Err(e) => {
            eprintln!("Error revoking key: {e}");
        }
    }
}

// ── Withdrawal ───────────────────────────────────────────────────────────────

fn handle_withdraw(
    _svc: &WalletService,
    amount_rj: u64,
    to: String,
    chain: Option<String>,
    private: bool,
) {
    let chain = parse_chain(chain.as_deref());
    let privacy = if private {
        PrivacyMode::Shielded
    } else {
        PrivacyMode::Transparent
    };

    // Withdrawal is async (requires chain port)
    // For now, show a message since chain ports are not yet implemented.
    println!("Withdrawal Request");
    println!("=================");
    println!();
    println!("  Amount:   {amount_rj} rJ");
    println!("  To:       {to}");
    println!("  Chain:    {chain}");
    println!("  Privacy:  {privacy}");
    println!();
    println!("  (Withdrawal execution requires chain port implementations —");
    println!("   solana.rs and hedera.rs are deferred to SDK integration.)");
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_chain(s: Option<&str>) -> ChainId {
    match s {
        Some("hedera") => ChainId::Hedera,
        Some("hinkal") => ChainId::Hinkal,
        _ => ChainId::Solana, // default
    }
}
