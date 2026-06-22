//! QA commands — fuzz triage with LLM classifier (Gemma 4).
//!
//! Reads bolero output from stdin, classifies each failure via
//! `classify_batch`, routes by confidence, emits CNS spans.
//!
//! Autonomous interactive scripts: `kask qa run --script <manifest.yaml>`
//! executes a YAML-defined QA pipeline with classifier-driven branching.

use crate::cli::QaAction;
use hkask_mcp::runtime::McpRuntime;
use hkask_services_classify::{self, ClassifierConfig};
use hkask_test_harness::qa_script::{ClassifyResult, QaScriptRunner};
use hkask_test_harness::triage::{self, BoleroFailure, QaDiagnosis, TriageReport};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

pub fn run(rt: &tokio::runtime::Runtime, action: QaAction) {
    match action {
        QaAction::Triage { input } => {
            if let Err(e) = rt.block_on(triage(input)) {
                eprintln!("QA triage error: {e}");
                std::process::exit(1);
            }
        }
        QaAction::SuggestFuzz { input } => {
            if let Err(e) = rt.block_on(suggest_fuzz(input)) {
                eprintln!("QA suggest-fuzz error: {e}");
                std::process::exit(1);
            }
        }
        QaAction::RunScript { script } => {
            if let Err(e) = run_script(script) {
                eprintln!("QA script error: {e}");
                std::process::exit(1);
            }
        }
    }
}

async fn triage(input_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let stdin: Box<dyn BufRead> = match &input_path {
        Some(path) => {
            let file = File::open(path).map_err(|e| format!("Cannot open {path:?}: {e}"))?;
            Box::new(BufReader::new(file))
        }
        None => Box::new(BufReader::new(io::stdin())),
    };

    let failures = hkask_test_harness::triage::parse_bolero_stdin(stdin)?;

    if failures.is_empty() {
        println!("[QA] No bolero failures detected.");
        return Ok(());
    }

    println!("[QA] {} bolero failure(s) detected", failures.len());

    // Try to load classifier config
    let registry_dir = find_registry_dir();
    let config = match hkask_services_classify::load_classifier_config("qa-triage", &registry_dir) {
        Ok(def) => {
            println!("[QA] Classifier loaded: {} via {}", def.model, def.provider);
            Some(ClassifierConfig::from_def(&def))
        }
        Err(e) => {
            eprintln!(
                "[QA] Classifier config not found at {}/classify/qa-triage.yaml: {e}",
                registry_dir.display()
            );
            eprintln!("[QA] Falling back to parse-only mode — no LLM triage.");
            None
        }
    };

    if let Some(cfg) = config {
        if cfg.api_key.is_empty() {
            println!(
                "[QA] No DEEPINFRA_API_KEY set — falling back to parse-only mode.\n\
                 [QA] Set DEEPINFRA_API_KEY for LLM-powered triage."
            );
            print_failures(&failures);
            emit_cns_spans(&failures);
            return Ok(());
        }

        // Classify each failure
        println!("[QA] Classifying {} failure(s) with LLM...", failures.len());
        let passages: Vec<String> = failures.iter().map(|f| f.to_passage()).collect();
        let results = match hkask_services_classify::classify_batch(&passages, cfg, None).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[QA] Classifier API error: {e}");
                eprintln!("[QA] Is DEEPINFRA_API_KEY valid? Is network available?");
                print_failures(&failures);
                emit_cns_spans(&failures);
                return Ok(());
            }
        };

        let mut report = TriageReport::default();

        for (i, result) in results.iter().enumerate() {
            let failure = &failures[i];
            let diagnosis = parse_diagnosis(&result.category);

            match diagnosis {
                Ok(diag) => {
                    triage::emit_cns_span(failure, &diag);

                    if diag.is_flake {
                        println!(
                            "  [{i}] FLAKE: {}::{} (skipped)",
                            failure.crate_name, failure.test_name
                        );
                        report.flakes += 1;
                    } else if diag.confidence >= 0.95 {
                        println!(
                            "  [{i}] HIGH confidence ({:.2}): {}::{} — {}\n       auto-repair suggested: {}",
                            diag.confidence,
                            failure.crate_name,
                            failure.test_name,
                            diag.root_cause,
                            if diag.proposed_fix.is_empty() {
                                "none"
                            } else {
                                "yes"
                            }
                        );
                        report.auto_repaired += 1;
                    } else if diag.confidence >= 0.70 {
                        println!(
                            "  [{i}] MEDIUM confidence ({:.2}): {}::{} — {}",
                            diag.confidence, failure.crate_name, failure.test_name, diag.root_cause
                        );
                        report.issues_opened += 1;
                    } else {
                        println!(
                            "  [{i}] LOW confidence ({:.2}): {}::{} — {}\n       investigation needed",
                            diag.confidence, failure.crate_name, failure.test_name, diag.root_cause
                        );
                        report.issues_opened += 1;
                    }

                    if !diag.suggested_fuzz_target.is_empty() {
                        println!(
                            "       suggested fuzz target: {}",
                            diag.suggested_fuzz_target
                        );
                    }
                }
                Err(_) => {
                    println!(
                        "  [{i}] UNPARSEABLE: {}::{} — raw: {}",
                        failure.crate_name,
                        failure.test_name,
                        &result.category.chars().take(80).collect::<String>()
                    );
                    report.unparseable += 1;
                }
            }
        }

        println!(
            "\n[QA] Triage complete: {} auto-repairs, {} issues, {} flakes, {} unparseable",
            report.auto_repaired, report.issues_opened, report.flakes, report.unparseable
        );
    } else {
        print_failures(&failures);
        emit_cns_spans(&failures);
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn find_registry_dir() -> PathBuf {
    // Try in order: env var, CWD-relative, XDG config
    if let Ok(dir) = std::env::var("HKASK_REGISTRY_DIR") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return p;
        }
    }
    let cwd_registry = Path::new("registry");
    if cwd_registry.is_dir() {
        return cwd_registry.to_path_buf();
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("hkask").join("registry")
}

fn parse_diagnosis(raw: &str) -> Result<QaDiagnosis, serde_json::Error> {
    // Strip markdown code fences if present
    let json = raw
        .trim()
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(raw);
    serde_json::from_str(json)
}

fn emit_cns_spans(failures: &[BoleroFailure]) {
    for f in failures {
        tracing::info!(
            target: "cns.qa.bolero_failure",
            crate_name = %f.crate_name,
            test_name = %f.test_name,
            "Bolero fuzz failure detected (parse-only mode)"
        );
    }
}

fn print_failures(failures: &[BoleroFailure]) {
    println!();
    for (i, f) in failures.iter().enumerate() {
        println!(
            "  #{i}: {crate}::{test}\n      panic: {panic}\n      input: {input}\n",
            i = i + 1,
            crate = f.crate_name,
            test = f.test_name,
            panic = f.panic_message,
            input = f.failing_input,
        );
    }
}

// ── suggest-fuzz ────────────────────────────────────────────────────────────

async fn suggest_fuzz(input_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let stdin: Box<dyn BufRead> = match &input_path {
        Some(path) => {
            let file = File::open(path).map_err(|e| format!("Cannot open {path:?}: {e}"))?;
            Box::new(BufReader::new(file))
        }
        None => Box::new(BufReader::new(io::stdin())),
    };

    // Parse surviving mutant lines
    let mutants: Vec<hkask_test_harness::feedback::SurvivingMutant> = stdin
        .lines()
        .map_while(|l| l.ok())
        .filter_map(|line| hkask_test_harness::feedback::parse_mutant_line(&line))
        .collect();

    if mutants.is_empty() {
        println!("[QA] No surviving mutants found in input.");
        return Ok(());
    }

    println!("[QA] {} surviving mutant(s) found", mutants.len());

    // Load classifier config
    let registry_dir = find_registry_dir();
    let config = match hkask_services_classify::load_classifier_config("qa-feedback", &registry_dir)
    {
        Ok(def) => {
            println!(
                "[QA] Feedback classifier loaded: {} via {}",
                def.model, def.provider
            );
            Some(ClassifierConfig::from_def(&def))
        }
        Err(e) => {
            eprintln!("[QA] Feedback config not found: {e}");
            print_mutant_summary(&mutants);
            return Ok(());
        }
    };

    let Some(cfg) = config else { return Ok(()) };

    if cfg.api_key.is_empty() {
        println!("[QA] No DEEPINFRA_API_KEY set — printing mutant summary instead.");
        print_mutant_summary(&mutants);
        return Ok(());
    }

    // Format passages and classify
    println!("[QA] Requesting fuzz target suggestions from LLM...");
    let passages: Vec<String> = mutants
        .iter()
        .map(|m| {
            hkask_test_harness::feedback::mutant_passage(
                &m.crate_name,
                &m.file,
                m.line,
                &m.original,
                &m.mutated,
            )
        })
        .collect();

    let results = hkask_services_classify::classify_batch(&passages, cfg, None).await?;

    println!("\n[QA] Fuzz target suggestions:\n");
    for (i, result) in results.iter().enumerate() {
        let m = &mutants[i];
        println!(
            "  {crate}::{file}:{line} ({original} → {mutated})\n    → {suggestion}\n",
            crate = m.crate_name,
            file = m.file,
            line = m.line,
            original = m.original,
            mutated = m.mutated,
            suggestion = result.category.trim(),
        );
    }

    Ok(())
}

fn print_mutant_summary(mutants: &[hkask_test_harness::feedback::SurvivingMutant]) {
    println!("\n[QA] Surviving mutants:\n");
    for m in mutants {
        println!(
            "  {crate}::{file}:{line} ({original} → {mutated})",
            crate = m.crate_name,
            file = m.file,
            line = m.line,
            original = m.original,
            mutated = m.mutated,
        );
    }
    println!("\n[QA] Set DEEPINFRA_API_KEY for LLM-powered fuzz target suggestions.");
}

// ── Autonomous script runner ───────────────────────────────────────────────────

fn run_script(script_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("[QA] Loading script: {}", script_path.display());

    let registry_dir = find_registry_dir();

    // Build classify closure. Runs on a dedicated OS thread to avoid
    // Tokio's nested-block_on restriction (the CLI main runs inside a
    // Tokio runtime; block_on cannot be called from within it).
    let classify = move |config_name: &str, passages: &[String]| {
        let rd = registry_dir.clone();
        let cfg_name = config_name.to_string();
        let passages_owned: Vec<String> = passages.to_vec();

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("Classify runtime: {}", e))?
                .block_on(async move {
                    let config = hkask_services_classify::load_classifier_config(&cfg_name, &rd)
                        .map_err(|e| format!("Failed to load classifier '{}': {}", cfg_name, e))?;

                    let cfg = ClassifierConfig::from_def(&config);

                    let results =
                        hkask_services_classify::classify_batch(&passages_owned, cfg, None)
                            .await
                            .map_err(|e| format!("Classify API error: {}", e))?;

                    Ok(results
                        .into_iter()
                        .map(|r| ClassifyResult {
                            category: r.category,
                            prompt_tokens: r.prompt_tokens,
                            completion_tokens: r.completion_tokens,
                            cost_urj: r.cost_urj,
                            failed: r.failed,
                            provider: r.provider,
                        })
                        .collect::<Vec<_>>())
                })
        })
        .join()
        .map_err(|_| "Classify thread panicked".to_string())?
    };

    let runner = {
        let content = std::fs::read_to_string(&script_path)
            .map_err(|e| format!("Cannot read {}: {}", script_path.display(), e))?;
        let manifest: hkask_test_harness::qa_script::QaScriptManifest =
            serde_yaml_neo::from_str(&content)
                .map_err(|e| format!("Failed to parse {}: {}", script_path.display(), e))?;

        // Build tool invocation closure backed by McpRuntime.
        // All 12 MCP servers are registered as child processes. The closure
        // resolves tool_name → server_id → live Peer connection and dispatches
        // through the MCP protocol (JSON-RPC via rmcp).
        //
        // Pre-condition: server binaries must be built (cargo build --bin ...).
        // Failed server startups are non-fatal — the closure returns an error
        // for tools on unstarted servers, and the QA script handles the failure branch.
        //
        // A persistent Tokio runtime keeps background tasks alive for the
        // lifetime of the script execution. Server connections are established
        // sequentially (no parallelism needed — rmcp handshakes are fast).
        let server_rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create server runtime: {}", e))?;
        let mcp_runtime = McpRuntime::new();

        // Server registry: server_id → binary name (relative to target/debug/)
        let servers: HashMap<&str, &str> = HashMap::from([
            ("communication", "hkask-mcp-communication"),
            ("companies", "hkask-mcp-companies"),
            ("condenser", "hkask-mcp-condenser"),
            ("docproc", "hkask-mcp-docproc"),
            ("kanban", "hkask-mcp-kanban"),
            ("media", "hkask-mcp-media"),
            ("memory", "hkask-mcp-memory"),
            ("replica", "hkask-mcp-replica"),
            ("research", "hkask-mcp-research"),
            ("skill", "hkask-mcp-skill"),
            ("spec", "hkask-mcp-spec"),
            ("training", "hkask-mcp-training"),
        ]);

        // Start all servers on the persistent runtime.
        // Sequential startup is fine — rmcp handshake takes <1s per server.
        let mut started = 0usize;
        let mut failed = 0usize;
        server_rt.block_on(async {
            for (&server_id, &binary_name) in &servers {
                let bin_path = format!("./target/debug/{}", binary_name);
                match mcp_runtime.start_server(server_id, &bin_path).await {
                    Ok(()) => {
                        started += 1;
                        let tools = mcp_runtime.discover_tools().await;
                        tracing::info!(
                            target: "cns.qa.mcp",
                            server = %server_id,
                            tools = tools.len(),
                            "MCP server started for QA runner"
                        );
                    }
                    Err(e) => {
                        failed += 1;
                        tracing::warn!(
                            target: "cns.qa.mcp",
                            server = %server_id,
                            error = %e,
                            "MCP server failed to start — tools on this server will be unavailable"
                        );
                    }
                }
            }
        });
        println!(
            "[QA] MCP servers: {} started, {} failed (of {} total)",
            started,
            failed,
            servers.len()
        );

        // Verify dispatch works in-situ on server_rt
        server_rt.block_on(async {
            let ping_tools = ["skill_ping", "kanban_board_list", "condenser_ping"];
            for tool in &ping_tools {
                if let Some(info) = mcp_runtime.get_tool_info(tool).await {
                    let result = mcp_runtime
                        .call_tool(&info.server_id, tool, serde_json::Map::new())
                        .await;
                    match result {
                        Ok(r) => {
                            let text: String = r
                                .content
                                .iter()
                                .filter_map(|c| match &**c {
                                    rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            println!(
                                "[QA] Dispatch verify: {} → OK ({})",
                                tool,
                                &text[..text.len().min(80)]
                            );
                            break;
                        }
                        Err(e) => {
                            println!("[QA] Dispatch verify: {} → ERR: {}", tool, e);
                        }
                    }
                }
            }
        });

        let server_rt = std::sync::Arc::new(server_rt);

        let tool_invoke = move |tool_name: &str, params: &str| -> Result<String, String> {
            let tool = tool_name.to_string();
            let p = params.to_string();
            let rt = mcp_runtime.clone();
            let srt = server_rt.clone();
            srt.block_on(async move {
                // Resolve tool → server
                let tool_info = rt.get_tool_info(&tool).await.ok_or_else(|| {
                    format!("Tool '{}' not found in any registered MCP server", tool)
                })?;

                // Parse params as JSON object
                let params_val: serde_json::Value = serde_json::from_str(&p)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                let args = match params_val {
                    serde_json::Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };

                // Dispatch through MCP protocol
                let result = rt
                    .call_tool(&tool_info.server_id, &tool, args)
                    .await
                    .map_err(|e| format!("MCP call '{}' failed: {}", tool, e))?;

                // Extract text content from CallToolResult
                let text: String = result
                    .content
                    .iter()
                    .filter_map(|c| match &**c {
                        rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let output = serde_json::json!({
                    "outcome": "success",
                    "text": text,
                    "server": tool_info.server_id,
                    "tool": tool
                });
                Ok(output.to_string())
            })
        };

        let ledger_path = cost_ledger_path();
        let runner = QaScriptRunner::new(manifest, Box::new(classify))
            .with_tool_invoke(Box::new(tool_invoke));
        if let Some(p) = ledger_path {
            runner.with_ledger_path(p)
        } else {
            runner
        }
    };

    println!(
        "[QA] Running script '{}' — {} steps",
        runner.manifest().id,
        runner.step_count()
    );
    println!("[QA] ──────────────────────────────────────────────");

    let report = runner.run()?;

    println!("[QA] ──────────────────────────────────────────────");
    println!(
        "[QA] Script complete: {} steps executed, QA outcome: {}",
        report.total_steps, report.qa_outcome
    );

    for step in &report.steps_executed {
        let classify_info = match &step.classify_category {
            Some(cat) if cat.len() > 80 => format!(" | category: {}…", &cat[..80]),
            Some(cat) => format!(" | category: {cat}"),
            None => String::new(),
        };
        let cost_total = step.cost.gas_urj + step.cost.api_token_urj + step.cost.failed_api_urj;
        let mut cost_parts = vec![format!("{} gas", step.cost.gas_urj)];
        if step.cost.api_token_urj > 0 {
            cost_parts.push(format!("{} API", step.cost.api_token_urj));
        }
        if step.cost.failed_api_urj > 0 {
            cost_parts.push(format!("{} failed", step.cost.failed_api_urj));
        }
        let cost_str = if cost_total > 0 {
            format!(" | {} µrJ ({})", cost_total, cost_parts.join(" + "))
        } else {
            String::new()
        };
        println!(
            "  [{ordinal}] {action} → {outcome} ({qa}) ({duration_ms}ms{retry_info}){classify_info}{cost_str}",
            ordinal = step.ordinal,
            action = step.action,
            outcome = step.outcome,
            qa = step.qa_outcome,
            duration_ms = step.duration_ms,
            retry_info = if step.retries > 0 {
                format!(", {} retries", step.retries)
            } else {
                String::new()
            },
        );
    }

    if report.exceeded_gas {
        println!("[QA] ⚠ rJoule budget cap exceeded");
    }

    // Cost summary
    let c = &report.cost;
    let gas_rj = c.gas_urj as f64 / 1_000_000.0;
    let api_rj = c.api_token_urj as f64 / 1_000_000.0;
    let total_rj = c.total_urj as f64 / 1_000_000.0;
    let _cap_rj = c.cap_urj as f64 / 1_000_000.0;
    let pct = if c.cap_urj > 0 {
        (c.total_urj as f64 / c.cap_urj as f64) * 100.0
    } else {
        0.0
    };
    println!("[QA] Cost summary:");
    println!(
        "       Gas (software):     {} gas              {} µrJ    ({:.6} rJ)",
        c.gas_used, c.gas_urj, gas_rj
    );
    println!(
        "       API tokens:         {} calls, {} µrJ    ({:.6} rJ)",
        c.classify_calls, c.api_token_urj, api_rj
    );
    if c.failed_api_cost_urj > 0 {
        let failed_rj = c.failed_api_cost_urj as f64 / 1_000_000.0;
        println!(
            "       API failed calls:                     {} µrJ    ({:.6} rJ)",
            c.failed_api_cost_urj, failed_rj
        );
    }
    println!("       ───────────────────────────────────────────────────");
    println!(
        r"       Run total:                              {} µrJ    ({:.6} rJ, ${:.6})",
        c.total_urj, total_rj, total_rj
    );
    if c.monthly_subscriptions_urj > 0 {
        let sub_rj = c.monthly_subscriptions_urj as f64 / 1_000_000.0;
        println!(
            r"       Monthly recurring: ${:.2} = {} µrJ (not in run total)",
            sub_rj, c.monthly_subscriptions_urj
        );
    }
    println!(
        "[QA] Budget: {} / {} µrJ ({:.1}%)",
        c.total_urj, c.cap_urj, pct
    );
    if c.ledger_committed {
        println!("[QA] Ledger: costs committed to ledger-cost.db");
    }

    Ok(())
}

/// Open the cost ledger at the default path. Returns None if the config
/// directory cannot be determined.
fn cost_ledger_path() -> Option<std::path::PathBuf> {
    let config_dir = dirs::config_dir()?;
    let ledger_dir = config_dir.join("hkask");
    let _ = std::fs::create_dir_all(&ledger_dir);
    Some(ledger_dir.join("ledger-cost.db"))
}
