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
}
