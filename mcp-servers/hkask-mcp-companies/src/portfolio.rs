//! hKask MCP Companies — Portfolio tracking
//!
//! A portfolio is a ledger. Everything else is arithmetic on the ledger
//! at a point in time. This module manages the SQLite-backed transaction
//! ledger — create, read, validate, import, export.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    base_dir: PathBuf,
}

impl PortfolioManager {
    pub fn new() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("hkask");
        path.push("portfolios");
        let _ = std::fs::create_dir_all(&path);
        Self { base_dir: path }
    }

    #[cfg(test)]
    pub fn with_dir(base_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&base_dir);
        Self { base_dir }
    }

    fn db_path(&self, name: &str) -> PathBuf {
        self.base_dir.join(format!("{name}.db"))
    }

    // ── Portfolio CRUD ───────────────────────────────────────────

    pub fn create(&self, name: &str) -> Result<(), String> {
        if name.is_empty() || name.contains('/') || name.contains('\\') {
            return Err("portfolio name must not be empty or contain path separators".into());
        }
        let path = self.db_path(name);
        if path.exists() {
            return Err(format!("portfolio '{name}' already exists"));
        }
        let conn = Connection::open(&path).map_err(|e| format!("db open: {e}"))?;
        conn.execute_batch(
            "CREATE TABLE transactions (
                id TEXT PRIMARY KEY,
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
            CREATE INDEX idx_tx_date ON transactions(date);
            CREATE INDEX idx_tx_symbol ON transactions(symbol);",
        )
        .map_err(|e| format!("schema: {e}"))?;
        Ok(())
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let path = self.db_path(name);
        if !path.exists() {
            return Err(format!("portfolio '{name}' does not exist"));
        }
        std::fs::remove_file(&path).map_err(|e| format!("delete: {e}"))?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "db") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    fn open(&self, name: &str) -> Result<Connection, String> {
        let path = self.db_path(name);
        if !path.exists() {
            return Err(format!("portfolio '{name}' does not exist"));
        }
        Connection::open(&path).map_err(|e| format!("db open: {e}"))
    }

    pub fn add_transaction(&self, name: &str, tx: &Transaction) -> Result<(), String> {
        let conn = self.open(name)?;
        conn.execute(
            "INSERT INTO transactions (id, date, type, symbol, quantity, price, commission, amount, currency, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                tx.id,
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
        let conn = self.open(name)?;
        let existing: String = conn
            .query_row(
                "SELECT notes FROM transactions WHERE id = ?1",
                params![tx_id],
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
            "UPDATE transactions SET notes = ?1 WHERE id = ?2",
            params![updated, tx_id],
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
        let conn = self.open(name)?;
        let mut sql = "SELECT id, date, type, symbol, quantity, price, commission, amount, currency, notes, created_at FROM transactions WHERE 1=1".to_string();
        let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

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
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        self.import_transactions(name, txs)
    }

    fn import_transactions(
        &self,
        name: &str,
        txs: Vec<Transaction>,
    ) -> Result<Vec<String>, String> {
        let conn = self.open(name)?;
        let mut imported = Vec::new();
        for tx in &txs {
            match conn.execute(
                "INSERT OR IGNORE INTO transactions (id, date, type, symbol, quantity, price, commission, amount, currency, notes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    tx.id,
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

    // REQ: PORTFOLIO-CRUD — create, list, delete
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

    // REQ: PORTFOLIO-TX — add transaction, retrieve, append note
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

    // REQ: PORTFOLIO-TX — filter by symbol, type, date range
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

    // REQ: PORTFOLIO-VALIDATE — positions = buys - sells, cash consistency
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

    // REQ: PORTFOLIO-IMPORT — CSV import with header mapping
    #[test]
    fn csv_import() {
        let dir = tempfile::tempdir().unwrap();
        let pm = PortfolioManager::with_dir(dir.path().to_path_buf());
        pm.create("test").unwrap();

        let csv = "\
type,date,symbol,quantity,price,commission
buy,2024-01-15,AAPL,10,150.0,1.0
sell,2024-02-20,AAPL,3,160.0,1.0
dividend,2024-03-01,AAPL,,,0.5
deposit,2024-01-01,,,10000.0
";

        let imported = pm.import_csv("test", csv).unwrap();
        assert_eq!(imported.len(), 4);

        let txs = pm.get_transactions("test", None, None, None, None).unwrap();
        assert_eq!(txs.len(), 4);

        let report = pm.validate("test").unwrap();
        assert!(report.valid);
    }

    // REQ: PORTFOLIO-IMPORT — JSON import
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

    // REQ: PORTFOLIO-EXPORT — CSV and JSON export round-trips
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

    // REQ: PORTFOLIO-VALIDATE — detects bad transactions
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
}
