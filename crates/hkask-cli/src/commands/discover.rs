//! Style corpus discovery command — thin CLI orchestrator
//!
//! Searches Semantic Scholar and arXiv for an academic author's works,
//! downloads and caches content, and generates a corpus.yaml ready for
//! `kask style embed-corpus`.

use hkask_services::{DiscoverRequest, DiscoveryService};

pub fn run(
    rt: &tokio::runtime::Runtime,
    author_name: String,
    max_works: usize,
    output_dir: Option<String>,
    cache_dir: String,
) {
    eprintln!("=== Discovering corpus for '{}' ===", author_name);

    let req = DiscoverRequest {
        author_name,
        max_works,
        cache_dir,
        output_dir,
    };

    let result = rt.block_on(DiscoveryService::discover(&req));

    match result {
        Ok(r) => {
            eprintln!();
            eprintln!("Author: {} (slug: {})", r.author_slug, r.author_slug);
            eprintln!("Works found: {}", r.works_found);
            eprintln!("Works cached: {}", r.works_cached);
            eprintln!("Sources: {}", r.sources.join(", "));
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
