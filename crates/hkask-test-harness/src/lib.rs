//! hKask Test Harness — Shared fixtures for test infrastructure
//!
//! Public API:
//! - `TestDb` — isolated temp SQLite database with full schema
//! - `TestKeystore` — temp directory with test master key
//! - `TestWebId` — factory for valid test WebIDs
//! - `MockCnsRuntime` — CNS runtime with controllable state
//! - `MockInferencePort` — mock inference with canned responses
//! - `temp_dir()` — guarded temp directory, auto-cleans on drop
//! - `test_event()` / `test_triple()` — factories for well-formed test entities
//! - `strategies` — proptest strategy functions for core types
//! - `test_runner` — cargo test invocation and REQ-tagged failure parsing
//! - `ProbContractRunner` — (p, δ, k)-satisfaction for non-deterministic contracts
//!
//! # Principle grounding
//! - P5 (Essentialism): each public item does one thing well
//! - P8 (Semantic Grounding): every test using these fixtures carries REQ tags
//! - P12 (Replicant Host Mandate): all test identities use TestWebId (authenticated)

pub mod fuzz;
pub mod mocks;
pub mod prob_contract;
mod schema;
pub mod strategies;
pub mod test_runner;

pub use prob_contract::{ProbContractResult, ProbContractRunner};
pub use schema::SCHEMA_SQL;

use chrono::Utc;
use hkask_storage::Triple;
use hkask_types::event::{NuEvent, Phase, Span};
use hkask_types::id::WebID;
use rand::Rng;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tempfile::TempDir;

// ── TestDb ────────────────────────────────────────────────────────────────────

/// Isolated temp SQLite database with full hKask schema initialized.
///
/// Creates an in-memory database and runs the complete schema DDL.
/// The database is destroyed when `TestDb` is dropped.
///
/// # Example
/// ```ignore
/// let db = TestDb::new();
/// db.conn().execute("INSERT INTO triples ...", [])?;
/// ```
pub struct TestDb {
    conn: Arc<Mutex<Connection>>,
}

impl Default for TestDb {
    /// REQ: HARN-012b
    /// post: returns TestDb with in-memory SQLite connection and full schema initialized
    fn default() -> Self {
        Self::new()
    }
}

impl TestDb {
    /// Create a new in-memory test database with full schema.
    ///
    /// REQ: HARN-012
    /// post: returns TestDb with in-memory SQLite connection and full schema initialized
    pub fn new() -> Self {
        let conn = Connection::open_in_memory().expect("in-memory SQLite should always open");
        conn.execute_batch(SCHEMA_SQL)
            .expect("schema initialization should succeed");
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Borrow the underlying SQLite connection (locks the mutex).
    ///
    /// REQ: HARN-013
    /// post: returns `MutexGuard<Connection>` for direct SQL access
    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("mutex should not be poisoned")
    }

    /// Get the Arc<Mutex<Connection>> for Store constructors.
    ///
    /// REQ: HARN-014
    /// post: returns `Arc<Mutex<Connection>>` clone for Store::new()
    pub fn conn_arc(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Execute a batch of SQL statements (for seeding test data).
    ///
    /// REQ: HARN-015
    /// pre:  sql is valid SQL
    /// post: batch executed on the connection
    pub fn execute_batch(&self, sql: &str) -> Result<(), rusqlite::Error> {
        self.conn().execute_batch(sql)
    }
}

// ── TestKeystore ──────────────────────────────────────────────────────────────

/// Temp directory with a test master key file.
///
/// Creates a temporary directory containing a generated master key.
/// The directory and all contents are deleted when `TestKeystore` is dropped.
pub struct TestKeystore {
    dir: TempDir,
    key_path: PathBuf,
    master_key: [u8; 32],
}

impl Default for TestKeystore {
    /// REQ: HARN-016b
    /// post: returns TestKeystore with temp dir, key file written, 32-byte master key
    fn default() -> Self {
        Self::new()
    }
}

impl TestKeystore {
    /// Create a new test keystore with a randomly generated master key.
    ///
    /// REQ: HARN-016
    /// post: returns TestKeystore with temp dir, key file written, 32-byte master key
    pub fn new() -> Self {
        let dir = TempDir::new().expect("temp dir creation should succeed");
        let key_path = dir.path().join("master.key");
        let master_key: [u8; 32] = rand::rng().random();
        std::fs::write(&key_path, master_key).expect("key file write should succeed");
        Self {
            dir,
            key_path,
            master_key,
        }
    }

    /// Path to the keystore directory.
    ///
    /// REQ: HARN-017
    /// post: returns &Path to the temp directory
    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    /// Path to the master key file.
    ///
    /// REQ: HARN-018
    /// post: returns &Path to the master.key file
    pub fn key_path(&self) -> &std::path::Path {
        &self.key_path
    }

    /// The generated master key bytes.
    ///
    /// REQ: HARN-019
    /// post: returns &[u8; 32] reference to the master key
    pub fn master_key(&self) -> &[u8; 32] {
        &self.master_key
    }
}

// ── TestWebId ─────────────────────────────────────────────────────────────────

/// Factory for valid test WebIDs with known identities.
///
/// Provides deterministic WebIDs for common test personas (alice, bob, carol)
/// plus a random generator. All WebIDs are valid and carry authenticated identity.
///
/// # Example
/// ```ignore
/// let alice = TestWebId::alice();
/// let bob = TestWebId::bob();
/// let random = TestWebId::random();
/// ```
pub struct TestWebId;

impl TestWebId {
    /// Deterministic WebID for test user "alice".
    ///
    /// REQ: HARN-020
    /// post: returns deterministic WebID from persona b"alice"
    pub fn alice() -> WebID {
        WebID::from_persona(b"alice")
    }

    /// Deterministic WebID for test user "bob".
    ///
    /// REQ: HARN-021
    /// post: returns deterministic WebID from persona b"bob"
    pub fn bob() -> WebID {
        WebID::from_persona(b"bob")
    }

    /// Deterministic WebID for test user "carol".
    ///
    /// REQ: HARN-022
    /// post: returns deterministic WebID from persona b"carol"
    pub fn carol() -> WebID {
        WebID::from_persona(b"carol")
    }

    /// Generate a new random WebID.
    ///
    /// REQ: HARN-023
    /// post: returns new random WebID
    pub fn random() -> WebID {
        WebID::new()
    }

    /// Generate a WebID from arbitrary persona bytes.
    ///
    /// REQ: HARN-024
    /// pre:  bytes is non-empty
    /// post: returns deterministic WebID from persona bytes
    pub fn from_persona(bytes: &[u8]) -> WebID {
        WebID::from_persona(bytes)
    }
}

// ── MockCnsRuntime ────────────────────────────────────────────────────────────

/// CNS state for mock runtime — controllable in tests.
#[derive(Debug, Clone)]
pub struct MockCnsState {
    pub homeostatic: bool,
    pub throttled_tools: Vec<String>,
    pub recent_signals: Vec<MockAlgedonicSignal>,
    pub variety_counters: HashMap<String, u64>,
}

impl MockCnsState {
    /// Create a homeostatic (healthy) CNS state.
    ///
    /// REQ: HARN-025
    /// post: returns MockCnsState with homeostatic=true, no throttled tools, empty signals
    pub fn homeostatic() -> Self {
        Self {
            homeostatic: true,
            throttled_tools: Vec::new(),
            recent_signals: Vec::new(),
            variety_counters: HashMap::new(),
        }
    }

    /// Create a perturbed CNS state with a specific tool throttled.
    ///
    /// REQ: HARN-026
    /// pre:  throttled_tool is non-empty
    /// post: returns MockCnsState with homeostatic=false, tool throttled
    pub fn perturbed(throttled_tool: &str) -> Self {
        let mut state = Self::homeostatic();
        state.homeostatic = false;
        state.throttled_tools.push(throttled_tool.to_string());
        state
    }
}

/// Simplified algedonic signal for mock CNS.
#[derive(Debug, Clone)]
pub struct MockAlgedonicSignal {
    pub valence: SignalValence,
    pub message: String,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalValence {
    Positive,
    Negative,
    Neutral,
}

impl MockAlgedonicSignal {
    /// Check if signal has negative valence.
    ///
    /// REQ: HARN-027
    /// post: returns true iff valence == Negative
    pub fn is_negative_valence(&self) -> bool {
        self.valence == SignalValence::Negative
    }

    /// Check if signal has positive valence.
    ///
    /// REQ: HARN-028
    /// post: returns true iff valence == Positive
    pub fn is_positive_valence(&self) -> bool {
        self.valence == SignalValence::Positive
    }
}

/// Simplified CNS runtime mock for integration tests.
///
/// Provides controllable state, event injection, time advancement,
/// and signal observation — sufficient for testing CNS-dependent code
/// without a full running CNS daemon.
#[derive(Clone)]
pub struct MockCnsRuntime {
    state: Arc<RwLock<MockCnsState>>,
}

impl MockCnsRuntime {
    /// Create a new mock CNS runtime with homeostatic state.
    ///
    /// REQ: HARN-029
    /// post: returns MockCnsRuntime with homeostatic state
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(MockCnsState::homeostatic())),
        }
    }

    /// Create a mock CNS with a specific initial state.
    ///
    /// REQ: HARN-030
    /// pre:  state is a valid MockCnsState
    /// post: returns MockCnsRuntime with the given state
    pub fn with_state(state: MockCnsState) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Inject an event into the CNS (simulates a perturbation).
    ///
    /// REQ: HARN-031
    /// pre:  event is a valid NuEvent
    /// post: homeostatic set to false, negative signal appended
    pub fn inject(&self, event: NuEvent) {
        let mut state = self.state.write().unwrap();
        state.homeostatic = false;
        let signal = MockAlgedonicSignal {
            valence: SignalValence::Negative,
            message: format!("event: {:?}", event.span),
            timestamp: Utc::now(),
        };
        state.recent_signals.push(signal);
    }

    /// Advance mock time by a duration (simulates feedback processing).
    /// After sufficient time, the CNS may return toward homeostasis.
    ///
    /// REQ: HARN-032
    /// post: if duration >= 5s, homeostatic restored, throttled tools cleared, positive signal appended
    pub fn advance_time(&self, duration: std::time::Duration) {
        let mut state = self.state.write().unwrap();
        // After 5+ seconds, system trends toward homeostasis
        if duration >= std::time::Duration::from_secs(5) {
            state.homeostatic = true;
            state.throttled_tools.clear();
            let signal = MockAlgedonicSignal {
                valence: SignalValence::Positive,
                message: "homeostasis restored".to_string(),
                timestamp: Utc::now(),
            };
            state.recent_signals.push(signal);
        }
    }

    /// Get recent algedonic signals.
    ///
    /// REQ: HARN-033
    /// post: returns clone of recent_signals vector
    pub fn recent_signals(&self) -> Vec<MockAlgedonicSignal> {
        self.state.read().unwrap().recent_signals.clone()
    }

    /// Check if a specific tool is throttled.
    ///
    /// REQ: HARN-034
    /// pre:  tool_name is non-empty
    /// post: returns Throttled if tool in throttled_tools, Active otherwise
    pub fn tool_state(&self, tool_name: &str) -> MockToolState {
        let state = self.state.read().unwrap();
        if state.throttled_tools.iter().any(|t| t == tool_name) {
            MockToolState::Throttled
        } else {
            MockToolState::Active
        }
    }

    /// Check if the CNS is in homeostatic state.
    ///
    /// REQ: HARN-035
    /// post: returns true iff homeostatic flag is true
    pub fn is_homeostatic(&self) -> bool {
        self.state.read().unwrap().homeostatic
    }

    /// Record variety for a domain (simulates tool dispatch).
    ///
    /// REQ: HARN-036
    /// pre:  domain is non-empty
    /// post: variety counter for domain incremented by 1
    pub fn record_variety(&self, domain: &str) {
        let mut state = self.state.write().unwrap();
        *state
            .variety_counters
            .entry(domain.to_string())
            .or_insert(0) += 1;
    }

    /// Get variety count for a domain.
    ///
    /// REQ: HARN-037
    /// pre:  domain is non-empty
    /// post: returns variety count for domain, 0 if never recorded
    pub fn variety_for_domain(&self, domain: &str) -> u64 {
        self.state
            .read()
            .unwrap()
            .variety_counters
            .get(domain)
            .copied()
            .unwrap_or(0)
    }
}

impl Default for MockCnsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool state as reported by mock CNS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockToolState {
    Active,
    Throttled,
}

// ── temp_dir ──────────────────────────────────────────────────────────────────

/// Create a guarded temp directory that auto-cleans on drop.
///
/// # Example
/// ```ignore
/// let dir = temp_dir();
/// let path = dir.path().join("test.txt");
/// std::fs::write(&path, b"data")?;
/// // dir and contents deleted when `dir` goes out of scope
/// ```
///
/// REQ: HARN-038
/// post: returns TempDir that auto-cleans on drop
pub fn temp_dir() -> TempDir {
    TempDir::new().expect("temp dir creation should succeed")
}

// ── test_event ────────────────────────────────────────────────────────────────

/// Create a well-formed test NuEvent with required fields.
///
/// Uses a random observer WebID unless `observer` is provided.
///
/// # Example
/// ```ignore
/// let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
/// let event = test_event(span, Phase::Observation, None);
/// assert!(event.observer_webid.as_uuid().is_set());
/// ```
///
/// REQ: HARN-039
/// pre:  span is a valid Span, phase is a valid Phase
/// post: returns NuEvent with random observer if observer is None, depth=0, test observation
pub fn test_event(span: Span, phase: Phase, observer: Option<WebID>) -> NuEvent {
    NuEvent::new(
        observer.unwrap_or_else(TestWebId::random),
        span,
        phase,
        serde_json::json!({"test": true}),
        0,
    )
}

// ── test_triple ───────────────────────────────────────────────────────────────

/// Create a well-formed test Triple with required fields.
///
/// Uses a random owner WebID unless `owner` is provided.
///
/// # Example
/// ```ignore
/// let triple = test_triple("entity:test", "attribute:name", json!("value"), None);
/// assert_eq!(triple.entity, "entity:test");
/// ```
///
/// REQ: HARN-041
/// pre:  entity and attribute are non-empty, value is valid JSON
/// post: returns Triple with random owner if owner is None, specified owner otherwise
pub fn test_triple(entity: &str, attribute: &str, value: Value, owner: Option<WebID>) -> Triple {
    Triple::new(
        entity,
        attribute,
        value,
        owner.unwrap_or_else(TestWebId::random),
    )
}

// ── Internal helpers (not public) ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::SpanNamespace;

    // REQ: HAR-001 — TestDb creates valid in-memory database (P5, P8)
    #[test]
    fn test_db_creates_valid_database() {
        let db = TestDb::new();
        // Verify schema was initialized by querying a known table
        let result: Result<String, _> = db.conn().query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='triples'",
            [],
            |row| row.get(0),
        );
        assert_eq!(result.unwrap(), "triples");
    }

    // REQ: HAR-002 — TestKeystore creates valid key file (P5, P8)
    #[test]
    fn test_keystore_creates_key_file() {
        let ks = TestKeystore::new();
        assert!(ks.key_path().exists());
        assert_eq!(ks.master_key().len(), 32);
        let stored = std::fs::read(ks.key_path()).unwrap();
        assert_eq!(stored.len(), 32);
    }

    // REQ: HAR-003 — TestWebId produces deterministic identities (P12, P8)
    #[test]
    fn test_webid_deterministic() {
        let a1 = TestWebId::alice();
        let a2 = TestWebId::alice();
        assert_eq!(a1, a2, "same persona must produce same WebID");

        let b = TestWebId::bob();
        assert_ne!(a1, b, "different personas must produce different WebIDs");
    }

    // REQ: HAR-004 — MockCnsRuntime detects perturbations (P9, P8)
    #[test]
    fn mock_cns_detects_perturbation() {
        let cns = MockCnsRuntime::new();
        assert!(cns.is_homeostatic());

        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let event = test_event(span, Phase::Sense, None);
        cns.inject(event);

        assert!(!cns.is_homeostatic());
        let signals = cns.recent_signals();
        assert!(signals.iter().any(|s| s.is_negative_valence()));
    }

    // REQ: HAR-005 — MockCnsRuntime restores homeostasis after time (P9, P8)
    #[test]
    fn mock_cns_restores_homeostasis() {
        let cns = MockCnsRuntime::new();
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        cns.inject(test_event(span, Phase::Sense, None));
        assert!(!cns.is_homeostatic());

        cns.advance_time(std::time::Duration::from_secs(10));
        assert!(cns.is_homeostatic());
        let signals = cns.recent_signals();
        assert!(signals.iter().any(|s| s.is_positive_valence()));
    }

    // REQ: HAR-006 — temp_dir creates usable directory (P5, P8)
    #[test]
    fn temp_dir_is_usable() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();
        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hello");
    }

    // REQ: HAR-007 — test_event produces valid NuEvent (P8)
    #[test]
    fn test_event_is_valid() {
        let span = Span::new(SpanNamespace::new("cns.tool"), "invoked");
        let event = test_event(span, Phase::Sense, None);
        assert!(!event.id.as_uuid().is_nil());
        assert!(!event.observer_webid.as_uuid().is_nil());
        assert_eq!(event.recursion_depth, 0);
    }

    // REQ: HAR-008 — test_triple produces valid Triple (P8)
    #[test]
    fn test_triple_is_valid() {
        let triple = test_triple("entity:test", "attr:name", serde_json::json!("value"), None);
        assert_eq!(triple.entity, "entity:test");
        assert_eq!(triple.attribute, "attr:name");
        assert_eq!(triple.value, serde_json::json!("value"));
        assert!(!triple.id.as_uuid().is_nil());
    }
}
