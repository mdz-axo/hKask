//! `kask repair` — detect and fix passphrase-mismatched databases.
//!
//! Scans all agent directories for SQLCipher databases that can't be opened
//! with the current passphrase, reports them, and (optionally) deletes them
//! so they can be regenerated with the correct passphrase on next startup.

use hkask_storage::DatabaseError;
use hkask_storage::check_passphrase;
use std::path::PathBuf;

/// Database files found under an agent directory, keyed by path.
const AGENT_DB_FILES: &[&str] = &[
    "memory.db",
    "kanban.db",
    "pod.db",
    "wallet.db",
    "training.db",
    "style.db",
];

/// Discover all agent directories under `agents/` in the current working directory.
fn discover_agent_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let agents_dir = std::path::Path::new("agents");
    if !agents_dir.is_dir() {
        return dirs;
    }
    if let Ok(entries) = std::fs::read_dir(agents_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                dirs.push(entry.path());
            }
        }
    }
    dirs
}

/// Discover all database files across all agent directories.
fn discover_databases() -> Vec<PathBuf> {
    let mut dbs = Vec::new();
    for agent_dir in discover_agent_dirs() {
        for &db_name in AGENT_DB_FILES {
            let db_path = agent_dir.join(db_name);
            if db_path.is_file() {
                dbs.push(db_path);
            }
        }
    }
    dbs
}

/// Run the repair command.
pub fn run(dry_run: bool, force: bool) {
    let passphrase = match std::env::var("HKASK_DB_PASSPHRASE") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            eprintln!("HKASK_DB_PASSPHRASE is not set — cannot verify databases.");
            eprintln!("Set it to your replicant passphrase and try again.");
            eprintln!("  export HKASK_DB_PASSPHRASE=<your-passphrase>");
            std::process::exit(1);
        }
    };

    let databases = discover_databases();

    if databases.is_empty() {
        println!("No agent databases found under agents/.");
        println!("Nothing to repair.");
        return;
    }

    let mut broken = Vec::new();
    let mut healthy = Vec::new();

    println!(
        "Checking {} database(s) under agents/...\n",
        databases.len()
    );

    for db_path in &databases {
        let path_str = db_path.to_string_lossy();
        match check_passphrase(&path_str, &passphrase) {
            Ok(()) => {
                healthy.push(db_path.clone());
            }
            Err(DatabaseError::PassphraseMismatch(_)) => {
                println!("  BROKEN: {}  (wrong passphrase)", path_str);
                broken.push(db_path.clone());
            }
            Err(e) => {
                println!("  CORRUPT: {}  ({})", path_str, e);
                broken.push(db_path.clone());
            }
        }
    }

    if !healthy.is_empty() {
        println!("  OK: {} database(s) healthy", healthy.len());
    }

    if broken.is_empty() {
        println!("\nAll databases are healthy — nothing to repair.");
        return;
    }

    println!(
        "\n{} database(s) are unreadable with the current passphrase.",
        broken.len()
    );

    if dry_run {
        println!("Dry run — no changes made. To fix:");
        println!("  kask repair          (interactive prompt)");
        println!("  kask repair --force  (delete all broken databases)");
        return;
    }

    if !force {
        println!("\nThese databases will be DELETED and regenerated on next startup.");
        println!("This is safe — they only contain cache/state, not user data.");
        println!();
        print!("Delete {} broken database(s)? [y/N] ", broken.len());
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            eprintln!("Failed to read input.");
            return;
        }
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Cancelled — no databases deleted.");
            println!("To recover old data, set HKASK_DB_PASSPHRASE to your previous passphrase.");
            return;
        }
    }

    let mut deleted = 0;
    for db_path in &broken {
        let path_str = db_path.to_string_lossy();
        let salt_path = format!("{}.salt", path_str);

        if let Err(e) = std::fs::remove_file(db_path) {
            eprintln!("  Failed to delete {}: {}", path_str, e);
        } else {
            println!("  Deleted: {}", path_str);
            deleted += 1;
        }

        // Also remove the salt file so a fresh salt is generated on next open.
        let salt = std::path::Path::new(&salt_path);
        if salt.is_file() {
            let _ = std::fs::remove_file(salt);
        }
    }

    println!(
        "\nRepaired: {} database(s) deleted. They will be regenerated on next startup.",
        deleted
    );
    println!("Run 'kask chat' to start fresh.");
}
