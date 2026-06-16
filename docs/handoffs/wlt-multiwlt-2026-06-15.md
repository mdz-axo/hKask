# Handoff: Wallet System — Multi-Wallet Foundation & Remaining Gaps

**Date:** 2026-06-15
**Session scope:** Turn the wallet/crypto/energy system into a functional payments and use system.
**Progress:** ~70% complete. Core payment infrastructure (deposits, withdrawals, gas accounting, API keys, encumbrance) is functional. Multi-wallet foundation laid. Self-custody chain ports restored after Circle detour.

---

## 1. Session Context

This session closed 8 of 9 gaps identified in the wallet/crypto/energy system audit. The system now has: deposit monitoring wired into daemon startup, deposit idempotency, Solana write path with ATA creation, Hedera gRPC write path, API key auth middleware with encumbrance checking, inference budget membrane, CNS gas span emission, price feed + fee estimation, and the multi-wallet data model foundation. A brief detour into Circle (custodial API) was backed out after it was identified as a P1 (User Sovereignty) violation — self-custody via direct chain RPC is the only principles-aligned path.

---

## 2. What Was Done

### Wallet Infrastructure (Gaps 1–2)
- **Deposit monitor wired into daemon startup:** `AgentService::build()` constructs full wallet stack (WalletStore → WalletManager → ApiKeyIssuer → WalletService), spawns deposit monitor as `tokio::spawn` background task polling every 30s (configurable via `HKASK_DEPOSIT_MONITOR_INTERVAL_SECS`).
  - Files: `crates/hkask-services/src/config.rs` (added `wallet_config`), `crates/hkask-services/src/context.rs` (wallet construction + monitor spawn)
- **Deposit idempotency:** `WalletStore::transaction_exists_by_hash()` checks `on_chain_tx_hash` before crediting. Applied in both `process_deposit()` and `process_shielded_deposit()`.
  - Files: `crates/hkask-storage/src/wallet_store.rs`, `crates/hkask-wallet/src/manager.rs`

### Chain Ports (Gaps 3, 6)
- **Solana write path:** `build_withdrawal_tx` now includes `create_associated_token_account` instruction before the SPL transfer (idempotent — no-op if ATA exists). Integration tests added (2 pass, 1 ignored for manual devnet run).
  - Files: `crates/hkask-wallet/src/solana.rs`
- **Hedera write path:** Full gRPC implementation using `hiero-sdk-proto` + `tonic` (rustls, no openssl). Builds `CryptoTransferTransactionBody` with `TokenTransferList`, serializes via protobuf, signs, wraps in `SignedTransaction` → `Transaction`, submits via `CryptoServiceClient::crypto_transfer`.
  - Files: `crates/hkask-wallet/src/hedera.rs`, `crates/hkask-wallet/Cargo.toml` (added `hiero-sdk-proto`, `tonic`, `prost` as optional deps)
- **Chain port wiring:** `AgentService::build()` reads `SOLANA_RPC_URL` + `SOLANA_TREASURY_PUBKEY` and/or `HEDERA_TREASURY_ACCOUNT` from env, constructs chain ports. Graceful degradation: read-only mode if no ports configured.
  - Files: `crates/hkask-services/src/context.rs`, `crates/hkask-services/Cargo.toml` (pass-through features: `solana`, `hedera`, `hinkal`)

### API Key Auth & Encumbrance (Gap 4)
- **API key auth middleware wired:** `ApiKeyAuthService` constructed from shared `WalletStore`, applied as router layer in `create_router()`. Accepts `Authorization: Bearer <hex-private-key>`.
- **Encumbrance check:** Middleware verifies active encumbrance exists with remaining rJoules. Returns HTTP 402 `PaymentRequired` if no allocation.
- **CLI encumber commands:** `kask wallet encumber --key-id <id> --amount <rj>` and `kask wallet release-encumbrance --key-id <id>`.
  - Files: `crates/hkask-api/src/lib.rs`, `crates/hkask-api/src/middleware/api_key_auth.rs`, `crates/hkask-cli/src/cli/actions.rs`, `crates/hkask-cli/src/commands/wallet.rs`

### Inference Budget (Gap 5)
- **GovernedInference membrane:** Wraps `InferencePort` with hold-settle energy budget enforcement. Cost estimated from `max_tokens` (1 token ≈ 1 gas unit). Emits `cns.gas.*` + `cns.inference.*` spans. Wired into `AgentService::build()` — all inference calls go through the membrane.
  - Files: `crates/hkask-cns/src/governed_inference.rs` (new), `crates/hkask-cns/src/lib.rs`, `crates/hkask-services/src/context.rs`

### CNS Observability (Gap 7)
- **Gas span emission:** `GovernedTool::invoke()` now emits `cns.gas.depleted` (budget exceeded), `cns.gas.reserved` (after reservation), `cns.gas.settled` (after settlement with reserved/actual/refunded). `GovernedInference` emits the same spans for inference calls.
  - Files: `crates/hkask-cns/src/governed_tool.rs`, `crates/hkask-cns/src/governed_inference.rs`

### Price Feed (Gap 8)
- **PriceFeed trait + StaticPriceFeed + fee estimation:** `estimate_withdrawal_fee()` calculates rJoule cost from native token fee × USD rate ÷ rj_per_usdc. Solana: ~0.000005 SOL, Hedera: ~$0.001.
  - Files: `crates/hkask-wallet/src/price_feed.rs` (new), `crates/hkask-wallet/src/lib.rs`

### Multi-Wallet Foundation (Gap 9, partial)
- **Data model:** `ReplicantIdentity.wallet_id: Option<WalletId>` added. SQL schema updated with `wallet_id TEXT` column + migration. `UserStore` reads/writes via `get_wallet_id()` / `set_wallet_id()`.
- **Startup binding:** `AgentService::build()` binds default wallet to system replicant via `user_store.set_wallet_id()`.
  - Files: `crates/hkask-types/src/identity.rs`, `crates/hkask-storage/src/sql/users.sql`, `crates/hkask-storage/src/user_store.rs`, `crates/hkask-services/src/context.rs`

### Circle Detour (BACKED OUT)
- CirclePort was proposed, implemented, then fully removed after identified as P1 violation (custodial — Circle holds keys). Self-custody Solana/Hedera ports restored as primary path. `circle.rs` deleted. Feature flags reverted to `solana`/`hedera`/`hinkal`.
- **This decision must not be reversed.** See §5 below.

### Build & Test Status
- **All 259 tests pass, zero failures** across `hkask-storage`, `hkask-wallet`, `hkask-cns`, `hkask-services`, `hkask-api`, `hkask-cli`.
- Default build (`cargo check -p hkask-services`) compiles cleanly (no chain SDK deps).
- Feature builds: `--features solana` and `--features hedera` both compile cleanly.

---

## 3. What Remains

### HIGH — Wire encumbrance consumption into API request flow
**What:** When an API key makes a request, gas should be consumed from the encumbrance. Currently the middleware checks the encumbrance exists, but no gas is actually debited during tool/inference calls. The `WalletBackedBudget` exists and `GovernedTool`/`GovernedInference` check wallet budgets first — but the wallet budget is never registered for API key requests.

**Where:** The missing link is registering a `WalletBackedBudget` when an API key authenticates. Options:
- A) In the API key auth middleware, after authentication, call `WalletService::register_wallet_budget()` — but middleware only has `WalletStore`, not `WalletService` or `CyberneticsLoop`
- B) Add a post-request hook in route handlers that consumes from encumbrance based on CNS gas spans
- C) Pass `WalletService` to the middleware by adding it to `ApiKeyAuthService`

**Recommended:** Option C — add `wallet_service: Arc<WalletService>` to `ApiKeyAuthService`. After authentication, call `wallet_service.register_wallet_budget(agent_webid, wallet_id)` to register a temporary `WalletBackedBudget`. The `GovernedTool`/`GovernedInference` membranes will then debit from it automatically.

**Files:** `crates/hkask-api/src/middleware/api_key_auth.rs`, `crates/hkask-api/src/lib.rs` (construct `ApiKeyAuthService` with `WalletService`)

### HIGH — Per-user wallet creation during onboarding
**What:** `OnboardingService::register_replicant()` creates the ACP agent and registry entry but doesn't create a wallet. New replicants get no wallet.

**Where:** `OnboardingService::register_replicant()` needs a `WalletStore` parameter. After registration, call `wallet_store.ensure_wallet(new_wallet_id)` and `user_store.set_wallet_id(replicant_name, new_wallet_id)`.

**Files:** `crates/hkask-services/src/onboarding.rs`, `crates/hkask-cli/src/onboarding.rs` (caller)

### HIGH — Deposit address → wallet resolution
**What:** `WalletManager::process_deposit()` hardcodes `WalletId::default()` with a `TODO: resolve from deposit address lookup`. When multiple users have wallets, deposits must be credited to the correct wallet.

**Where:** Add a `resolve_wallet_for_address(address: &str) -> Option<WalletId>` method to `WalletStore` that queries `deposit_addresses` table. Call it in `process_deposit()` instead of `WalletId::default()`.

**Files:** `crates/hkask-storage/src/wallet_store.rs`, `crates/hkask-wallet/src/manager.rs` (L178)

### MEDIUM — CLI/API wallet_id parameter support
**What:** All CLI commands and API routes use `WalletId::default()`. They need `--wallet` flag / `?wallet_id=` query param.

**Where:**
- CLI: Add `--wallet` arg to `WalletAction` variants, parse in handlers
- API: Add `wallet_id: Option<WalletId>` query param to route handlers, resolve from authenticated user if not provided

**Files:** `crates/hkask-cli/src/cli/actions.rs`, `crates/hkask-cli/src/commands/wallet.rs`, `crates/hkask-api/src/routes/wallet.rs`

### MEDIUM — Scope enforcement middleware
**What:** `ApiKeyCapability.scope` exists (e.g., `["read-specs"]`) but no middleware checks it. A key scoped to `read-specs` can call `/api/chat`.

**Where:** Add scope check in `api_key_auth_middleware` or as a separate layer. Compare request path against key's declared scope. `ApiMeteringAlert::ScopeViolation` is already defined.

**Files:** `crates/hkask-api/src/middleware/api_key_auth.rs`, `crates/hkask-cns/src/api_metering.rs`

### LOW — Deposit notifications
**What:** No webhook or alert when a deposit is credited. CNS spans are emitted but not surfaced to users.

**Where:** Add a `cns.wallet.deposit_credited` algedonic alert in `WalletManager::process_deposit()`. Optionally add webhook support via a user-configurable URL.

**Files:** `crates/hkask-wallet/src/manager.rs`, `crates/hkask-cns/src/algedonic.rs`

### LOW — Billing / usage reports
**What:** No spending summary per API key. Transaction history exists but no aggregation or export.

**Where:** Add `kask wallet report --key-id <id>` command that queries `wallet_transactions` for `Spend` types, aggregates by tool/date, shows rJoule and USDC equivalent.

**Files:** `crates/hkask-cli/src/commands/wallet.rs`, `crates/hkask-storage/src/wallet_store.rs`

---

## 4. Recommended Skills and Tools

### Skills to Load
| Skill | Why |
|-------|-----|
| **coding-guidelines** | Required before any code changes. Enforces think-before-coding, simplicity, surgical changes, goal-driven execution. |
| **tdd** | For any new functionality (scope enforcement, wallet resolution). Write contract tests first. |
| **pragmatic-semantics** | When reasoning about constraint forces (Prohibition vs Guideline). Critical for P1 self-custody decisions. |
| **constraint-forces** | If any proposal touches custody model or third-party services — classify as Prohibition before implementing. |
| **deep-module** | When adding methods to WalletStore, UserStore, or middleware. Keep public surface ≤7. |

### Build & Test Commands
```bash
# Default build (no chain SDKs)
cargo check -p hkask-services -p hkask-api -p hkask-cli

# With Solana chain port
cargo check -p hkask-services --features solana

# With Hedera chain port
cargo check -p hkask-services --features hedera

# Full test suite
cargo test -p hkask-storage -p hkask-wallet -p hkask-cns -p hkask-services -p hkask-api -p hkask-cli

# Solana integration tests (requires devnet + funded treasury)
cargo test -p hkask-wallet --features solana -- integration_tests

# Constraint verification (run before committing)
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"
```

---

## 5. Key Decisions to Preserve

1. **Self-custody only (P1 — User Sovereignty).** Circle (custodial API) was proposed, implemented, and fully backed out after identified as a P1 violation. Treasury keys are derived from the user's master key via HKDF. No third party holds keys. No custodial service. The `ChainPort` trait boundary is the plug point — any future provider must implement it, and must not hold the user's keys. **Do not reintroduce custodial services.**

2. **ChainPort trait is the architecture boundary.** Everything above it (rJoule accounting, OCAP API keys, encumbrance, CNS spans) is hKask-specific and always compiled. Everything below it (SolanaPort, HederaPort) is feature-gated and pluggable. The trait has 6 methods — keep it minimal.

3. **Encumbrance system is the payment membrane.** API keys don't spend directly from wallet balance. rJoules must be explicitly allocated (encumbered) to a key. The encumber→consume→release lifecycle is atomic at the SQL level. This is the core payment mechanism — don't bypass it.

4. **GovernedTool + GovernedInference are the enforcement membranes.** All tool invocations and inference calls go through hold-settle gas accounting. CNS spans (`cns.gas.*`, `cns.inference.*`) provide observability. New operations that consume resources must go through these membranes.

5. **`WalletId::default()` is a temporary single-tenant placeholder.** The multi-wallet data model is in place (`ReplicantIdentity.wallet_id`, SQL column, UserStore methods). The remaining work is wiring it through the CLI/API surfaces and deposit resolution. Don't add new code that hardcodes `WalletId::default()` — use the replicant's wallet_id from UserStore.

6. **Feature flags are per-chain, not per-provider.** `solana`, `hedera`, `hinkal` — each enables a specific chain port. Default build has zero chain SDK dependencies. Pass-through features on `hkask-services` mirror `hkask-wallet` features.
