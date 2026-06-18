//! Wallet command handlers for `kask wallet`
//!
//! Implements CLI display logic for wallet operations: balance, deposits,
//! withdrawals, API key management, and transaction history.

use crate::cli::{KeyAction, WalletAction};
use hkask_services::WalletService;
use hkask_storage::WalletStore;
use hkask_storage::database::in_memory_db;
use hkask_types::wallet::{ChainId, PrivacyMode, RJoule, WalletConfig, WalletId};
use hkask_wallet::{ApiKeyIssuer, StaticPriceFeed, WalletManager};
use std::str::FromStr;
use std::sync::Arc;

/// Run a wallet subcommand. Builds a standalone WalletService for CLI use.
/// REQ: CLI-081
/// REQ: P9-CNS-SURF-003 pre: valid WalletAction post: cns.cli span emitted
/// pre:  action is a valid WalletAction variant
/// post: dispatches to balance, deposit, history, key, fee, withdraw, encumber, release, or report operations
pub fn run(action: WalletAction) {
    // P9: CNS span
    tracing::info!(target: "cns.cli", operation = "wallet", action = ?action, "CNS");
    let svc = build_wallet_service();
    match action {
        WalletAction::Balance { wallet } => handle_balance(&svc, wallet),
        WalletAction::DepositAddress {
            chain,
            private,
            transparent,
            wallet,
        } => handle_deposit_address(&svc, chain, private, transparent, wallet),
        WalletAction::DepositReference { chain, wallet } => {
            handle_deposit_reference(&svc, chain, wallet)
        }
        WalletAction::History { limit, wallet } => handle_history(&svc, limit, wallet),
        WalletAction::Key { action } => handle_key(&svc, action),
        WalletAction::Fee { chain } => handle_fee(&svc, chain),
        WalletAction::Withdraw {
            amount_rj,
            to,
            chain,
            private,
            transparent,
            wallet,
        } => handle_withdraw(&svc, amount_rj, to, chain, private, transparent, wallet),
        WalletAction::Encumber {
            key_id,
            amount,
            wallet,
        } => handle_encumber(&svc, key_id, amount, wallet),
        WalletAction::ReleaseEncumbrance { key_id } => handle_release_encumbrance(&svc, key_id),
        WalletAction::Report { key_id, wallet } => handle_report(&svc, key_id, wallet),
    }
}

fn build_wallet_service() -> WalletService {
    let db = in_memory_db();
    let store = Arc::new(WalletStore::new(db.conn_arc()));
    let config = WalletConfig::default();
    let manager = Arc::new(
        WalletManager::build(
            config,
            Arc::clone(&store),
            Default::default(),
            None,
            Arc::new(StaticPriceFeed::new()),
        )
        .expect("Failed to build WalletManager"),
    );
    let issuer =
        Arc::new(ApiKeyIssuer::new(Arc::clone(&store)).expect("Failed to build ApiKeyIssuer"));
    WalletService::new(manager, issuer)
}

// ── Balance ──────────────────────────────────────────────────────────────────

fn handle_balance(svc: &WalletService, wallet: Option<String>) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
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

fn handle_deposit_address(
    svc: &WalletService,
    chain: Option<String>,
    private: bool,
    transparent: bool,
    wallet: Option<String>,
) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let chain = match parse_chain(chain.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let privacy = resolve_privacy_mode(private, transparent);

    match svc.get_deposit_address(wallet_id, chain, privacy) {
        Ok(addr) => {
            println!("Deposit Address");
            println!("===============");
            println!();
            println!("  Chain:    {chain}");
            println!("  Privacy:  {privacy}");
            println!("  Address:  {}", addr.address);
            if privacy == PrivacyMode::Shielded {
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

fn handle_deposit_reference(svc: &WalletService, chain: String, wallet: Option<String>) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let chain = match parse_chain(Some(&chain)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

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

fn handle_history(svc: &WalletService, limit: Option<u32>, wallet: Option<String>) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
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

// ── Fee quote ───────────────────────────────────────────────────────────────

fn handle_fee(svc: &WalletService, chain: Option<String>) {
    let chain = match parse_chain(chain.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            let webid = hkask_types::WebID::from_persona(b"cli-user");
            match rt.block_on(svc.estimate_withdrawal_fee(&webid, chain)) {
                Ok(fee) => {
                    println!("Withdrawal Fee Estimate");
                    println!("=======================");
                    println!();
                    println!("  Chain:        {chain}");
                    println!("  rJoules:      {}", fee.rjoules);
                    println!("  Native units: {:.8}", fee.native_units);
                    println!(
                        "  USDC:         ~{:.6}",
                        fee.usdc_micro as f64 / 1_000_000.0
                    );
                }
                Err(e) => eprintln!("Error estimating fee: {e}"),
            }
        }
        Err(e) => eprintln!("Error initializing runtime: {e}"),
    }
}

// ── API Keys ─────────────────────────────────────────────────────────────────

fn handle_key(svc: &WalletService, action: KeyAction) {
    match action {
        KeyAction::Create {
            limit,
            expiry,
            private,
            transparent,
            chain,
            wallet,
        } => handle_key_create(svc, limit, expiry, private, transparent, chain, wallet),
        KeyAction::List { wallet } => handle_key_list(svc, wallet),
        KeyAction::Revoke { key_id } => handle_key_revoke(svc, key_id),
    }
}

fn handle_key_create(
    svc: &WalletService,
    limit: u64,
    expiry: Option<u32>,
    private: bool,
    transparent: bool,
    chain: Option<String>,
    wallet: Option<String>,
) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let privacy = resolve_privacy_mode(private, transparent);
    let preferred_chain = match chain.as_deref() {
        Some(c) => match parse_chain(Some(c)) {
            Ok(chain_id) => Some(chain_id),
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        },
        None => Some(ChainId::Hinkal),
    };

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

fn handle_key_list(svc: &WalletService, wallet: Option<String>) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
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
    svc: &WalletService,
    amount_rj: u64,
    to: String,
    chain: Option<String>,
    private: bool,
    transparent: bool,
    wallet: Option<String>,
) {
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let chain = match parse_chain(chain.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let privacy = resolve_privacy_mode(private, transparent);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    // CLI user is the operator — use deterministic operator WebID for consent check
    let webid = hkask_types::WebID::from_persona(b"cli-user");
    match rt.block_on(svc.withdraw(
        &webid,
        wallet_id,
        RJoule::new(amount_rj),
        &to,
        chain,
        privacy,
    )) {
        Ok(tx_hash) => {
            println!("Withdrawal Submitted");
            println!("====================");
            println!();
            println!("  Amount:   {amount_rj} rJ");
            println!("  To:       {to}");
            println!("  Chain:    {chain}");
            println!("  Privacy:  {privacy}");
            println!("  Tx Hash:  {}", tx_hash.0);
            println!();
            println!("  (Set SOLANA_RPC_URL + SOLANA_TREASURY_PUBKEY or");
            println!("   HEDERA_TREASURY_ACCOUNT to enable chain execution.)");
        }
        Err(e) => {
            eprintln!("Withdrawal failed: {e}");
            eprintln!();
            eprintln!("  (Ensure SOLANA_RPC_URL + SOLANA_TREASURY_PUBKEY or");
            eprintln!("   HEDERA_TREASURY_ACCOUNT are set for chain execution.)");
        }
    }
}

// ── Encumber ─────────────────────────────────────────────────────────────────

fn handle_encumber(svc: &WalletService, key_id_str: String, amount: u64, wallet: Option<String>) {
    use std::str::FromStr;
    let key_id = match hkask_types::wallet::ApiKeyId::from_str(&key_id_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid key ID: {e}");
            return;
        }
    };
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    match svc.encumber_key(wallet_id, key_id, RJoule::new(amount)) {
        Ok(()) => {
            println!("Encumbered {} rJ to key {}", amount, key_id_str);
            println!("The key can now make API calls up to this allocation.");
        }
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

fn handle_release_encumbrance(svc: &WalletService, key_id_str: String) {
    use std::str::FromStr;
    let key_id = match hkask_types::wallet::ApiKeyId::from_str(&key_id_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid key ID: {e}");
            return;
        }
    };

    match svc.release_encumbrance(key_id) {
        Ok(()) => {
            println!("Encumbrance released for key {}", key_id_str);
            println!("Unspent rJoules returned to wallet.");
        }
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

// ── Report ────────────────────────────────────────────────────────────────────

fn handle_report(svc: &WalletService, key_id_str: String, wallet: Option<String>) {
    use std::str::FromStr;
    let key_id = match hkask_types::wallet::ApiKeyId::from_str(&key_id_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Invalid key ID: {e}");
            return;
        }
    };
    let wallet_id = match resolve_wallet(wallet) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    // Get the API key capability for context
    let key_info = match svc.get_api_key(key_id) {
        Ok(Some(cap)) => cap,
        Ok(None) => {
            eprintln!("Key not found: {}", key_id_str);
            return;
        }
        Err(e) => {
            eprintln!("Error fetching key: {e}");
            return;
        }
    };

    // Get all transactions for this wallet and filter by key_id
    match svc.get_transactions(wallet_id, 1000, 0) {
        Ok(txs) => {
            let spends: Vec<_> = txs
                .iter()
                .filter(|tx| {
                    matches!(
                        tx.tx_type,
                        hkask_types::wallet::TransactionType::Spend { .. }
                    )
                })
                .collect();

            println!("Spending Report");
            println!("===============");
            println!();
            println!("  Key ID:      {}", key_id_str);
            println!(
                "  Spent:       {}/{} rJ",
                key_info.spent_rj, key_info.spending_limit_rj
            );
            if let Some(exp) = key_info.expiry {
                println!("  Expires:     {}", exp.format("%Y-%m-%d"));
            }
            println!();

            if spends.is_empty() {
                println!("  (no spending transactions)");
            } else {
                // Aggregate by tool
                let mut by_tool: std::collections::HashMap<String, (u64, u32)> =
                    std::collections::HashMap::new();
                let mut total_spent: i64 = 0;
                for tx in &spends {
                    if let hkask_types::wallet::TransactionType::Spend { ref tool, gas, .. } =
                        tx.tx_type
                    {
                        let entry = by_tool.entry(tool.clone()).or_insert((0, 0));
                        entry.0 += gas;
                        entry.1 += 1;
                    }
                    total_spent += tx.rjoules_delta.abs();
                }

                println!("  Total rJ spent:  {}", total_spent);
                println!("  Total gas units:  (see breakdown below)");
                println!();
                println!("  By Tool:");
                println!("  ────────");
                let mut tools: Vec<_> = by_tool.iter().collect();
                tools.sort_by(|a, b| b.1.0.cmp(&a.1.0));
                for (tool, (gas, count)) in &tools {
                    println!("    {tool}:  {gas} gas units ({count} calls)");
                }
                println!();

                // Show recent transactions
                println!("  Recent Spend Transactions:");
                println!("  ─────────────────────────");
                for tx in spends.iter().rev().take(10) {
                    let tool_name = match &tx.tx_type {
                        hkask_types::wallet::TransactionType::Spend { tool, .. } => tool.as_str(),
                        _ => "unknown",
                    };
                    println!(
                        "    {} | {} rJ | {}",
                        tx.timestamp.format("%Y-%m-%d %H:%M"),
                        tx.rjoules_delta.abs(),
                        tool_name
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching transactions: {e}");
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn resolve_wallet(wallet_arg: Option<String>) -> Result<WalletId, String> {
    match wallet_arg {
        Some(ref s) => {
            WalletId::from_str(s).map_err(|e| format!("Invalid wallet ID '{}': {}", s, e))
        }
        None => Ok(WalletId::default()),
    }
}

fn parse_chain(s: Option<&str>) -> Result<ChainId, String> {
    match s {
        Some("solana") => Ok(ChainId::Solana),
        Some("hedera") => Ok(ChainId::Hedera),
        Some("hinkal") | None => Ok(ChainId::Hinkal),
        Some(other) => Err(format!(
            "Invalid chain '{}'. Expected one of: hinkal, solana, hedera",
            other
        )),
    }
}

fn resolve_privacy_mode(_private: bool, transparent: bool) -> PrivacyMode {
    if transparent {
        PrivacyMode::Transparent
    } else {
        PrivacyMode::Shielded
    }
}
