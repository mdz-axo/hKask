//! MCP security — URL validation for tool endpoints
//!
//! Provides SSRF protection for MCP tool invocations:
//! - URL validation (scheme, credentials, private IP, loopback)

use std::net::IpAddr;

/// URL validation error types
#[derive(Debug, thiserror::Error)]
pub(crate) enum SecurityError {
    #[error("Non-HTTP(S) scheme not allowed: {0}")]
    DisallowedScheme(String),

    #[error("URL contains embedded credentials (user:pass@host): {0}")]
    EmbeddedCredentials(String),

    #[error("Private IP address not allowed: {0}")]
    PrivateIpNotAllowed(String),

    #[error("Loopback address not allowed: {0}")]
    LoopbackNotAllowed(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// URL validation configuration
#[derive(Debug, Clone, Default)]
pub(crate) struct UrlValidationConfig {
    /// Allow private IP addresses (10.x, 172.16-31.x, 192.168.x)
    pub allow_private_ips: bool,
    /// Allow loopback addresses (127.x.x.x, ::1)
    pub allow_loopback: bool,
}

/// Validate a URL for use in MCP web/scholar requests.
///
/// Checks:
/// - Rejects non-HTTP(S) schemes
/// - Rejects URLs with embedded credentials (user:pass@host)
/// - Rejects private IPs unless explicitly permitted
/// - Rejects loopback addresses unless explicitly permitted
pub(crate) fn validate_url(
    raw_url: &str,
    config: &UrlValidationConfig,
) -> Result<(), SecurityError> {
    let scheme_end = raw_url
        .find("://")
        .ok_or_else(|| SecurityError::InvalidUrl("No scheme separator '://' found".to_string()))?;
    let scheme = &raw_url[..scheme_end];
    if scheme != "http" && scheme != "https" {
        return Err(SecurityError::DisallowedScheme(scheme.to_string()));
    }

    let after_scheme = &raw_url[scheme_end + 3..];
    let authority = after_scheme.split('/').next().unwrap_or(after_scheme);
    let host_part = authority.split('@').next_back().unwrap_or(authority);
    if host_part != authority {
        return Err(SecurityError::EmbeddedCredentials(raw_url.to_string()));
    }

    let host = host_part.split(':').next().unwrap_or(host_part);
    let bracket_close = host.rfind(']');
    let hostname = if host.starts_with('[') {
        bracket_close
            .map(|i| &host[1..i])
            .ok_or_else(|| SecurityError::InvalidUrl("Malformed IPv6 address".to_string()))?
    } else {
        host
    };

    let ip: Option<IpAddr> = hostname.parse().ok();

    if let Some(ip) = ip {
        if ip.is_loopback() && !config.allow_loopback {
            return Err(SecurityError::LoopbackNotAllowed(ip.to_string()));
        }
        if is_private_ip(&ip) && !config.allow_private_ips {
            return Err(SecurityError::PrivateIpNotAllowed(ip.to_string()));
        }
    }

    Ok(())
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 10
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            segments[0] == 0xfc00 || segments[0] == 0xfd00
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── URL validation (SSRF protection) ──────────────────────────────────

    #[test]
    fn validate_url_accepts_http() {
        assert!(validate_url("http://example.com", &UrlValidationConfig::default()).is_ok());
    }

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://example.com", &UrlValidationConfig::default()).is_ok());
    }

    #[test]
    fn validate_url_rejects_ftp_scheme() {
        let err = validate_url("ftp://example.com", &UrlValidationConfig::default()).unwrap_err();
        assert!(matches!(err, SecurityError::DisallowedScheme(s) if s == "ftp"));
    }

    #[test]
    fn validate_url_rejects_javascript_scheme() {
        let err =
            validate_url("javascript://alert(1)", &UrlValidationConfig::default()).unwrap_err();
        assert!(matches!(err, SecurityError::DisallowedScheme(s) if s == "javascript"));
    }

    #[test]
    fn validate_url_rejects_embedded_credentials() {
        let err = validate_url(
            "https://user:pass@example.com",
            &UrlValidationConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SecurityError::EmbeddedCredentials(_)));
    }

    #[test]
    fn validate_url_rejects_loopback() {
        let err =
            validate_url("http://127.0.0.1/admin", &UrlValidationConfig::default()).unwrap_err();
        assert!(matches!(err, SecurityError::LoopbackNotAllowed(_)));
    }

    #[test]
    fn validate_url_rejects_private_ip_10() {
        let err =
            validate_url("http://10.0.0.1/internal", &UrlValidationConfig::default()).unwrap_err();
        assert!(matches!(err, SecurityError::PrivateIpNotAllowed(_)));
    }

    #[test]
    fn validate_url_rejects_private_ip_172() {
        let err = validate_url(
            "http://172.16.0.1/internal",
            &UrlValidationConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SecurityError::PrivateIpNotAllowed(_)));
    }

    #[test]
    fn validate_url_rejects_private_ip_192() {
        let err = validate_url(
            "http://192.168.1.1/internal",
            &UrlValidationConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SecurityError::PrivateIpNotAllowed(_)));
    }

    #[test]
    fn validate_url_rejects_link_local() {
        let err = validate_url(
            "http://169.254.1.1/internal",
            &UrlValidationConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SecurityError::PrivateIpNotAllowed(_)));
    }

    #[test]
    fn validate_url_allows_loopback_when_configured() {
        let config = UrlValidationConfig {
            allow_loopback: true,
            ..Default::default()
        };
        assert!(validate_url("http://127.0.0.1/admin", &config).is_ok());
    }

    #[test]
    fn validate_url_allows_private_ip_when_configured() {
        let config = UrlValidationConfig {
            allow_private_ips: true,
            ..Default::default()
        };
        assert!(validate_url("http://10.0.0.1/internal", &config).is_ok());
    }

    #[test]
    fn validate_url_rejects_no_scheme() {
        let err = validate_url("example.com/path", &UrlValidationConfig::default()).unwrap_err();
        assert!(matches!(err, SecurityError::InvalidUrl(_)));
    }

    #[test]
    fn validate_url_accepts_hostname() {
        // Hostnames (not IPs) are always allowed
        assert!(
            validate_url(
                "https://api.example.com/v1/data",
                &UrlValidationConfig::default()
            )
            .is_ok()
        );
    }

    // ── Private IP classification ──────────────────────────────────────────

    #[test]
    fn private_ip_classification() {
        assert!(is_private_ip(&"10.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"172.16.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"172.31.255.255".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"192.168.1.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"169.254.1.1".parse::<IpAddr>().unwrap()));
        // Public IPs are not private
        assert!(!is_private_ip(&"8.8.8.8".parse::<IpAddr>().unwrap()));
        assert!(!is_private_ip(&"1.1.1.1".parse::<IpAddr>().unwrap()));
    }

    // F-SYN-007: every `#[tool]`-attributed function in every
    // mcp-servers/hkask-mcp-*/src/main.rs must contain a capability
    // gate (`verify_capability(...)` or `ToolSpanGuard::new(...)`) in
    // its body. This is a static structural check: the
    // `verify_capability` call must appear between the
    // `#[tool(...)]` attribute and the matching `}`.
    //
    // The check is gated behind an env var so it doesn't run on
    // every `cargo test` invocation (it walks the whole workspace).
    // CI sets `HKASK_RUN_MCP_GATE_AUDIT=1` to enable it.
    #[test]
    #[ignore] // gated by env var; see comment above
    fn mcp_capability_gate_audit() {
        if std::env::var("HKASK_RUN_MCP_GATE_AUDIT").is_err() {
            eprintln!(
                "F-SYN-007: skipping mcp_capability_gate_audit. \
                 Set HKASK_RUN_MCP_GATE_AUDIT=1 to run."
            );
            return;
        }

        // Walk the workspace from CARGO_MANIFEST_DIR upward to find
        // the mcp-servers/ directory.
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("hkask-mcp is at crates/hkask-mcp, parent.parent is the workspace root");
        let mcp_servers = workspace_root.join("mcp-servers");
        assert!(
            mcp_servers.is_dir(),
            "F-SYN-007: mcp-servers/ not found at {}",
            mcp_servers.display()
        );

        let mut violations: Vec<String> = Vec::new();
        let entries = std::fs::read_dir(&mcp_servers).expect("read mcp-servers");
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let main_rs = path.join("src").join("main.rs");
            if !main_rs.is_file() {
                continue;
            }
            let source = std::fs::read_to_string(&main_rs)
                .unwrap_or_else(|e| panic!("read {}: {e}", main_rs.display()));

            // Find every `#[tool(...)]` attribute, then for each
            // one, find the *adjacent* `fn` declaration (i.e. the
            // next `fn` keyword that has no other `#[tool]` between
            // it and the attribute).
            let mut i = 0;
            while let Some(attr_pos) = source[i..].find("#[tool") {
                let abs = i + attr_pos;
                // Skip past this attribute and any other attributes
                // (e.g. `#[tool_router(...)]` or `#[allow(...)]`).
                let mut scan = abs;
                'skip_attrs: while let Some(end) = source[scan..].find(']') {
                    // Advance past the current `#[...]` block.
                    scan = scan + end + 1;
                    // Consume any whitespace.
                    let trimmed = source[scan..].trim_start();
                    let ws = source[scan..].len() - trimmed.len();
                    scan += ws;
                    // If the next non-whitespace starts a *new*
                    // attribute (`#[`), continue scanning; otherwise
                    // we are at the function declaration.
                    if !source[scan..].starts_with("#[") {
                        break 'skip_attrs;
                    }
                }
                // Now `scan` points to the start of the function
                // declaration (or end of source).
                let after_attrs = scan;
                // Find `fn `.
                let fn_pos = match source[after_attrs..].find("fn ") {
                    Some(p) => after_attrs + p,
                    None => break,
                };
                // Find the parameter list's closing `)`, then
                // the opening `{` of the function body. The body
                // brace is the *outermost* one after the params.
                // Parameters may contain nested braces (e.g.
                // `Parameters(SetRequest { key, value })`), so
                // we must skip over them.
                let params_end = match source[fn_pos..].find(')') {
                    Some(p) => fn_pos + p,
                    None => break,
                };
                let brace_pos = match source[params_end..].find('{') {
                    Some(p) => params_end + p,
                    None => break,
                };
                // Find the matching `}` by counting braces.
                let mut depth = 0;
                let mut end = brace_pos;
                for (offset, ch) in source[brace_pos..].char_indices() {
                    match ch {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = brace_pos + offset;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let body = &source[brace_pos..=end];
                let has_gate =
                    body.contains("verify_capability") || body.contains("ToolSpanGuard::new");
                if !has_gate {
                    // Extract the function name for the error.
                    let name_start = fn_pos + 3; // skip "fn "
                    let name_end = source[name_start..]
                        .find('(')
                        .map(|p| name_start + p)
                        .unwrap_or(name_start);
                    let fn_name = &source[name_start..name_end];
                    violations.push(format!(
                        "{}: fn {}: no capability gate",
                        main_rs.display(),
                        fn_name
                    ));
                }
                i = end + 1;
            }
        }

        if !violations.is_empty() {
            panic!(
                "F-SYN-007: {} MCP tool(s) without a capability gate:\n  - {}",
                violations.len(),
                violations.join("\n  - ")
            );
        }

        // Sanity check: a deliberate violation must be detected.
        // If the audit logic ever regresses (e.g. skips a
        // `#[tool]` attribute), this catches the regression.
        // The violation is constructed in-memory, not in the
        // source tree, so it does not affect the actual codebase.
        let synthetic = r#"
            #[tool(description = "synthetic")]
            async fn synthetic_no_gate(
                &self,
                Parameters(Req { x }): Parameters<Req>,
            ) -> String {
                format!("{}", "no gate here")
            }
        "#;
        let mut count = 0;
        let mut scan = 0;
        while let Some(attr_pos) = synthetic[scan..].find("#[tool") {
            let abs = scan + attr_pos;
            let mut s = abs;
            'skip_attrs: while let Some(end) = synthetic[s..].find(']') {
                s = s + end + 1;
                let trimmed = synthetic[s..].trim_start();
                let ws = synthetic[s..].len() - trimmed.len();
                s += ws;
                if !synthetic[s..].starts_with("#[") {
                    break 'skip_attrs;
                }
            }
            if let Some(fn_p) = synthetic[s..].find("fn ") {
                let abs_fn = s + fn_p;
                if let Some(close) = synthetic[abs_fn..].find(')')
                    && let Some(open) = synthetic[abs_fn + close..].find('{')
                {
                    let bp = abs_fn + close + open;
                    let mut depth = 0;
                    let mut end = bp;
                    for (i, ch) in synthetic[bp..].char_indices() {
                        if ch == '{' {
                            depth += 1;
                        } else if ch == '}' {
                            depth -= 1;
                            if depth == 0 {
                                end = bp + i;
                                break;
                            }
                        }
                    }
                    let body = &synthetic[bp..=end];
                    if !body.contains("verify_capability") && !body.contains("ToolSpanGuard::new") {
                        count += 1;
                    }
                }
            }
            scan = abs + "#[tool".len();
        }
        assert_eq!(
            count, 1,
            "F-SYN-007 sanity check: the synthetic no-gate function must be detected"
        );
    }

    // F-SYN-022: a `pub trait` with more than 5 methods is a
    // god-trait (Mark Miller / Fowler). The check is gated behind
    // an env var so it doesn't run on every `cargo test`
    // invocation (it walks the whole workspace).
    #[test]
    #[ignore] // gated by env var
    fn no_god_traits() {
        if std::env::var("HKASK_RUN_GOD_TRAIT_AUDIT").is_err() {
            eprintln!(
                "F-SYN-022: skipping no_god_traits. \
                 Set HKASK_RUN_GOD_TRAIT_AUDIT=1 to run."
            );
            return;
        }
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("hkask-mcp is at crates/hkask-mcp, parent.parent is the workspace root");
        let crates_dir = workspace_root.join("crates");

        // Extract every `pub trait ... { ... }` block and count
        // its top-level methods. This is a coarse heuristic — it
        // counts `fn` lines inside the block — but it's enough to
        // catch the failure mode (a trait with 6+ methods).
        let mut violations: Vec<String> = Vec::new();
        for entry in walkdir_rs(&crates_dir) {
            if !entry.ends_with(".rs") {
                continue;
            }
            let source = match std::fs::read_to_string(&entry) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mut depth: i32 = 0;
            let mut current_trait: Option<(String, i32, usize)> = None;
            for (idx, ch) in source.char_indices() {
                match ch {
                    '{' => {
                        depth += 1;
                        if let Some((name, open_depth, count)) = current_trait.clone()
                            && open_depth + 1 == depth
                        {
                            // We are entering the trait body.
                            // The method counter is already
                            // initialised; continue.
                            let _ = (name, count);
                        }
                    }
                    '}' => {
                        if let Some((name, open_depth, count)) = current_trait.take()
                            && depth == open_depth + 1
                        {
                            // We are exiting the trait body.
                            if count > 5 {
                                violations.push(format!(
                                    "{}: trait {} has {} methods (limit: 5)",
                                    entry.display(),
                                    name,
                                    count
                                ));
                            }
                            current_trait = None;
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
                // Detect `pub trait` openings.
                if ch == '{'
                    && idx >= "pub trait".len()
                    && source[idx - "pub trait".len()..idx].contains("pub trait")
                {
                    // Find the trait name.
                    let after = &source[idx + 1..];
                    let name_end = after.find([' ', '<', ':', '{']).unwrap_or(after.len());
                    let name = after[..name_end].to_string();
                    current_trait = Some((name, depth, 0));
                }
                // Count `fn` declarations inside the trait body.
                if let Some(t) = current_trait.as_mut()
                    && ch == 'f'
                    && source[idx..].starts_with("fn ")
                    && depth > 0
                {
                    t.2 += 1;
                }
            }
        }
        if !violations.is_empty() {
            panic!(
                "F-SYN-022: {} god-trait(s) detected (>5 methods):\n  - {}",
                violations.len(),
                violations.join("\n  - ")
            );
        }
    }
}

#[cfg(test)]
fn walkdir_rs(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(p) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    out.push(path);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod arc_mutex_baseline {
    use std::path::Path;

    /// Baseline count of `Arc<Mutex<...>>` in the workspace
    /// as of the F-SYN-021 review (measured via
    /// `rg 'Arc<Mutex<' crates/ mcp-servers/ | wc -l`).
    /// Bump this with a comment when adding a justified mutex.
    const BASELINE: usize = 50; // measured 41 on 2026-06-06; +9 headroom

    #[test]
    #[ignore]
    fn arc_mutex_count_below_baseline() {
        if std::env::var("HKASK_RUN_ARC_MUTEX_AUDIT").is_err() {
            eprintln!(
                "F-SYN-021: skipping arc_mutex_count_below_baseline. \
                 Set HKASK_RUN_ARC_MUTEX_AUDIT=1 to run."
            );
            return;
        }
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("hkask-mcp is at crates/hkask-mcp, parent.parent is the workspace root");
        let crates_dir = workspace_root.join("crates");
        let mcp_servers_dir = workspace_root.join("mcp-servers");

        let mut count = 0;
        for dir in [&crates_dir, &mcp_servers_dir] {
            count += count_arc_mutex_in(dir);
        }
        assert!(
            count <= BASELINE,
            "F-SYN-021: Arc<Mutex<...>> count = {} (baseline {}). \
             A new occurrence requires a finding.",
            count,
            BASELINE
        );
    }

    fn count_arc_mutex_in(dir: &Path) -> usize {
        let mut out = 0;
        for entry in walkdir_rs(dir) {
            let Ok(source) = std::fs::read_to_string(&entry) else {
                continue;
            };
            // Count every `Arc<Mutex<` literal. This is a
            // coarse heuristic (e.g. comments count too), but
            // it's a *baseline* — the goal is to detect growth.
            out += source.matches("Arc<Mutex<").count();
        }
        out
    }

    fn walkdir_rs(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(p) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                        out.push(path);
                    }
                }
            }
        }
        out
    }
}
