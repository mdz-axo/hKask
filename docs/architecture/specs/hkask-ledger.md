# hKask Ledger — Core Service Specification

**Version:** v0.30.0
**Status:** Draft
**Last updated:** 2026-06-20
**References:** [Formance Ledger](https://github.com/formancehq/ledger), [Blnk](https://github.com/blnkfinance/blnk)

---

## 1. Purpose

A single core crate (`crates/hkask-ledger`) providing immutable double-entry accounting. Three domain ledgers share this backend:

| Domain | What It Tracks | Account Namespace |
|--------|---------------|-------------------|
| **Cost ledger** | System costs (gas, API, training, subscriptions) | `cost:*` |
| **Crypto ledger** | Wallet transactions (Hedera, rJ token) | `wallet:*` |
| **Securities ledger** | Portfolio transactions (buy, sell, dividends) | `portfolio:*` |

Same crate, same SQLite schema, same transaction semantics. Different account namespaces, different balance queries. The existing `PortfolioManager` in `hkask-mcp-companies` is ported to use this.

## 2. Schema

```
accounts
  id: TEXT PRIMARY KEY  — "cost:api/deepinfra", "wallet:hedera/main"
  namespace: TEXT        — "cost", "wallet", "portfolio"
  created_at: TEXT

transactions
  id: TEXT PRIMARY KEY   — UUIDv4
  timestamp: TEXT        — ISO 8601
  reference: TEXT UNIQUE — idempotency key (e.g., "qa-run-abc-step-2")
  metadata: TEXT         — JSON blob for domain-specific data
  created_at: TEXT

postings
  id: INTEGER PRIMARY KEY AUTOINCREMENT
  transaction_id: TEXT REFERENCES transactions(id)
  source: TEXT           — debited account
  destination: TEXT      — credited account
  asset: TEXT            — "rJ", "USD", "gas", "BTC", "AAPL"
  amount: INTEGER        — in asset's smallest unit (µrJ, µUSD, µBTC, etc.)
  created_at: TEXT

-- Query performance indexes
CREATE INDEX IF NOT EXISTS idx_postings_destination_asset ON postings(destination, asset);
CREATE INDEX IF NOT EXISTS idx_postings_source_asset ON postings(source, asset);
CREATE INDEX IF NOT EXISTS idx_transactions_reference ON transactions(reference);
```

## 3. Core API

```rust
pub struct Ledger {
    db: Connection,
}

impl Ledger {
    /// Open (or create) the ledger database at the given path.
    pub fn open(path: &Path) -> Result<Self, LedgerError>;

    /// Create a named account. Idempotent.
    pub fn ensure_account(&self, id: &str, namespace: &str) -> Result<(), LedgerError>;

    /// Commit a transaction with 1+ postings. Idempotent by reference.
    /// Sum of all posting amounts must be 0 (double-entry invariant).
    /// source = 0 after all postings applied.
    pub fn commit(&self, tx: &LedgerTransaction) -> Result<(), LedgerError>;

    /// Get balance for a single account, optionally filtered by asset.
    /// Balance = sum(destination.amount) - sum(source.amount) where asset matches.
    pub fn balance(&self, account: &str, asset: Option<&str>) -> Result<i64, LedgerError>;

    /// Get all balances for a namespace, grouped by account and asset.
    pub fn namespace_balances(&self, namespace: &str) -> Result<Vec<AccountBalance>, LedgerError>;

    /// Query transactions within a time range.
    pub fn query(&self, range: DateRange, filter: QueryFilter) -> Result<Vec<LedgerTransaction>, LedgerError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerTransaction {
    pub id: String,              // UUIDv4
    pub timestamp: String,       // ISO 8601
    pub reference: String,        // idempotency key — unique
    pub postings: Vec<Posting>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    pub source: String,          // debited account
    pub destination: String,     // credited account
    pub asset: String,           // "rJ", "USD", "gas", "BTC"
    pub amount: i64,             // in smallest unit (µrJ, µUSD, µBTC)
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountBalance {
    pub account: String,
    pub asset: String,
    pub balance: i64,
}
```

## 4. Invariants

1. **Idempotency:** Same `reference` committed twice → second call is a no-op, returns `Ok`.
2. **Double-entry:** Every transaction's postings must sum to 0: `Σ source.amount == Σ destination.amount` across all postings in the transaction. This is the Formance/Blnk model — every movement has equal and opposite entries.
3. **Immutability:** Once committed, a transaction is never modified or deleted. Balances are always computed from postings.
4. **Integer amounts:** All amounts in the asset's smallest unit (µrJ, µUSD, µBTC, satoshis). No floating-point.

## 5. Cost Ledger Usage

The rJoule CostTracker becomes a client of the ledger:

```rust
// After a QA script run, commit the cost:
ledger.commit(&LedgerTransaction {
    id: Uuid::new_v4().to_string(),
    timestamp: now_rfc3339(),
    reference: format!("qa-run:{}/{}", manifest_id, step_ordinal),
    postings: vec![
        Posting {
            source: "cost:qa/run".into(),
            destination: "cost:gas/functions".into(),
            asset: "rJ".into(),
            amount: gas_urj as i64,
        },
        Posting {
            source: "cost:qa/run".into(),
            destination: "cost:api/deepinfra".into(),
            asset: "rJ".into(),
            amount: api_token_urj as i64,
        },
    ],
    metadata: json!({"manifest_id": manifest_id, "steps": step_count}),
})?;

// Query: how much did DeepInfra cost this month?
let deepinfra_cost = ledger.balance("cost:api/deepinfra", Some("rJ"))?;
```

## 6. Crypto Ledger Usage

```rust
// On-chain transaction received on Hedera:
ledger.commit(&LedgerTransaction {
    id: Uuid::new_v4().to_string(),
    reference: format!("hedera:txn:{}", hedera_txn_id),
    postings: vec![
        Posting {
            source: "external:hedera".into(),
            destination: "wallet:hedera/main".into(),
            asset: "HBAR".into(),
            amount: hbar_amount_in_tinybar,
        },
    ],
    metadata: json!({"chain": "hedera", "txn_id": hedera_txn_id}),
})?;
```

## 7. Securities Ledger Usage (Replaces PortfolioManager)

```rust
// Buy 100 shares of AAPL at $150:
ledger.commit(&LedgerTransaction {
    id: Uuid::new_v4().to_string(),
    reference: format!("trade:{}", trade_id),
    postings: vec![
        Posting {
            source: "portfolio:cash/main".into(),
            destination: "portfolio:position/AAPL".into(),
            asset: "USD".into(),
            amount: 15000_00000, // $15,000.00 in µUSD
        },
        Posting {
            source: "portfolio:cash/main".into(),
            destination: "cost:brokerage/fees".into(),
            asset: "USD".into(),
            amount: 9_99000, // $9.99 commission in µUSD
        },
    ],
    metadata: json!({"symbol": "AAPL", "shares": 100, "price": 150.00}),
})?;
```

## 8. Account Naming Convention

```
cost:gas/functions          — software function carbon cost
cost:api/deepinfra          — DeepInfra API token costs
cost:api/together           — Together AI API costs
cost:api/openrouter         — OpenRouter costs
cost:api/brave              — Brave Search costs
cost:api/firecrawl          — Firecrawl costs
cost:api/tavily             — Tavily costs
cost:api/exa                — Exa costs
cost:api/fmp                — FMP subscription costs
cost:api/eodhd              — EODHD subscription costs
cost:training/together      — Together training job costs
cost:training/runpod        — Runpod training costs
cost:training/baseten       — Baseten training costs
cost:subscription/fmp       — FMP monthly subscription (amortized)
cost:brokerage/fees         — Brokerage fees (portfolio)
cost:qa/run                 — Temporary sink for QA run costs
wallet:hedera/main          — Hedera main wallet
external:hedera             — External Hedera chain (counterparty)
external:income             — External income source (dividends, deposits)
portfolio:cash/main         — Cash balance
portfolio:position/AAPL     — AAPL position
```

## 9. Provider Intelligence Integration

The `ProviderIntelligence` trait from the provider-intelligence spec writes to the cost ledger:

```rust
// When a provider shifts from pre-paid to marginal:
provider_intelligence.on_rate_change(
    provider: "brave",
    old_rate: CostRate { is_marginal: false, ... },
    new_rate: CostRate { is_marginal: true, ... },
    ledger: &ledger,
)
// writes:
//   Posting { source: "cost:qa/run", destination: "cost:api/brave",
//             asset: "rJ", amount: <overage_cost> }
//   metadata: { "shift": "pre-paid to marginal", "triggered_at": "..." }
```

## 10. Code Impact

| File | Change |
|------|--------|
| `crates/hkask-ledger/src/lib.rs` | New crate: `Ledger`, `LedgerTransaction`, `Posting`, `AccountBalance`, invariants |
| `crates/hkask-test-harness/src/qa_script.rs` | CostTracker optionally accepts `Arc<Ledger>` and commits costs on run completion |
| `crates/hkask-cli/src/commands/qa.rs` | Pass ledger to CostTracker; display ledger-confirmed balances |
| `mcp-servers/hkask-mcp-companies/src/portfolio.rs` | Port `PortfolioManager` to use `hkask-ledger` (or keep as-is with ledger as alternative backend) |
| `crates/hkask-wallet/src/` | Wallet operations commit to `wallet:*` accounts |
| `crates/hkask-services-runtime/src/classify_impl.rs` | Classify service optionally accepts `Arc<ProviderIntelligence>` for actual cost lookups |

## 11. Resolved Design Decisions

1. **Per-namespace files:** Three separate SQLite databases: `~/.config/hkask/ledger-cost.db`, `~/.config/hkask/ledger-crypto.db`, `~/.config/hkask/ledger-portfolio.db`. Clean separation, different backup/recovery policies.

2. **Computed balances:** Balance = `SUM(destination) - SUM(source)` at query time. No materialized balance table. Acceptable for thousands of postings per file. Add index on `(destination, asset)` and `(source, asset)`.

3. **Portfolio migration:** Existing `PortfolioManager` (`mcp-servers/hkask-mcp-companies/src/portfolio.rs`) will use the ledger. Legacy SQLite schema ported to ledger transactions during migration.

4. **Internal-only:** `hkask-ledger` is a core crate used by other crates. Not exposed as an MCP server tool. Balance queries are programmatic only.
