//! TUI domain bridge implementations for `TuiReplBridge`.
//!
//! Implements the domain-specific bridge traits from `hkask-tui` so that
//! TUI windows can receive service data rather than mock fallbacks.

use hkask_capability::{DelegationAction, DelegationResource, DelegationToken, derive_signing_key};
use hkask_ports::ToolPort;
use hkask_templates::BundleRegistryIndex;
use hkask_tui::SystemBridge;
use hkask_tui::bridges::{
    backup::{BackupDataBridge, BackupSnapshot},
    companies::{CompaniesDataBridge, CompanySummary, FinancialSummary, PortfolioSummary},
    config::{ConfigDataBridge, ConfigSnapshot},
    docproc::{ChunkInfo, DocprocDataBridge, QAPair},
    kanban::{KanbanBoardSummary, KanbanDataBridge, KanbanStatusCounts, KanbanTaskSummary},
    media::{GalleryStatus, ImageSummary, MediaDataBridge},
    memory::{ConsolidationStatus, MemoryDataBridge, MemoryHMem, MemorySummary},
    registry::{BundleListItem, RegistryDataBridge, SkillSummary, TemplateListItem},
    replica::{ReplicaDataBridge, ReplicaInfo},
    research::{ExtractResult, FeedInfo, ResearchDataBridge, SearchResult},
    scenarios::{
        CalibrationSummary, EventNode, EventTreeDetail, ScenarioForecastSummary,
        ScenarioPipelineState, ScenariosDataBridge,
    },
    skills::{SkillExecResult, SkillListItem, SkillsDataBridge},
    training::{AdapterSummary, DeploymentSummary, TrainingDataBridge},
    wallet::{WalletDataBridge, WalletSnapshot},
};

#[allow(unused_imports)]
use hkask_tui::bridges::matrix::{
    MatrixConnectionStatus, MatrixDataBridge, MatrixMessageSummary, MatrixRoomSummary,
};

use crate::TuiReplBridge;

// ── ConfigDataBridge ────────────────────────────────────────────────

impl ConfigDataBridge for TuiReplBridge {
    fn config_snapshot(&self) -> ConfigSnapshot {
        let mut snapshot = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let s = &state.repl_settings;
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
                mcp_loaded: 0,
                mcp_total: 0,
            }
        };
        (snapshot.mcp_loaded, snapshot.mcp_total) = self.mcp_status();
        snapshot
    }
}

// ── RegistryDataBridge ──────────────────────────────────────────────

impl RegistryDataBridge for TuiReplBridge {
    fn template_count(&self) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state
            .service_context
            .storage()
            .registry
            .clone()
            .try_lock()
            .map(|r| r.list_skills_owned().len())
            .unwrap_or(0)
    }

    fn skill_count(&self) -> usize {
        self.template_count()
    }

    fn bundle_count(&self) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state
            .service_context
            .storage()
            .registry
            .clone()
            .try_lock()
            .map(|r| r.list_bundles().len())
            .unwrap_or(0)
    }

    fn list_templates(&self) -> Vec<TemplateListItem> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state
            .service_context
            .storage()
            .registry
            .clone()
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state
            .service_context
            .storage()
            .registry
            .clone()
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state
            .service_context
            .storage()
            .registry
            .clone()
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
    fn snapshot(&self, _transaction_limit: usize) -> WalletSnapshot {
        WalletSnapshot::Unavailable {
            reason: "live wallet ledger data is not exposed to the TUI; use `kask wallet`".into(),
        }
    }
}

// ── MemoryDataBridge ────────────────────────────────────────────────

impl MemoryDataBridge for TuiReplBridge {
    fn memory_summary(&self) -> MemorySummary {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mem = match state.service_context.per_agent_memory(&state.current_agent) {
            Ok(m) => m,
            Err(_) => {
                return MemorySummary {
                    episodic_count: 0,
                    episodic_budget: 0,
                    semantic_count: 0,
                    semantic_low_confidence: 0,
                    consolidation_candidates: 0,
                };
            }
        };
        let episodic_count = mem
            .episodic_storage
            .episodic_storage_usage(&state.agent_webid)
            .unwrap_or(0);
        let episodic_budget = mem.episodic_storage.episodic_storage_budget();
        let semantic_count = mem.semantic_storage.semantic_storage_usage("").unwrap_or(0);
        let candidates = mem
            .consolidation_service
            .consolidation_candidate_count(&state.agent_webid);
        MemorySummary {
            episodic_count,
            episodic_budget,
            semantic_count,
            semantic_low_confidence: 0,
            consolidation_candidates: candidates,
        }
    }

    fn recent_episodic(&self, _limit: usize) -> Vec<MemoryHMem> {
        Vec::new()
    }

    fn recent_semantic(&self, _limit: usize) -> Vec<MemoryHMem> {
        Vec::new()
    }

    fn consolidation_status(&self) -> ConsolidationStatus {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mem = state
            .service_context
            .per_agent_memory(&state.current_agent)
            .ok();
        let candidates = mem
            .as_ref()
            .map(|m| {
                m.consolidation_service
                    .consolidation_candidate_count(&state.agent_webid)
            })
            .unwrap_or(0);
        let semantic_count = state
            .service_context
            .consolidation_status_for(&state.current_agent)
            .map(|(_, sc, _)| sc)
            .unwrap_or(0);
        let episodic_budget = mem
            .as_ref()
            .map(|m| m.episodic_storage.episodic_storage_budget())
            .unwrap_or(0);
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref ks) = state.kanban_service {
            let board_list = ks.board_list(&state.agent_webid).unwrap_or_default();
            if let Some(board) = board_list.first() {
                let status_enum = match status {
                    "backlog" => hkask_services_kata_kanban::TaskStatus::Backlog,
                    "ready" => hkask_services_kata_kanban::TaskStatus::Ready,
                    "in_progress" => hkask_services_kata_kanban::TaskStatus::InProgress,
                    "review" => hkask_services_kata_kanban::TaskStatus::Review,
                    "done" => hkask_services_kata_kanban::TaskStatus::Done,
                    _ => return Vec::new(),
                };
                let filter = hkask_services_kata_kanban::TaskFilter::by_status(status_enum);
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref ks) = state.kanban_service {
            let board_list = ks.board_list(&state.agent_webid).unwrap_or_default();
            if let Some(board) = board_list.first() {
                let count_status = |s: hkask_services_kata_kanban::TaskStatus| -> usize {
                    let filter = hkask_services_kata_kanban::TaskFilter::by_status(s);
                    ks.task_list(board.id, filter).map(|t| t.len()).unwrap_or(0)
                };
                KanbanStatusCounts {
                    backlog: count_status(hkask_services_kata_kanban::TaskStatus::Backlog),
                    ready: count_status(hkask_services_kata_kanban::TaskStatus::Ready),
                    in_progress: count_status(hkask_services_kata_kanban::TaskStatus::InProgress),
                    review: count_status(hkask_services_kata_kanban::TaskStatus::Review),
                    done: count_status(hkask_services_kata_kanban::TaskStatus::Done),
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref ks) = state.kanban_service {
            let board_list = ks.board_list(&state.agent_webid).unwrap_or_default();
            if let Some(board) = board_list.first() {
                let filter = hkask_services_kata_kanban::TaskFilter::all();
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

    fn move_task(&self, task_id: &str, to_status: &str) -> anyhow::Result<KanbanTaskSummary> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let ks = state
            .kanban_service
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("kanban service not initialized"))?;

        let tid: hkask_types::TaskId = task_id
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid task id '{}': {}", task_id, e))?;

        let target = hkask_services_kata_kanban::TaskStatus::parse_str(to_status)
            .ok_or_else(|| anyhow::anyhow!("unknown status: {}", to_status))?;

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
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// ── MatrixDataBridge ─────────────────────────────────────────────────

#[cfg(feature = "communication")]
impl MatrixDataBridge for TuiReplBridge {
    fn connection_status(&self) -> MatrixConnectionStatus {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let connected = state
            .service_context
            .infra()
            .matrix
            .as_ref()
            .and_then(|mt| mt.try_lock().ok().map(|t| t.healthy()))
            .unwrap_or(false);
        MatrixConnectionStatus {
            connected,
            homeserver: String::new(),
            user_id: None,
        }
    }

    fn list_rooms(&self) -> Vec<MatrixRoomSummary> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let transport = state.service_context.infra().matrix.as_ref().cloned();
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
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let transport = state.service_context.infra().matrix.as_ref().cloned();
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
    fn snapshot(&self) -> BackupSnapshot {
        BackupSnapshot::Unavailable {
            reason: "live pod-directory backup status is not exposed to the TUI; use `kask backup status`".into(),
        }
    }
}

// ── MediaDataBridge ──────────────────────────────────────────────────

impl TuiReplBridge {
    async fn invoke_mcp_tool(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, hkask_ports::ToolPortError> {
        let (runtime, principal, agent) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                state.service_context.infra().mcp.clone(),
                state.host.resolve_user_webid(),
                state.agent_webid,
            )
        };
        let token = DelegationToken::new(
            DelegationResource::Tool,
            tool.to_string(),
            DelegationAction::Execute,
            principal,
            agent,
            &derive_signing_key(self.a2a_secret.as_bytes()),
        );
        runtime
            .invoke(server, tool, serde_json::Value::Object(args), &token)
            .await
    }
}

fn extract_mcp_text(result: &serde_json::Value) -> Option<String> {
    result.as_str().map(String::from)
}

fn parse_mcp_json(result: &serde_json::Value) -> Option<serde_json::Value> {
    match result {
        serde_json::Value::String(text) => serde_json::from_str(text).ok(),
        value => Some(value.clone()),
    }
}

impl MediaDataBridge for TuiReplBridge {
    fn gallery_status(&self) -> GalleryStatus {
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match self.invoke_mcp_tool("media", "gallery_status", args).await {
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
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("query".into(), serde_json::Value::String(String::new()));
            args.insert("limit".into(), serde_json::Value::from(limit as u64));
            match self.invoke_mcp_tool("media", "gallery_search", args).await {
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
//
// NOTE: The adapter list and deployment list previously called the
// `training_list_adapters` MCP tool. That tool was deleted in the
// 2026-07-19 simplification (replaced by `AdapterPort::list_adapters`).
// The proper migration is to route through `AdapterRouter` directly via
// the service context's adapter store. For now, these return empty lists
// (graceful degradation) — the TUI shows no adapters until the migration
// is complete.

impl TrainingDataBridge for TuiReplBridge {
    fn adapter_list(&self) -> Vec<AdapterSummary> {
        // TODO: migrate to AdapterPort::list_adapters via AdapterRouter.
        // The training MCP server no longer exposes training_list_adapters.
        Vec::new()
    }

    fn deployment_list(&self) -> Vec<DeploymentSummary> {
        // TODO: migrate to AdapterPort::endpoint_status via AdapterRouter.
        // The training MCP server no longer exposes training_deployment_status.
        Vec::new()
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
        let query = query.to_string();
        // Store the query for last_searched
        *self
            .last_companies_search
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(query.clone());
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("query".into(), serde_json::Value::String(query));
            args.insert("limit".into(), serde_json::Value::from(10_u64));
            match self
                .invoke_mcp_tool("companies", "symbol_search", args)
                .await
            {
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
        self.last_companies_search
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn financials(&self) -> Option<FinancialSummary> {
        let symbol = self
            .last_companies_search
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()?;
        let sym = symbol.clone();
        let sym2 = symbol.clone();
        self.rt_handle.block_on(async {
            // Fetch key metrics (P/E, revenue growth)
            let mut metric_args = serde_json::Map::new();
            metric_args.insert("symbol".into(), serde_json::Value::String(sym.clone()));
            metric_args.insert("limit".into(), serde_json::Value::from(1_u64));
            let metrics = self
                .invoke_mcp_tool("companies", "key_metrics", metric_args)
                .await
                .ok()
                .and_then(|r| parse_mcp_json(&r));

            // Fetch stock quote (price, change)
            let mut quote_args = serde_json::Map::new();
            quote_args.insert("symbol".into(), serde_json::Value::String(sym2.clone()));
            let quote = self
                .invoke_mcp_tool("companies", "stock_quote", quote_args)
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
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match self
                .invoke_mcp_tool("companies", "portfolio_list", args)
                .await
            {
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
        let query = query.to_string();
        *self
            .last_research_search
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(query.clone());
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("query".into(), serde_json::Value::String(query));
            args.insert("count".into(), serde_json::Value::from(10_u64));
            match self.invoke_mcp_tool("research", "web_search", args).await {
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
        let url = url.to_string();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("url".into(), serde_json::Value::String(url.clone()));
            args.insert(
                "format".into(),
                serde_json::Value::String("markdown".into()),
            );
            match self.invoke_mcp_tool("research", "web_extract", args).await {
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
        self.last_research_search
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

// ── DocprocDataBridge (live MCP dispatch to hkask-mcp-docproc) ──

impl DocprocDataBridge for TuiReplBridge {
    fn chunk_list(&self) -> Vec<ChunkInfo> {
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("text".into(), serde_json::Value::String("".into()));
            args.insert("max_tokens".into(), serde_json::Value::from(512_u64));
            match self.invoke_mcp_tool("docproc", "docproc_chunk", args).await {
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
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("question".into(), serde_json::Value::String("".into()));
            args.insert("top_k".into(), serde_json::Value::from(1_u64));
            match self.invoke_mcp_tool("docproc", "docproc_query", args).await {
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
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("action".into(), serde_json::Value::String("list".into()));
            match self
                .invoke_mcp_tool("replica", "replica_registry", args)
                .await
            {
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
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match self.invoke_mcp_tool("skill", "skill_list", args).await {
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
        let skill_id = skill_id.to_string();
        let context = context.to_string();
        self.rt_handle.block_on(async {
            let mut args = serde_json::Map::new();
            args.insert("skill_id".into(), serde_json::Value::String(skill_id));
            args.insert("context".into(), serde_json::Value::String(context));
            match self.invoke_mcp_tool("skill", "skill_execute", args).await {
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

// ── ScenariosDataBridge (MCP dispatch to hkask-mcp-scenarios) ──

impl ScenariosDataBridge for TuiReplBridge {
    fn pipeline_state(&self) -> Option<ScenarioPipelineState> {
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match self
                .invoke_mcp_tool("scenarios", "scenario_status", args)
                .await
            {
                Ok(ref result) => {
                    let content = parse_mcp_json(result)?;
                    let p = content.get("pipeline")?;
                    let forecasts: Vec<ScenarioForecastSummary> = p
                        .get("recent_forecasts")?
                        .as_array()?
                        .iter()
                        .filter_map(|f| {
                            Some(ScenarioForecastSummary {
                                forecast_id: f.get("forecast_id")?.as_str()?.to_string(),
                                event_id: f.get("event_id")?.as_str()?.to_string(),
                                event_name: f.get("event_name")?.as_str()?.to_string(),
                                subject: f.get("subject")?.as_str()?.to_string(),
                                probability: f.get("probability")?.as_f64()?,
                                created_at: f.get("created_at")?.as_str()?.to_string(),
                                outcome: f.get("outcome")?.as_bool(),
                            })
                        })
                        .collect();
                    Some(ScenarioPipelineState {
                        forecast_count: p.get("forecast_count")?.as_u64()? as usize,
                        resolved_count: p.get("resolved_count")?.as_u64()? as usize,
                        pending_count: p.get("pending_count")?.as_u64()? as usize,
                        overall_brier: p.get("overall_brier")?.as_f64(),
                        recent_forecasts: forecasts,
                    })
                }
                Err(_) => None,
            }
        })
    }

    fn calibration(&self) -> Option<CalibrationSummary> {
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match self
                .invoke_mcp_tool("scenarios", "scenario_status", args)
                .await
            {
                Ok(ref result) => {
                    let content = parse_mcp_json(result)?;
                    let c = content.get("calibration")?;
                    if c.is_null() {
                        return None;
                    }
                    Some(CalibrationSummary {
                        total_forecasts: c.get("total_forecasts")?.as_u64()? as usize,
                        resolved_forecasts: c.get("resolved_forecasts")?.as_u64()? as usize,
                        overall_brier: c.get("overall_brier")?.as_f64(),
                        overconfidence_score: c.get("overconfidence_score")?.as_f64(),
                        interpretation: c.get("interpretation")?.as_str()?.to_string(),
                    })
                }
                Err(_) => None,
            }
        })
    }

    fn event_tree(&self) -> Option<EventTreeDetail> {
        self.rt_handle.block_on(async {
            let args = serde_json::Map::new();
            match self
                .invoke_mcp_tool("scenarios", "scenario_status", args)
                .await
            {
                Ok(ref result) => {
                    let content = parse_mcp_json(result)?;
                    let tree = content.get("event_tree")?;
                    if tree.is_null() {
                        return None;
                    }
                    let nodes_json = tree.get("nodes")?.as_array()?;
                    // Build parent→children index
                    let mut children_map: std::collections::HashMap<String, Vec<EventNode>> =
                        std::collections::HashMap::new();
                    let all_nodes: Vec<EventNode> = nodes_json
                        .iter()
                        .filter_map(|n| {
                            let id = n.get("id")?.as_str()?.to_string();
                            let node = EventNode {
                                id: id.clone(),
                                name: n.get("name")?.as_str()?.to_string(),
                                question: n.get("question")?.as_str()?.to_string(),
                                probability: n.get("probability")?.as_f64()?,
                                certainty_tier: n.get("certainty_tier")?.as_str()?.to_string(),
                                basis: n.get("basis")?.as_str().map(|s| s.to_string()),
                                marginal_probability: n.get("marginal_probability")?.as_f64(),
                                parent_ids: n
                                    .get("parent_ids")?
                                    .as_array()
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                children: vec![],
                                sub_question_count: n.get("sub_question_count")?.as_u64()? as usize,
                                has_base_rate: n.get("has_base_rate")?.as_bool()?,
                                brier_score: n.get("brier_score")?.as_f64(),
                            };
                            for pid in &node.parent_ids {
                                children_map
                                    .entry(pid.clone())
                                    .or_default()
                                    .push(node.clone());
                            }
                            Some(node)
                        })
                        .collect();
                    // Build tree from roots
                    let root_ids: Vec<String> = tree
                        .get("root_ids")?
                        .as_array()?
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    fn attach_children(
                        node: &EventNode,
                        children_map: &std::collections::HashMap<String, Vec<EventNode>>,
                    ) -> EventNode {
                        let mut n = node.clone();
                        if let Some(kids) = children_map.get(&node.id) {
                            n.children = kids
                                .iter()
                                .map(|c| attach_children(c, children_map))
                                .collect();
                        }
                        n
                    }
                    let all_map: std::collections::HashMap<String, EventNode> =
                        all_nodes.into_iter().map(|n| (n.id.clone(), n)).collect();
                    let root_nodes: Vec<EventNode> = root_ids
                        .iter()
                        .filter_map(|id| all_map.get(id))
                        .map(|n| attach_children(n, &children_map))
                        .collect();
                    Some(EventTreeDetail {
                        subject: tree.get("subject")?.as_str()?.to_string(),
                        time_horizon: tree.get("time_horizon")?.as_str()?.to_string(),
                        event_count: tree.get("event_count")?.as_u64()? as usize,
                        all_events_probability: tree.get("all_events_probability")?.as_f64()?,
                        root_nodes,
                    })
                }
                Err(_) => None,
            }
        })
    }
}
