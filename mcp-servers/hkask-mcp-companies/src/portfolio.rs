//! hKask MCP Companies — Portfolio tracking
//!
//! A portfolio is a ledger. Everything else is arithmetic on the ledger
//! at a point in time. This module manages the SQLite-backed transaction
//! ledger — create, read, validate, import, export, notes, and file attachments.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use hkask_types::time::now_rfc3339;

// ── Transaction ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub date: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub symbol: Option<String>,
    pub quantity: Option<f64>,
    pub price: Option<f64>,
    pub commission: Option<f64>,
    pub amount: Option<f64>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub notes: String,
    pub created_at: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

// ── Validation report ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub transaction_count: usize,
    pub positions: Vec<PositionSummary>,
    pub cash_balance: f64,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PositionSummary {
    pub symbol: String,
    pub shares: f64,
    pub total_buys: f64,
    pub total_sells: f64,
}

// ── PortfolioManager ────────────────────────────────────────────────

pub struct PortfolioManager {
    db_path: PathBuf,
}

impl Default for PortfolioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PortfolioManager {
    pub fn new() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("hkask");
        path.push("portfolios");
        let _ = std::fs::create_dir_all(&path);
        path.push("master.db");
        // Ensure schema exists on first use
        if let Ok(conn) = Connection::open(&path) {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS portfolios (
                    name TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS transactions (
                    id TEXT PRIMARY KEY,
                    portfolio_name TEXT NOT NULL REFERENCES portfolios(name) ON DELETE CASCADE,
                    date TEXT NOT NULL,
                    type TEXT NOT NULL CHECK(type IN ('buy','sell','dividend','deposit','withdrawal')),
                    symbol TEXT,
                    quantity REAL,
                    price REAL,
                    commission REAL DEFAULT 0,
                    amount REAL,
                    currency TEXT DEFAULT 'USD',
                    notes TEXT DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_tx_portfolio ON transactions(portfolio_name);
                CREATE INDEX IF NOT EXISTS idx_tx_date ON transactions(date);
                CREATE INDEX IF NOT EXISTS idx_tx_symbol ON transactions(symbol);
                CREATE TABLE IF NOT EXISTS price_cache (
                    portfolio_name TEXT NOT NULL REFERENCES portfolios(name) ON DELETE CASCADE,
                    symbol TEXT NOT NULL,
                    date TEXT NOT NULL,
                    close REAL NOT NULL,
                    source TEXT NOT NULL,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (portfolio_name, symbol, date)
                );
                CREATE TABLE IF NOT EXISTS security_links (
                    portfolio_name TEXT NOT NULL REFERENCES portfolios(name) ON DELETE CASCADE,
                    ledger_symbol TEXT NOT NULL,
                    data_symbol TEXT NOT NULL,
                    PRIMARY KEY (portfolio_name, ledger_symbol)
                );
                CREATE TABLE IF NOT EXISTS notes (
                    id TEXT PRIMARY KEY,
                    portfolio_name TEXT NOT NULL REFERENCES portfolios(name) ON DELETE CASCADE,
                    symbol TEXT NOT NULL,
                    date TEXT NOT NULL,
                    title TEXT NOT NULL,
                    body TEXT NOT NULL,
                    tags TEXT DEFAULT '[]',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_notes_portfolio ON notes(portfolio_name);
                CREATE INDEX IF NOT EXISTS idx_notes_symbol ON notes(symbol);
                CREATE TABLE IF NOT EXISTS files (
                    id TEXT PRIMARY KEY,
                    portfolio_name TEXT NOT NULL REFERENCES portfolios(name) ON DELETE CASCADE,
                    symbol TEXT NOT NULL,
                    date TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    mime_type TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    path TEXT NOT NULL,
                    notes TEXT DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_files_portfolio ON files(portfolio_name);
                CREATE INDEX IF NOT EXISTS idx_files_symbol ON files(symbol);"
            );
        }
        Self { db_path: path }
    }

    #[cfg(test)]
    pub fn with_dir(base_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&base_dir);
        let db_path = base_dir.join("master.db");
        if let Ok(conn) = Connection::open(&db_path) {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS portfolios (
                    name TEXT PRIMARY KEY,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS transactions (
                    id TEXT PRIMARY KEY,
                    portfolio_name TEXT NOT NULL,
                    date TEXT NOT NULL,
                    type TEXT NOT NULL CHECK(type IN ('buy','sell','dividend','deposit','withdrawal')),
                    symbol TEXT,
                    quantity REAL,
                    price REAL,
                    commission REAL DEFAULT 0,
                    amount REAL,
                    currency TEXT DEFAULT 'USD',
                    notes TEXT DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_tx_portfolio ON transactions(portfolio_name);
                CREATE INDEX IF NOT EXISTS idx_tx_date ON transactions(date);
                CREATE INDEX IF NOT EXISTS idx_tx_symbol ON transactions(symbol);
                CREATE TABLE IF NOT EXISTS price_cache (
                    portfolio_name TEXT NOT NULL,
                    symbol TEXT NOT NULL,
                    date TEXT NOT NULL,
                    close REAL NOT NULL,
                    source TEXT NOT NULL,
                    fetched_at TEXT NOT NULL,
                    PRIMARY KEY (portfolio_name, symbol, date)
                );
                CREATE TABLE IF NOT EXISTS security_links (
                    portfolio_name TEXT NOT NULL,
                    ledger_symbol TEXT NOT NULL,
                    data_symbol TEXT NOT NULL,
                    PRIMARY KEY (portfolio_name, ledger_symbol)
                );
                CREATE TABLE IF NOT EXISTS notes (
                    id TEXT PRIMARY KEY,
                    portfolio_name TEXT NOT NULL,
                    symbol TEXT NOT NULL,
                    date TEXT NOT NULL,
                    title TEXT NOT NULL,
                    body TEXT NOT NULL,
                    tags TEXT DEFAULT '[]',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_notes_portfolio ON notes(portfolio_name);
                CREATE INDEX IF NOT EXISTS idx_notes_symbol ON notes(symbol);
                CREATE TABLE IF NOT EXISTS files (
                    id TEXT PRIMARY KEY,
                    portfolio_name TEXT NOT NULL,
                    symbol TEXT NOT NULL,
                    date TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    mime_type TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    path TEXT NOT NULL,
                    notes TEXT DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_files_portfolio ON files(portfolio_name);
                CREATE INDEX IF NOT EXISTS idx_files_symbol ON files(symbol);"
            );
        }
        Self { db_path }
    }

    fn open(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| format!("db open: {e}"))
    }

    /// Base directory for portfolio file storage (parent of master.db).
    fn base_dir(&self) -> &std::path::Path {
        self.db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
    }

    // ── Portfolio CRUD ───────────────────────────────────────────

    pub fn create(&self, name: &str) -> Result<(), String> {
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return Err("portfolio name must not be empty or contain path separators".into());
        }
        let conn = self.open()?;
        conn.execute(
            "INSERT INTO portfolios (name, created_at) VALUES (?1, ?2)",
            params![name, now_rfc3339()],
        )
        .map_err(|e| format!("create: {e}"))?;
        Ok(())
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let conn = self.open()?;
        let rows = conn
            .execute("DELETE FROM portfolios WHERE name = ?1", params![name])
            .map_err(|e| format!("delete: {e}"))?;
        if rows == 0 {
            return Err(format!("portfolio '{name}' does not exist"));
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare("SELECT name FROM portfolios ORDER BY name")
            .map_err(|e| format!("query: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query: {e}"))?;
        let mut names = Vec::new();
        for row in rows {
            names.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(names)
    }

    fn check_exists(&self, conn: &Connection, name: &str) -> Result<(), String> {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM portfolios WHERE name = ?1",
                params![name],
                |_| Ok(()),
            )
            .is_ok();
        if !exists {
            return Err(format!("portfolio '{name}' does not exist"));
        }
        Ok(())
    }

    #[allow(dead_code)] // exercised by the test suite only
    pub fn add_transaction(&self, name: &str, tx: &Transaction) -> Result<(), String> {
        let conn = self.open()?;
        self.check_exists(&conn, name)?;
        conn.execute(
            "INSERT INTO transactions (id, portfolio_name, date, type, symbol, quantity, price, commission, amount, currency, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                tx.id,
                name,
                tx.date,
                tx.tx_type,
                tx.symbol,
                tx.quantity,
                tx.price,
                tx.commission,
                tx.amount,
                tx.currency,
                tx.notes,
                tx.created_at,
            ],
        )
        .map_err(|e| format!("insert: {e}"))?;
        Ok(())
    }

    pub fn append_note(&self, name: &str, tx_id: &str, note: &str) -> Result<(), String> {
        let conn = self.open()?;
        self.check_exists(&conn, name)?;
        let existing: String = conn
            .query_row(
                "SELECT notes FROM transactions WHERE id = ?1 AND portfolio_name = ?2",
                params![tx_id, name],
                |row| row.get(0),
            )
            .map_err(|e| format!("lookup: {e}"))?;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let updated = if existing.is_empty() {
            format!("[{timestamp}] {note}")
        } else {
            format!("{existing}\n[{timestamp}] {note}")
        };
        conn.execute(
            "UPDATE transactions SET notes = ?1 WHERE id = ?2 AND portfolio_name = ?3",
            params![updated, tx_id, name],
        )
        .map_err(|e| format!("update: {e}"))?;
        Ok(())
    }

    pub fn get_transactions(
        &self,
        name: &str,
        symbol: Option<&str>,
        tx_type: Option<&str>,
        from_date: Option<&str>,
        to_date: Option<&str>,
    ) -> Result<Vec<Transaction>, String> {
        let conn = self.open()?;
        self.check_exists(&conn, name)?;
        let mut sql = "SELECT id, date, type, symbol, quantity, price, commission, amount, currency, notes, created_at FROM transactions WHERE portfolio_name = ?1".to_string();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(name.to_string())];

        if let Some(s) = symbol {
            bind_values.push(Box::new(s.to_string()));
            sql.push_str(&format!(" AND symbol = ?{}", bind_values.len()));
        }
        if let Some(t) = tx_type {
            bind_values.push(Box::new(t.to_string()));
            sql.push_str(&format!(" AND type = ?{}", bind_values.len()));
        }
        if let Some(f) = from_date {
            bind_values.push(Box::new(f.to_string()));
            sql.push_str(&format!(" AND date >= ?{}", bind_values.len()));
        }
        if let Some(t) = to_date {
            bind_values.push(Box::new(t.to_string()));
            sql.push_str(&format!(" AND date <= ?{}", bind_values.len()));
        }
        sql.push_str(" ORDER BY date ASC");

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            bind_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("query: {e}"))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(Transaction {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    tx_type: row.get(2)?,
                    symbol: row.get(3)?,
                    quantity: row.get(4)?,
                    price: row.get(5)?,
                    commission: row.get(6)?,
                    amount: row.get(7)?,
                    currency: row.get::<_, String>(8).unwrap_or_default(),
                    notes: row.get::<_, String>(9).unwrap_or_default(),
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut txs = Vec::new();
        for row in rows {
            txs.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(txs)
    }

    // ── Validation ───────────────────────────────────────────────

    pub fn validate(&self, name: &str) -> Result<ValidationReport, String> {
        let txs = self.get_transactions(name, None, None, None, None)?;
        let mut issues = Vec::new();
        let mut positions: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        let mut cash = 0.0f64;

        for tx in &txs {
            match tx.tx_type.as_str() {
                "buy" => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    let price = tx.price.unwrap_or(0.0);
                    let comm = tx.commission.unwrap_or(0.0);
                    if qty <= 0.0 {
                        issues.push(format!("{}: buy with non-positive quantity {}", tx.id, qty));
                    }
                    if price <= 0.0 {
                        issues.push(format!("{}: buy with non-positive price {}", tx.id, price));
                    }
                    if let Some(ref sym) = tx.symbol {
                        let entry = positions.entry(sym.clone()).or_insert((0.0, 0.0));
                        entry.0 += qty;
                    }
                    cash -= qty * price + comm;
                }
                "sell" => {
                    let qty = tx.quantity.unwrap_or(0.0);
                    let price = tx.price.unwrap_or(0.0);
                    let comm = tx.commission.unwrap_or(0.0);
                    if qty <= 0.0 {
                        issues.push(format!(
                            "{}: sell with non-positive quantity {}",
                            tx.id, qty
                        ));
                    }
                    if price <= 0.0 {
                        issues.push(format!("{}: sell with non-positive price {}", tx.id, price));
                    }
                    if let Some(ref sym) = tx.symbol {
                        let entry = positions.entry(sym.clone()).or_insert((0.0, 0.0));
                        entry.1 += qty;
                    }
                    cash += qty * price - comm;
                }
                "dividend" => {
                    let amt = tx.amount.unwrap_or(0.0);
                    cash += amt;
                }
                "deposit" => {
                    let amt = tx.amount.unwrap_or(0.0);
                    if amt <= 0.0 {
                        issues.push(format!(
                            "{}: deposit with non-positive amount {}",
                            tx.id, amt
                        ));
                    }
                    cash += amt;
                }
                "withdrawal" => {
                    let amt = tx.amount.unwrap_or(0.0);
                    if amt <= 0.0 {
                        issues.push(format!(
                            "{}: withdrawal with non-positive amount {}",
                            tx.id, amt
                        ));
                    }
                    cash -= amt;
                }
                other => {
                    issues.push(format!("{}: unknown transaction type '{}'", tx.id, other));
                }
            }
        }

        let position_summaries: Vec<PositionSummary> = positions
            .into_iter()
            .map(|(symbol, (buys, sells))| PositionSummary {
                symbol,
                shares: buys - sells,
                total_buys: buys,
                total_sells: sells,
            })
            .filter(|p| p.shares.abs() > 0.0001 || p.total_buys > 0.0 || p.total_sells > 0.0)
            .collect();

        Ok(ValidationReport {
            valid: issues.is_empty(),
            transaction_count: txs.len(),
            positions: position_summaries,
            cash_balance: cash,
            issues,
        })
    }

    // ── Import / Export ──────────────────────────────────────────

    pub fn import_json(&self, name: &str, json: &str) -> Result<Vec<String>, String> {
        let txs: Vec<Transaction> =
            serde_json::from_str(json).map_err(|e| format!("invalid JSON: {e}"))?;
        self.import_transactions(name, txs)
    }

    pub fn import_csv(&self, name: &str, csv: &str) -> Result<Vec<String>, String> {
        let mut txs = Vec::new();
        let mut lines = csv.lines();
        let header = lines.next().ok_or("CSV has no header row")?;
        let columns: Vec<&str> = header.split(',').map(|c| c.trim()).collect();

        // Map column names to field indices
        let idx = |name: &str| columns.iter().position(|c| *c == name);

        for (line_num, line) in lines.enumerate() {
            let line_num = line_num + 2; // 1-indexed, header is line 1
            if line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split(',').map(|f| f.trim()).collect();

            let get_str = |col: &str| -> Option<String> {
                idx(col).and_then(|i| fields.get(i)).map(|s| s.to_string())
            };
            let get_f64 = |col: &str| -> Option<f64> {
                idx(col)
                    .and_then(|i| fields.get(i))
                    .and_then(|s| s.parse().ok())
            };

            let tx_type =
                get_str("type").ok_or(format!("line {line_num}: missing 'type' column"))?;
            let date = get_str("date").unwrap_or_default();
            let symbol = get_str("symbol");
            let quantity = get_f64("quantity");
            let price = get_f64("price");
            let commission = get_f64("commission");
            let amount = get_f64("amount");
            let currency = get_str("currency").unwrap_or_else(|| "USD".into());
            let notes = get_str("notes").unwrap_or_default();

            if date.is_empty() {
                return Err(format!("line {line_num}: missing date"));
            }

            txs.push(Transaction {
                id: uuid::Uuid::new_v4().to_string(),
                date,
                tx_type,
                symbol,
                quantity,
                price,
                commission,
                amount,
                currency,
                notes,
                created_at: now_rfc3339(),
            });
        }

        self.import_transactions(name, txs)
    }

    fn import_transactions(
        &self,
        name: &str,
        txs: Vec<Transaction>,
    ) -> Result<Vec<String>, String> {
        let conn = self.open()?;
        self.check_exists(&conn, name)?;
        let mut imported = Vec::new();
        for tx in &txs {
            match conn.execute(
                "INSERT OR IGNORE INTO transactions (id, portfolio_name, date, type, symbol, quantity, price, commission, amount, currency, notes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    tx.id,
                    name,
                    tx.date,
                    tx.tx_type,
                    tx.symbol,
                    tx.quantity,
                    tx.price,
                    tx.commission,
                    tx.amount,
                    tx.currency,
                    tx.notes,
                    tx.created_at,
                ],
            ) {
                Ok(1) => imported.push(tx.id.clone()),
                Ok(0) => {} // duplicate, silently skipped
                Ok(_) => unreachable!(),
                Err(e) => return Err(format!("insert {}: {e}", tx.id)),
            }
        }
        Ok(imported)
    }

    pub fn export_json(&self, name: &str) -> Result<String, String> {
        let txs = self.get_transactions(name, None, None, None, None)?;
        serde_json::to_string_pretty(&txs).map_err(|e| format!("serialize: {e}"))
    }

    pub fn export_csv(&self, name: &str) -> Result<String, String> {
        let txs = self.get_transactions(name, None, None, None, None)?;
        let mut out = String::from(
            "id,date,type,symbol,quantity,price,commission,amount,currency,notes,created_at\n",
        );
        for tx in &txs {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                tx.id,
                tx.date,
                tx.tx_type,
                tx.symbol.as_deref().unwrap_or(""),
                tx.quantity.map_or("".to_string(), |v| v.to_string()),
                tx.price.map_or("".to_string(), |v| v.to_string()),
                tx.commission.map_or("".to_string(), |v| v.to_string()),
                tx.amount.map_or("".to_string(), |v| v.to_string()),
                tx.currency,
                tx.notes.replace(',', ";"),
                tx.created_at,
            ));
        }
        Ok(out)
    }

    // ── Data linkage ─────────────────────────────────────────────

    /// Get all unique symbols from a portfolio's ledger.
    pub fn get_symbols(&self, name: &str) -> Result<Vec<String>, String> {
        let conn = self.open()?;
        self.check_exists(&conn, name)?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT symbol FROM transactions WHERE portfolio_name = ?1 AND symbol IS NOT NULL AND symbol != ''")
            .map_err(|e| format!("query: {e}"))?;
        let rows = stmt
            .query_map(params![name], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query: {e}"))?;
        let mut symbols = Vec::new();
        for row in rows {
            symbols.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(symbols)
    }

    /// Get cached prices for a symbol in a date range.
    pub fn get_prices(
        &self,
        name: &str,
        symbol: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, f64, String)>, String> {
        let conn = self.open()?;
        let mut stmt = conn
            .prepare("SELECT date, close, source FROM price_cache WHERE portfolio_name = ?1 AND symbol = ?2 AND date >= ?3 AND date <= ?4 ORDER BY date")
            .map_err(|e| format!("query: {e}"))?;
        let rows = stmt
            .query_map(params![name, symbol, from, to], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("query: {e}"))?;
        let mut prices = Vec::new();
        for row in rows {
            prices.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(prices)
    }

    // ── Portfolio comparison ────────────────────────────────────

    /// Compare two portfolios — side-by-side positions, overlap, unique symbols.
    pub fn compare(&self, name_a: &str, name_b: &str) -> Result<serde_json::Value, String> {
        let report_a = self.validate(name_a)?;
        let report_b = self.validate(name_b)?;

        let positions_a: std::collections::HashMap<&str, &PositionSummary> = report_a
            .positions
            .iter()
            .map(|p| (p.symbol.as_str(), p))
            .collect();
        let positions_b: std::collections::HashMap<&str, &PositionSummary> = report_b
            .positions
            .iter()
            .map(|p| (p.symbol.as_str(), p))
            .collect();

        let all_symbols: std::collections::BTreeSet<&str> = positions_a
            .keys()
            .chain(positions_b.keys())
            .copied()
            .collect();

        let mut shared = Vec::new();
        let mut only_a = Vec::new();
        let mut only_b = Vec::new();

        for sym in &all_symbols {
            match (positions_a.get(sym), positions_b.get(sym)) {
                (Some(pa), Some(pb)) => shared.push(serde_json::json!({
                    "symbol": sym,
                    "shares_a": pa.shares,
                    "shares_b": pb.shares,
                    "buys_a": pa.total_buys,
                    "sells_a": pa.total_sells,
                    "buys_b": pb.total_buys,
                    "sells_b": pb.total_sells,
                })),
                (Some(pa), None) => only_a.push(serde_json::json!({
                    "symbol": sym,
                    "shares": pa.shares,
                    "buys": pa.total_buys,
                    "sells": pa.total_sells,
                })),
                (None, Some(pb)) => only_b.push(serde_json::json!({
                    "symbol": sym,
                    "shares": pb.shares,
                    "buys": pb.total_buys,
                    "sells": pb.total_sells,
                })),
                (None, None) => unreachable!(),
            }
        }

        Ok(serde_json::json!({
            "portfolio_a": {
                "name": name_a,
                "transactions": report_a.transaction_count,
                "positions": report_a.positions.len(),
                "cash": report_a.cash_balance,
            },
            "portfolio_b": {
                "name": name_b,
                "transactions": report_b.transaction_count,
                "positions": report_b.positions.len(),
                "cash": report_b.cash_balance,
            },
            "shared_positions": shared,
            "only_in_a": only_a,
            "only_in_b": only_b,
        }))
    }

    // ── Notes ────────────────────────────────────────────────────

    /// Add a note to a company/security as of a date. Returns the note ID.
    pub fn add_note(
        &self,
        portfolio: &str,
        symbol: &str,
        date: &str,
        title: &str,
        body: &str,
        tags: &[String],
    ) -> Result<String, String> {
        let conn = self.open()?;
        self.check_exists(&conn, portfolio)?;
        let id = uuid::Uuid::new_v4().to_string();
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO notes (id, portfolio_name, symbol, date, title, body, tags, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, portfolio, symbol, date, title, body, tags_json, now],
        )
        .map_err(|e| format!("add_note: {e}"))?;
        Ok(id)
    }

    /// List notes for a symbol, optionally filtered by date range or tags.
    pub fn list_notes(
        &self,
        portfolio: &str,
        symbol: &str,
        date_from: Option<&str>,
        date_to: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.open()?;
        self.check_exists(&conn, portfolio)?;
        let mut sql = "SELECT id, symbol, date, title, body, tags, created_at FROM notes WHERE portfolio_name = ?1 AND symbol = ?2".to_string();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
            Box::new(portfolio.to_string()),
            Box::new(symbol.to_string()),
        ];

        if let Some(f) = date_from {
            bind_values.push(Box::new(f.to_string()));
            sql.push_str(&format!(" AND date >= ?{}", bind_values.len()));
        }
        if let Some(t) = date_to {
            bind_values.push(Box::new(t.to_string()));
            sql.push_str(&format!(" AND date <= ?{}", bind_values.len()));
        }
        sql.push_str(" ORDER BY date DESC");

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            bind_values.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql).map_err(|e| format!("query: {e}"))?;
        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let tags_str: String = row.get::<_, String>(5).unwrap_or_default();
                let parsed_tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "symbol": row.get::<_, String>(1)?,
                    "date": row.get::<_, String>(2)?,
                    "title": row.get::<_, String>(3)?,
                    "body": row.get::<_, String>(4)?,
                    "tags": parsed_tags,
                    "created_at": row.get::<_, String>(6)?,
                }))
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut notes = Vec::new();
        for row in rows {
            let note = row.map_err(|e| format!("row: {e}"))?;
            // Apply tag filter in-memory if specified
            if let Some(filter_tags) = tags {
                let note_tags: Vec<&str> = note["tags"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();
                let has_any = filter_tags.iter().any(|t| note_tags.contains(&t.as_str()));
                if !has_any {
                    continue;
                }
            }
            notes.push(note);
        }
        Ok(notes)
    }

    /// Delete a note by ID.
    pub fn delete_note(&self, note_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        let rows = conn
            .execute("DELETE FROM notes WHERE id = ?1", params![note_id])
            .map_err(|e| format!("delete_note: {e}"))?;
        if rows == 0 {
            return Err(format!("note '{note_id}' not found"));
        }
        Ok(())
    }

    // ── File attachments ─────────────────────────────────────────

    /// Attach a file to a company/security. Receives base64-encoded content,
    /// writes to `portfolios/{name}/files/{uuid}_{filename}`, returns the file ID.
    #[allow(clippy::too_many_arguments)]
    pub fn attach_file(
        &self,
        portfolio: &str,
        symbol: &str,
        date: &str,
        filename: &str,
        mime_type: &str,
        data_b64: &str,
        notes: &str,
    ) -> Result<String, String> {
        let conn = self.open()?;
        self.check_exists(&conn, portfolio)?;

        // Decode base64
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
            .map_err(|e| format!("invalid base64 data: {e}"))?;

        let id = uuid::Uuid::new_v4().to_string();
        let safe_filename = format!("{id}_{filename}");
        let files_dir = self.base_dir().join(portfolio).join("files");
        let _ = std::fs::create_dir_all(&files_dir);
        let file_path = files_dir.join(&safe_filename);

        std::fs::write(&file_path, &bytes).map_err(|e| format!("write file: {e}"))?;

        let path_str = file_path.to_string_lossy().to_string();
        let size = bytes.len() as i64;
        let now = now_rfc3339();

        conn.execute(
            "INSERT INTO files (id, portfolio_name, symbol, date, filename, mime_type, size, path, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, portfolio, symbol, date, filename, mime_type, size, path_str, notes, now],
        )
        .map_err(|e| format!("attach_file: {e}"))?;

        Ok(id)
    }

    /// List attached files for a symbol in a portfolio.
    pub fn list_files(
        &self,
        portfolio: &str,
        symbol: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        let conn = self.open()?;
        self.check_exists(&conn, portfolio)?;
        let mut stmt = conn
            .prepare(
                "SELECT id, symbol, date, filename, mime_type, size, path, notes, created_at FROM files WHERE portfolio_name = ?1 AND symbol = ?2 ORDER BY date DESC",
            )
            .map_err(|e| format!("query: {e}"))?;
        let rows = stmt
            .query_map(params![portfolio, symbol], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "symbol": row.get::<_, String>(1)?,
                    "date": row.get::<_, String>(2)?,
                    "filename": row.get::<_, String>(3)?,
                    "mime_type": row.get::<_, String>(4)?,
                    "size": row.get::<_, i64>(5)?,
                    "path": row.get::<_, String>(6)?,
                    "notes": row.get::<_, String>(7)?,
                    "created_at": row.get::<_, String>(8)?,
                }))
            })
            .map_err(|e| format!("query: {e}"))?;

        let mut files = Vec::new();
        for row in rows {
            files.push(row.map_err(|e| format!("row: {e}"))?);
        }
        Ok(files)
    }

    /// Delete an attached file by ID — removes DB record and physical file.
    pub fn delete_file(&self, file_id: &str) -> Result<(), String> {
        let conn = self.open()?;
        // Look up the file path first
        let path: String = conn
            .query_row(
                "SELECT path FROM files WHERE id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("lookup: {e}"))?;

        let rows = conn
            .execute("DELETE FROM files WHERE id = ?1", params![file_id])
            .map_err(|e| format!("delete_file: {e}"))?;
        if rows == 0 {
            return Err(format!("file '{file_id}' not found"));
        }

        // Best-effort delete of the physical file
        let _ = std::fs::remove_file(&path);
        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tx(id: &str, tx_type: &str, symbol: &str, qty: f64, price: f64) -> Transaction {
        Transaction {
            id: id.to_string(),
            date: "2024-06-15".to_string(),
            tx_type: tx_type.to_string(),
            symbol: Some(symbol.to_string()),
            quantity: Some(qty),
            price: Some(price),
            commission: Some(1.0),
            amount: None,
            currency: "USD".to_string(),
            notes: String::new(),
            created_at: "2024-06-15T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn portfolio_create_list_delete() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());

        pm.create("test1").unwrap();
        pm.create("test2").unwrap();

        let list = pm.list().unwrap();
        assert!(list.contains(&"test1".to_string()));
        assert!(list.contains(&"test2".to_string()));

        assert!(pm.create("test1").is_err()); // duplicate

        pm.delete("test1").unwrap();
        let list = pm.list().unwrap();
        assert!(!list.contains(&"test1".to_string()));
        assert!(list.contains(&"test2".to_string()));

        assert!(pm.delete("nonexistent").is_err());
    }

    #[test]
    fn transaction_add_and_retrieve() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let tx = sample_tx("t1", "buy", "AAPL", 10.0, 150.0);
        pm.add_transaction("test", &tx).unwrap();

        let txs = pm.get_transactions("test", None, None, None, None).unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].id, "t1");
        assert_eq!(txs[0].symbol.as_deref(), Some("AAPL"));

        pm.append_note("test", "t1", "buying the dip").unwrap();
        let txs = pm.get_transactions("test", None, None, None, None).unwrap();
        assert!(txs[0].notes.contains("buying the dip"));
    }

    #[test]
    fn transaction_filtering() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        pm.add_transaction("test", &sample_tx("t1", "buy", "AAPL", 10.0, 150.0))
            .unwrap();
        let mut tx2 = sample_tx("t2", "sell", "MSFT", 5.0, 300.0);
        tx2.date = "2024-07-01".to_string();
        pm.add_transaction("test", &tx2).unwrap();

        let aapl = pm
            .get_transactions("test", Some("AAPL"), None, None, None)
            .unwrap();
        assert_eq!(aapl.len(), 1);

        let sells = pm
            .get_transactions("test", None, Some("sell"), None, None)
            .unwrap();
        assert_eq!(sells.len(), 1);

        let july = pm
            .get_transactions("test", None, None, Some("2024-07-01"), None)
            .unwrap();
        assert_eq!(july.len(), 1);
    }

    #[test]
    fn validate_positions_and_cash() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        pm.add_transaction("test", &sample_tx("t1", "buy", "AAPL", 10.0, 150.0))
            .unwrap();
        pm.add_transaction("test", &sample_tx("t2", "buy", "AAPL", 5.0, 155.0))
            .unwrap();
        pm.add_transaction("test", &sample_tx("t3", "sell", "AAPL", 3.0, 160.0))
            .unwrap();

        let deposit = Transaction {
            id: "d1".to_string(),
            date: "2024-06-15".to_string(),
            tx_type: "deposit".to_string(),
            symbol: None,
            quantity: None,
            price: None,
            commission: None,
            amount: Some(10000.0),
            currency: "USD".to_string(),
            notes: String::new(),
            created_at: "2024-06-15T00:00:00Z".to_string(),
        };
        pm.add_transaction("test", &deposit).unwrap();

        let report = pm.validate("test").unwrap();
        assert!(report.valid);
        assert_eq!(report.transaction_count, 4);

        let aapl = report
            .positions
            .iter()
            .find(|p| p.symbol == "AAPL")
            .unwrap();
        assert!((aapl.shares - 12.0).abs() < 0.001);
        assert!((aapl.total_buys - 15.0).abs() < 0.001);
        assert!((aapl.total_sells - 3.0).abs() < 0.001);

        // Cash: 10000 - (10*150 + 1) - (5*155 + 1) + (3*160 - 1) = 8202
        assert!((report.cash_balance - 8202.0).abs() < 0.01);
    }

    #[test]
    fn csv_import() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let csv = "\
type,date,symbol,quantity,price,commission,amount
buy,2024-01-15,AAPL,10,150.0,1.0,
sell,2024-02-20,AAPL,3,160.0,1.0,
dividend,2024-03-01,AAPL,,,,0.5
deposit,2024-01-01,,,,,10000.0
";

        let imported = pm.import_csv("test", csv).unwrap();
        assert_eq!(imported.len(), 4);

        let txs = pm.get_transactions("test", None, None, None, None).unwrap();
        assert_eq!(txs.len(), 4);

        let report = pm.validate("test").unwrap();
        assert!(report.valid);
    }

    #[test]
    fn json_import() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let json = r#"[
            {"id":"a","date":"2024-01-15","type":"buy","symbol":"AAPL","quantity":10.0,"price":150.0,"commission":1.0,"created_at":"2024-01-15T00:00:00Z"},
            {"id":"b","date":"2024-02-20","type":"sell","symbol":"AAPL","quantity":3.0,"price":160.0,"commission":1.0,"created_at":"2024-02-20T00:00:00Z"}
        ]"#;

        let imported = pm.import_json("test", json).unwrap();
        assert_eq!(imported.len(), 2);

        let txs = pm.get_transactions("test", None, None, None, None).unwrap();
        assert_eq!(txs.len(), 2);
    }

    #[test]
    fn export_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        pm.add_transaction("test", &sample_tx("t1", "buy", "AAPL", 10.0, 150.0))
            .unwrap();

        let json = pm.export_json("test").unwrap();
        assert!(json.contains("AAPL"));

        let csv = pm.export_csv("test").unwrap();
        assert!(csv.contains("AAPL"));
        assert!(csv.contains("buy"));
    }

    #[test]
    fn validate_detects_issues() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let mut bad = sample_tx("bad1", "buy", "AAPL", 0.0, 150.0);
        bad.quantity = Some(0.0);
        pm.add_transaction("test", &bad).unwrap();

        let report = pm.validate("test").unwrap();
        assert!(!report.valid);
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.contains("non-positive quantity"))
        );
    }

    #[test]
    fn notes_crud() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let id = pm
            .add_note(
                "test",
                "AAPL",
                "2024-06-15",
                "Earnings review",
                "Beat estimates by 5%",
                &["earnings".into(), "bullish".into()],
            )
            .unwrap();
        assert!(!id.is_empty());

        // List all
        let notes = pm.list_notes("test", "AAPL", None, None, None).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0]["title"], "Earnings review");

        // Filter by tag
        let tagged = pm
            .list_notes("test", "AAPL", None, None, Some(&["earnings".into()]))
            .unwrap();
        assert_eq!(tagged.len(), 1);

        let not_found = pm
            .list_notes("test", "AAPL", None, None, Some(&["bearish".into()]))
            .unwrap();
        assert!(not_found.is_empty());

        // Filter by date range
        let in_range = pm
            .list_notes("test", "AAPL", Some("2024-01-01"), Some("2024-12-31"), None)
            .unwrap();
        assert_eq!(in_range.len(), 1);

        let out_of_range = pm
            .list_notes("test", "AAPL", Some("2025-01-01"), None, None)
            .unwrap();
        assert!(out_of_range.is_empty());

        // Delete
        pm.delete_note(&id).unwrap();
        let empty = pm.list_notes("test", "AAPL", None, None, None).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn delete_nonexistent_note() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        assert!(pm.delete_note("nonexistent-id").is_err());
    }

    #[test]
    fn file_attach_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let data = base64::engine::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"Hello, portfolio!",
        );

        let id = pm
            .attach_file(
                "test",
                "AAPL",
                "2024-06-15",
                "notes.txt",
                "text/plain",
                &data,
                "my research notes",
            )
            .unwrap();
        assert!(!id.is_empty());

        let files = pm.list_files("test", "AAPL").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["filename"], "notes.txt");
        assert_eq!(files[0]["mime_type"], "text/plain");
        assert_eq!(files[0]["size"], 17);
        assert_eq!(files[0]["notes"], "my research notes");

        // Verify file exists on disk
        let disk_path = files[0]["path"].as_str().unwrap();
        assert!(std::path::Path::new(disk_path).exists());
        let contents = std::fs::read_to_string(disk_path).unwrap();
        assert_eq!(contents, "Hello, portfolio!");
    }

    #[test]
    fn file_delete() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let data = base64::engine::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"temp file",
        );

        let id = pm
            .attach_file(
                "test",
                "MSFT",
                "2024-01-01",
                "temp.txt",
                "text/plain",
                &data,
                "",
            )
            .unwrap();

        let files = pm.list_files("test", "MSFT").unwrap();
        let disk_path = files[0]["path"].as_str().unwrap().to_string();

        pm.delete_file(&id).unwrap();
        assert!(pm.list_files("test", "MSFT").unwrap().is_empty());
        assert!(!std::path::Path::new(&disk_path).exists());

        assert!(pm.delete_file("nonexistent").is_err());
    }
}
