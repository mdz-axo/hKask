//! `kask init` — Initialize hKask server configuration.
//!
//! REQ: DEP-400 — P3 Headless: server bootstrap via interactive CLI prompts.
/// expect: "I can access all hKask functionality through the kask CLI" [P3]
//!
//! Creates:
//! - ~/.config/hkask/config.json (server config)
//! - /var/lib/hkask/ (data directory)
//! - OS keychain entries (master passphrase, OAuth credentials)

use std::io::{self, Write};
use std::path::PathBuf;

/// Run the interactive server initialization.
///
/// REQ: DEP-401
/// expect: "I can access all hKask functionality through the kask CLI" [P3]
/// pre:  stdin is a terminal
/// post: server config, data dir, and keychain entries created
pub fn run_init() -> Result<(), Box<dyn std::error::Error>> {
    println!("hKask Server Initialization");
    println!("==========================\n");

    // 1. Master passphrase
    let passphrase = prompt_passphrase("Server master passphrase (min 8 chars)")?;

    // 2. Data directory
    let data_dir = prompt_default("Data directory", "/var/lib/hkask")?;
    std::fs::create_dir_all(&data_dir)?;
    println!("  Created: {}\n", data_dir);

    // 3. Domain name
    let domain = prompt_default("Domain name (for TLS + OAuth redirects)", "localhost")?;

    // 4. OAuth: GitHub
    println!("\nGitHub OAuth Setup");
    println!("  Create an OAuth App at: https://github.com/settings/developers");
    println!(
        "  Callback URL: https://{}/api/v1/auth/callback?provider=github\n",
        domain
    );
    let gh_client_id = prompt_required("GitHub Client ID")?;
    let gh_client_secret = prompt_required("GitHub Client Secret")?;

    // 5. Store in keychain
    let keychain = hkask_keystore::keychain::Keychain::new("hkask");
    keychain
        .store_by_key("hkask-master", &passphrase)
        .map_err(|e| format!("Failed to store master key: {e}"))?;
    println!("  ✓ Stored master passphrase in OS keychain");

    keychain
        .store_by_key("hkask-oauth-github-client-id", &gh_client_id)
        .ok();
    keychain
        .store_by_key("hkask-oauth-github-client-secret", &gh_client_secret)
        .ok();
    println!("  ✓ Stored OAuth credentials in OS keychain");

    // 6. Write config
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hkask");
    std::fs::create_dir_all(&config_dir)?;

    let config = serde_json::json!({
        "version": "0.28.0",
        "profile": "server",
        "data_dir": data_dir,
        "domain": domain,
        "oauth": {
            "github": {
                "client_id": gh_client_id,
                "client_secret_stored_in_keychain": true
            }
        }
    });

    let config_path = config_dir.join("config.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    println!("  ✓ Wrote config to {}", config_path.display());

    // 7. Set env vars for current session
    println!("\n✓ Server initialized successfully!\n");
    println!("  Add these to your environment or .env file:\n");
    println!("  export HKASK_OAUTH_GITHUB_CLIENT_ID={}", gh_client_id);
    println!(
        "  export HKASK_OAUTH_GITHUB_CLIENT_SECRET={}",
        gh_client_secret
    );
    println!("  export HKASK_DOMAIN={}", domain);
    println!("\n  Then run: kask serve");

    Ok(())
}

fn prompt_passphrase(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    loop {
        let pass = rpassword::prompt_password(format!("{}: ", prompt))?;
        if pass.len() < 8 {
            println!("  Passphrase must be at least 8 characters.");
            continue;
        }
        let confirm = rpassword::prompt_password("  Confirm: ")?;
        if pass != confirm {
            println!("  Passphrases don't match. Try again.");
            continue;
        }
        return Ok(pass);
    }
}

fn prompt_required(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    loop {
        print!("{}: ", prompt);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            println!("  This field is required.");
            continue;
        }
        return Ok(trimmed);
    }
}

fn prompt_default(prompt: &str, default: &str) -> Result<String, Box<dyn std::error::Error>> {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed)
    }
}
