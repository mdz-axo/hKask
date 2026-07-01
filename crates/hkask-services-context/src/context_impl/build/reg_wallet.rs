//! Registry + wallet: agent records, A2A restore, rJoule payments.

use super::super::*;
use super::foundation::Foundation;
use super::loops::LoopWiring;
use super::mcp_pods::McpPods;
use hkask_services_core::self_heal::{
    HealAction, HealContext, HealRegistry, HealStrategy, SelfHealer,
};
use hkask_services_core::{ServiceConfig, ServiceError};
use hkask_wallet::manager::{WalletManager, WalletSelfHealer};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry + wallet: agent records, A2A restore, rJoule payments.
pub(super) struct RegWallet {
    pub registry: Arc<tokio::sync::Mutex<SqliteRegistry>>,
    pub agent_registry_store: hkask_storage::AgentRegistryStore,
    pub wallet_service: Option<Arc<WalletService>>,
    pub wallet_store: Option<Arc<WalletStore>>,
    pub wallet_gas_calibrator: Option<Arc<hkask_cns::WalletGasCalibrator>>,
}

pub(super) async fn build_registry_and_wallet(
    config: &ServiceConfig,
    f: &Foundation,
    l: &LoopWiring,
    _mcp: &McpPods,
) -> Result<RegWallet, ServiceError> {
    // Registry
    let registry = Arc::new(tokio::sync::Mutex::new(
        SqliteRegistry::new_with_conn(f.primary_conn.clone()).map_err(|e| {
            ServiceError::Template {
                source: None,
                message: e.to_string(),
            }
        })?,
    ));

    // Agent registry store
    let agent_registry_store = hkask_storage::AgentRegistryStore::new(f.primary_conn.clone());
    agent_registry_store
        .initialize_schema()
        .map_err(|e| ServiceError::AgentRegistryStore {
            source: None,
            message: e.to_string(),
        })?;

    // Restore A2A state from persistent storage
    let registered_agents =
        agent_registry_store
            .list()
            .map_err(|e| ServiceError::AgentRegistryStore {
                source: None,
                message: e.to_string(),
            })?;
    if !registered_agents.is_empty() {
        use std::str::FromStr;
        let agents: Vec<hkask_agents::a2a::A2AAgent> = registered_agents
            .iter()
            .map(|ra| hkask_agents::a2a::A2AAgent {
                webid: hkask_types::WebID::from_str(&ra.definition.name).unwrap_or_else(|_| {
                    hkask_types::WebID::from_persona(ra.definition.name.as_bytes())
                }),
                agent_type: ra.definition.agent_kind,
                capabilities: ra.definition.capabilities.clone(),
                registered_at: chrono::DateTime::parse_from_rfc3339(&ra.registered_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
                active: true,
            })
            .collect();
        let tokens = std::collections::HashMap::new();
        l.a2a_runtime
            .restore_from_storage(agents, tokens)
            .await
            .map_err(|e| ServiceError::A2A {
                source: None,
                message: e.to_string(),
            })?;
    }

    // Wallet — non-fatal if config or build fails (daemon can run without wallet)
    let (wallet_service, wallet_store, wallet_gas_calibrator) = match build_wallet(config, f, l) {
        Ok(tuple) => tuple,
        Err(e) => {
            tracing::warn!(target: "cns.wallet", error = %e, "Wallet unavailable — running without rJoule");
            (None, None, None)
        }
    };

    Ok(RegWallet {
        registry,
        agent_registry_store,
        wallet_service,
        wallet_store,
        wallet_gas_calibrator,
    })
}

/// Build wallet subsystem — returns (service, store, gas_calibrator) or error.
#[allow(clippy::type_complexity)]
fn build_wallet(
    config: &ServiceConfig,
    f: &Foundation,
    l: &LoopWiring,
) -> Result<
    (
        Option<Arc<WalletService>>,
        Option<Arc<WalletStore>>,
        Option<Arc<hkask_cns::WalletGasCalibrator>>,
    ),
    ServiceError,
> {
    let wallet_db_path = if config.in_memory {
        None
    } else {
        let path = hkask_types::agent_paths::agent_wallet_db(&config.agent_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Some(path)
    };

    let wallet_conn = if let Some(ref path) = wallet_db_path {
        let path_str = path.to_string_lossy().to_string();
        match hkask_storage::Database::open(&path_str, &config.db_passphrase) {
            Ok(db) => {
                tracing::info!(
                    target: "cns.wallet",
                    path = %path_str,
                    agent = %config.agent_name,
                    "Per-agent wallet database opened"
                );
                db.conn_arc()
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.wallet",
                    path = %path_str,
                    error = %e,
                    "Failed to open per-agent wallet DB, falling back to shared connection"
                );
                Arc::clone(&f.db.conn_arc())
            }
        }
    } else {
        Arc::clone(&f.db.conn_arc())
    };
    let wallet_store = Arc::new(WalletStore::new(wallet_conn));

    struct WalletSelfHealerAdapter {
        healer: SelfHealer,
        manager: Arc<WalletManager>,
    }

    impl WalletSelfHealer for WalletSelfHealerAdapter {
        fn heal(&self, operation: &str, error: &str) {
            if operation == "wallet.deposit.process"
                && error.starts_with("deposit address unresolvable:")
                && let Some((_, address)) = error.split_once(':')
            {
                let address = address.trim();
                if let Ok(true) = self.manager.repair_deposit_address_mapping(address) {
                    tracing::info!(
                        target: "cns.heal",
                        operation = %operation,
                        address = %address,
                        "Wallet self-heal repaired deposit address mapping"
                    );
                    return;
                }
            }

            let ctx = HealContext {
                operation: operation.to_string(),
                error_message: error.to_string(),
                env_vars: HashMap::new(),
                config_search_paths: Vec::new(),
                can_retry: true,
            };
            let _ = self.healer.attempt(error, &ctx);
        }
    }

    let mut heal_registry = HealRegistry::with_defaults();
    heal_registry.add(HealStrategy {
        name: "wallet-deposit-unresolvable".into(),
        error_pattern: "deposit address unresolvable".into(),
        description: "Deposit address unresolvable — retry after wallet auto-repair".into(),
        action: HealAction::RetryWithBackoff {
            max_attempts: 2,
            delay_ms: 500,
        },
    });
    let svc = WalletService::build(
        &config.wallet_config,
        Arc::clone(&wallet_store),
        Arc::clone(&f.cns_event_sink),
        Arc::clone(&l.cybernetics_loop),
    )?;
    let svc = Arc::new(
        svc.as_ref()
            .clone()
            .with_consent_manager(Arc::clone(&f.consent_manager)),
    );
    let wallet_manager = svc.manager();

    let wallet_self_healer = WalletSelfHealerAdapter {
        healer: SelfHealer::with_registry(heal_registry),
        manager: Arc::clone(wallet_manager),
    };
    wallet_manager.set_self_healer(Arc::new(wallet_self_healer));

    // Ensure default wallet
    let default_wallet = WalletId::default();
    wallet_manager
        .ensure_wallet(default_wallet)
        .map_err(|e| ServiceError::Wallet {
            source: Some(Box::new(e)),
            message: "Failed to ensure default wallet".into(),
        })?;

    // Bind wallets to replicants
    {
        let user_guard = f.user_store.lock().map_err(|_| ServiceError::UserStore {
            source: None,
            message: hkask_types::InfrastructureError::LockPoisoned.to_string(),
        })?;
        if let Ok(Some(system_identity)) = user_guard.get_replicant(&config.agent_name) {
            let user_id = system_identity.user_id;
            let replicants =
                user_guard
                    .list_replicants(&user_id)
                    .map_err(|e| ServiceError::UserStore {
                        source: None,
                        message: e.to_string(),
                    })?;
            for identity in &replicants {
                if identity.wallet_id.is_some() {
                    continue;
                }
                let wallet_id = WalletId::from_name(&identity.replicant_name);
                if let Err(e) = wallet_manager.ensure_wallet(wallet_id) {
                    tracing::warn!(
                        target: "cns.wallet",
                        replicant = %identity.replicant_name,
                        error = %e,
                        "Failed to create wallet for replicant"
                    );
                    continue;
                }
                if let Err(e) = user_guard.set_wallet_id(&identity.replicant_name, wallet_id) {
                    tracing::warn!(
                        target: "cns.wallet",
                        replicant = %identity.replicant_name,
                        error = %e,
                        "Failed to bind wallet to replicant"
                    );
                } else {
                    tracing::info!(
                        target: "cns.wallet",
                        replicant = %identity.replicant_name,
                        wallet_id = %wallet_id,
                        "Wallet created and bound to replicant"
                    );
                }
            }
        }
    }

    // Spawn deposit monitor
    let monitor_manager = Arc::clone(wallet_manager);
    let interval_secs: u64 = std::env::var("HKASK_DEPOSIT_MONITOR_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    tokio::spawn(async move {
        tracing::info!(
            target: "cns.wallet.deposit",
            interval_secs = %interval_secs,
            "Deposit monitor started — polling every {}s",
            interval_secs
        );
        if let Err(e) = monitor_manager.start_deposit_monitor(interval_secs).await {
            tracing::error!(
                target: "cns.wallet.deposit",
                error = %e,
                "Deposit monitor loop exited with error"
            );
        }
    });

    // Spawn wallet gas calibrator (P9 feedback loop for gas→rJoule rate).
    let wallet_gas_calibrator = {
        let calibrator = Arc::new(
            hkask_cns::WalletGasCalibrator::new(
                Arc::clone(&f.gas_event_store),
                Arc::clone(wallet_manager),
            )
            .with_event_sink(Arc::clone(&f.cns_event_sink)),
        );
        calibrator
            .clone()
            .spawn_calibration(hkask_cns::DEFAULT_WALLET_CALIBRATION_INTERVAL);
        Some(calibrator)
    };

    Ok((Some(svc), Some(wallet_store), wallet_gas_calibrator))
}
