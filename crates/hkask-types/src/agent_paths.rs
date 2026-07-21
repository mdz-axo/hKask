//! Agent directory layout conventions.
//!
//! Standard on-disk organization for agent artifacts. Each agent gets a
//! self-contained directory under `agents/{name}/` that holds all their
//! databases, sessions, media, documents, research, and financial data.
//!
//! ```text
//! agents/
//!   {agent_name}/
//!     # ── Core databases ───────────────────────────────────────────
//!     pod.db            Per-pod SQLCipher database (HMemStore, EmbeddingStore, CNS)
//!     pod.db.salt       SQLCipher salt file
//!     pod.webid         WebID sidecar for passphrase derivation bootstrap
//!     pod.kind          PodKind classifier ("curator", "team", "replicant")
//!     pod.name          Original unsanitized agent name
//!     memory.db         Memory MCP server database (episodic + semantic tool storage)
//!     memory.db.salt
//!     style.db          Corpus embeddings and centroids for style composition
//!     style.db.salt
//!     kanban.db         Kanban board — tasks, unjam items, board state
//!     kanban.db.salt
//!     training.db       LoRA adapter training jobs — model, dataset, status, host
//!     training.db.salt
//!     wallet.db         Per-agent wallet — rJoule balances, API keys, encumbrances
//!     wallet.db.salt
//!
//!     # ── File directories (no DB, just filesystem) ───────────────
//!     gallery/          Media server assets — images, video, audio produced or consumed
//!     documents/        Docproc parsed/extracted documents and metadata
//!     library/          Research materials — downloaded papers, RSS feeds, search cache
//!     sessions/         MCP session transcripts — one file per session, timestamped
//!     threads/          Chat thread metadata — one JSON file per thread, _active marker
//!     adapters/         LoRA adapter weight files (.safetensors, adapter_config.json)
//!     portfolios/       Financial portfolio/watchlist data (FMP, EODHD responses)
//!     artifacts/        Agent-specific artifacts — styles, bots, templates, bundles
//!     agent.yaml        Self-contained agent definition (identity, charter, capabilities)
//! ```rust,no_run
//!
//! # Wiring Plan
//!
//! | Resource     | DB or Dir      | Wired To                               | Status |
//! |-------------|---------------|----------------------------------------|--------|
//! | pod.db      | SQLCipher DB   | PodFactory, PodContext, MemoryLoopForwarder | ✅ done |
//! | memory.db   | SQLCipher DB   | hkask-mcp-memory, REPL init, consolidation | ✅ done |
//! | style.db    | SQLCipher DB   | hkask-services-embed (corpus embeddings) | 🔧 path exists, not wired |
//! | kanban.db   | SQLCipher DB   | hkask-mcp-kata-kanban, hkask-services-kanban | 🔧 path exists, not wired |
//! | training.db | SQLCipher DB   | hkask-mcp-training (JobStore)           | 🔧 path exists, not wired |
//! | wallet.db   | SQLCipher DB   | hkask-wallet (per-agent balances/keys)  | 🔧 path exists, not wired |
//! | gallery/    | directory      | hkask-mcp-media (image/video assets)    | 🔧 dir exists, not wired |
//! | documents/  | directory      | hkask-mcp-docproc (parsed documents)    | 🔧 dir exists, not wired |
//! | library/    | directory      | hkask-mcp-research (papers, feeds)      | 🔧 dir exists, not wired |
//! | sessions/   | directory      | DaemonHandler record_experience         | 🔧 dir exists, not wired |
//! | threads/    | directory      | REPL thread registry (ChatThread JSON)  | ✅ wired |
//! | adapters/   | directory      | hkask-mcp-training, hkask-adapter       | 🔧 dir exists, not wired |
//! | portfolios/ | directory      | hkask-services-wallet (financial data)  | 🔧 dir exists, not wired |
//! | artifacts/  | directory      | BundleService, StyleService, BotService | 🔧 dir exists, not wired |
//! | agent.yaml  | YAML file      | onboarding (register_userpod) + ensure_agent_dirs fallback | ✅ created |
//!
//! # Not agent-scoped (system-level, correct as-is)
//!
//! | Resource         | Stored In      | Rationale |
//! |-----------------|---------------|-----------|
//! | Spec store       | data/hkask.db  | MDS specifications are global contracts — all agents read the same specs |
//! | User store       | data/hkask.db  | Human users and replicant identities are system-wide |
//! | Consent store    | data/hkask.db  | Sovereignty consent grants are per-human-user, not per-agent |
//! | Escalation queue | data/hkask.db  | Curator escalations are cross-agent governance events |
//! | Goal repo        | data/hkask.db  | Goals span multiple agents and pods |
//! | Cost ledger      | ~/.config/hkask | System-wide inference cost accounting |
//!
//! Each "wired" resource means the MCP server or service uses the agent's
//! dedicated path rather than a shared global database or in-memory store.
//! "path exists" means the directory is created at onboarding but no code
//! reads or writes to it yet — the path is available for wiring.
//!
//! The agent name is the replicant/curator/team name (e.g., "curator",
//! "Jacques (Zuck)", "alice"). All paths are relative to the hKask data
//! directory (default: `.`).

use std::path::PathBuf;

/// Root directory for agent artifacts.
pub const AGENTS_DIR: &str = "userpods";

/// Resolve a relative agent path against the hKask data directory.
///
/// Checks `HKASK_DATA_DIR` env var, falls back to CWD. This ensures
/// agent databases end up in a predictable location regardless of where
/// the MCP server process is spawned from.
#[must_use]
pub fn resolve_under_data_dir(relative: &std::path::Path) -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("HKASK_DATA_DIR") {
        return std::path::PathBuf::from(dir).join(relative);
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return std::path::PathBuf::from(xdg).join("hkask").join(relative);
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("hkask")
            .join(relative);
    }
    relative.to_path_buf()
}

/// Get the directory for a specific agent.
pub fn agent_dir(agent_name: &str) -> PathBuf {
    PathBuf::from(AGENTS_DIR).join(sanitize_name(agent_name))
}

// ── Database paths ───────────────────────────────────────────────────────────

/// Pod database — HMemStore, EmbeddingStore, CNS events.
pub fn agent_pod_db(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("pod.db")
}

/// Memory database — episodic + semantic tool storage via hkask-mcp-memory.
pub fn agent_memory_db(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("memory.db")
}

/// Style database — corpus embeddings and centroids for style composition.
pub fn agent_style_db(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("style.db")
}

/// Kanban database — tasks, unjam items, board state for the agent.
pub fn agent_kanban_db(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("kanban.db")
}

/// Training database — LoRA adapter training jobs (model, dataset, status).
pub fn agent_training_db(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("training.db")
}

/// Wallet database — per-agent rJoule balances, API keys, encumbrances.
pub fn agent_wallet_db(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("wallet.db")
}

// ── Directory paths ──────────────────────────────────────────────────────────

/// Gallery directory — media server assets (images, video, audio).
pub fn agent_gallery_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("gallery")
}

/// Documents directory — docproc parsed/extracted documents.
pub fn agent_documents_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("documents")
}

/// Library directory — research materials, downloaded papers, RSS feeds.
pub fn agent_library_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("library")
}

/// Sessions directory — MCP session transcripts.
pub fn agent_sessions_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("sessions")
}

/// Adapters directory — LoRA adapter weight files.
pub fn agent_adapters_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("adapters")
}

/// Portfolios directory — financial portfolio/watchlist data.
pub fn agent_portfolios_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("portfolios")
}

/// Artifacts directory — agent-specific styles, bots, templates, bundles.
pub fn agent_artifacts_dir(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("artifacts")
}

/// Agent definition file — self-contained YAML (identity, charter, capabilities).
pub fn agent_definition_yaml(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("agent.yaml")
}

/// Artifact manifest — per-agent index of published artifacts.
pub fn agent_manifest_json(agent_name: &str) -> PathBuf {
    agent_dir(agent_name).join("manifest.json")
}

// ── Initialization ───────────────────────────────────────────────────────────

/// All subdirectories created by `ensure_agent_dirs`.
const AGENT_SUBDIRS: &[&str] = &[
    "gallery",
    "documents",
    "library",
    "sessions",
    "adapters",
    "portfolios",
    "artifacts",
];

/// Create the full agent directory structure on disk.
///
/// Called during agent onboarding to ensure the agent's space exists
/// before any pods or databases are deployed. Safe to call multiple times
/// (idempotent — directories already existing are not errors).
///
/// Creates the agent root directory and all subdirectories listed in
/// `AGENT_SUBDIRS`. Also writes the agent definition YAML with the public/private
/// directory declarations used by the Curator for artifact sync.
pub fn ensure_agent_dirs(agent_name: &str) -> std::io::Result<()> {
    let dir = agent_dir(agent_name);
    std::fs::create_dir_all(&dir)?;
    for sub in AGENT_SUBDIRS {
        std::fs::create_dir_all(dir.join(sub))?;
    }
    // Write agent.yaml with public/private directory declarations.
    // Safe to overwrite — idempotent, same content each time.
    let _ = write_agent_definition(agent_name);
    Ok(())
}

/// Write the agent definition YAML declaring public and private directories.
///
/// The Curator reads this file to determine which agent directories to
/// index for cross-agent artifact discovery.
fn write_agent_definition(agent_name: &str) -> std::io::Result<()> {
    let yaml_path = agent_definition_yaml(agent_name);
    // Only write if no definition exists — onboarding writes the full YAML.
    // The auto-generated stub is a fallback for directories created outside onboarding.
    if yaml_path.exists() {
        return Ok(());
    }
    let yaml = format!(
        "# Auto-generated by hKask during agent onboarding.\n\
         # Defines the agent's digital sphere — which directories contain\n\
         # public artifacts (indexed by Curator) vs private data.\n\
         agent:\n  name: \"{}\"\n  kind: replicant\n\n\
         # Directories containing public artifacts synced to the Curator.\n\
         public_dirs:\n  - artifacts\n  - library\n  - gallery\n  - documents\n  - adapters\n\n\
         # Directories containing private data (never leaves agent folder).\n\
         private_dirs:\n  - sessions\n  - portfolios\n",
        agent_name
    );
    std::fs::write(&yaml_path, yaml)
}

/// Publish an artifact to the agent's manifest for Curator indexing.
///
/// Called when an agent produces a shareable artifact (style, bot, gallery
/// item, trained adapter). The CuratorSync reads manifest files to build
/// the cross-agent artifact index.
pub fn publish_artifact(
    agent_name: &str,
    artifact_type: &str,
    artifact_name: &str,
    content_hash: &str,
) -> std::io::Result<()> {
    let manifest_path = agent_manifest_json(agent_name);
    let entry = serde_json::json!({
        "type": artifact_type,
        "name": artifact_name,
        "hash": content_hash,
        "published_at": chrono::Utc::now().to_rfc3339(),
    });

    // Read existing manifest, append, write back
    let mut manifest: serde_json::Value = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({"artifacts": []}))
    } else {
        serde_json::json!({"artifacts": []})
    };

    if let Some(artifacts) = manifest.get_mut("artifacts").and_then(|a| a.as_array_mut()) {
        // Replace existing entry with same type+name, or append new
        if let Some(existing) = artifacts.iter_mut().find(|a| {
            a.get("type").and_then(|t| t.as_str()) == Some(artifact_type)
                && a.get("name").and_then(|n| n.as_str()) == Some(artifact_name)
        }) {
            *existing = entry;
        } else {
            artifacts.push(entry);
        }
    }

    let json = serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| String::from("{}"));
    std::fs::write(&manifest_path, json)
}

/// Sanitize an agent name for filesystem use.
///
/// Replaces characters that are problematic in filenames with hyphens.
/// Agent names can contain spaces (e.g., "Jacques (Zuck)") but filenames shouldn't.
/// Guards against path traversal: names that sanitize to `.` or `..` are
/// replaced with `unnamed` to prevent directory escape.
pub fn sanitize_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '(' | ')' | ' ' => '-',
            other => other,
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    // Guard against path traversal: `.` and `..` resolve to current/parent dir.
    if sanitized == "." || sanitized == ".." {
        return "unnamed".to_string();
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_agent_names() {
        assert_eq!(sanitize_name("curator"), "curator");
        assert_eq!(sanitize_name("Jacques (Zuck)"), "Jacques--Zuck");
        assert_eq!(sanitize_name("alice"), "alice");
        assert_eq!(sanitize_name("team 7r7"), "team-7r7");
    }

    #[test]
    fn sanitize_rejects_path_traversal() {
        assert_eq!(sanitize_name(".."), "unnamed");
        assert_eq!(sanitize_name("."), "unnamed");
        assert_eq!(sanitize_name("---..---"), "unnamed");
        assert_eq!(sanitize_name("---.---"), "unnamed");
    }

    #[test]
    fn db_paths() {
        assert_eq!(
            agent_pod_db("curator"),
            PathBuf::from("agents/curator/pod.db")
        );
        assert_eq!(
            agent_memory_db("alice"),
            PathBuf::from("agents/alice/memory.db")
        );
        assert_eq!(
            agent_style_db("curator"),
            PathBuf::from("agents/curator/style.db")
        );
        assert_eq!(
            agent_wallet_db("alice"),
            PathBuf::from("agents/alice/wallet.db")
        );
    }

    #[test]
    fn dir_paths() {
        assert_eq!(
            agent_gallery_dir("curator"),
            PathBuf::from("agents/curator/gallery")
        );
        assert_eq!(
            agent_documents_dir("alice"),
            PathBuf::from("agents/alice/documents")
        );
        assert_eq!(
            agent_library_dir("curator"),
            PathBuf::from("agents/curator/library")
        );
        assert_eq!(
            agent_portfolios_dir("alice"),
            PathBuf::from("agents/alice/portfolios")
        );
    }

    #[test]
    fn ensure_dirs_creates_all_subdirs() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        ensure_agent_dirs("test-agent").expect("create dirs");

        assert!(agent_dir("test-agent").exists());
        for sub in AGENT_SUBDIRS {
            assert!(
                agent_dir("test-agent").join(sub).exists(),
                "missing subdir: {sub}"
            );
        }

        // Idempotent: calling again should not error
        ensure_agent_dirs("test-agent").expect("idempotent");

        std::env::set_current_dir(cwd).unwrap();
    }
}
