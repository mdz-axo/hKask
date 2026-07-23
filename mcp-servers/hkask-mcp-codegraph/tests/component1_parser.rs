//! Component 1: Verify tree-sitter parses hKask's own source code
//! and extracts symbols + edges.
//!
//! Success criteria:
//! - tree-sitter compiles and the Rust grammar is functional
//! - Can parse a real hKask source file
//! - Extracts functions, structs, traits, impls, modules
//! - Edge extraction finds calls and imports

use hkask_mcp_codegraph::codegraph::indexer::{
    extractor::extract_symbols, parser::parse_rust_file,
};
use hkask_mcp_codegraph::codegraph::types::{EdgeKind, SymbolKind};

/// Parse a simple Rust snippet and verify basic extraction.
#[test]
fn test_parse_simple_function() {
    let source = br#"
/// A simple function that adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    let (tree, src) = parse_rust_file(source).expect("parse should succeed");
    let (symbols, _edges) = extract_symbols(&tree, &src, "test.rs");

    // Should find the `add` function
    let add_fn = symbols
        .iter()
        .find(|s| s.name == "add")
        .expect("should find 'add' function");

    assert_eq!(add_fn.kind, SymbolKind::Function);
    assert_eq!(add_fn.file, "test.rs");
    assert!(add_fn.signature.contains("add"));
    assert!(add_fn.signature.contains("i32"));
    // Doc comment
    let doc = add_fn
        .doc_comment
        .as_ref()
        .expect("should have doc comment");
    assert!(doc.contains("A simple function"));
}

/// Parse a struct with fields.
#[test]
fn test_parse_struct() {
    let source = br#"
pub struct Config {
    /// The server name.
    pub name: String,
    port: u16,
}
"#;

    let (tree, src) = parse_rust_file(source).expect("parse should succeed");
    let (symbols, edges) = extract_symbols(&tree, &src, "test.rs");

    let config = symbols
        .iter()
        .find(|s| s.name == "Config")
        .expect("should find Config struct");

    assert_eq!(config.kind, SymbolKind::Struct);

    // Should have References edges for field types
    let ref_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::References)
        .collect();
    assert!(!ref_edges.is_empty(), "should find field type references");
}

/// Parse a trait with methods.
#[test]
fn test_parse_trait() {
    let source = br#"
pub trait Handler {
    fn handle(&self, input: String) -> String;
    fn name(&self) -> &'static str;
}
"#;

    let (tree, src) = parse_rust_file(source).expect("parse should succeed");
    let (symbols, _edges) = extract_symbols(&tree, &src, "test.rs");

    let handler = symbols
        .iter()
        .find(|s| s.name == "Handler")
        .expect("should find Handler trait");

    assert_eq!(handler.kind, SymbolKind::Trait);

    // Methods inside the trait should have qualified names
    let handle_method = symbols
        .iter()
        .find(|s| s.name == "Handler::handle")
        .expect("should find Handler::handle method");

    assert_eq!(handle_method.kind, SymbolKind::Method);
}

/// Parse an impl block with methods.
#[test]
fn test_parse_impl() {
    let source = br#"
struct Service;

impl Service {
    pub fn new() -> Self {
        Service
    }

    fn process(&self, data: &str) -> bool {
        !data.is_empty()
    }
}
"#;

    let (tree, src) = parse_rust_file(source).expect("parse should succeed");
    let (symbols, edges) = extract_symbols(&tree, &src, "test.rs");

    // Should find Service struct
    let service = symbols
        .iter()
        .find(|s| s.name == "Service")
        .expect("should find Service struct");
    assert_eq!(service.kind, SymbolKind::Struct);

    // Should find methods with qualified names
    let new_method = symbols
        .iter()
        .find(|s| s.name == "Service::new")
        .expect("should find Service::new method");
    assert_eq!(new_method.kind, SymbolKind::Method);

    let _process_method = symbols
        .iter()
        .find(|s| s.name == "Service::process")
        .expect("should find Service::process method");

    // Should find call edges (is_empty() in process)
    let call_edges: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();
    assert!(!call_edges.is_empty(), "should find call edges");
}

/// Parse hKask's own code: the MCP runtime (a real, non-trivial file).
#[test]
fn test_parse_hkask_mcp_lib() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/hkask-mcp-server/src/lib.rs");

    let (tree, src) =
        parse_rust_file(&std::fs::read(&path).expect("should read hkask-mcp-server/src/lib.rs"))
            .expect("should parse hkask-mcp-server/src/lib.rs");

    let (symbols, edges) = extract_symbols(&tree, &src, "hkask-mcp/src/lib.rs");

    // Verify we got a reasonable count (this file has many declarations)
    println!(
        "Parsed hkask-mcp/src/lib.rs: {} symbols, {} edges",
        symbols.len(),
        edges.len()
    );

    assert!(
        symbols.len() >= 5,
        "expected at least 5 symbols, got {}",
        symbols.len()
    );

    // Print what we found for inspection
    for sym in &symbols {
        println!("  {:?} {} (line {})", sym.kind, sym.name, sym.start_line);
    }

    // Edges breakdown
    let calls = edges.iter().filter(|e| e.kind == EdgeKind::Calls).count();
    let imports = edges.iter().filter(|e| e.kind == EdgeKind::Imports).count();
    let refs = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::References)
        .count();
    println!("  edges: {calls} calls, {imports} imports, {refs} refs");

    assert!(
        imports > 0,
        "expected at least 1 import edge, got {imports}"
    );
}

/// Parse a module structure and verify qualified names.
#[test]
fn test_qualified_names() {
    let source = br#"
pub mod outer {
    pub mod inner {
        pub fn deep_function() -> u32 {
            42
        }
    }
}
"#;

    let (tree, src) = parse_rust_file(source).expect("parse should succeed");
    let (symbols, _edges) = extract_symbols(&tree, &src, "test.rs");

    // Check module symbols
    let outer = symbols
        .iter()
        .find(|s| s.name == "outer")
        .expect("should find outer module");
    assert_eq!(outer.kind, SymbolKind::Module);

    let _inner = symbols
        .iter()
        .find(|s| s.name == "outer::inner")
        .expect("should find inner module under outer");

    // The function should have a qualified name through module containment
    let deep = symbols
        .iter()
        .find(|s| s.name == "outer::inner::deep_function")
        .expect("should find deep_function with qualified name");
    assert_eq!(deep.kind, SymbolKind::Function);
}

/// Parse a file with use statements and verify import edges.
#[test]
fn test_import_edges() {
    let source = br#"
use std::collections::HashMap;
use crate::types::Symbol;

pub fn process() {
    let _map: HashMap<String, Symbol> = HashMap::new();
}
"#;

    let (tree, src) = parse_rust_file(source).expect("parse should succeed");
    let (_symbols, edges) = extract_symbols(&tree, &src, "test.rs");

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();

    println!("Found {} import edges", imports.len());
    for edge in &imports {
        println!("  import at line {}", edge.line);
    }

    // Should find at least 2 import edges
    assert!(
        imports.len() >= 2,
        "expected at least 2 import edges, got {}",
        imports.len()
    );
}
