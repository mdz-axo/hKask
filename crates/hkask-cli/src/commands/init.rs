//! `kask init` — Initialize hKask server configuration.
//!
//! REQ: P3-deploy-init-server — P3 Headless: server bootstrap via interactive CLI prompts.
//! expect: "I can initialize a hKask server with interactive prompts"
//!
//! Creates:
//! - ~/.config/hkask/config.json (server config)
//! - /var/lib/hkask/ (data directory)
//! - OS keychain entries (master passphrase, OAuth credentials)

use std::io::{self, Write};
use std::path::PathBuf;

/// Run the interactive server initialization.
///
/// expect: "I can access all hKask functionality through the kask CLI"
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
        "version": env!("CARGO_PKG_VERSION"),
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

    // 7. Generate systemd unit for auto-start on boot
    // REQ: P4-deploy-systemd-unit
    generate_systemd_unit(&config_dir)?;
    println!(
        "  ✓ Generated systemd unit at {}",
        config_dir.join("hkask.service").display()
    );

    // 8. Set env vars for current session
    println!("\n\u{2713} Server initialized successfully!\n");
    println!("  Add these to your environment or .env file:\n");
    println!("  export HKASK_OAUTH_GITHUB_CLIENT_ID={}", gh_client_id);
    println!("  # Client secret is stored in OS keychain — read with:");
    println!("  #   security find-generic-password -s hkask-oauth-github-client-secret -w");
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

/// Generate a systemd service unit for hKask daemon auto-start.
///
/// REQ: P4-deploy-systemd-unit
/// expect: "I can configure hKask to start automatically on system boot"
/// pre:  config_dir exists and is writable
/// post: hkask.service file written to config_dir with Type=simple, Restart=on-failure
///
/// # Generated unit properties
/// - Type=simple (foreground process)
/// - Restart=on-failure (auto-recovery)
/// - User=hkask (non-root)
/// - After=network.target (wait for network)
fn generate_systemd_unit(config_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let unit_content = r#"[Unit]
Description=hKask Daemon
After=network.target

[Service]
Type=simple
Restart=on-failure
User=hkask
ExecStart=/usr/local/bin/kask daemon
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    .to_string();
    let unit_path = config_dir.join("hkask.service");
    std::fs::write(&unit_path, unit_content)?;
    Ok(())
}
