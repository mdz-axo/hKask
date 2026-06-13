//! Style corpus discovery command — thin CLI orchestrator
//!
//! Searches Semantic Scholar, arXiv, web (SerpAPI), and YouTube transcripts
//! for an academic author's works. In curated mode (default), presents web
//! and YouTube results for user confirmation before including them.
//! Generates a corpus.yaml ready for `kask style embed-corpus`.

use hkask_services::{DiscoverRequest, DiscoveredWork, DiscoveryService, download_and_cache};

use std::io::{BufRead, Write};

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
) {
    eprintln!("=== Discovering corpus for '{}' ===", author_name);

    if include_transcripts && serpapi_key.is_none() {
        eprintln!("Note: No SerpAPI key set. YouTube transcript search will be skipped.");
        eprintln!("      Set HKASK_SERPAPI_API_KEY or pass --serpapi-key to enable.");
    }
    if include_web && serpapi_key.is_none() {
        eprintln!("Note: No SerpAPI key set. Web search will be skipped.");
    }

    let req = DiscoverRequest {
        author_name: author_name.clone(),
        max_works,
        cache_dir,
        output_dir,
        serpapi_key,
        include_transcripts,
        include_web,
        curated,
    };

    let result = rt.block_on(DiscoveryService::discover(&req));

    match result {
        Ok(mut r) => {
            eprintln!();
            eprintln!("Author: {}", author_name);
            eprintln!("Academic works found: {}", r.works_found);
            eprintln!("Sources: {}", r.sources.join(", "));

            // ── Curation: web candidates ──────────────────────────────
            if !r.web_candidates.is_empty() {
                eprintln!();
                eprintln!("─── Web Search Candidates ───");
                let selected = curate_candidates(&r.web_candidates, "web pages", rt);
                // Download and cache selected web candidates
                for idx in &selected {
                    let work = &r.web_candidates[*idx];
                    eprintln!("  Downloading: {} ...", work.title);
                    let cache_path =
                        std::path::PathBuf::from(&req.cache_dir).join(format!("{}.txt", work.slug));
                    match rt.block_on(download_and_cache(&work.url, &cache_path)) {
                        Ok(()) => {
                            r.works_cached += 1;
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
                let selected = curate_candidates(&r.youtube_candidates, "YouTube videos", rt);
                // Download and cache selected YouTube transcripts
                for idx in &selected {
                    let work = &r.youtube_candidates[*idx];
                    eprintln!("  Fetching transcript: {} ...", work.title);
                    let cache_path =
                        std::path::PathBuf::from(&req.cache_dir).join(format!("{}.txt", work.slug));
                    match rt.block_on(download_and_cache(&work.url, &cache_path)) {
                        Ok(()) => {
                            r.works_cached += 1;
                            eprintln!("    Cached: {}", cache_path.display());
                        }
                        Err(e) => {
                            eprintln!("    Failed: {e}");
                        }
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

/// Present candidates to the user and return indices of selected items.
fn curate_candidates(
    candidates: &[DiscoveredWork],
    _label: &str,
    _rt: &tokio::runtime::Runtime,
) -> Vec<usize> {
    for (i, work) in candidates.iter().enumerate() {
        eprintln!("  [{i}] {} — {}", work.title, work.url);
    }
    eprintln!();
    eprintln!("  Enter indices to include (e.g., \"0 2 4\"), \"all\", or \"none\":");
    eprint!("  > ");
    std::io::stderr().flush().ok();

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).ok();

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
