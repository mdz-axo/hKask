//! Style corpus discovery command — thin CLI orchestrator
//!
//! Starts the hkask-mcp-research server, then delegates to DiscoveryService
//! which orchestrates multi-provider search (Semantic Scholar, arXiv, Brave,
//! Tavily, Exa, Firecrawl, SerpAPI) via MCP dispatch. In curated mode
//! (default), presents web and YouTube results for user confirmation.
//! Generates a corpus.yaml ready for `kask style embed-corpus`.

use hkask_services::{
    DiscoverRequest, DiscoveredWork, DiscoveryService, default_corpus_config, download_and_cache,
    slugify,
};
use hkask_templates::ports::McpPort;

use std::io::{BufRead, Write};

#[allow(clippy::too_many_arguments)]
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  rt is a valid tokio Runtime; author_name is non-empty; cache_dir is a valid path
/// post: discovers academic and web works for the author; generates corpus.yaml; prints summary and next steps
pub fn run(
    rt: &tokio::runtime::Runtime,
    author_name: String,
    max_works: usize,
    output_dir: Option<String>,
    cache_dir: String,
    serpapi_key: Option<String>,
    include_transcripts: bool,
    include_web: bool,
    curated: bool,
    search_terms: Option<String>,
    include_methods: bool,
    bio: Option<String>,
) {
    eprintln!("=== Discovering corpus for '{}' ===", author_name);

    if include_transcripts && serpapi_key.is_none() {
        eprintln!("Note: No SerpAPI key set. YouTube transcript search will be skipped.");
        eprintln!("      Set HKASK_SERPAPI_API_KEY or pass --serpapi-key to enable.");
    }

    // Build service context and start the research MCP server
    let config = crate::commands::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );
    let ctx = crate::commands::helpers::or_exit(
        rt.block_on(hkask_services::AgentService::build(config)),
        "Failed to build AgentService",
    );
    match rt.block_on(
        ctx.mcp_runtime()
            .start_server("research", "hkask-mcp-research"),
    ) {
        Ok(()) => {
            tracing::info!(target: "hkask.cli", "MCP research server started")
        }
        Err(e) => {
            eprintln!("Failed to start MCP research server: {e}");
            eprintln!("Make sure hkask-mcp-research binary is built and on PATH.");
            std::process::exit(1);
        }
    }

    let mcp = ctx.mcp_dispatcher().clone() as std::sync::Arc<dyn McpPort>;
    let from = super::helpers::resolve_user_webid();
    let to = super::helpers::resolve_user_webid();
    let token = ctx
        .mcp_dispatcher()
        .issue_capability("web_search".to_string(), from, to);

    // ── Augment detection ──────────────────────────────────────────────
    let author_slug = slugify(&author_name);
    let output_dir_resolved = output_dir
        .clone()
        .unwrap_or_else(|| format!("./{}", author_slug));
    let corpus_yaml_path = std::path::PathBuf::from(&output_dir_resolved).join("corpus.yaml");

    let augment = if curated && corpus_yaml_path.exists() {
        eprintln!();
        eprintln!(
            "Found existing corpus for '{}' at {}",
            author_name,
            corpus_yaml_path.display()
        );
        eprintln!(
            "  [A]ugment — merge new works into existing corpus (preserves entities, methods, rules)"
        );
        eprintln!("  [N]ew    — create a fresh corpus (overwrites existing)");
        eprint!("  > ");
        std::io::stderr().flush().ok();

        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).ok();
        matches!(line.trim().to_lowercase().as_str(), "a" | "augment")
    } else {
        false
    };

    let req = DiscoverRequest {
        author_name: author_name.clone(),
        max_works,
        cache_dir: cache_dir.clone(),
        output_dir: output_dir.clone(),
        serpapi_key: serpapi_key.clone(),
        include_transcripts,
        include_web,
        curated,
        web_search_terms: search_terms.clone(),
        augment,
        include_methods,
        biographical_details: bio,
    };

    let result = rt.block_on(DiscoveryService::discover(&req, mcp.as_ref(), &token));

    // Shutdown MCP server
    rt.block_on(ctx.mcp_dispatcher().shutdown_all());

    match result {
        Ok(mut r) => {
            eprintln!();
            eprintln!("Author: {}", author_name);
            eprintln!("Academic works found: {}", r.works_found);
            eprintln!("Sources: {}", r.sources.join(", "));

            let mut all_works: Vec<DiscoveredWork> = Vec::new();

            // ── Curation: web candidates ──────────────────────────────
            if !r.web_candidates.is_empty() {
                eprintln!();
                eprintln!("─── Web Search Candidates ───");
                let selected = curate_candidates(&r.web_candidates);
                for idx in &selected {
                    let work = &r.web_candidates[*idx];
                    eprintln!("  Downloading: {} ...", work.title);
                    let cache_path =
                        std::path::PathBuf::from(&req.cache_dir).join(format!("{}.txt", work.slug));
                    match rt.block_on(download_and_cache(&work.url, &cache_path)) {
                        Ok(()) => {
                            r.works_cached += 1;
                            all_works.push(work.clone());
                            eprintln!("    Cached: {}", cache_path.display());
                        }
                        Err(e) => {
                            eprintln!("    Failed: {e}");
                        }
                    }
                }
            }

            // ── Curation: YouTube candidates ──────────────────────────
            if !r.youtube_candidates.is_empty() {
                eprintln!();
                eprintln!("─── YouTube Transcript Candidates ───");
                let selected = curate_candidates(&r.youtube_candidates);
                for idx in &selected {
                    let work = &r.youtube_candidates[*idx];
                    eprintln!("  Fetching transcript: {} ...", work.title);
                    let cache_path =
                        std::path::PathBuf::from(&req.cache_dir).join(format!("{}.txt", work.slug));
                    match rt.block_on(download_and_cache(&work.url, &cache_path)) {
                        Ok(()) => {
                            r.works_cached += 1;
                            all_works.push(work.clone());
                            eprintln!("    Cached: {}", cache_path.display());
                        }
                        Err(e) => {
                            eprintln!("    Failed: {e}");
                        }
                    }
                }
            }

            // ── Regenerate corpus.yaml with curated selections ────────
            if !all_works.is_empty() {
                eprintln!();
                eprintln!("Regenerating corpus.yaml with curated selections...");
                let output_dir = std::path::PathBuf::from(
                    req.output_dir
                        .clone()
                        .unwrap_or_else(|| format!("./{}", r.author_slug)),
                );
                let config_path = output_dir.join("corpus.yaml");

                // Load the config that DiscoveryService already wrote
                // (which preserves entities/methods/rules when augmenting)
                let existing_yaml = std::fs::read_to_string(&config_path).unwrap_or_default();
                let mut config: hkask_services::CorpusConfig =
                    serde_yaml_neo::from_str(&existing_yaml)
                        .unwrap_or_else(|_| default_corpus_config(&r.author_slug));

                // Dedup: only add curated works whose URLs aren't already in the config
                let existing_urls: std::collections::HashSet<&str> =
                    config.works.iter().map(|w| w.url.as_str()).collect();
                let new_works: Vec<hkask_services::Work> = all_works
                    .iter()
                    .filter(|w| !existing_urls.contains(w.url.as_str()))
                    .map(|w| hkask_services::Work {
                        title: w.title.clone(),
                        slug: w.slug.clone(),
                        url: w.url.clone(),
                        local_path: None,
                        format: match w.work_type.as_str() {
                            "video_transcript" => "text",
                            _ => "web",
                        }
                        .to_string(),
                        document_type: None,
                        dimensions: vec![],
                        section_types: vec![],
                        mds_categories: vec![],
                    })
                    .collect();
                config.works.extend(new_works);

                // Serialize and write back, preserving all metadata
                match serde_yaml_neo::to_string(&config) {
                    Ok(yaml) => {
                        if let Err(e) = std::fs::write(&config_path, &yaml) {
                            eprintln!("Warning: failed to write config: {e}");
                        } else {
                            r.config_path = config_path.to_string_lossy().to_string();
                            eprintln!("Updated config: {}", r.config_path);
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to serialize config: {e}");
                    }
                }
            }

            eprintln!();
            eprintln!("Works cached: {}", r.works_cached);
            eprintln!("Config: {}", r.config_path);
            eprintln!();
            eprintln!("Done. Run the following to build the corpus:");
            eprintln!(
                "  kask style embed-corpus --config {} --db <path> --passphrase <phrase>",
                r.config_path
            );
        }
        Err(e) => {
            eprintln!("Discovery failed: {e}");
            std::process::exit(1);
        }
    }
}

fn curate_candidates(candidates: &[DiscoveredWork]) -> Vec<usize> {
    for (i, work) in candidates.iter().enumerate() {
        eprintln!("  [{i}] {} — {}", work.title, work.url);
    }
    eprintln!();
    eprintln!("  Enter indices to include (e.g., \"0 2 4\"), \"all\", or \"none\":");
    eprint!("  > ");
    std::io::stderr().flush().ok();

    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).ok();

    let input = line.trim().to_lowercase();
    match input.as_str() {
        "all" => (0..candidates.len()).collect(),
        "none" | "" => vec![],
        _ => input
            .split_whitespace()
            .filter_map(|s| s.parse::<usize>().ok())
            .filter(|i| *i < candidates.len())
            .collect(),
    }
}
