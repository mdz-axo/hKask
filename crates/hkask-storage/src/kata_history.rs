//! Kata practice history — SQLite-backed persistence for habit tracking,
//! automaticity scoring, and streak computation.
//!
//! Replaces the flat JSON file (`data/kata-history.json`) with a queryable,
//! concurrent-safe SQLite table. Each practice session logs agent name, date,
//! kata type, practice name, steps completed, and gas consumed.
//!
//! Integrated with the daemon's dual-encoding memory pipeline: the daemon reads
//! `kata_history` rows to build episodic narratives and feeds CNS counters
//! for variety and automaticity monitoring.

use hkask_types::InfrastructureError;

use crate::Store;
use crate::define_store;
use crate::impl_from_rusqlite;

define_store!(KataHistoryStore);

/// A single kata practice session entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KataHistoryEntry {
    /// Auto-incrementing primary key.
    pub id: i64,
    /// Bot name (agent_name).
    pub agent_name: String,
    /// ISO date (YYYY-MM-DD).
    pub date: String,
    /// Kata type: "starter", "improvement", "coaching".
    pub kata_type: String,
    /// Manifest ID or practice name.
    pub practice_name: String,
    /// Number of steps/practices/questions completed.
    pub steps_completed: usize,
    /// Gas consumed during this session.
    pub gas_consumed: u64,
    /// ISO timestamp of record creation.
    pub created_at: String,
}

/// Error type for kata history operations.
#[derive(Debug, thiserror::Error)]
pub enum KataHistoryError {
    #[error("Infrastructure error: {0}")]
    Infra(#[from] InfrastructureError),
    #[error("Parse error: {0}")]
    Parse(String),
}

impl_from_rusqlite!(KataHistoryError, Infra);

impl KataHistoryStore {
    /// Record a kata practice session for an agent.
    ///
    /// # Arguments
    /// * `agent_name` — Bot name
    /// * `date` — ISO date (YYYY-MM-DD)
    /// * `kata_type` — Kata type string
    /// * `practice_name` — Manifest ID or practice name
    /// * `steps_completed` — Number of steps completed
    /// * `gas_consumed` — Gas consumed
    pub fn record(
        &self,
        agent_name: &str,
        date: &str,
        kata_type: &str,
        practice_name: &str,
        steps_completed: usize,
        gas_consumed: u64,
    ) -> Result<i64, KataHistoryError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO kata_history (agent_name, date, kata_type, practice_name, steps_completed, gas_consumed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![agent_name, date, kata_type, practice_name, steps_completed as i64, gas_consumed as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Retrieve all entries for an agent, ordered by date descending.
    pub fn entries_for_agent(
        &self,
        agent_name: &str,
    ) -> Result<Vec<KataHistoryEntry>, KataHistoryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_name, date, kata_type, practice_name, steps_completed, gas_consumed, created_at FROM kata_history WHERE agent_name = ?1 ORDER BY date DESC",
        )?;
        let mapped = stmt.query_map(
            rusqlite::params![agent_name],
            |row| -> rusqlite::Result<KataHistoryRow> {
                Ok(KataHistoryRow {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    date: row.get(2)?,
                    kata_type: row.get(3)?,
                    practice_name: row.get(4)?,
                    steps_completed: row.get::<_, i64>(5)? as usize,
                    gas_consumed: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            },
        )?;
        let mut results = Vec::new();
        for row_result in mapped {
            let row = row_result?;
            results.push(KataHistoryEntry {
                id: row.id,
                agent_name: row.agent_name,
                date: row.date,
                kata_type: row.kata_type,
                practice_name: row.practice_name,
                steps_completed: row.steps_completed,
                gas_consumed: row.gas_consumed,
                created_at: row.created_at,
            });
        }
        Ok(results)
    }

    /// Count total entries for an agent. Useful for CNS variety counter aggregation.
    pub fn count_entries_for_agent(&self, agent_name: &str) -> Result<usize, KataHistoryError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM kata_history WHERE agent_name = ?1",
            rusqlite::params![agent_name],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Count entries for an agent on a specific date. Returns count > 0 if practiced today.
    pub fn count_entries_on(
        &self,
        agent_name: &str,
        date: &str,
    ) -> Result<usize, KataHistoryError> {
        let conn = self.lock_conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM kata_history WHERE agent_name = ?1 AND date = ?2",
            rusqlite::params![agent_name, date],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get the most recent entry for an agent.
    pub fn last_entry_for_agent(
        &self,
        agent_name: &str,
    ) -> Result<Option<KataHistoryEntry>, KataHistoryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_name, date, kata_type, practice_name, steps_completed, gas_consumed, created_at FROM kata_history WHERE agent_name = ?1 ORDER BY date DESC, id DESC LIMIT 1",
        )?;
        let mapped = stmt.query_map(
            rusqlite::params![agent_name],
            |row| -> rusqlite::Result<KataHistoryRow> {
                Ok(KataHistoryRow {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    date: row.get(2)?,
                    kata_type: row.get(3)?,
                    practice_name: row.get(4)?,
                    steps_completed: row.get::<_, i64>(5)? as usize,
                    gas_consumed: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            },
        )?;
        let mut results: Vec<rusqlite::Result<KataHistoryRow>> = mapped.collect();
        match results.pop() {
            Some(Ok(row)) => Ok(Some(KataHistoryEntry {
                id: row.id,
                agent_name: row.agent_name,
                date: row.date,
                kata_type: row.kata_type,
                practice_name: row.practice_name,
                steps_completed: row.steps_completed,
                gas_consumed: row.gas_consumed,
                created_at: row.created_at,
            })),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Get all entries for an agent within a date range (inclusive).
    pub fn entries_in_range(
        &self,
        agent_name: &str,
        from_date: &str,
        to_date: &str,
    ) -> Result<Vec<KataHistoryEntry>, KataHistoryError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_name, date, kata_type, practice_name, steps_completed, gas_consumed, created_at FROM kata_history WHERE agent_name = ?1 AND date >= ?2 AND date <= ?3 ORDER BY date DESC",
        )?;
        let mapped = stmt.query_map(
            rusqlite::params![agent_name, from_date, to_date],
            |row| -> rusqlite::Result<KataHistoryRow> {
                Ok(KataHistoryRow {
                    id: row.get(0)?,
                    agent_name: row.get(1)?,
                    date: row.get(2)?,
                    kata_type: row.get(3)?,
                    practice_name: row.get(4)?,
                    steps_completed: row.get::<_, i64>(5)? as usize,
                    gas_consumed: row.get::<_, i64>(6)? as u64,
                    created_at: row.get(7)?,
                })
            },
        )?;
        let mut results = Vec::new();
        for row_result in mapped {
            let row = row_result?;
            results.push(KataHistoryEntry {
                id: row.id,
                agent_name: row.agent_name,
                date: row.date,
                kata_type: row.kata_type,
                practice_name: row.practice_name,
                steps_completed: row.steps_completed,
                gas_consumed: row.gas_consumed,
                created_at: row.created_at,
            });
        }
        Ok(results)
    }

    /// Delete entries older than a given date. Useful for CNS routine cleanup.
    pub fn delete_entries_before(&self, before_date: &str) -> Result<usize, KataHistoryError> {
        let conn = self.lock_conn()?;
        let count = conn.execute(
            "DELETE FROM kata_history WHERE date < ?1",
            rusqlite::params![before_date],
        )?;
        Ok(count)
    }
}

/// Internal row struct for mapping from database.
struct KataHistoryRow {
    id: i64,
    agent_name: String,
    date: String,
    kata_type: String,
    practice_name: String,
    steps_completed: usize,
    gas_consumed: u64,
    created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_memory_db;

    // REQ: storage-kata-history-001 — KataHistoryStore records and retrieves practice entries
    #[test]
    fn record_and_retrieve_entry() {
        let db = in_memory_db();
        let store = KataHistoryStore::new(db.conn_arc());

        store
            .record("Alice", "2026-06-15", "starter", "starter-kata", 5, 0)
            .unwrap();

        let entries = store.entries_for_agent("Alice").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_name, "Alice");
        assert_eq!(entries[0].kata_type, "starter");
        assert_eq!(entries[0].steps_completed, 5);
    }

    // REQ: storage-kata-history-002 — KataHistoryStore counts entries per agent and per date
    #[test]
    fn count_entries_per_date() {
        let db = in_memory_db();
        let store = KataHistoryStore::new(db.conn_arc());

        store
            .record("Alice", "2026-06-14", "coaching", "coaching-kata", 5, 200)
            .unwrap();
        store
            .record("Alice", "2026-06-15", "starter", "starter-kata", 3, 0)
            .unwrap();
        store
            .record(
                "Bob",
                "2026-06-15",
                "improvement",
                "improvement-kata",
                4,
                15000,
            )
            .unwrap();

        assert_eq!(store.count_entries_for_agent("Alice").unwrap(), 2);
        assert_eq!(store.count_entries_on("Alice", "2026-06-15").unwrap(), 1);
        assert_eq!(store.count_entries_for_agent("Bob").unwrap(), 1);
    }

    // REQ: storage-kata-history-003 — KataHistoryStore returns most recent entry for agent
    #[test]
    fn last_entry_for_agent() {
        let db = in_memory_db();
        let store = KataHistoryStore::new(db.conn_arc());

        store
            .record("Alice", "2026-06-14", "starter", "starter-kata", 5, 0)
            .unwrap();
        store
            .record(
                "Alice",
                "2026-06-15",
                "improvement",
                "improvement-kata",
                4,
                15000,
            )
            .unwrap();

        let last = store.last_entry_for_agent("Alice").unwrap().unwrap();
        assert_eq!(last.date, "2026-06-15");
        assert_eq!(last.kata_type, "improvement");
        assert_eq!(last.gas_consumed, 15000);
    }

    // REQ: storage-kata-history-004 — KataHistoryStore returns None for agent with no entries
    #[test]
    fn no_entries_returns_none() {
        let db = in_memory_db();
        let store = KataHistoryStore::new(db.conn_arc());
        let last = store.last_entry_for_agent("Nobody").unwrap();
        assert!(last.is_none());
    }

    // REQ: storage-kata-history-005 — KataHistoryStore deletes entries before a cutoff date
    #[test]
    fn delete_entries_before() {
        let db = in_memory_db();
        let store = KataHistoryStore::new(db.conn_arc());

        store
            .record("Alice", "2026-06-13", "starter", "starter-kata", 5, 0)
            .unwrap();
        store
            .record("Alice", "2026-06-14", "starter", "starter-kata", 5, 0)
            .unwrap();
        store
            .record(
                "Alice",
                "2026-06-15",
                "improvement",
                "improvement-kata",
                4,
                15000,
            )
            .unwrap();

        let deleted = store.delete_entries_before("2026-06-14").unwrap();
        assert_eq!(deleted, 1);
        let remaining = store.entries_for_agent("Alice").unwrap();
        assert_eq!(remaining.len(), 2);
    }
}
