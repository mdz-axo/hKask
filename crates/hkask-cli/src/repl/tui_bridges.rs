//! TUI domain bridge implementations for `TuiReplBridge`.
//!
//! Implements the 9 domain-specific bridge traits from `hkask-tui` so that
//! TUI windows receive live service data rather than mock fallbacks.

use hkask_templates::BundleRegistryIndex;
use hkask_tui::ReplBridge;
use hkask_tui::bridges::{
    backup::{BackupConfigSummary, BackupDataBridge, SnapshotInfo},
    companies::{CompaniesDataBridge, CompanySummary, FinancialSummary, PortfolioSummary},
    config::{ConfigDataBridge, ConfigSnapshot},
    docproc::{ChunkInfo, DocprocDataBridge, QAPair},
    kanban::{KanbanBoardSummary, KanbanDataBridge, KanbanStatusCounts, KanbanTaskSummary},
    matrix::{MatrixConnectionStatus, MatrixDataBridge, MatrixMessageSummary, MatrixRoomSummary},
    media::{GalleryStatus, ImageSummary, MediaDataBridge},
    memory::{ConsolidationStatus, MemoryDataBridge, MemorySummary, MemoryTriple},
    registry::{BundleListItem, RegistryDataBridge, SkillSummary, TemplateListItem},
    replica::{ReplicaDataBridge, ReplicaInfo},
    research::{ExtractResult, FeedInfo, ResearchDataBridge, SearchResult},
    skills::{SkillExecResult, SkillListItem, SkillsDataBridge},
    training::{AdapterSummary, DeploymentSummary, TrainingDataBridge},
    wallet::{WalletDataBridge, WalletTxSummary},
};
use hkask_wallet::GAS_PER_RJOULE;

use crate::repl::TuiReplBridge;

// ── ConfigDataBridge ────────────────────────────────────────────────

impl ConfigDataBridge for TuiReplBridge {
    fn config_snapshot(&self) -> ConfigSnapshot {
        let state = self.state.lock().expect("ReplState lock poisoned");
        let s = &state.repl_settings;
        let (mcp_loaded, mcp_total) = self.mcp_status();
        ConfigSnapshot {
            model: self.model_name().to_string(),
            temperature: s.temperature,
            top_p: s.top_p,
            max_tokens: s.max_tokens,
            tool_loop_limit: s.tool_loop_limit,
            context_turns: s.context_turns,
            gas_heuristic: s.gas_heuristic,
            gas_cap: s.gas_cap,
            auto_condense: s.auto_condense,
            embedding_model: s.embedding_model.clone(),
            classifier_model: s.classifier_model.clone(),
            mcp_loaded,
            mcp_total,
        }
    }
}

// ── RegistryDataBridge ──────────────────────────────────────────────

impl RegistryDataBridge for TuiReplBridge {
    fn template_count(&self) -> usize {
        let state = self.state.lock().expect("lock");
        state
            .service_context
            .registry()
            .try_lock()
            .map(|r| r.list_skills_owned().len())
            .unwrap_or(0)
    }

    fn skill_count(&self) -> usize {
        self.template_count()
    }

    fn bundle_count(&self) -> usize {
        let state = self.state.lock().expect("lock");
        state
            .service_context
            .registry()
            .try_lock()
            .map(|r| r.list_bundles().len())
            .unwrap_or(0)
    }

    fn list_templates(&self) -> Vec<TemplateListItem> {
        let state = self.state.lock().expect("lock");
        state
            .service_context
            .registry()
            .try_lock()
            .map(|r| {
                r.list_skills_owned()
                    .into_iter()
                    .map(|s| TemplateListItem {
                        id: s.id.clone(),
                        name: s.id.clone(),
                        description: None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn list_skills(&self) -> Vec<SkillSummary> {
        let state = self.state.lock().expect("lock");
        state
            .service_context
            .registry()
            .try_lock()
            .map(|r| {
                r.list_skills_owned()
                    .into_iter()
                    .map(|s| SkillSummary {
                        id: s.id.clone(),
                        name: s.id.clone(),
                        domain: format!("{:?}", s.domain),
                        description: None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn list_bundles(&self) -> Vec<BundleListItem> {
        let state = self.state.lock().expect("lock");
        state
            .service_context
            .registry()
            .try_lock()
            .map(|r| {
                r.list_bundles()
                    .into_iter()
                    .map(|b| BundleListItem {
                        id: b.id.clone(),
                        name: b.name.clone(),
                        version: b.version.clone(),
                        description: Some(b.description.clone()),
                        skill_count: b.skills.len(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

// ── WalletDataBridge ────────────────────────────────────────────────

impl WalletDataBridge for TuiReplBridge {
    fn wallet_balance(&self) -> (u64, u64, u64) {
        let rjoules = self.gas_remaining();
        let usdc_micro = rjoules.saturating_mul(10);
        let gas_equiv = rjoules;
        (rjoules, usdc_micro, gas_equiv)
    }

    fn wallet_transactions(&self, limit: usize) -> Vec<WalletTxSummary> {
        let total = self.gas_cap();
        let remaining = self.gas_remaining();
        let consumed = total.saturating_sub(remaining);
        let mut txs = Vec::new();
        if total > 0 {
            txs.push(WalletTxSummary {
                timestamp: chrono::Utc::now().to_rfc3339(),
                rjoules_delta: total as i64,
                tx_type: "Session Budget".into(),
                balance_after: total,
                detail: Some("gas cap".into()),
            });
        }
        if consumed > 0 {
            txs.push(WalletTxSummary {
                timestamp: chrono::Utc::now().to_rfc3339(),
                rjoules_delta: -(consumed as i64),
                tx_type: "Consumed".into(),
                balance_after: remaining,
                detail: Some("inference + tool calls".into()),
            });
        }
        txs.truncate(limit.max(1));
        txs
    }

    fn gas_per_rjoule(&self) -> u64 {
        GAS_PER_RJOULE
    }

    fn transaction_count(&self) -> u64 {
        if self.gas_cap() > 0 { 2 } else { 0 }
    }
}

// ── MemoryDataBridge ────────────────────────────────────────────────

impl MemoryDataBridge for TuiReplBridge {
    fn memory_summary(&self) -> MemorySummary {
        let state = self.state.lock().expect("lock");
        let episodic_count = state
            .episodic_storage
            .episodic_storage_usage(&state.agent_webid)
            .unwrap_or(0);
        let episodic_budget = state.episodic_storage.episodic_storage_budget();
        let semantic_count = state
            .semantic_storage
            .semantic_storage_usage("")
            .unwrap_or(0);
        let candidates = state
            .consolidation_service
            .as_ref()
            .map(|cs| cs.consolidation_candidate_count(&state.agent_webid))
            .unwrap_or(0);
        MemorySummary {
            episodic_count,
            episodic_budget,
            semantic_count,
            semantic_low_confidence: 0,
            consolidation_candidates: candidates,
        }
    }

    fn recent_episodic(&self, _limit: usize) -> Vec<MemoryTriple> {
        Vec::new()
    }

    fn recent_semantic(&self, _limit: usize) -> Vec<MemoryTriple> {
        Vec::new()
    }

    fn consolidation_status(&self) -> ConsolidationStatus {
        let state = self.state.lock().expect("lock");
        let candidates = state
            .consolidation_service
            .as_ref()
            .map(|cs| cs.consolidation_candidate_count(&state.agent_webid))
            .unwrap_or(0);
        let semantic_count = state
            .semantic_storage
            .semantic_storage_usage("")
            .unwrap_or(0);
        let episodic_budget = state.episodic_storage.episodic_storage_budget();
        ConsolidationStatus {
            candidate_count: candidates,
            semantic_count,
            low_confidence_count: 0,
            episodic_budget,
        }
    }
}

// ── KanbanDataBridge ────────────────────────────────────────────────

impl KanbanDataBridge for TuiReplBridge {
    fn board_list(&self) -> Vec<KanbanBoardSummary> {
        let state = self.state.lock().expect("lock");
        if let Some(ref ks) = state.kanban_service {
            match ks.board_list(&state.agent_webid) {
                Ok(boards) => boards
                    .into_iter()
                    .map(|b| KanbanBoardSummary {
                        id: b.id.to_string(),
                        name: b.name,
                        columns: b.columns.iter().map(|c| c.name.clone()).collect(),
                        task_count: 0,
                    })
                    .collect(),
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    fn tasks_by_status(&self, status: &str, _limit: usize) -> Vec<KanbanTaskSummary> {
        let state = self.state.lock().expect("lock");
        if let Some(ref ks) = state.kanban_service {
            let board_list = ks.board_list(&state.agent_webid).unwrap_or_default();
            if let Some(board) = board_list.first() {
                let status_enum = match status {
                    "backlog" => hkask_services::TaskStatus::Backlog,
                    "ready" => hkask_services::TaskStatus::Ready,
                    "in_progress" => hkask_services::TaskStatus::InProgress,
                    "review" => hkask_services::TaskStatus::Review,
                    "done" => hkask_services::TaskStatus::Done,
                    _ => return Vec::new(),
                };
                let filter = hkask_services::TaskFilter::by_status(status_enum);
                match ks.task_list(board.id, filter) {
                    Ok(tasks) => tasks
                        .into_iter()
                        .map(|t| KanbanTaskSummary {
                            id: t.id.to_string(),
                            title: t.title,
                            status: format!("{:?}", t.status).to_lowercase(),
                            assignee: t.assignee.map(|a| a.to_string()),
                            priority: t.priority.map(|p| format!("{:?}", p).to_lowercase()),
                            labels: t.labels,
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    fn status_counts(&self) -> KanbanStatusCounts {
        let state = self.state.lock().expect("lock");
        if let Some(ref ks) = state.kanban_service {
            let board_list = ks.board_list(&state.agent_webid).unwrap_or_default();
            if let Some(board) = board_list.first() {
                let count_status = |s: hkask_services::TaskStatus| -> usize {
                    let filter = hkask_services::TaskFilter::by_status(s);
                    ks.task_list(board.id, filter).map(|t| t.len()).unwrap_or(0)
                };
                KanbanStatusCounts {
                    backlog: count_status(hkask_services::TaskStatus::Backlog),
                    ready: count_status(hkask_services::TaskStatus::Ready),
                    in_progress: count_status(hkask_services::TaskStatus::InProgress),
                    review: count_status(hkask_services::TaskStatus::Review),
                    done: count_status(hkask_services::TaskStatus::Done),
                }
            } else {
                KanbanStatusCounts {
                    backlog: 0,
                    ready: 0,
                    in_progress: 0,
                    review: 0,
                    done: 0,
                }
            }
        } else {
            KanbanStatusCounts {
                backlog: 0,
                ready: 0,
                in_progress: 0,
                review: 0,
                done: 0,
            }
        }
    }

    fn all_tasks(&self, _limit: usize) -> Vec<KanbanTaskSummary> {
        let state = self.state.lock().expect("lock");
        if let Some(ref ks) = state.kanban_service {
            let board_list = ks.board_list(&state.agent_webid).unwrap_or_default();
            if let Some(board) = board_list.first() {
                let filter = hkask_services::TaskFilter::all();
                match ks.task_list(board.id, filter) {
                    Ok(tasks) => tasks
                        .into_iter()
                        .map(|t| KanbanTaskSummary {
                            id: t.id.to_string(),
                            title: t.title,
                            status: format!("{:?}", t.status).to_lowercase(),
                            assignee: t.assignee.map(|a| a.to_string()),
                            priority: t.priority.map(|p| format!("{:?}", p).to_lowercase()),
                            labels: t.labels,
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    fn move_task(&self, task_id: &str, to_status: &str) -> Result<KanbanTaskSummary, String> {
        let state = self.state.lock().expect("lock");
        let ks = state
            .kanban_service
            .as_ref()
            .ok_or_else(|| "kanban service not initialized".to_string())?;

        let tid: hkask_types::TaskId = task_id
            .parse()
            .map_err(|e| format!("invalid task id '{}': {}", task_id, e))?;

        let target = hkask_services::TaskStatus::parse_str(to_status)
            .ok_or_else(|| format!("unknown status: {}", to_status))?;

        let actor = state.agent_webid;

        ks.task_move(tid, target, actor)
            .map(|task| KanbanTaskSummary {
                id: task.id.to_string(),
                title: task.title,
                status: task.status.as_str().to_string(),
                assignee: task.assignee.map(|a| a.to_string()),
                priority: task.priority.map(|p| format!("{:?}", p).to_lowercase()),
                labels: task.labels,
            })
            .map_err(|e| format!("{}", e))
    }
}

// ── MatrixDataBridge ─────────────────────────────────────────────────

#[cfg(feature = "communication")]
impl MatrixDataBridge for TuiReplBridge {
    fn connection_status(&self) -> MatrixConnectionStatus {
        let state = self.state.lock().expect("lock");
        let connected = state
            .service_context
            .matrix_transport()
            .and_then(|mt| mt.try_lock().ok().map(|t| t.healthy()))
            .unwrap_or(false);
        MatrixConnectionStatus {
            connected,
            homeserver: String::new(),
            user_id: None,
        }
    }

    fn list_rooms(&self) -> Vec<MatrixRoomSummary> {
        let state = self.state.lock().expect("lock");
        let transport = state.service_context.matrix_transport().cloned();
        transport
            .and_then(|mt| {
                self.rt_handle.block_on(async {
                    let t = mt.lock().await;
                    t.list_rooms().await.ok().map(|rooms| {
                        rooms
                            .into_iter()
                            .map(|thread| MatrixRoomSummary {
                                id: thread.room_id.0.clone(),
                                title: thread.title,
                                member_count: thread.participants.len(),
                                escalated: thread.escalated,
                                last_active: String::new(),
                            })
                            .collect()
                    })
                })
            })
            .unwrap_or_default()
    }

    fn recent_messages(&self, room_id: &str, limit: usize) -> Vec<MatrixMessageSummary> {
        let state = self.state.lock().expect("lock");
        let transport = state.service_context.matrix_transport().cloned();
        transport
            .and_then(|mt| {
                self.rt_handle.block_on(async {
                    let t = mt.lock().await;
                    let rid = hkask_communication::matrix::RoomId::new(room_id);
                    t.get_messages(&rid, limit).await.ok().map(|msgs| {
                        msgs.into_iter()
                            .map(|m| MatrixMessageSummary {
                                sender: m.sender.0.clone(),
                                body: m.body,
                                timestamp: m.timestamp.to_string(),
                            })
                            .collect()
                    })
                })
            })
            .unwrap_or_default()
    }

    fn room_count(&self) -> usize {
        self.list_rooms().len()
    }
}

// ── BackupDataBridge ─────────────────────────────────────────────────

impl BackupDataBridge for TuiReplBridge {
    fn last_snapshot(&self) -> Option<SnapshotInfo> {
        // Pod-directory backup replaced the old CAS-based system.
        // Use `kask backup status` for per-pod snapshot info.
        None
    }

    fn snapshot_count(&self) -> usize {
        0
    }

    fn config(&self) -> BackupConfigSummary {
        BackupConfigSummary {
            auto_snapshot: false,
            verify_after_snapshot: false,
            encryption_enabled: false,
            tracked_types_count: 0,
            retention_daily_days: 0,
            retention_weekly_weeks: 0,
        }
    }

    fn verify_status(&self) -> (bool, String) {
        (
            true,
            "Pod-directory backup active — run `kask backup verify` for CAS integrity".into(),
        )
    }
}

// ── MediaDataBridge ──────────────────────────────────────────────────

fn extract_mcp_text(result: &rmcp::model::CallToolResult) -> Option<String> {
    result
        .content
        .iter()
        .filter_map(|c| match &**c {
            rmcp::model::RawContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .next()
}

fn parse_mcp_json(result: &rmcp::model::CallToolResult) -> Option<serde_json::Value> {
    extract_mcp_text(result).and_then(|text| serde_json::from_str(&text).ok())
}

impl MediaDataBridge for TuiReplBridge {
    fn gallery_status(&self) -> GalleryStatus {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match runtime.call_tool("media", "gallery_status", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    GalleryStatus {
                        active: content
                            .as_ref()
                            .and_then(|v| v.get("image_count"))
                            .is_some(),
                        gallery_id: content
                            .as_ref()
                            .and_then(|v| v["gallery_id"].as_str())
                            .map(String::from),
                        image_count: content
                            .as_ref()
                            .and_then(|v| v["image_count"].as_u64())
                            .unwrap_or(0) as usize,
                        root_path: content
                            .as_ref()
                            .and_then(|v| v["root_path"].as_str())
                            .map(String::from),
                    }
                }
                Err(_) => GalleryStatus {
                    active: false,
                    gallery_id: None,
                    image_count: 0,
                    root_path: None,
                },
            }
        })
    }

    fn recent_images(&self, limit: usize) -> Vec<ImageSummary> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("query".into(), serde_json::Value::String(String::new()));
            args.insert("limit".into(), serde_json::Value::from(limit as u64));
            match runtime.call_tool("media", "gallery_search", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["results"].as_array())
                        .map(|results| {
                            results
                                .iter()
                                .filter_map(|r| {
                                    Some(ImageSummary {
                                        index: r["image_index"].as_u64()? as usize,
                                        path: r["image"].as_str()?.to_string(),
                                        format: String::new(),
                                        width: 0,
                                        height: 0,
                                        tags: r["matching_tags"]
                                            .as_array()
                                            .map(|a| {
                                                a.iter()
                                                    .filter_map(|t| t.as_str().map(String::from))
                                                    .collect()
                                            })
                                            .unwrap_or_default(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn tagged_images(&self, _tag: &str, _limit: usize) -> Vec<ImageSummary> {
        Vec::new()
    }
}

// ── TrainingDataBridge ───────────────────────────────────────────────

impl TrainingDataBridge for TuiReplBridge {
    fn adapter_list(&self) -> Vec<AdapterSummary> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match runtime
                .call_tool("training", "training_list_adapters", args)
                .await
            {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["adapters"].as_array())
                        .map(|adapters| {
                            adapters
                                .iter()
                                .filter_map(|a| {
                                    Some(AdapterSummary {
                                        name: a["name"].as_str()?.to_string(),
                                        base_model: a["base_model"].as_str()?.to_string(),
                                        version: a["version"].as_str().unwrap_or("v1").to_string(),
                                        size_bytes: a["size_bytes"].as_u64().unwrap_or(0),
                                        expertise: a["expertise"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn deployment_list(&self) -> Vec<DeploymentSummary> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match runtime
                .call_tool("training", "training_list_adapters", args)
                .await
            {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["adapters"].as_array())
                        .map(|adapters| {
                            adapters
                                .iter()
                                .filter_map(|a| {
                                    let deployed = a["deployment"].as_object()?;
                                    Some(DeploymentSummary {
                                        adapter_name: a["name"].as_str()?.to_string(),
                                        provider: deployed
                                            .get("provider")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("-")
                                            .to_string(),
                                        status: deployed
                                            .get("status")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("inactive")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn session_count(&self) -> usize {
        0
    }

    fn adapter_count(&self) -> usize {
        self.adapter_list().len()
    }
}

// ── CompaniesDataBridge (live MCP dispatch to hkask-mcp-companies) ──

impl CompaniesDataBridge for TuiReplBridge {
    fn search(&self, query: &str) -> Vec<CompanySummary> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        let query = query.to_string();
        // Store the query for last_searched
        *self.last_companies_search.lock().expect("lock") = Some(query.clone());
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("query".into(), serde_json::Value::String(query));
            args.insert("limit".into(), serde_json::Value::from(10_u64));
            match runtime.call_tool("companies", "symbol_search", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v.as_array())
                        .map(|results| {
                            results
                                .iter()
                                .filter_map(|r| {
                                    Some(CompanySummary {
                                        symbol: r["symbol"].as_str()?.to_string(),
                                        name: r["name"].as_str()?.to_string(),
                                        exchange: r["exchangeShortName"].as_str().map(String::from),
                                        industry: r["industry"].as_str().map(String::from),
                                        sector: r["sector"].as_str().map(String::from),
                                        market_cap: r["marketCap"].as_f64(),
                                        description: None,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn last_searched(&self) -> Option<String> {
        self.last_companies_search.lock().expect("lock").clone()
    }

    fn financials(&self) -> Option<FinancialSummary> {
        let symbol = self.last_companies_search.lock().expect("lock").clone()?;
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        let sym = symbol.clone();
        let sym2 = symbol.clone();
        self.rt_handle.block_on(async {
            // Fetch key metrics (P/E, revenue growth)
            let mut metric_args = serde_json::Map::new();
            metric_args.insert("symbol".into(), serde_json::Value::String(sym.clone()));
            metric_args.insert("limit".into(), serde_json::Value::from(1_u64));
            let metrics = runtime
                .call_tool("companies", "key_metrics", metric_args)
                .await
                .ok()
                .and_then(|r| parse_mcp_json(&r));

            // Fetch stock quote (price, change)
            let mut quote_args = serde_json::Map::new();
            quote_args.insert("symbol".into(), serde_json::Value::String(sym2.clone()));
            let quote = runtime
                .call_tool("companies", "stock_quote", quote_args)
                .await
                .ok()
                .and_then(|r| parse_mcp_json(&r));

            let metrics_array = metrics
                .as_ref()
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first());
            let quote_array = quote
                .as_ref()
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first());

            let pe_ratio = metrics_array.and_then(|m| m["peRatio"].as_f64());
            let revenue_growth = metrics_array.and_then(|m| m["revenueGrowth"].as_f64());
            let price = quote_array.and_then(|q| q["price"].as_f64());
            let change_pct = quote_array.and_then(|q| q["changesPercentage"].as_f64());

            Some(FinancialSummary {
                symbol: sym,
                price,
                change_pct,
                pe_ratio,
                revenue_growth,
            })
        })
    }

    fn portfolio_list(&self) -> Vec<PortfolioSummary> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match runtime.call_tool("companies", "portfolio_list", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["portfolios"].as_array())
                        .map(|portfolios| {
                            portfolios
                                .iter()
                                .filter_map(|p| {
                                    let name = p.as_str()?.to_string();
                                    Some(PortfolioSummary {
                                        name,
                                        holdings: 0,
                                        created: None,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }
}

// ── ResearchDataBridge (live MCP dispatch to hkask-mcp-research) ──

impl ResearchDataBridge for TuiReplBridge {
    fn search(&self, query: &str) -> Vec<SearchResult> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        let query = query.to_string();
        *self.last_research_search.lock().expect("lock") = Some(query.clone());
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("query".into(), serde_json::Value::String(query));
            args.insert("count".into(), serde_json::Value::from(10_u64));
            match runtime.call_tool("research", "web_search", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["results"].as_array())
                        .map(|results| {
                            results
                                .iter()
                                .filter_map(|r| {
                                    Some(SearchResult {
                                        title: r["title"].as_str()?.to_string(),
                                        url: r["url"].as_str()?.to_string(),
                                        snippet: r["snippet"]
                                            .as_str()
                                            .map(String::from)
                                            .unwrap_or_default(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn feed_list(&self) -> Vec<FeedInfo> {
        // No MCP tool for RSS feeds — feeds are managed through search queries.
        Vec::new()
    }

    fn extract(&self, url: &str) -> Option<ExtractResult> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        let url = url.to_string();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("url".into(), serde_json::Value::String(url.clone()));
            args.insert(
                "format".into(),
                serde_json::Value::String("markdown".into()),
            );
            match runtime.call_tool("research", "web_extract", args).await {
                Ok(ref result) => {
                    let text = extract_mcp_text(result)?;
                    Some(ExtractResult {
                        url: url.clone(),
                        content: text,
                        format: "markdown".into(),
                    })
                }
                Err(_) => None,
            }
        })
    }

    fn last_query(&self) -> Option<String> {
        self.last_research_search.lock().expect("lock").clone()
    }
}

// ── DocprocDataBridge (live MCP dispatch to hkask-mcp-docproc) ──

impl DocprocDataBridge for TuiReplBridge {
    fn chunk_list(&self) -> Vec<ChunkInfo> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("text".into(), serde_json::Value::String("".into()));
            args.insert("max_tokens".into(), serde_json::Value::from(512_u64));
            match runtime.call_tool("docproc", "docproc_chunk", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["chunks"].as_array())
                        .map(|chunks| {
                            chunks
                                .iter()
                                .enumerate()
                                .map(|(i, c)| ChunkInfo {
                                    index: i,
                                    token_count: c["token_count"].as_u64().unwrap_or(0) as usize,
                                    preview: c["text"]
                                        .as_str()
                                        .map(|t| t.chars().take(80).collect())
                                        .unwrap_or_default(),
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn qa_list(&self) -> Vec<QAPair> {
        // QA pairs are generated on demand via generate_qa tool — not listed.
        Vec::new()
    }

    fn index_status(&self) -> (usize, usize) {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("question".into(), serde_json::Value::String("".into()));
            args.insert("top_k".into(), serde_json::Value::from(1_u64));
            match runtime.call_tool("docproc", "docproc_query", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    let total = content
                        .as_ref()
                        .and_then(|v| v["index_size"].as_u64())
                        .unwrap_or(0) as usize;
                    (total, total)
                }
                Err(_) => (0, 0),
            }
        })
    }
}

// ── ReplicaDataBridge (live MCP dispatch to hkask-mcp-replica) ──

impl ReplicaDataBridge for TuiReplBridge {
    fn list_replicas(&self) -> Vec<ReplicaInfo> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("action".into(), serde_json::Value::String("list".into()));
            match runtime.call_tool("replica", "replica_registry", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["replicas"].as_array())
                        .map(|replicas| {
                            replicas
                                .iter()
                                .filter_map(|r| {
                                    Some(ReplicaInfo {
                                        author: r["author"].as_str()?.to_string(),
                                        centroid_count: r["centroid_count"].as_u64().unwrap_or(0)
                                            as usize,
                                        status: r["status"]
                                            .as_str()
                                            .map(String::from)
                                            .unwrap_or_else(|| "unknown".into()),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn replica_count(&self) -> usize {
        self.list_replicas().len()
    }
}

// ── SkillsDataBridge (live MCP dispatch to hkask-mcp-skill) ──

impl SkillsDataBridge for TuiReplBridge {
    fn skill_list(&self) -> Vec<SkillListItem> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match runtime.call_tool("skill", "skill_list", args).await {
                Ok(ref result) => {
                    let content = parse_mcp_json(result);
                    content
                        .as_ref()
                        .and_then(|v| v["skills"].as_array())
                        .map(|skills| {
                            skills
                                .iter()
                                .filter_map(|s| {
                                    Some(SkillListItem {
                                        id: s["id"].as_str()?.to_string(),
                                        description: s["description"]
                                            .as_str()
                                            .map(String::from)
                                            .unwrap_or_default(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        })
    }

    fn skill_execute(&self, skill_id: &str, context: &str) -> Option<SkillExecResult> {
        let state = self.state.lock().expect("lock");
        let runtime = state.service_context.mcp_runtime().clone();
        let skill_id = skill_id.to_string();
        let context = context.to_string();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("skill_id".into(), serde_json::Value::String(skill_id));
            args.insert("context".into(), serde_json::Value::String(context));
            match runtime.call_tool("skill", "skill_execute", args).await {
                Ok(ref result) => {
                    let text = extract_mcp_text(result)?;
                    Some(SkillExecResult {
                        skill_id: "".into(),
                        output: text,
                        tokens_used: 0,
                    })
                }
                Err(_) => None,
            }
        })
    }

    fn skill_count(&self) -> usize {
        self.skill_list().len()
    }
}
