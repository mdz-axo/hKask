# hKask Implementation Plan — ADV-REVIEW-F2 Tasks

**Status:** Pre-release. No backward compatibility requirements. Aggressive redesign permitted.

**Design Decisions (resolved):**
- **DQ1:** Option A — Add `caveats: Vec<Caveat>` to `CapabilityToken`. Delete `Macaroon` and `OkapiCapability` entirely.
- **DQ2:** Option A — Full async cascade. `McpPort` → `#[async_trait]`, `ManifestExecutor::execute()` → `async fn`.
- **DQ3:** Option A — UUID v5 for deterministic WebID derivation.
- **DQ4:** Option A — Inject `CnsEmit` into `AcpRuntime` constructor. No optional.
- **DQ5:** Option A — hKask spawns Russell as child process over stdio JSON-RPC.
- **DQ6:** **Defer** — Typestate pod lifecycle is elegant but high-risk for minimal gain. Keep enum + runtime checks.
- **DQ7:** Option B — OCAP-idiomatic. `verify_tool_capability` accepts token directly. No internal store.
- **DQ8:** Option C — Shared `TokenBucket` primitive, separate implementations keyed by domain.

---

## Task T01: Delete Duplicate `hkask-agents::CapabilityToken`

**Goal:** Eliminate the parallel `CapabilityToken` in `hkask-agents/src/capability.rs`. Use only `hkask_types::CapabilityToken`.

**Files to modify:**
1. `crates/hkask-agents/src/capability.rs` — **DELETE ENTIRE FILE**
2. `crates/hkask-agents/src/lib.rs` — Remove `pub mod capability;` and re-exports
3. `crates/hkask-agents/Cargo.toml` — No changes (already depends on `hkask-types`)
4. All call sites in `hkask-agents` that import from `crate::capability`

**Detailed steps:**

### Step 1: Identify all imports of `crate::capability`
```bash
cd /home/mdz-axolotl/Clones/hKask
rg "use crate::capability" crates/hkask-agents/
rg "crate::capability::" crates/hkask-agents/
```

### Step 2: Update `crates/hkask-agents/src/lib.rs`
**Current (lines 39-40):**
```rust
pub mod capability;
```

**Change to:**
```rust
// DELETE: pub mod capability;
```

**Current (line 53):**
```rust
pub use capability::{BotCapabilities, CapabilityChecker, CapabilityToken};
```

**Change to:**
```rust
pub use hkask_types::{BotCapabilities, CapabilityChecker, CapabilityToken};
```

### Step 3: Update call sites
For each file that imports `crate::capability::*`:
- Replace `use crate::capability::CapabilityToken;` with `use hkask_types::CapabilityToken;`
- Replace `use crate::capability::CapabilityChecker;` with `use hkask_types::CapabilityChecker;`
- Replace `use crate::capability::BotCapabilities;` with `use hkask_types::BotCapabilities;`

**Expected call sites:**
- `crates/hkask-agents/src/pod.rs`
- `crates/hkask-agents/src/acp.rs`
- `crates/hkask-agents/src/adapters/mcp_runtime.rs`
- `crates/hkask-mcp/src/dispatch.rs`

### Step 4: Delete the file
```bash
rm crates/hkask-agents/src/capability.rs
```

### Step 5: Verify compilation
```bash
cargo check -p hkask-agents
cargo test -p hkask-agents
```

**Test strategy:**
- Existing tests should pass unchanged (they test behavior, not specific type locations)
- Add test: verify `CapabilityToken` from `hkask_types` has constant-time comparison (already exists in `hkask-types`)

**Success criteria:**
- `crates/hkask-agents/src/capability.rs` deleted
- All imports resolve to `hkask_types::CapabilityToken`
- `cargo check -p hkask-agents` succeeds
- `cargo test -p hkask-agents` passes

---

## Task T02: Fix `check_resource_for_holder` Security Bypass

**Goal:** Replace the no-op `check_resource_for_holder` with OCAP-idiomatic verification. The holder presents a token; the checker verifies it.

**Files to modify:**
1. `crates/hkask-types/src/capability.rs`
2. `crates/hkask-agents/src/acp.rs` (update `verify_tool_capability` call sites)

**Detailed steps:**

### Step 1: Redesign `verify_tool_capability` signature
**Current (`crates/hkask-types/src/capability.rs`, lines 694-708):**
```rust
pub fn verify_tool_capability(
    &self,
    holder: impl Into<WebID>,
    resource: CapabilityResource,
    resource_id: &str,
    action: CapabilityAction,
) -> bool {
    let holder = holder.into();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    self.check_resource_for_holder(holder, resource, resource_id, action, current_time)
}
```

**Change to:**
```rust
/// Verify a capability token for tool access (OCAP-idiomatic)
///
/// The holder presents the token; the checker verifies it.
/// Checks: signature, expiry, holder match, resource/action match.
pub fn verify_tool_capability(
    &self,
    token: &CapabilityToken,
    expected_holder: &WebID,
    resource: CapabilityResource,
    resource_id: &str,
    action: CapabilityAction,
) -> bool {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // 1. Verify signature
    if !self.verify_with_time(token, current_time) {
        return false;
    }

    // 2. Verify holder matches
    if token.delegated_to != *expected_holder {
        return false;
    }

    // 3. Verify resource/action match
    if !token.is_valid_for(resource, resource_id, action) {
        return false;
    }

    true
}
```

### Step 2: Delete `check_resource_for_holder`
**Current (lines 710-720):**
```rust
fn check_resource_for_holder(
    &self,
    holder: WebID,
    resource: CapabilityResource,
    resource_id: &str,
    action: CapabilityAction,
    current_time: i64,
) -> bool {
    let _ = (holder, resource, resource_id, action, current_time);
    true
}
```

**Change to:**
```rust
// DELETE ENTIRE METHOD
```

### Step 3: Update call sites in `hkask-agents`
Find all calls to `verify_tool_capability` and update to pass the token directly.

**Expected call sites:**
- `crates/hkask-agents/src/acp.rs` — `AcpRuntime::verify_capability`
- `crates/hkask-mcp/src/dispatch.rs` — `McpDispatcher::check_capability`

**Example update for `acp.rs`:**
```rust
// Before:
let valid = self.capability_checker.verify_tool_capability(
    webid,
    CapabilityResource::Tool,
    tool_name,
    CapabilityAction::Execute,
);

// After:
let token = self.get_token_for_agent(webid, tool_name)?; // lookup from agent's token list
let valid = self.capability_checker.verify_tool_capability(
    &token,
    webid,
    CapabilityResource::Tool,
    tool_name,
    CapabilityAction::Execute,
);
```

### Step 4: Add tests
**File:** `crates/hkask-types/src/capability.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn test_verify_tool_capability_valid() {
    let secret = b"test-secret";
    let checker = CapabilityChecker::new(secret);
    let holder = WebID::new();
    let issuer = WebID::new();
    
    let token = checker.grant_tool("tool:search".to_string(), issuer, holder);
    
    assert!(checker.verify_tool_capability(
        &token,
        &holder,
        CapabilityResource::Tool,
        "tool:search",
        CapabilityAction::Execute,
    ));
}

#[test]
fn test_verify_tool_capability_wrong_holder() {
    let secret = b"test-secret";
    let checker = CapabilityChecker::new(secret);
    let holder = WebID::new();
    let wrong_holder = WebID::new();
    let issuer = WebID::new();
    
    let token = checker.grant_tool("tool:search".to_string(), issuer, holder);
    
    assert!(!checker.verify_tool_capability(
        &token,
        &wrong_holder,
        CapabilityResource::Tool,
        "tool:search",
        CapabilityAction::Execute,
    ));
}

#[test]
fn test_verify_tool_capability_wrong_resource() {
    let secret = b"test-secret";
    let checker = CapabilityChecker::new(secret);
    let holder = WebID::new();
    let issuer = WebID::new();
    
    let token = checker.grant_tool("tool:search".to_string(), issuer, holder);
    
    assert!(!checker.verify_tool_capability(
        &token,
        &holder,
        CapabilityResource::Tool,
        "tool:other",
        CapabilityAction::Execute,
    ));
}
```

### Step 5: Verify
```bash
cargo test -p hkask-types capability
cargo check -p hkask-agents
cargo test -p hkask-agents
```

**Success criteria:**
- `check_resource_for_holder` deleted
- `verify_tool_capability` accepts token directly
- All tests pass
- No call site uses the old signature

---

## Task T03: Fix Wildcard Capability Contradiction

**Goal:** Eliminate `"*"` wildcard capabilities. Pod creation uses actual capabilities from persona.

**Files to modify:**
1. `crates/hkask-agents/src/pod.rs`
2. `crates/hkask-agents/src/acp.rs`

**Detailed steps:**

### Step 1: Fix `AgentPod::new` wildcard
**Current (`crates/hkask-agents/src/pod.rs`, line 345):**
```rust
let token = CapabilityToken::new(
    CapabilityResource::Tool,
    "*".to_string(),  // ← WILDCARD
    CapabilityAction::Execute,
    root_webid,
    webid,
    secret,
);
```

**Change to:**
```rust
// Use first capability from persona, or default to "tool:execute"
let first_capability = self.persona.capabilities
    .first()
    .cloned()
    .unwrap_or_else(|| "tool:execute".to_string());

let token = CapabilityToken::new(
    CapabilityResource::Tool,
    first_capability,
    CapabilityAction::Execute,
    root_webid,
    webid,
    secret,
);
```

### Step 2: Remove wildcard check in `has_capability`
**Current (`crates/hkask-agents/src/acp.rs`, line 633):**
```rust
pub fn has_capability(&self, webid: &WebID, cap: &str) -> bool {
    let caps = self.capabilities.read().unwrap();
    caps.get(webid)
        .map(|c| c.capabilities.contains(&cap.to_string()) || c.capabilities.contains(&"*".to_string()))
        .unwrap_or(false)
}
```

**Change to:**
```rust
pub fn has_capability(&self, webid: &WebID, cap: &str) -> bool {
    let caps = self.capabilities.read().unwrap();
    caps.get(webid)
        .map(|c| c.capabilities.contains(&cap.to_string()))
        .unwrap_or(false)
}
```

### Step 3: Fix default capability in `register_agent`
**Current (`crates/hkask-agents/src/acp.rs`, line 403):**
```rust
let capabilities = if capabilities.is_empty() {
    vec!["agent:basic".to_string()]  // ← UNPARSEABLE
} else {
    capabilities
};
```

**Change to:**
```rust
let capabilities = if capabilities.is_empty() {
    vec!["tool:execute".to_string()]  // Valid parseable capability
} else {
    capabilities
};
```

### Step 4: Add tests
**File:** `crates/hkask-agents/src/acp.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn test_register_agent_no_wildcard() {
    let runtime = AcpRuntime::default();
    let webid = WebID::new();
    
    // Register with empty capabilities (should get default "tool:execute")
    let token = runtime.register_agent(webid, "Bot", vec![]).unwrap();
    
    assert_eq!(token.resource_id, "tool:execute");
    assert_ne!(token.resource_id, "*");
}

#[test]
fn test_has_capability_no_wildcard() {
    let runtime = AcpRuntime::default();
    let webid = WebID::new();
    
    runtime.register_agent(webid, "Bot", vec!["tool:search".to_string()]).unwrap();
    
    assert!(runtime.has_capability(&webid, "tool:search"));
    assert!(!runtime.has_capability(&webid, "*"));
    assert!(!runtime.has_capability(&webid, "tool:other"));
}
```

### Step 5: Verify
```bash
cargo test -p hkask-agents acp
cargo test -p hkask-agents pod
```

**Success criteria:**
- No `"*"` in capability creation
- `has_capability` rejects `"*"`
- Default capability is `"tool:execute"`
- All tests pass

---

## Task T04: Fix `verify_capability` Blocking Read Panic

**Goal:** Make `verify_capability` async to avoid `blocking_read` panic.

**Files to modify:**
1. `crates/hkask-agents/src/acp.rs`

**Detailed steps:**

### Step 1: Make `verify_capability` async
**Current (`crates/hkask-agents/src/acp.rs`, line 543):**
```rust
pub fn verify_capability(&self, token: &CapabilityToken) -> bool {
    // ... signature verification ...
    
    let revoked = self.revoked_tokens.blocking_read();
    !revoked.contains(&token.id)
}
```

**Change to:**
```rust
pub async fn verify_capability(&self, token: &CapabilityToken) -> bool {
    // ... signature verification ...
    
    let revoked = self.revoked_tokens.read().await;
    !revoked.contains(&token.id)
}
```

### Step 2: Update `delegate_capability` to await
**Current (line 578):**
```rust
pub fn delegate_capability(
    &self,
    parent_token: &CapabilityToken,
    new_to: WebID,
) -> Option<CapabilityToken> {
    if !self.verify_capability(parent_token) {
        return None;
    }
    // ...
}
```

**Change to:**
```rust
pub async fn delegate_capability(
    &self,
    parent_token: &CapabilityToken,
    new_to: WebID,
) -> Option<CapabilityToken> {
    if !self.verify_capability(parent_token).await {
        return None;
    }
    // ...
}
```

### Step 3: Update `verify_capability_chain` to await
**Current (line 590):**
```rust
pub fn verify_capability_chain(&self, token: &CapabilityToken) -> bool {
    if !self.verify_capability(token) {
        return false;
    }
    // ...
}
```

**Change to:**
```rust
pub async fn verify_capability_chain(&self, token: &CapabilityToken) -> bool {
    if !self.verify_capability(token).await {
        return false;
    }
    // ...
}
```

### Step 4: Update call sites
Find all calls to these methods and add `.await`.

**Expected call sites:**
- `crates/hkask-agents/src/pod.rs` — `AgentPod::delegate`
- `crates/hkask-agents/src/acp.rs` — internal calls

### Step 5: Add clippy lint
**File:** `Cargo.toml` (workspace root)

Add to `[workspace.lints.clippy]`:
```toml
blocking_read_in_async = "deny"
```

### Step 6: Verify
```bash
cargo clippy -p hkask-agents -- -D warnings
cargo test -p hkask-agents
```

**Success criteria:**
- `verify_capability` is `async fn`
- No `blocking_read` in async context
- All tests pass
- Clippy passes

---

## Task T05: Eliminate `OKAPI_DEV_KEY` Hardcoded Secret

**Goal:** Remove hardcoded 32-byte key from source. Use keystore resolution.

**Files to modify:**
1. `crates/hkask-ensemble/src/okapi_integration.rs`
2. `crates/hkask-ensemble/Cargo.toml` (add `hkask-keystore` dependency if not present)

**Detailed steps:**

### Step 1: Delete `OKAPI_DEV_KEY` constant
**Current (`crates/hkask-ensemble/src/okapi_integration.rs`, lines 20-23):**
```rust
const OKAPI_DEV_KEY: [u8; 32] = [
    0x68, 0x6b, 0x61, 0x73, 0x6b, 0x2d, 0x6f, 0x6b, 0x61, 0x70, 0x69, 0x2d, 0x64, 0x65, 0x76, 0x2d,
    0x6b, 0x65, 0x79, 0x2d, 0x32, 0x30, 0x32, 0x36, 0x2d, 0x30, 0x35, 0x2d, 0x32, 0x32, 0x21, 0x21,
];
```

**Delete entirely.**

### Step 2: Add keystore resolution function
**Add after line 19:**
```rust
use hkask_keystore::{Keychain, KeychainError};
use zeroize::Zeroizing;

/// Load Okapi HMAC key from keystore
///
/// Resolution order:
/// 1. Environment variable `HKASK_OKAPI_KEY` (for CI/testing)
/// 2. Keychain entry `okapi-cap-key`
/// 3. Generate new key and store in keychain
fn load_okapi_key() -> Result<Zeroizing<Vec<u8>>, KeychainError> {
    // Try environment variable first (for CI/testing)
    if let Ok(key) = std::env::var("HKASK_OKAPI_KEY") {
        return Ok(Zeroizing::new(key.into_bytes()));
    }

    // Try keychain
    let keychain = Keychain::default();
    match keychain.retrieve_by_key("okapi-cap-key") {
        Ok(secret) => Ok(Zeroizing::new(secret.into_bytes())),
        Err(KeychainError::NotFound(_)) => {
            // Generate new key and store
            let generated: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
            let hex_key = hex::encode(&generated);
            keychain.store_by_key("okapi-cap-key", &hex_key)?;
            Ok(Zeroizing::new(generated))
        }
        Err(e) => Err(e),
    }
}
```

### Step 3: Update `OkapiIntegration::new`
**Current (line 38):**
```rust
pub fn new(base_url: String, cns_runtime: Arc<CnsRuntime>) -> Self {
    let holder = WebID::new();
    let capability = crate::capability::default_system_capability(holder, &OKAPI_DEV_KEY);

    Self {
        base_url,
        capability,
        cns_runtime,
    }
}
```

**Change to:**
```rust
pub fn new(base_url: String, cns_runtime: Arc<CnsRuntime>) -> Result<Self, OkapiIntegrationError> {
    let holder = WebID::new();
    let key = load_okapi_key().map_err(|e| {
        OkapiIntegrationError::CapabilityError(format!("Failed to load Okapi key: {}", e))
    })?;
    let key_array: [u8; 32] = key.as_slice().try_into().map_err(|_| {
        OkapiIntegrationError::CapabilityError("Okapi key must be 32 bytes".to_string())
    })?;
    let capability = crate::capability::default_system_capability(holder, &key_array);

    Ok(Self {
        base_url,
        capability,
        cns_runtime,
    })
}
```

### Step 4: Update `verify_generate_ocap` and `verify_chat_ocap`
**Current (lines 79, 100):**
```rust
self.capability.verify(&OKAPI_DEV_KEY, &[OkapiOperation::Generate])
```

**Change to:**
```rust
let key = load_okapi_key().map_err(|e| {
    OkapiIntegrationError::CapabilityError(format!("Failed to load Okapi key: {}", e))
})?;
let key_array: [u8; 32] = key.as_slice().try_into().map_err(|_| {
    OkapiIntegrationError::CapabilityError("Okapi key must be 32 bytes".to_string())
})?;
self.capability.verify(&key_array, &[OkapiOperation::Generate])
```

### Step 5: Add CI check
**File:** `.github/workflows/ci.yml` (or create `.github/workflows/secret-scan.yml`)

```yaml
name: Secret Scan
on: [push, pull_request]
jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check for hardcoded 32-byte keys
        run: |
          if rg -n '\[0x[0-9a-fA-F]{2}(,\s*0x[0-9a-fA-F]{2}){31}\]' crates/ --type rust; then
            echo "ERROR: Hardcoded 32-byte key found in source"
            exit 1
          fi
```

### Step 6: Update documentation
**File:** `docs/architecture/security-architecture.md`

Add section:
```markdown
## Okapi Key Rotation

The Okapi HMAC key is stored in the OS keychain under `okapi-cap-key`.

**Rotation procedure:**
1. Generate new key: `openssl rand -hex 32`
2. Store in keychain: `kask keystore store okapi-cap-key <new-key>`
3. Restart hKask services
4. Old tokens will fail verification (expected)
5. Re-issue tokens to active agents
```

### Step 7: Verify
```bash
cargo check -p hkask-ensemble
cargo test -p hkask-ensemble
```

**Success criteria:**
- `OKAPI_DEV_KEY` constant deleted
- Key loaded from keystore or environment
- CI rejects hardcoded 32-byte arrays
- Documentation updated

---

## Task T06: Deterministic WebID Derivation

**Goal:** Same persona → same WebID across processes using UUID v5.

**Files to modify:**
1. `crates/hkask-types/src/id.rs`
2. `crates/hkask-agents/src/pod.rs`

**Detailed steps:**

### Step 1: Add `WebID::from_persona` using UUID v5
**File:** `crates/hkask-types/src/id.rs`

**Add after line 31:**
```rust
impl WebID {
    /// Derive WebID deterministically from persona using UUID v5
    ///
    /// Uses SHA-1 name-based UUID with a fixed namespace.
    /// Same persona bytes → same WebID.
    pub fn from_persona(persona_bytes: &[u8]) -> Self {
        // Fixed namespace UUID for hKask personas
        const HKASK_PERSONA_NAMESPACE: Uuid = Uuid::from_bytes([
            0x68, 0x6b, 0x61, 0x73, 0x6b, 0x2d, 0x70, 0x65,
            0x72, 0x73, 0x6f, 0x6e, 0x61, 0x2d, 0x6e, 0x73,
        ]);
        
        Self(Uuid::new_v5(&HKASK_PERSONA_NAMESPACE, persona_bytes))
    }
}
```

### Step 2: Update `AgentPersona::webid()`
**Current (`crates/hkask-agents/src/pod.rs`, line 207):**
```rust
pub fn webid(&self) -> WebID {
    // In production, this would be derived from a deterministic hash
    // of the persona content. For now, generate a new UUID.
    WebID::new()
}
```

**Change to:**
```rust
pub fn webid(&self) -> WebID {
    let canonical = serde_json::to_string(&self.agent).unwrap_or_default();
    WebID::from_persona(canonical.as_bytes())
}
```

### Step 3: Cache WebID in persona struct
**Current (line 180):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    pub agent: AgentPersonaConfig,
    pub charter: Charter,
}
```

**Change to:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    pub agent: AgentPersonaConfig,
    pub charter: Charter,
    #[serde(skip)]
    cached_webid: Option<WebID>,
}

impl AgentPersona {
    pub fn new(agent: AgentPersonaConfig, charter: Charter) -> Self {
        let canonical = serde_json::to_string(&agent).unwrap_or_default();
        let cached_webid = Some(WebID::from_persona(canonical.as_bytes()));
        Self { agent, charter, cached_webid }
    }

    pub fn webid(&self) -> WebID {
        self.cached_webid.unwrap_or_else(|| {
            let canonical = serde_json::to_string(&self.agent).unwrap_or_default();
            WebID::from_persona(canonical.as_bytes())
        })
    }
}
```

### Step 4: Update `AcpRuntime::default`
**Current (`crates/hkask-agents/src/acp.rs`, line 870):**
```rust
impl Default for AcpRuntime {
    fn default() -> Self {
        Self::new()
    }
}
```

**Change to:**
```rust
impl Default for AcpRuntime {
    fn default() -> Self {
        // Derive root WebID from a fixed "root" persona
        let root_persona = b"hkask-root-authority";
        let root_webid = WebID::from_persona(root_persona);
        Self::with_root(root_webid)
    }
}
```

### Step 5: Add tests
**File:** `crates/hkask-types/src/id.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn test_webid_from_persona_deterministic() {
    let persona = b"test-persona-yaml";
    let id1 = WebID::from_persona(persona);
    let id2 = WebID::from_persona(persona);
    assert_eq!(id1, id2);
}

#[test]
fn test_webid_from_persona_different() {
    let persona1 = b"persona-1";
    let persona2 = b"persona-2";
    let id1 = WebID::from_persona(persona1);
    let id2 = WebID::from_persona(persona2);
    assert_ne!(id1, id2);
}
```

### Step 6: Verify
```bash
cargo test -p hkask-types id
cargo test -p hkask-agents pod
```

**Success criteria:**
- `WebID::from_persona` exists
- Same persona → same WebID
- `AgentPersona::webid()` is deterministic
- All tests pass

---

## Task T07: Tighten `Zeroizing` Discipline

**Goal:** Secrets wrapped in `Arc<Zeroizing<Vec<u8>>>` so `Clone` doesn't copy bytes.

**Files to modify:**
1. `crates/hkask-agents/src/acp.rs`

**Detailed steps:**

### Step 1: Update `RootAuthority` secret type
**Current (`crates/hkask-agents/src/acp.rs`, line 133):**
```rust
pub struct RootAuthority {
    webid: WebID,
    secret: Zeroizing<Vec<u8>>,
}
```

**Change to:**
```rust
pub struct RootAuthority {
    webid: WebID,
    secret: Arc<Zeroizing<Vec<u8>>>,
}
```

### Step 2: Update `RootAuthority::new`
**Current (line 137):**
```rust
pub fn new(webid: WebID, secret: Vec<u8>) -> Self {
    Self {
        webid,
        secret: Zeroizing::new(secret),
    }
}
```

**Change to:**
```rust
pub fn new(webid: WebID, secret: Vec<u8>) -> Self {
    Self {
        webid,
        secret: Arc::new(Zeroizing::new(secret)),
    }
}
```

### Step 3: Update `mint_capability` to use `Arc`
**Current (line 145):**
```rust
pub fn mint_capability(&self, to: WebID, tool_name: String) -> CapabilityToken {
    CapabilityToken::new(
        // ...
        &self.secret,
    )
}
```

**Change to:**
```rust
pub fn mint_capability(&self, to: WebID, tool_name: String) -> CapabilityToken {
    CapabilityToken::new(
        // ...
        self.secret.as_ref(),
    )
}
```

### Step 4: Update `AcpRuntime` secret type
**Current (line 293):**
```rust
pub struct AcpRuntime {
    // ...
    secret: Zeroizing<Vec<u8>>,
}
```

**Change to:**
```rust
pub struct AcpRuntime {
    // ...
    secret: Arc<Zeroizing<Vec<u8>>>,
}
```

### Step 5: Update `AcpRuntime::new`
**Current (line 297):**
```rust
pub fn new() -> Self {
    let root = RootAuthority::default();
    Self {
        // ...
        secret: root.secret.clone(),  // ← Copies bytes
    }
}
```

**Change to:**
```rust
pub fn new() -> Self {
    let root = RootAuthority::default();
    Self {
        // ...
        secret: Arc::clone(&root.secret),  // ← Clones Arc, not bytes
    }
}
```

### Step 6: Add `ZeroizeOnDrop` derive where feasible
**File:** `crates/hkask-agents/src/acp.rs`

Add to `RootAuthority`:
```rust
#[derive(Clone, zeroize::ZeroizeOnDrop)]
pub struct RootAuthority {
    webid: WebID,
    #[zeroize(skip)]
    webid: WebID,
    secret: Arc<Zeroizing<Vec<u8>>>,
}
```

### Step 7: Verify
```bash
cargo check -p hkask-agents
cargo test -p hkask-agents
```

**Success criteria:**
- `RootAuthority::secret` is `Arc<Zeroizing<Vec<u8>>>`
- `AcpRuntime::secret` is `Arc<Zeroizing<Vec<u8>>>`
- `Clone` doesn't copy bytes
- All tests pass

---

## Task T08: Unify Capability Primitive (Single Miller-style Cap)

**Goal:** One unforgeable token type with caveats. Delete `Macaroon` and `OkapiCapability`.

**Files to modify:**
1. `crates/hkask-types/src/capability.rs` — Add `Caveat` type and `caveats` field
2. `crates/hkask-ensemble/src/macaroon.rs` — **DELETE ENTIRE FILE**
3. `crates/hkask-ensemble/src/capability.rs` — **DELETE ENTIRE FILE**
4. `crates/hkask-ensemble/src/okapi_integration.rs` — Use `CapabilityToken` directly
5. `crates/hkask-ensemble/src/lib.rs` — Remove `pub mod macaroon;` and `pub mod capability;`
6. All call sites in `hkask-ensemble`

**Detailed steps:**

### Step 1: Add `Caveat` type to `hkask-types`
**File:** `crates/hkask-types/src/capability.rs`

**Add after line 115 (after `CapabilityAction`):**
```rust
/// Caveat — Additional restriction on a capability
///
/// Caveats are additive restrictions beyond the base resource/action/expiry.
/// Modeled after Macaroon caveats (Google, 2014).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caveat {
    /// Caveat type identifier
    pub caveat_id: String,
    /// Caveat data (type-specific)
    pub data: String,
}

impl Caveat {
    /// Create expiration caveat
    pub fn expiration(unix_timestamp: i64) -> Self {
        Self {
            caveat_id: "expiration".to_string(),
            data: unix_timestamp.to_string(),
        }
    }

    /// Create operation caveat
    pub fn operation(op: &str) -> Self {
        Self {
            caveat_id: "operation".to_string(),
            data: op.to_string(),
        }
    }

    /// Create template caveat
    pub fn template(template_id: &str) -> Self {
        Self {
            caveat_id: "template".to_string(),
            data: template_id.to_string(),
        }
    }

    /// Create visibility caveat
    pub fn visibility(vis: &str) -> Self {
        Self {
            caveat_id: "visibility".to_string(),
            data: vis.to_string(),
        }
    }
}
```

### Step 2: Add `caveats` field to `CapabilityToken`
**Current (line 119):**
```rust
pub struct CapabilityToken {
    pub id: String,
    pub resource: CapabilityResource,
    pub resource_id: String,
    pub action: CapabilityAction,
    pub delegated_from: WebID,
    pub delegated_to: WebID,
    pub signature: String,
    pub expires_at: Option<i64>,
    pub attenuation_level: u8,
    pub max_attenuation: u8,
    pub context_nonce: String,
}
```

**Change to:**
```rust
pub struct CapabilityToken {
    pub id: String,
    pub resource: CapabilityResource,
    pub resource_id: String,
    pub action: CapabilityAction,
    pub delegated_from: WebID,
    pub delegated_to: WebID,
    pub signature: String,
    pub expires_at: Option<i64>,
    pub attenuation_level: u8,
    pub max_attenuation: u8,
    pub context_nonce: String,
    /// Additional restrictions (caveats)
    #[serde(default)]
    pub caveats: Vec<Caveat>,
}
```

### Step 3: Add `attenuate_with_caveat` method
**Add after line 335 (after `attenuate_with_expiry`):**
```rust
/// Create attenuated child token with additional caveat
pub fn attenuate_with_caveat(
    &self,
    new_to: WebID,
    caveat: Caveat,
    secret: &[u8],
) -> Option<CapabilityToken> {
    if !self.can_attenuate() {
        return None;
    }

    let mut child = CapabilityToken::new_with_attenuation(
        self.resource,
        self.resource_id.clone(),
        self.action,
        self.delegated_to,
        new_to,
        secret,
        self.expires_at,
        self.attenuation_level + 1,
        self.max_attenuation,
        Some(format!("{}-attenuated-{}", self.context_nonce, uuid::Uuid::new_v4())),
    );
    
    child.caveats = self.caveats.clone();
    child.caveats.push(caveat);
    
    Some(child)
}
```

### Step 4: Add caveat verification
**Add after line 415 (after `verify_attenuation_chain`):**
```rust
/// Verify all caveats are satisfied
pub fn verify_caveats(&self, current_time: i64, context: &CaveatContext) -> bool {
    for caveat in &self.caveats {
        match caveat.caveat_id.as_str() {
            "expiration" => {
                if let Ok(expiry) = caveat.data.parse::<i64>() {
                    if current_time > expiry {
                        return false;
                    }
                }
            }
            "operation" => {
                if !context.allowed_operations.is_empty()
                    && !context.allowed_operations.contains(&caveat.data)
                {
                    return false;
                }
            }
            "template" => {
                if let Some(ref tid) = context.template_id {
                    if tid != &caveat.data {
                        return false;
                    }
                }
            }
            "visibility" => {
                if !context.visibility.is_empty() && context.visibility != caveat.data {
                    return false;
                }
            }
            _ => return false, // Unknown caveat = deny
        }
    }
    true
}
```

**Add `CaveatContext` struct:**
```rust
/// Context for caveat verification
#[derive(Debug, Default)]
pub struct CaveatContext {
    pub allowed_operations: Vec<String>,
    pub template_id: Option<String>,
    pub visibility: String,
}
```

### Step 5: Delete `macaroon.rs` and `capability.rs` from `hkask-ensemble`
```bash
rm crates/hkask-ensemble/src/macaroon.rs
rm crates/hkask-ensemble/src/capability.rs
```

### Step 6: Update `hkask-ensemble/src/lib.rs`
**Current:**
```rust
pub mod macaroon;
pub mod capability;
```

**Change to:**
```rust
// DELETE: pub mod macaroon;
// DELETE: pub mod capability;
```

### Step 7: Update `okapi_integration.rs`
Replace all `OkapiCapability` with `CapabilityToken`.

**Current (line 28):**
```rust
capability: OkapiCapability,
```

**Change to:**
```rust
capability: CapabilityToken,
```

**Current (line 40):**
```rust
let capability = crate::capability::default_system_capability(holder, &key_array);
```

**Change to:**
```rust
let capability = CapabilityToken::new(
    CapabilityResource::Tool,
    "okapi:generate".to_string(),
    CapabilityAction::Execute,
    WebID::from_persona(b"hkask-system"),
    holder,
    key.as_ref(),
);
```

### Step 8: Update all call sites
Find all imports of `crate::macaroon::*` and `crate::capability::*` in `hkask-ensemble` and replace with `hkask_types::CapabilityToken`.

### Step 9: Verify
```bash
cargo check -p hkask-ensemble
cargo test -p hkask-ensemble
cargo check --workspace
```

**Success criteria:**
- `Macaroon` and `OkapiCapability` deleted
- `CapabilityToken` has `caveats` field
- All tests pass
- No compilation errors

---

## Task T09: Replace MCP `call_tool` Stub with Real Transport

**Goal:** `McpRuntime::call_tool` dispatches over real `McpTransport` port.

**Files to modify:**
1. `crates/hkask-mcp/src/runtime.rs` — Add `McpTransport` trait and implementations
2. `crates/hkask-agents/src/adapters/mcp_runtime.rs` — Use real transport

**Detailed steps:**

### Step 1: Define `McpTransport` trait
**File:** `crates/hkask-mcp/src/runtime.rs`

**Add after line 27:**
```rust
use async_trait::async_trait;

/// MCP transport port — abstract interface for tool dispatch
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Call a tool on an MCP server
    async fn call(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String>;
}
```

### Step 2: Implement `InProcessMcpTransport`
**Add after `McpTransport` trait:**
```rust
/// In-process MCP transport (for co-located servers)
pub struct InProcessMcpTransport {
    handlers: Arc<RwLock<HashMap<String, Box<dyn Fn(Value) -> Result<Value, String> + Send + Sync>>>>,
}

impl InProcessMcpTransport {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_handler<F>(&self, tool_name: &str, handler: F)
    where
        F: Fn(Value) -> Result<Value, String> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(tool_name.to_string(), Box::new(handler));
    }
}

#[async_trait]
impl McpTransport for InProcessMcpTransport {
    async fn call(
        &self,
        _server_id: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let handlers = self.handlers.read().await;
        let handler = handlers
            .get(tool_name)
            .ok_or_else(|| format!("Tool '{}' not registered", tool_name))?;
        handler(arguments)
    }
}
```

### Step 3: Update `McpRuntime` to hold transports
**Current (line 56):**
```rust
pub struct McpRuntime {
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    tool_registry: Arc<RwLock<HashMap<String, String>>>,
}
```

**Change to:**
```rust
pub struct McpRuntime {
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
    tool_registry: Arc<RwLock<HashMap<String, String>>>,
    transports: Arc<RwLock<HashMap<String, Arc<dyn McpTransport>>>>,
}
```

### Step 4: Update `McpRuntime::new`
**Current (line 65):**
```rust
pub fn new() -> Self {
    Self {
        servers: Arc::new(RwLock::new(HashMap::new())),
        tool_registry: Arc::new(RwLock::new(HashMap::new())),
    }
}
```

**Change to:**
```rust
pub fn new() -> Self {
    Self {
        servers: Arc::new(RwLock::new(HashMap::new())),
        tool_registry: Arc::new(RwLock::new(HashMap::new())),
        transports: Arc::new(RwLock::new(HashMap::new())),
    }
}
```

### Step 5: Add `register_transport` method
**Add after `register_server`:**
```rust
/// Register a transport for a server
pub async fn register_transport(&self, server_id: &str, transport: Arc<dyn McpTransport>) {
    let mut transports = self.transports.write().await;
    transports.insert(server_id.to_string(), transport);
}
```

### Step 6: Replace `call_tool` stub
**Current (line 177):**
```rust
pub async fn call_tool(
    &self,
    server_id: &str,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !self.tool_exists(tool_name).await {
        return Err(format!("Tool '{}' not found", tool_name));
    }

    Ok(serde_json::json!({
        "server": server_id,
        "tool": tool_name,
        "arguments": arguments,
        "result": "simulated"
    }))
}
```

**Change to:**
```rust
pub async fn call_tool(
    &self,
    server_id: &str,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !self.tool_exists(tool_name).await {
        return Err(format!("Tool '{}' not found", tool_name));
    }

    let transports = self.transports.read().await;
    let transport = transports
        .get(server_id)
        .ok_or_else(|| format!("No transport registered for server '{}'", server_id))?;

    transport.call(server_id, tool_name, arguments).await
}
```

### Step 7: Update `McpRuntimeAdapter`
**File:** `crates/hkask-agents/src/adapters/mcp_runtime.rs`

**Current (line 36):**
```rust
fn invoke_tool(
    &self,
    tool_name: &str,
    input: serde_json::Value,
    token: &CapabilityToken,
) -> Result<serde_json::Value, McpError> {
    let token_id = token.id.clone();
    if token_id.is_empty() {
        return Err(McpError::CapabilityDenied(
            "Invalid capability token".to_string(),
        ));
    }

    Ok(serde_json::json!({
        "tool": tool_name,
        "status": "invoked",
        "input": input
    }))
}
```

**Change to:**
```rust
fn invoke_tool(
    &self,
    tool_name: &str,
    input: serde_json::Value,
    token: &CapabilityToken,
) -> Result<serde_json::Value, McpError> {
    // Delegate to real MCP runtime
    let runtime = hkask_mcp::McpRuntime::new();
    // In production, this would be injected. For now, create a new instance.
    // TODO: Inject McpRuntime via constructor
    
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            runtime
                .call_tool("default", tool_name, input)
                .await
                .map_err(|e| McpError::InvocationFailed(e))
        })
    })
}
```

### Step 8: Add failing test
**File:** `crates/hkask-mcp/src/runtime.rs` (in `#[cfg(test)] mod tests`)

```rust
#[tokio::test]
async fn test_call_tool_no_simulated() {
    let runtime = McpRuntime::new();
    let server = McpServer {
        id: "test".to_string(),
        name: "test".to_string(),
        tools: vec![McpTool {
            name: "tool:test".to_string(),
            description: "test".to_string(),
            input_schema: Value::Null,
            server_id: "test".to_string(),
        }],
        connected: true,
    };
    runtime.register_server(server).await;
    
    // Should fail because no transport registered
    let result = runtime.call_tool("test", "tool:test", Value::Null).await;
    assert!(result.is_err());
    assert!(!result.unwrap_err().contains("simulated"));
}
```

### Step 9: Verify
```bash
cargo test -p hkask-mcp
cargo check -p hkask-agents
```

**Success criteria:**
- `McpTransport` trait defined
- `call_tool` dispatches to real transport
- No `"simulated"` string in responses
- Test fails if no transport registered

---

## Task T10: Make `McpPort` Async

**Goal:** Eliminate `block_in_place`/`block_on` in `McpDispatcher`.

**Files to modify:**
1. `crates/hkask-templates/src/ports.rs` — Make `McpPort` async
2. `crates/hkask-templates/src/manifest.rs` — Make `ManifestExecutor::execute` async
3. `crates/hkask-mcp/src/dispatch.rs` — Remove `block_in_place`
4. All call sites

**Detailed steps:**

### Step 1: Make `McpPort` async
**Current (`crates/hkask-templates/src/ports.rs`, line 200):**
```rust
pub trait McpPort {
    fn discover_tools(&self) -> Vec<String>;
    fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
    fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}
```

**Change to:**
```rust
#[async_trait::async_trait]
pub trait McpPort: Send + Sync {
    async fn discover_tools(&self) -> Vec<String>;
    async fn invoke(&self, tool_name: &str, input: Value) -> Result<Value>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}
```

### Step 2: Make `ManifestExecutor::execute` async
**Current (`crates/hkask-templates/src/ports.rs`, line 123):**
```rust
pub trait ManifestExecutor {
    fn load(&self, path: &Path) -> Result<ProcessManifest>;
    fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value>;
}
```

**Change to:**
```rust
#[async_trait::async_trait]
pub trait ManifestExecutor: Send + Sync {
    fn load(&self, path: &Path) -> Result<ProcessManifest>;
    async fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value>;
}
```

### Step 3: Update `ManifestExecutorImpl`
**Current (`crates/hkask-templates/src/manifest.rs`, line 488):**
```rust
fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
    // ...
}
```

**Change to:**
```rust
async fn execute(&self, manifest: &ProcessManifest, input: Value) -> Result<Value> {
    // ...
    for step in &manifest.steps {
        let step_result = self.execute_step(manifest, step, state.clone(), 0, &mut energy).await?;
        state = merge_state(state, step_result);
    }
    // ...
}
```

### Step 4: Make `execute_step` async
**Current (line 200):**
```rust
fn execute_step(
    &self,
    manifest: &ProcessManifest,
    step: &ManifestStep,
    input: Value,
    depth: u8,
    energy: &mut EnergyAccount,
) -> Result<Value> {
    // ...
    let tool_result = self.mcp.invoke(&step.mcp, input)?;
    // ...
}
```

**Change to:**
```rust
async fn execute_step(
    &self,
    manifest: &ProcessManifest,
    step: &ManifestStep,
    input: Value,
    depth: u8,
    energy: &mut EnergyAccount,
) -> Result<Value> {
    // ...
    let tool_result = self.mcp.invoke(&step.mcp, input).await?;
    // ...
}
```

### Step 5: Remove `block_in_place` from `McpDispatcher`
**Current (`crates/hkask-mcp/src/dispatch.rs`, line 104):**
```rust
impl McpPort for McpDispatcher {
    fn discover_tools(&self) -> Vec<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.runtime.discover_tools())
        })
    }

    fn invoke(&self, tool_name: &str, input: Value) -> Result<Value> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                // ...
            })
        })
    }

    fn get_tool_info(&self, tool_name: &str) -> Option<hkask_templates::ports::ToolInfo> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                // ...
            })
        })
    }
}
```

**Change to:**
```rust
#[async_trait::async_trait]
impl McpPort for McpDispatcher {
    async fn discover_tools(&self) -> Vec<String> {
        self.runtime.discover_tools().await
    }

    async fn invoke(&self, tool_name: &str, input: Value) -> Result<Value> {
        let tool_info = self.runtime.get_tool_info(tool_name).await.ok_or_else(|| {
            TemplateError::Mcp(format!("Tool not found: {}", tool_name))
        })?;

        self.runtime
            .call_tool(&tool_info.server_id, tool_name, input)
            .await
            .map_err(|e| TemplateError::Mcp(format!("Tool call failed: {}", e)))
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<hkask_templates::ports::ToolInfo> {
        self.runtime.get_tool_info(tool_name).await.map(|t| {
            hkask_templates::ports::ToolInfo {
                name: t.name,
                description: t.description,
                input_schema: t.input_schema,
                server_id: t.server_id,
                required_capability: t.required_capability,
                rate_limit_hint: t.rate_limit_hint,
            }
        })
    }
}
```

### Step 6: Update call sites
Find all calls to `ManifestExecutor::execute` and add `.await`.

**Expected call sites:**
- `crates/hkask-api/src/routes.rs`
- `crates/hkask-ensemble/src/chat.rs`
- `crates/hkask-cli/src/main.rs` (use `block_on`)

### Step 7: Add clippy lint
**File:** `Cargo.toml` (workspace root)

Add to `[workspace.lints.clippy]`:
```toml
block_in_place = "deny"
```

### Step 8: Verify
```bash
cargo clippy -p hkask-mcp -- -D warnings
cargo clippy -p hkask-templates -- -D warnings
cargo test -p hkask-templates
cargo test -p hkask-mcp
```

**Success criteria:**
- `McpPort` is `#[async_trait]`
- No `block_in_place` in library code
- All tests pass
- Clippy passes

---

## Task T11: Wire `MemoryStoragePort` into Pod Lifecycle

**Goal:** Remove `_memory_storage` dead field. Persist artifacts on pod events.

**Files to modify:**
1. `crates/hkask-agents/src/pod.rs`

**Detailed steps:**

### Step 1: Remove underscore from `_memory_storage`
**Current (`crates/hkask-agents/src/pod.rs`, line 713):**
```rust
pub struct PodManager {
    // ...
    _memory_storage: Arc<dyn MemoryStoragePort>,
}
```

**Change to:**
```rust
pub struct PodManager {
    // ...
    memory_storage: Arc<dyn MemoryStoragePort>,
}
```

### Step 2: Update `PodManager::new`
**Current (line 720):**
```rust
pub fn new(
    git_cas: impl GitCASPort + 'static,
    acp_runtime: Arc<AcpRuntime>,
    cns_emitter: impl CnsEmit + 'static,
    mcp_runtime: impl MCPRuntimePort + 'static,
    memory_storage: impl MemoryStoragePort + 'static,
) -> Self {
    Self {
        // ...
        _memory_storage: Arc::new(memory_storage),
    }
}
```

**Change to:**
```rust
pub fn new(
    git_cas: impl GitCASPort + 'static,
    acp_runtime: Arc<AcpRuntime>,
    cns_emitter: impl CnsEmit + 'static,
    mcp_runtime: impl MCPRuntimePort + 'static,
    memory_storage: impl MemoryStoragePort + 'static,
) -> Self {
    Self {
        // ...
        memory_storage: Arc::new(memory_storage),
    }
}
```

### Step 3: Persist artifact on `register`
**Current (line 750):**
```rust
pub async fn register(&self, pod: AgentPod) -> AgentPodResult<()> {
    // ...
    self.acp_runtime.register_agent(pod.webid(), "Bot", vec![]).await?;
    // ...
}
```

**Change to:**
```rust
pub async fn register(&self, pod: AgentPod) -> AgentPodResult<()> {
    // ...
    self.acp_runtime.register_agent(pod.webid(), "Bot", vec![]).await?;
    
    // Persist registration event
    let event = serde_json::json!({
        "event": "registered",
        "webid": pod.webid().to_string(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    self.memory_storage.store_artifact(
        pod.webid(),
        "episodic_triple",
        event,
        "private",
        pod.capability_token(),
    )?;
    
    // ...
}
```

### Step 4: Persist artifact on `activate`
**Current (line 770):**
```rust
pub async fn activate(&self, pod_id: &PodID) -> AgentPodResult<()> {
    // ...
}
```

**Change to:**
```rust
pub async fn activate(&self, pod_id: &PodID) -> AgentPodResult<()> {
    // ...
    
    // Persist activation event
    let event = serde_json::json!({
        "event": "activated",
        "pod_id": pod_id.to_string(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });
    let pod = self.get_pod(pod_id)?;
    self.memory_storage.store_artifact(
        pod.webid(),
        "episodic_triple",
        event,
        "private",
        pod.capability_token(),
    )?;
    
    // ...
}
```

### Step 5: Persist artifact on `deactivate`
Similar to `activate`.

### Step 6: Persist artifact on `delegate`
Similar pattern.

### Step 7: Add `recall` method
**Add to `PodManager`:**
```rust
/// Recall artifacts for a pod
pub fn recall(&self, pod_id: &PodID, query: &str) -> AgentPodResult<Vec<Value>> {
    let pod = self.get_pod(pod_id)?;
    let results = self.memory_storage.recall(query, pod.capability_token())?;
    Ok(results)
}
```

### Step 8: Verify
```bash
cargo check -p hkask-agents
cargo test -p hkask-agents
```

**Success criteria:**
- `_memory_storage` renamed to `memory_storage`
- Artifacts persisted on register/activate/deactivate/delegate
- `recall` method exists
- All tests pass

---

## Task T12: Persist Revocation List and Fix AuditLogPort

**Goal:** Revocation survives restart. AuditLogPort writes to SQLite.

**Files to modify:**
1. `crates/hkask-storage/src/lib.rs` — Add `RevocationStore`
2. `crates/hkask-storage/src/revocation.rs` — **NEW FILE**
3. `crates/hkask-agents/src/acp.rs` — Use `RevocationStore`
4. `crates/hkask-agents/src/acp.rs` — Fix `AuditLogPort` impl

**Detailed steps:**

### Step 1: Create `RevocationStore`
**File:** `crates/hkask-storage/src/revocation.rs` (NEW)

```rust
//! RevocationStore — Persistent SQL-backed token revocation list

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RevocationError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

pub struct RevocationStore {
    conn: Arc<Mutex<Connection>>,
}

impl RevocationStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(&self) -> Result<(), RevocationError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS revoked_tokens (
                token_id TEXT PRIMARY KEY,
                revoked_at TEXT NOT NULL,
                reason TEXT
            );",
        )?;
        Ok(())
    }

    pub fn revoke(&self, token_id: &str, reason: Option<&str>) -> Result<(), RevocationError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO revoked_tokens (token_id, revoked_at, reason) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                token_id,
                Utc::now().to_rfc3339(),
                reason,
            ],
        )?;
        Ok(())
    }

    pub fn is_revoked(&self, token_id: &str) -> Result<bool, RevocationError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM revoked_tokens WHERE token_id = ?1",
            [token_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn list_revoked(&self) -> Result<Vec<String>, RevocationError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT token_id FROM revoked_tokens")?;
        let ids = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }
}
```

### Step 2: Export from `hkask-storage`
**File:** `crates/hkask-storage/src/lib.rs`

**Add:**
```rust
pub mod revocation;
pub use revocation::{RevocationError, RevocationStore};
```

### Step 3: Update `AcpRuntime` to use `RevocationStore`
**Current (`crates/hkask-agents/src/acp.rs`, line 293):**
```rust
pub struct AcpRuntime {
    // ...
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}
```

**Change to:**
```rust
pub struct AcpRuntime {
    // ...
    revoked_tokens: Arc<hkask_storage::RevocationStore>,
}
```

### Step 4: Update `AcpRuntime::new`
**Current (line 297):**
```rust
pub fn new() -> Self {
    Self {
        // ...
        revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
    }
}
```

**Change to:**
```rust
pub fn new() -> Self {
    let conn = Arc::new(Mutex::new(
        rusqlite::Connection::open_in_memory().expect("Failed to open in-memory DB")
    ));
    let revoked_store = hkask_storage::RevocationStore::new(conn);
    revoked_store.init_schema().expect("Failed to init schema");
    
    Self {
        // ...
        revoked_tokens: Arc::new(revoked_store),
    }
}
```

### Step 5: Update `revoke_capability`
**Current (line 520):**
```rust
pub fn revoke_capability(&self, token_id: &str) {
    let mut revoked = self.revoked_tokens.write().unwrap();
    revoked.insert(token_id.to_string());
}
```

**Change to:**
```rust
pub fn revoke_capability(&self, token_id: &str) -> Result<(), hkask_storage::RevocationError> {
    self.revoked_tokens.revoke(token_id, None)?;
    Ok(())
}
```

### Step 6: Update `verify_capability`
**Current (line 543):**
```rust
let revoked = self.revoked_tokens.read().unwrap();
!revoked.contains(&token.id)
```

**Change to:**
```rust
let is_revoked = self.revoked_tokens.is_revoked(&token.id).unwrap_or(false);
!is_revoked
```

### Step 7: Fix `AuditLogPort` impl
**Current (line 839):**
```rust
impl AuditLogPort for AuditLog {
    fn log(&self, entry: AuditEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.push(entry);
    }

    fn get_recent(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read().unwrap();
        entries.iter().rev().take(limit).cloned().collect()
    }
}
```

**Change to:**
```rust
impl AuditLogPort for AuditLog {
    fn log(&self, entry: AuditEntry) {
        // Write to in-memory
        let mut entries = self.entries.write().unwrap();
        entries.push(entry.clone());
        
        // Write to storage if available
        if let Some(ref store) = self.store {
            let _ = store.insert(&entry);
        }
    }

    fn get_recent(&self, limit: usize) -> Vec<AuditEntry> {
        // Try storage first
        if let Some(ref store) = self.store {
            if let Ok(entries) = store.query_recent(limit) {
                return entries;
            }
        }
        
        // Fallback to in-memory
        let entries = self.entries.read().unwrap();
        entries.iter().rev().take(limit).cloned().collect()
    }
}
```

### Step 8: Verify
```bash
cargo check -p hkask-storage
cargo check -p hkask-agents
cargo test -p hkask-storage
cargo test -p hkask-agents
```

**Success criteria:**
- `RevocationStore` exists
- `AcpRuntime` uses `RevocationStore`
- `AuditLogPort` writes to storage
- All tests pass

---

## Task T13: CNS Spans on Every Capability Mutation

**Goal:** Emit `cns.cap.*` spans on mint, attenuate, revoke, verify.

**Files to modify:**
1. `crates/hkask-agents/src/acp.rs`
2. `crates/hkask-agents/src/pod.rs`

**Detailed steps:**

### Step 1: Add `cns` field to `AcpRuntime`
**Current (`crates/hkask-agents/src/acp.rs`, line 293):**
```rust
pub struct AcpRuntime {
    // ...
}
```

**Change to:**
```rust
pub struct AcpRuntime {
    // ...
    cns: Option<Arc<dyn hkask_cns::CnsEmit>>,
}
```

### Step 2: Update `AcpRuntime::new`
**Change to:**
```rust
pub fn new() -> Self {
    Self {
        // ...
        cns: None,
    }
}

pub fn with_cns(mut self, cns: Arc<dyn hkask_cns::CnsEmit>) -> Self {
    self.cns = Some(cns);
    self
}
```

### Step 3: Emit span on `mint_capability`
**Current (line 145):**
```rust
pub fn mint_capability(&self, to: WebID, tool_name: String) -> CapabilityToken {
    let token = self.root.mint_capability(to, tool_name);
    token
}
```

**Change to:**
```rust
pub fn mint_capability(&self, to: WebID, tool_name: String) -> CapabilityToken {
    let token = self.root.mint_capability(to, tool_name);
    
    if let Some(ref cns) = self.cns {
        cns.emit(
            "cns.cap.minted",
            serde_json::json!({
                "token_id": token.id,
                "holder": token.delegated_to.to_string(),
                "resource": token.resource_id,
            }),
            1.0,
        );
    }
    
    token
}
```

### Step 4: Emit span on `revoke_capability`
**Current (line 520):**
```rust
pub fn revoke_capability(&self, token_id: &str) {
    // ...
}
```

**Change to:**
```rust
pub fn revoke_capability(&self, token_id: &str) -> Result<(), hkask_storage::RevocationError> {
    self.revoked_tokens.revoke(token_id, None)?;
    
    if let Some(ref cns) = self.cns {
        cns.emit(
            "cns.cap.revoked",
            serde_json::json!({
                "token_id": token_id,
            }),
            1.0,
        );
    }
    
    Ok(())
}
```

### Step 5: Emit span on `delegate_capability`
**Current (line 578):**
```rust
pub async fn delegate_capability(
    &self,
    parent_token: &CapabilityToken,
    new_to: WebID,
) -> Option<CapabilityToken> {
    // ...
}
```

**Change to:**
```rust
pub async fn delegate_capability(
    &self,
    parent_token: &CapabilityToken,
    new_to: WebID,
) -> Option<CapabilityToken> {
    if !self.verify_capability(parent_token).await {
        return None;
    }
    
    let child = parent_token.attenuate(new_to, &self.secret)?;
    
    if let Some(ref cns) = self.cns {
        cns.emit(
            "cns.cap.attenuated",
            serde_json::json!({
                "parent_id": parent_token.id,
                "child_id": child.id,
                "attenuation_level": child.attenuation_level,
                "holder": child.delegated_to.to_string(),
            }),
            1.0,
        );
    }
    
    Some(child)
}
```

### Step 6: Emit span on `verify_capability`
**Current (line 543):**
```rust
pub async fn verify_capability(&self, token: &CapabilityToken) -> bool {
    // ...
}
```

**Change to:**
```rust
pub async fn verify_capability(&self, token: &CapabilityToken) -> bool {
    let valid = /* ... verification logic ... */;
    
    if let Some(ref cns) = self.cns {
        let span_name = if valid {
            "cns.cap.verified_ok"
        } else {
            "cns.cap.verified_denied"
        };
        cns.emit(
            span_name,
            serde_json::json!({
                "token_id": token.id,
                "holder": token.delegated_to.to_string(),
            }),
            1.0,
        );
    }
    
    valid
}
```

### Step 7: Emit span in `AgentPod::delegate`
**File:** `crates/hkask-agents/src/pod.rs`

**Current (line 496):**
```rust
pub async fn delegate(&self, new_to: WebID) -> Option<CapabilityToken> {
    self.acp_runtime.delegate_capability(&self.capability_token, new_to).await
}
```

**Change to:**
```rust
pub async fn delegate(&self, new_to: WebID) -> Option<CapabilityToken> {
    let child = self.acp_runtime.delegate_capability(&self.capability_token, new_to).await;
    
    // CNS span emitted by AcpRuntime::delegate_capability
    
    child
}
```

### Step 8: Verify
```bash
cargo check -p hkask-agents
cargo test -p hkask-agents
```

**Success criteria:**
- `cns.cap.minted` emitted on mint
- `cns.cap.attenuated` emitted on delegate
- `cns.cap.revoked` emitted on revoke
- `cns.cap.verified_ok` / `cns.cap.verified_denied` emitted on verify
- All tests pass

---

## Task T14: Russell ↔ hKask Symmetric ACP Bridge

**Goal:** Bidirectional ACP communication between hKask and Russell.

**Files to modify:**
1. `crates/hkask-agents/src/adapters/russell_acp.rs` — **NEW FILE**
2. `crates/hkask-agents/src/adapters/mod.rs` — Export
3. `registry/manifests/russell-mapping.yaml` — **NEW FILE**

**Detailed steps:**

### Step 1: Create `RussellAcpAdapter`
**File:** `crates/hkask-agents/src/adapters/russell_acp.rs` (NEW)

```rust
//! Russell ACP Adapter — Bidirectional ACP bridge to Russell

use crate::acp::{A2AMessage, AcpError};
use crate::ports::{AcpPort, AcpTransport, AcpWireMessage, AcpWireResponse};
use async_trait::async_trait;
use hkask_types::{CapabilityToken, WebID};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tracing::{debug, error, info};

/// Russell JSON-RPC request (matches russell-acp-server types)
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<AuthInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acp_version: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthInfo {
    auth_type: String,
    token: String,
}

/// Russell JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// Russell ACP adapter — spawns Russell as child process
pub struct RussellAcpAdapter {
    child: tokio::sync::Mutex<Option<Child>>,
    russell_binary: String,
    macaroon_token: Option<String>,
}

impl RussellAcpAdapter {
    pub fn new(russell_binary: String) -> Self {
        Self {
            child: tokio::sync::Mutex::new(None),
            russell_binary,
            macaroon_token: None,
        }
    }

    pub fn with_auth(mut self, token: String) -> Self {
        self.macaroon_token = Some(token);
        self
    }

    async fn ensure_started(&self) -> Result<(), AcpError> {
        let mut child_opt = self.child.lock().await;
        if child_opt.is_none() {
            let child = Command::new(&self.russell_binary)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| AcpError::TransportError(format!("Failed to spawn Russell: {}", e)))?;
            *child_opt = Some(child);
            info!("Russell ACP adapter started");
        }
        Ok(())
    }

    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, AcpError> {
        self.ensure_started().await?;
        
        let mut child_opt = self.child.lock().await;
        let child = child_opt.as_mut().ok_or_else(|| {
            AcpError::TransportError("Russell not started".to_string())
        })?;
        
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            AcpError::TransportError("Russell stdin not available".to_string())
        })?;
        
        let json = serde_json::to_string(&request)
            .map_err(|e| AcpError::TransportError(format!("Serialization failed: {}", e)))?;
        
        stdin.write_all(json.as_bytes()).await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {}", e)))?;
        stdin.write_all(b"\n").await
            .map_err(|e| AcpError::TransportError(format!("Write failed: {}", e)))?;
        stdin.flush().await
            .map_err(|e| AcpError::TransportError(format!("Flush failed: {}", e)))?;
        
        let stdout = child.stdout.as_mut().ok_or_else(|| {
            AcpError::TransportError("Russell stdout not available".to_string())
        })?;
        
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).await
            .map_err(|e| AcpError::TransportError(format!("Read failed: {}", e)))?;
        
        let response: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| AcpError::TransportError(format!("Parse failed: {}", e)))?;
        
        Ok(response)
    }
}

#[async_trait]
impl AcpTransport for RussellAcpAdapter {
    async fn send(&self, msg: &AcpWireMessage) -> Result<AcpWireResponse, AcpError> {
        let method = match &msg.payload {
            A2AMessage::TemplateDispatch(_) => "acp/session.message",
            A2AMessage::TemplateResponse(_) => "acp/session.message",
            _ => "acp/session.message",
        };
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String(msg.id.clone()),
            method: method.to_string(),
            params: Some(serde_json::to_value(&msg.payload).unwrap_or_default()),
            auth: self.macaroon_token.as_ref().map(|t| AuthInfo {
                auth_type: "macaroon".to_string(),
                token: t.clone(),
            }),
            acp_version: Some("0.1.0".to_string()),
        };
        
        let response = self.send_request(request).await?;
        
        if let Some(error) = response.error {
            return Err(AcpError::TransportError(format!(
                "Russell error {}: {}",
                error.code, error.message
            )));
        }
        
        Ok(AcpWireResponse {
            id: msg.id.clone(),
            success: true,
            result: response.result,
            error: None,
        })
    }

    async fn receive(&self) -> Result<AcpWireMessage, AcpError> {
        Err(AcpError::TransportError(
            "RussellAcpAdapter does not support receive(); use push model".to_string(),
        ))
    }

    fn is_connected(&self) -> bool {
        true
    }
}
```

### Step 2: Export from `adapters/mod.rs`
**File:** `crates/hkask-agents/src/adapters/mod.rs`

**Add:**
```rust
pub mod russell_acp;
pub use russell_acp::RussellAcpAdapter;
```

### Step 3: Create capability mapping
**File:** `registry/manifests/russell-mapping.yaml` (NEW)

```yaml
# Russell ↔ hKask capability mapping
# Maps hLexicon terms to Russell symptom codes

hlexicon_to_symptom:
  "diagnosis": "system_diagnosis"
  "remediation": "system_remediation"
  "monitoring": "system_monitoring"
  "security": "security_audit"

symptom_to_hlexicon:
  "system_diagnosis": "diagnosis"
  "system_remediation": "remediation"
  "system_monitoring": "monitoring"
  "security_audit": "security"
```

### Step 4: Add CNS span for federation
**File:** `crates/hkask-agents/src/adapters/russell_acp.rs`

**Add to `send` method:**
```rust
if let Some(ref cns) = self.cns {
    cns.emit(
        "cns.federation.translated",
        serde_json::json!({
            "direction": "hKask→Russell",
            "method": method,
        }),
        1.0,
    );
}
```

### Step 5: Verify
```bash
cargo check -p hkask-agents
```

**Success criteria:**
- `RussellAcpAdapter` exists
- Spawns Russell as child process
- Communicates over stdio JSON-RPC
- Capability mapping file exists
- CNS span emitted on federation

---

## Task T15: Replace `unwrap()` on Hot Paths with Typed Errors

**Goal:** No panics in long-running processes.

**Files to modify:**
1. `crates/hkask-agents/src/pod.rs`
2. `crates/hkask-agents/src/consent.rs`

**Detailed steps:**

### Step 1: Fix `current_timestamp` unwrap
**Current (`crates/hkask-agents/src/pod.rs`, line 529):**
```rust
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
```

**Change to:**
```rust
fn current_timestamp() -> Result<i64, AgentPodError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| AgentPodError::Clock(e.to_string()))
}
```

### Step 2: Update call sites
**Current (line 540):**
```rust
let now = current_timestamp();
```

**Change to:**
```rust
let now = current_timestamp()?;
```

### Step 3: Fix `MemoryStorageAdapter::in_memory().unwrap()`
**Current (line 759):**
```rust
let memory_storage = MemoryStorageAdapter::in_memory().unwrap();
```

**Change to:**
```rust
let memory_storage = MemoryStorageAdapter::in_memory()
    .map_err(|e| AgentPodError::Storage(e.to_string()))?;
```

### Step 4: Fix `ConsentManager` sync lock
**Current (`crates/hkask-agents/src/consent.rs`, line 73):**
```rust
pub struct ConsentManager {
    _store: Arc<RwLock<SovereigntyBoundaryStore>>,
    consent_cache: Arc<RwLock<Vec<ConsentRecord>>>,
}
```

**Change to:**
```rust
pub struct ConsentManager {
    _store: Arc<tokio::sync::RwLock<SovereigntyBoundaryStore>>,
    consent_cache: Arc<tokio::sync::RwLock<Vec<ConsentRecord>>>,
}
```

### Step 5: Make `ConsentManager` methods async
**Current (line 88):**
```rust
pub fn grant_consent(&self, webid: &str, category: &DataCategory) -> Result<(), ConsentError> {
    let mut cache = self.consent_cache.write().map_err(|_| {
        ConsentError::ConsentNotFound("Consent cache lock poisoned".to_string())
    })?;
    // ...
}
```

**Change to:**
```rust
pub async fn grant_consent(&self, webid: &str, category: &DataCategory) -> Result<(), ConsentError> {
    let mut cache = self.consent_cache.write().await;
    // ...
}
```

### Step 6: Add clippy lints
**File:** `Cargo.toml` (workspace root)

Add to `[workspace.lints.clippy]`:
```toml
unwrap_used = "deny"
expect_used = "deny"
```

**Exceptions:** Allow in tests and CLI:
```toml
[workspace.lints.clippy]
unwrap_used = { level = "deny", priority = -1 }
expect_used = { level = "deny", priority = -1 }

[[workspace.lints.clippy.overrides]]
crate = "hkask-cli"
unwrap_used = "allow"
expect_used = "allow"

[[workspace.lints.clippy.overrides]]
crate = "tests"
unwrap_used = "allow"
expect_used = "allow"
```

### Step 7: Verify
```bash
cargo clippy -p hkask-agents -- -D warnings
cargo test -p hkask-agents
```

**Success criteria:**
- No `unwrap()` in library code
- No `expect()` in library code
- `ConsentManager` uses `tokio::sync::RwLock`
- All tests pass
- Clippy passes

---

## Task T16: Okapi Inference — Shared Client + Concurrent `generate_n`

**Goal:** Connection pool reused. Ensemble fan-out is parallel.

**Files to modify:**
1. `mcp-servers/hkask-mcp-inference/src/tools.rs`
2. `crates/hkask-templates/src/inference_port.rs`

**Detailed steps:**

### Step 1: Hold `Arc<OkapiInference>` per model
**Current (`mcp-servers/hkask-mcp-inference/src/tools.rs`, line 146):**
```rust
async fn try_generate(
    &self,
    model: &str,
    prompt: &str,
    params: &LLMParameters,
) -> Result<hkask_templates::InferenceResult, hkask_templates::InferenceError> {
    let config = OkapiConfig::default();
    let inference = OkapiInference::new(model, config)?;
    inference.generate(prompt, params).await
}
```

**Change to:**
```rust
pub struct InferenceServer {
    metrics: Arc<InferenceMetrics>,
    active_models: Arc<RwLock<Vec<String>>>,
    rate_buckets: Arc<RwLock<HashMap<String, RateBucket>>>,
    inference_clients: Arc<RwLock<HashMap<String, Arc<OkapiInference>>>>,
}

impl InferenceServer {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(InferenceMetrics::default()),
            active_models: Arc::new(RwLock::new(vec![
                "ollama/llama-3.1-8b-instruct".to_string(),
                "ollama/llama-3.1-70b-instruct".to_string(),
                "ollama/codellama-34b".to_string(),
            ])),
            rate_buckets: Arc::new(RwLock::new(HashMap::new())),
            inference_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_or_create_client(&self, model: &str) -> Result<Arc<OkapiInference>, hkask_templates::InferenceError> {
        let mut clients = self.inference_clients.write().await;
        if let Some(client) = clients.get(model) {
            return Ok(Arc::clone(client));
        }
        
        let config = OkapiConfig::default();
        let inference = Arc::new(OkapiInference::new(model, config)?);
        clients.insert(model.to_string(), Arc::clone(&inference));
        Ok(inference)
    }

    async fn try_generate(
        &self,
        model: &str,
        prompt: &str,
        params: &LLMParameters,
    ) -> Result<hkask_templates::InferenceResult, hkask_templates::InferenceError> {
        let client = self.get_or_create_client(model).await?;
        client.generate(prompt, params).await
    }
}
```

### Step 2: Make `generate_n` concurrent
**Current (`crates/hkask-templates/src/inference_port.rs`, line 141):**
```rust
async fn generate_n(
    &self,
    prompt: &str,
    parameters: &LLMParameters,
    n: usize,
) -> Result<Vec<InferenceResult>, InferenceError> {
    let mut results = Vec::with_capacity(n);
    for _ in 0..n {
        results.push(self.generate(prompt, parameters).await?);
    }
    Ok(results)
}
```

**Change to:**
```rust
async fn generate_n(
    &self,
    prompt: &str,
    parameters: &LLMParameters,
    n: usize,
) -> Result<Vec<InferenceResult>, InferenceError> {
    let futures: Vec<_> = (0..n)
        .map(|_| self.generate(prompt, parameters))
        .collect();
    
    let results = futures::future::join_all(futures).await;
    results.into_iter().collect()
}
```

### Step 3: Add CNS gauge for pool idle
**File:** `crates/hkask-templates/src/inference_port.rs`

**Add to `OkapiInference`:**
```rust
pub async fn pool_idle_count(&self) -> usize {
    // reqwest doesn't expose pool stats directly
    // For now, return 0
    0
}
```

### Step 4: Verify
```bash
cargo check -p hkask-mcp-inference
cargo check -p hkask-templates
cargo test -p hkask-templates
```

**Success criteria:**
- `OkapiInference` instances reused
- `generate_n` is concurrent
- All tests pass

---

## Task T17: MCP Server Supervision Tree

**Goal:** 16 MCP servers reliably started, health-probed, restarted.

**Files to modify:**
1. `crates/hkask-mcp/src/supervisor.rs` — **NEW FILE**
2. `crates/hkask-mcp/src/lib.rs` — Export
3. `config/mcp-servers.toml` — **NEW FILE**

**Detailed steps:**

### Step 1: Create `supervisor.rs`
**File:** `crates/hkask-mcp/src/supervisor.rs` (NEW)

```rust
//! MCP Server Supervisor — Process lifecycle management

use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub binary: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_restart_limit")]
    pub restart_limit_per_minute: u32,
}

fn default_restart_limit_per_minute() -> u32 {
    5
}

#[derive(Debug, Deserialize)]
pub struct McpServersConfig {
    pub servers: Vec<McpServerConfig>,
}

pub struct McpSupervisor {
    config: McpServersConfig,
    children: tokio::sync::RwLock<HashMap<String, Child>>,
    restart_counts: tokio::sync::RwLock<HashMap<String, (u32, std::time::Instant)>>,
}

impl McpSupervisor {
    pub fn new(config: McpServersConfig) -> Self {
        Self {
            config,
            children: tokio::sync::RwLock::new(HashMap::new()),
            restart_counts: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn start_all(&self) -> Result<(), String> {
        for server in &self.config.servers {
            self.start_server(server).await?;
        }
        Ok(())
    }

    async fn start_server(&self, config: &McpServerConfig) -> Result<(), String> {
        let child = Command::new(&config.binary)
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", config.name, e))?;

        let mut children = self.children.write().await;
        children.insert(config.name.clone(), child);

        info!(server = %config.name, "MCP server started");
        Ok(())
    }

    pub async fn restart(&self, name: &str) -> Result<(), String> {
        // Check restart limit
        let mut counts = self.restart_counts.write().await;
        let (count, last_restart) = counts
            .entry(name.to_string())
            .or_insert((0, std::time::Instant::now()));

        if last_restart.elapsed() > Duration::from_secs(60) {
            *count = 0;
            *last_restart = std::time::Instant::now();
        }

        let config = self.config.servers.iter().find(|s| s.name == name);
        let config = config.ok_or_else(|| format!("Server '{}' not found", name))?;

        if *count >= config.restart_limit_per_minute {
            return Err(format!(
                "Restart limit exceeded for '{}' ({} per minute)",
                name, config.restart_limit_per_minute
            ));
        }

        *count += 1;

        // Stop existing
        self.stop_server(name).await?;

        // Start new
        self.start_server(config).await?;

        info!(server = %name, restarts = %count, "MCP server restarted");
        Ok(())
    }

    async fn stop_server(&self, name: &str) -> Result<(), String> {
        let mut children = self.children.write().await;
        if let Some(mut child) = children.remove(name) {
            let _ = child.kill().await;
            info!(server = %name, "MCP server stopped");
        }
        Ok(())
    }

    pub async fn status(&self) -> HashMap<String, bool> {
        let children = self.children.read().await;
        children
            .iter()
            .map(|(name, child)| {
                let running = child.id().is_some();
                (name.clone(), running)
            })
            .collect()
    }
}
```

### Step 2: Export from `lib.rs`
**File:** `crates/hkask-mcp/src/lib.rs`

**Add:**
```rust
pub mod supervisor;
pub use supervisor::{McpServerConfig, McpServersConfig, McpSupervisor};
```

### Step 3: Create config file
**File:** `config/mcp-servers.toml` (NEW)

```toml
[[servers]]
name = "inference"
binary = "hkask-mcp-inference"
args = []
restart_limit_per_minute = 5

[[servers]]
name = "registry"
binary = "hkask-mcp-registry"
args = []
restart_limit_per_minute = 5

# Add other 14 servers...
```

### Step 4: Add CLI commands
**File:** `crates/hkask-cli/src/main.rs`

**Add:**
```rust
#[derive(Subcommand)]
enum McpCommand {
    /// Show status of all MCP servers
    Status,
    /// Restart a specific MCP server
    Restart { name: String },
}
```

### Step 5: Verify
```bash
cargo check -p hkask-mcp
cargo check -p hkask-cli
```

**Success criteria:**
- `McpSupervisor` exists
- Config file exists
- CLI commands exist
- Restart limit enforced

---

## Task T18: Delete `PlaceholderGitCAS` from Production

**Goal:** Remove placeholder. Tests use dev-dependencies mock.

**Files to modify:**
1. `crates/hkask-agents/src/pod.rs` — Delete `PlaceholderGitCAS`
2. `crates/hkask-agents/src/adapters/git_cas.rs` — Move `MockGitCas` to dev-dependencies
3. `crates/hkask-testing/src/lib.rs` — **NEW FILE** (or add to existing test crate)

**Detailed steps:**

### Step 1: Delete `PlaceholderGitCAS`
**Current (`crates/hkask-agents/src/pod.rs`, line 637):**
```rust
pub struct PlaceholderGitCAS;

impl GitCASPort for PlaceholderGitCAS {
    fn load_template_crate(&self, _crate_name: &str) -> Result<TemplateCrate, GitError> {
        Ok(TemplateCrate {
            name: "placeholder".to_string(),
            git_sha: "0000000000000000000000000000000000000000".to_string(),
            persona_yaml: String::new(),
            dispatch_manifest_yaml: String::new(),
            templates: vec![],
            hlexicon_terms: vec![],
        })
    }

    fn resolve_sha(&self, _crate_name: &str) -> Result<String, GitError> {
        Ok("0000000000000000000000000000000000000000".to_string())
    }
}
```

**Delete entirely.**

### Step 2: Move `MockGitCas` to test crate
**Current (`crates/hkask-agents/src/adapters/git_cas.rs`, line 178):**
```rust
pub struct MockGitCas;
// ...
```

**Move to:** `crates/hkask-testing/src/mock_git_cas.rs` (NEW)

### Step 3: Update `PodManagerBuilder`
**Current (line 850):**
```rust
pub fn build(self) -> Result<PodManager, AgentPodError> {
    let git_cas = self.git_cas.unwrap_or_else(|| Box::new(PlaceholderGitCAS));
    // ...
}
```

**Change to:**
```rust
pub fn build(self) -> Result<PodManager, AgentPodError> {
    let git_cas = self.git_cas.ok_or_else(|| {
        AgentPodError::Config("GitCAS adapter required".to_string())
    })?;
    // ...
}
```

### Step 4: Verify
```bash
cargo check -p hkask-agents
cargo test -p hkask-agents
```

**Success criteria:**
- `PlaceholderGitCAS` deleted
- `MockGitCas` in test crate
- `PodManagerBuilder` requires explicit adapter
- All tests pass

---

## Task T19: Typestate Pod Lifecycle — **DEFERRED**

**Reason:** High-risk refactor for minimal gain. Current enum + runtime checks are correct and well-tested. Defer to post-v0.21.x.

---

## Task T20: Eliminate `dyn` on Hot Inference Path

**Goal:** Monomorphize where it matters.

**Files to modify:**
1. `crates/hkask-templates/src/inference_port.rs`

**Detailed steps:**

### Step 1: Replace `Box<dyn InferencePort>` with generics
**Current (line 580):**
```rust
pub async fn invoke_template_with_okapi(
    inference: Box<dyn InferencePort + Send + Sync>,
    // ...
) -> Result<TemplateInvocation, InferenceError> {
    // ...
}
```

**Change to:**
```rust
pub async fn invoke_template_with_okapi<I>(
    inference: &I,
    // ...
) -> Result<TemplateInvocation, InferenceError>
where
    I: InferencePort + Send + Sync,
{
    // ...
}
```

### Step 2: Delete non-generic versions
**Delete:** `invoke_template_with_okapi` and `invoke_template_with_selection` (lines 579-622).

**Keep:** `invoke_template_with_okapi_generic` and `invoke_template_with_selection_generic` (lines 644-691).

**Rename:** Remove `_generic` suffix.

### Step 3: Update call sites
Find all calls to `invoke_template_with_okapi` and update to use generic version.

### Step 4: Verify
```bash
cargo check -p hkask-templates
cargo test -p hkask-templates
```

**Success criteria:**
- No `Box<dyn InferencePort>` in non-config call sites
- Generic versions used
- All tests pass

---

## Task T21: Hex Purity Sweep — Port Surface Inventory

**Goal:** Every external boundary is a `pub trait …Port`.

**Deliverable:** `docs/architecture/ports-inventory.md`

**Detailed steps:**

### Step 1: Audit all crates
For each crate, identify:
- Ports (traits)
- Adapters (implementations)
- Missing ports (direct external access)

### Step 2: Create inventory document
**File:** `docs/architecture/ports-inventory.md` (NEW)

```markdown
# Port Surface Inventory

| Crate | Port | Status | Adapter(s) |
|-------|------|--------|------------|
| hkask-agents | AcpPort | ✓ exists | AcpRuntime, RussellAcpAdapter |
| hkask-agents | GitCASPort | ✓ exists | GitCasAdapter (gix) |
| hkask-agents | MCPRuntimePort | ✓ exists | McpRuntimeAdapter |
| hkask-agents | MemoryStoragePort | ✓ exists | MemoryStorageAdapter |
| hkask-agents | KeystorePort | ✗ → add | KeychainAdapter |
| hkask-mcp | McpTransport | ✓ added (T09) | InProcessMcpTransport |
| hkask-templates | InferencePort | ✓ exists | OkapiInference |
| hkask-templates | McpPort | ✓ exists → async (T10) | McpDispatcher |
| hkask-templates | CnsPort | ✓ exists | CnsRuntime |
```

### Step 3: Add missing `KeystorePort`
**File:** `crates/hkask-agents/src/ports/keystore.rs` (NEW)

```rust
use async_trait::async_trait;
use zeroize::Zeroizing;

#[async_trait]
pub trait KeystorePort: Send + Sync {
    async fn retrieve(&self, key: &str) -> Result<Zeroizing<Vec<u8>>, String>;
    async fn store(&self, key: &str, secret: &[u8]) -> Result<(), String>;
}
```

### Step 4: Move `Keychain` direct use behind port
**File:** `crates/hkask-agents/src/pod.rs`

**Current (line 338):**
```rust
let secret = hkask_keystore::get_or_create_ocap_secret(&keychain, &webid)?;
```

**Change to:**
```rust
let secret = self.keystore.retrieve(&webid.to_string()).await?;
```

### Step 5: Verify
```bash
cargo check --workspace
```

**Success criteria:**
- Inventory document exists
- All external boundaries have ports
- `KeystorePort` added
- No direct `Keychain` use in `pod.rs`

---

## Task T22: Documentation Alignment & CI Verification Gates

**Goal:** Docs, code, and ERDs co-evolve.

**Files to modify:**
1. `docs/architecture/hKask-erd.md`
2. `docs/architecture/subsystem-erds.md`
3. `.github/workflows/ci.yml`

**Detailed steps:**

### Step 1: Update ERD
**File:** `docs/architecture/hKask-erd.md`

**Add entities:**
- `RUSSELL_ACP_BRIDGE`
- `MCP_TRANSPORT`
- `REVOCATION_STORE`
- Unified `CAPABILITY` entity (drop dual macaroon entity)

### Step 2: Update subsystem ERDs
**File:** `docs/architecture/subsystem-erds.md`

**Update:**
- Remove `Macaroon` entity
- Add `Caveat` to `CapabilityToken`
- Add `RevocationStore`

### Step 3: Add CI gates
**File:** `.github/workflows/ci.yml`

**Add:**
```yaml
- name: Clippy
  run: cargo clippy --workspace -- -D warnings

- name: Deny
  run: cargo deny check

- name: Secret scan
  run: |
    if rg -n '\[0x[0-9a-fA-F]{2}(,\s*0x[0-9a-fA-F]{2}){31}\]' crates/ --type rust; then
      echo "ERROR: Hardcoded 32-byte key found"
      exit 1
    fi

- name: Test
  run: cargo test --workspace
```

### Step 4: Verify
```bash
cargo clippy --workspace -- -D warnings
cargo deny check
cargo test --workspace
```

**Success criteria:**
- ERDs updated
- CI gates pass
- No hardcoded secrets
- All tests pass

---

## Summary

**Total tasks:** 22 (T01–T22, with T19 deferred)

**Phase A (Security Critical):** T01–T07
**Phase B (Architectural):** T08–T15
**Phase C (Enhancements):** T16–T18, T20–T22

**Estimated effort:** ~22 days (1 day per task, parallelizable within phases)

**Success criteria:**
- All security issues resolved
- Single capability primitive
- Real MCP transport
- Async hex purity
- Persistent revocation
- CNS observability closed loop
- Russell ACP bridge
- No panics in library code
- Documentation aligned
