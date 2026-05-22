# hKask MCP Servers — Full rmcp Implementation Guide

**Version:** v0.21.0  
**Date:** 2026-05-20  
**rmcp Version:** 1.7.0

---

## Overview

All 20 MCP servers are defined in the hKask workspace with compiling stub implementations. This guide provides the exact steps to complete full rmcp integration.

---

## rmcp v1.7.0 Integration Pattern

### Working Code Template

```rust
use rmcp::{tool, tool_router, tool_handler, ServerHandler, ServiceExt};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::transport::stdio;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MyServer {
    tool_router: ToolRouter<Self>,
    // Your state here
}

impl MyServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            // Initialize state
        }
    }
}

#[tool_router]
impl MyServer {
    #[tool(description = "Tool description")]
    async fn my_tool(&self, param1: String, param2: Option<String>) -> String {
        // Your business logic here
        format!(r#"{{"result":"{}"}}"#, param1)
    }
    
    // Add more tools...
}

#[tool_handler]
impl ServerHandler for MyServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let server = MyServer::new();
    let service = server.serve(stdio());
    tracing::info!("MCP server started");
    service.await?;
    Ok(())
}
```

### Key Requirements

1. **Return Types:** Tools must return `impl IntoCallToolResult`
   - `String` ✅
   - `Json<T>` where `T: Serialize + JsonSchema` ✅
   - `CallToolResult` ✅
   - `Result<T, E>` where both implement `IntoCallToolResult` ✅

2. **Parameters:** Simple types work best
   - `String`, `i32`, `u64`, `bool`, `Option<T>` ✅
   - Complex structs need `#[derive(Deserialize, schemars::JsonSchema)]`

3. **Method Signature:** `async fn tool_name(&self, params...) -> ReturnType`

---

## Server-Specific Implementation Notes

### 1. hkask-mcp-ocap

**Tools to Implement:**
- `ocap_delegate(issuer, subject, capabilities) -> String`
- `ocap_verify(token_id, capability) -> String`
- `ocap_revoke(token_id) -> String`
- `ocap_enumerate(subject) -> String`
- `ocap_list_tokens() -> String`

**State:** `Arc<RwLock<Vec<String>>>` for token storage

### 2. hkask-mcp-keystore

**Tools to Implement:**
- `keystore_set(key, value, service) -> String`
- `keystore_get(key, service) -> String`
- `keystore_rotate(key, new_value, service) -> String`
- `keystore_delete(key, service) -> String`
- `keystore_list() -> String`
- `keystore_prompt(prompt_text) -> String`

**State:** `Arc<RwLock<HashMap<String, Secret<String>>>>`

### 3. hkask-mcp-cns

**Tools to Implement:**
- `cns_emit(span, observer_webid, phase, observation) -> String`
- `cns_variety(span_pattern) -> String`
- `cns_alert(span_pattern, severity) -> String`
- `cns_calibrate(span_pattern, new_threshold) -> String`
- `cns_list_alerts(limit) -> String`
- `cns_health() -> String`

**Integration:** Connect to `hkask-cns` crate for actual CNS operations

### 4. hkask-mcp-git

**Tools to Implement:**
- `git_resolve(git_ref) -> String`
- `git_snapshot(message, branch) -> String`
- `git_clone(url, target_path, branch) -> String`
- `git_fork(source_url, target_name, organization) -> String`
- `git_diff(sha1, sha2, path) -> String`
- `git_list(path) -> String`

**Integration:** Use `gix` crate for Git operations

### 5. hkask-mcp-registry

**Tools to Implement:**
- `registry_index(root_path, template_type) -> String`
- `registry_discover(template_type, domain_hint, limit) -> String`
- `registry_validate(template_id) -> String`
- `registry_reload(path) -> String`
- `registry_compose(root_template_id, cascade_template_ids) -> String`
- `registry_get(template_id) -> String`

**Integration:** Connect to `hkask-templates` crate

### 6. hkask-mcp-github

**Tools to Implement:**
- `github_get_repo(owner, repo) -> String`
- `github_list_issues(owner, repo, state) -> String`
- `github_get_issue(owner, repo, issue_number) -> String`
- `github_create_issue(owner, repo, title, body, labels) -> String`
- `github_list_prs(owner, repo, state) -> String`
- `github_get_pr(owner, repo, pr_number) -> String`
- `github_add_comment(owner, repo, issue_number, body) -> String`
- `github_search_repos(query, limit) -> String`

**Env:** `GITHUB_TOKEN` for write operations

**Integration:** Use `reqwest` for GitHub API calls

### 7. hkask-mcp-fmp

**Tools to Implement:**
- `fmp_ping() -> String`
- `fmp_company_profile(symbol) -> String`
- `fmp_quote(symbol) -> String`
- `fmp_income_statement(symbol, limit) -> String`
- `fmp_balance_sheet(symbol, limit) -> String`
- `fmp_cash_flow_statement(symbol, limit) -> String`
- `fmp_key_metrics(symbol, limit) -> String`
- `fmp_historical_price(symbol, from, to) -> String`
- `fmp_search(query, limit) -> String`
- `fmp_analyst_estimates(symbol) -> String`
- `fmp_dcf(symbol) -> String`

**Env:** `FMP_API_KEY` required for all except ping

**Integration:** Use `reqwest` for FMP API calls

### 8. hkask-mcp-telnyx

**Tools to Implement:**
- `telnyx_ping() -> String`
- `telnyx_list_numbers() -> String`
- `telnyx_buy_number(phone_number, messaging_profile_id) -> String`
- `telnyx_send_sms(from, to, text) -> String`
- `telnyx_make_call(from, to, webhook_url) -> String`
- `telnyx_send_whatsapp(from, to, content_type, content) -> String`
- `telnyx_tts(text, voice) -> String`
- `telnyx_list_voices() -> String`

**Env:** `TELNYX_API_KEY` required

**Integration:** Use `reqwest` for Telnyx API, `axum` for webhooks

### 9. hkask-mcp-fal

**Tools to Implement:**
- `fal_ping() -> String`
- `fal_generate_image(prompt, image_size, num_images) -> String`
- `fal_generate_image_fast(prompt, image_size) -> String`
- `fal_image_to_image(prompt, image_url, strength) -> String`
- `fal_upscale(image_url, scale) -> String`
- `fal_generate_video(prompt, duration) -> String`
- `fal_generate_music(prompt, duration_seconds) -> String`
- `fal_whisper(audio_url) -> String`
- `fal_caption(image_url) -> String`
- `fal_tts(text, voice) -> String`
- `fal_generate_3d(image_url) -> String`

**Env:** `FAL_KEY` required

**Integration:** Use `reqwest` for Fal.ai API

### 10. hkask-mcp-rss-reader

**Tools to Implement:**
- `rss_subscribe(url, label, folder) -> String`
- `rss_unsubscribe(stream_id) -> String`
- `rss_list_subscriptions(folder) -> String`
- `rss_fetch(stream_id) -> String`
- `rss_get_entries(stream_id, unread_only) -> String`
- `rss_mark_all_read(stream_id) -> String`
- `rss_get_unread_count(stream_id) -> String`
- `rss_search(query, limit) -> String`
- `rss_export_opml() -> String`
- `rss_discover_feeds(url) -> String`

**State:** `Arc<RwLock<Vec<Subscription>>>` for subscriptions

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ocap_delegate() {
        let server = OcapServer::new();
        let result = server.ocap_delegate(
            "issuer1".to_string(),
            "subject1".to_string(),
            "[]".to_string()
        ).await;
        assert!(result.contains("\"id\""));
    }
}
```

### Integration Tests

```bash
# Test server starts
cargo run -p hkask-mcp-ocap

# Test with MCP client
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  cargo run -p hkask-mcp-ocap
```

---

## CNS Integration

All tools should emit `cns.tool.*` spans:

```rust
#[tool(description = "My tool")]
async fn my_tool(&self, param: String) -> String {
    // Emit CNS span
    let event = NuEvent::new("cns.tool.my_tool", observer_webid);
    self.cns_runtime.emit(event).await;
    
    // Tool logic
    format!(r#"{{"result":"{}"}}"#, param)
}
```

---

## Build Verification

```bash
# All 10 servers compile
cargo check -p hkask-mcp-ocap -p hkask-mcp-keystore -p hkask-mcp-cns \
  -p hkask-mcp-git -p hkask-mcp-registry -p hkask-mcp-github \
  -p hkask-mcp-fmp -p hkask-mcp-telnyx -p hkask-mcp-fal -p hkask-mcp-rss-reader

# Format and lint
cargo fmt --all
cargo clippy -p hkask-mcp-ocap -- -D warnings
```

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*20 MCP servers defined. Integration guide complete.*
