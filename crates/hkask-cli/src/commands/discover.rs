//! Style corpus discovery command — thin CLI orchestrator
//!
//! Searches Semantic Scholar, arXiv, web (SerpAPI), and YouTube transcripts
//! for an academic author's works. In curated mode (default), presents web
//! and YouTube results for user confirmation before including them.
//! Generates a corpus.yaml ready for `kask style embed-corpus`.

use hkask_services::{
    DiscoverRequest, DiscoveredWork, DiscoveryService, download_and_cache, generate_corpus_yaml,
};

use std::io::{BufRead, Write};

#[allow(clippy::too_many_arguments)]
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

            // Collect all works that will go into the corpus
            let mut all_works: Vec<DiscoveredWork> = Vec::new();
            // Academic works are already cached by DiscoveryService
            // We need to reconstruct the list from the config
            // (they're in the YAML but not returned separately)

            // ── Curation: web candidates ──────────────────────────────
            if !r.web_candidates.is_empty() {
                eprintln!();
                eprintln!("─── Web Search Candidates ───");
                let selected = curate_candidates(&r.web_candidates, "web pages", rt);
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
                let selected = curate_candidates(&r.youtube_candidates, "YouTube videos", rt);
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
                // Load existing academic works from the generated config
                // and add curated selections
                let existing_config: hkask_services::CorpusConfig = serde_yaml::from_str(
                    &std::fs::read_to_string(&r.config_path).unwrap_or_default(),
                )
                .unwrap_or_else(|_| {
                    // Fallback: empty config
                    hkask_services::CorpusConfig {
                        author: r.author_slug.clone(),
                        embedding: hkask_services::EmbeddingConfig {
                            model: String::new(),
                            dim: 1024,
                            batch_size: 64,
                        },
                        works: vec![],
                        foundational_rules: vec![],
                        chunking: hkask_services::ChunkingConfig {
                            min_words: 50,
                            max_words: 200,
                            sentence_boundary: ".!? ".to_string(),
                        },
                        centroid_entity_ref: String::new(),
                        validation: hkask_services::ValidationConfig {
                            centroid_distance_max: 0.25,
                            exemplar_count_min: 3,
                            exemplar_count_max: 7,
                        },
                        budget: hkask_memory::salience::BudgetConfig::PerPage {
                            per_100_pages: 3750,
                        },
                        entities: Default::default(),
                        methods: vec![],
                    }
                });

                // Combine academic works from config with curated selections
                let mut combined: Vec<DiscoveredWork> = existing_config
                    .works
                    .iter()
                    .map(|w| DiscoveredWork {
                        title: w.title.clone(),
                        slug: w.slug.clone(),
                        url: w.url.clone(),
                        year: None,
                        source: "academic".to_string(),
                        work_type: "paper".to_string(),
                    })
                    .collect();
                combined.extend(all_works);

                match generate_corpus_yaml(&r.author_slug, &combined, &output_dir) {
                    Ok(new_path) => {
                        r.config_path = new_path.to_string_lossy().to_string();
                        eprintln!("Updated config: {}", r.config_path);
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to regenerate config: {e}");
                        eprintln!("Using original config: {}", r.config_path);
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
