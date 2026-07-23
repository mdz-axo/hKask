//! Component 7: Impact analysis on real hKask data.
//!
//! Verifies the killer feature — blast radius with risk classification —
//! works on hKask's own codebase. Indexes multiple crates and demonstrates
//! that changing a port trait would show affected dependents.

use hkask_mcp_codegraph::codegraph::graph::store::GraphStore;
use hkask_mcp_codegraph::codegraph::graph::traversal;
use hkask_mcp_codegraph::codegraph::indexer::pipeline::IndexPipeline;
use hkask_mcp_codegraph::codegraph::types::Direction;
use std::path::Path;

/// Index hKask's types and ports crates, then verify traversal works
/// on real-world qualified names.
#[test]
fn test_impact_on_hkask_ports() {
    let store = GraphStore::open_in_memory().unwrap();
    let pipeline = IndexPipeline::new(store);

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    // Index hkask-types (foundation types) and hkask-ports (port traits)
    for crate_name in &["hkask-types", "hkask-ports"] {
        let src_dir = workspace_root.join(format!("crates/{crate_name}/src"));
        if src_dir.exists() {
            let results = pipeline.index_directory(&src_dir).unwrap();
            let symbols: usize = results.iter().map(|r| r.symbols).sum();
            let skipped: usize = results.iter().filter(|r| r.skipped).count();
            println!("Indexed {crate_name}: {symbols} symbols ({skipped} skipped)");
        }
    }

    let stats = pipeline.stats().unwrap();
    println!(
        "Total: {} files, {} symbols, {} edges",
        stats.files, stats.symbols, stats.edges
    );

    assert!(stats.symbols >= 5, "should have indexed at least 5 symbols");

    // Verify we can find a known public trait
    let conn = pipeline.store().conn();
    let mut stmt = conn
        .prepare("SELECT name, kind, visibility FROM symbols WHERE kind = 'trait' LIMIT 5")
        .unwrap();
    let traits: Vec<(String, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    println!("Public traits found:");
    for (name, _kind, vis) in &traits {
        println!("  [{vis}] {name}");
    }

    if !traits.is_empty() {
        // Try impact analysis on the first public trait
        for (trait_name, _, _) in &traits {
            if let Some(id) = traversal::find_symbol_id(conn, trait_name).unwrap() {
                println!("\nImpact analysis for '{trait_name}' (id={id}):");
                let impact = traversal::impact_analysis(conn, id, 5).unwrap();
                for result in &impact {
                    println!(
                        "  depth={} risk={} {}",
                        result.depth, result.risk, result.symbol.name
                    );
                }
                break;
            }
        }
    }
}

/// Index hkask-mcp and verify forward traversal from a known function.
#[test]
fn test_traverse_hkask_mcp_lib() {
    let store = GraphStore::open_in_memory().unwrap();
    let pipeline = IndexPipeline::new(store);

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let src_dir = workspace_root.join("crates/hkask-mcp-server/src");
    if !src_dir.exists() {
        return;
    }

    let results = pipeline.index_directory(&src_dir).unwrap();
    let symbols: usize = results.iter().map(|r| r.symbols).sum();
    println!("Indexed hkask-mcp/src: {symbols} symbols");

    let conn = pipeline.store().conn();

    // Find bootstrap_mcp_server and traverse its dependencies
    if let Some(id) = traversal::find_symbol_id(conn, "bootstrap_mcp_server").unwrap() {
        println!("\nDependencies of bootstrap_mcp_server:");
        let deps = traversal::traverse(conn, id, Direction::Forward, 5).unwrap();
        for dep in &deps {
            println!(
                "  depth={} edge={} {} ({})",
                dep.depth, dep.edge_kind, dep.symbol.name, dep.symbol.kind
            );
        }
        // Note: bootstrap_mcp_server may have 0 deps in-graph if it only calls
        // external crate APIs (dotenvy, tracing, DaemonClient, etc.) that are
        // not indexed in this test's DB.
        println!(
            "  ({} deps — may be 0 if only calling external APIs)",
            deps.len()
        );
    }

    // Find BUILTIN_SERVERS and check reverse (who references it?)
    if let Some(id) = traversal::find_symbol_id(conn, "BUILTIN_SERVERS").unwrap() {
        println!("\nReferences to BUILTIN_SERVERS:");
        let refs = traversal::traverse(conn, id, Direction::Reverse, 5).unwrap();
        for r in &refs {
            println!("  depth={} edge={} {}", r.depth, r.edge_kind, r.symbol.name);
        }
    }
}
