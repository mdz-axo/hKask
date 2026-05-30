//! Priority-tagged lock acquisition for SQLite operations
//!
//! DISPATCH-critical messages (from the Communication loop, Loop 6) must be
//! able to acquire storage locks ahead of routine queries. `LockPriority`
//! tags transactions so the storage layer can schedule accordingly.
//!
//! Spec reference: §6.1 DISPATCH (GUARD+ROUTE) — critical messages take
//! priority over routine I/O.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::MutexGuard;
use std::time::Duration;

use rusqlite::Connection;

use crate::database::Database;

/// Priority level for storage lock acquisition.
///
/// Higher-priority operations acquire locks before lower-priority ones.
/// DISPATCH-critical messages (Loop 6.1) use `Critical`; routine background
/// operations use `Low`.
///
/// # Priority ordering
///
/// ```text
/// Critical > High > Normal > Low
/// ```
///
/// # Usage
///
/// ```ignore
/// let guard = db.acquire(LockPriority::Critical)?;
/// // guard derefs to &Connection — use like a normal connection
/// guard.execute_batch("BEGIN TRANSACTION")?;
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
pub enum LockPriority {
    /// DISPATCH-critical — must not wait behind routine operations.
    /// Used by Loop 6.1 (DISPATCH) for curator directives, escalations,
    /// and other messages that require immediate processing.
    Critical = 4,
    /// High priority — important but not life-critical.
    /// Used by governance operations and sovereignty checks.
    High = 3,
    /// Normal priority — routine queries and writes.
    /// Default for most storage operations.
    #[default]
    Normal = 2,
    /// Low priority — background maintenance, consolidation, indexing.
    /// Used by consolidation bridge (B.1–B.4) and periodic maintenance.
    Low = 1,
}

impl LockPriority {
    /// Numeric priority for comparison. Higher = more urgent.
    pub fn level(&self) -> u8 {
        match self {
            LockPriority::Critical => 4,
            LockPriority::High => 3,
            LockPriority::Normal => 2,
            LockPriority::Low => 1,
        }
    }

    /// Default priority for storage operations.
    pub fn default_priority() -> Self {
        LockPriority::Normal
    }

    /// Check if this priority is DISPATCH-critical.
    pub fn is_critical(&self) -> bool {
        matches!(self, LockPriority::Critical)
    }

    /// Convert from the Communication loop's `MessagePriority`.
    ///
    /// Maps:
    /// - `MessagePriority::Critical` → `LockPriority::Critical`
    /// - `MessagePriority::Warning` → `LockPriority::High`
    /// - `MessagePriority::Info` → `LockPriority::Normal`
    pub fn from_message_priority(mp: &hkask_types::MessagePriority) -> Self {
        match mp {
            hkask_types::MessagePriority::Critical => LockPriority::Critical,
            hkask_types::MessagePriority::Warning => LockPriority::High,
            hkask_types::MessagePriority::Info => LockPriority::Normal,
        }
    }
}

impl fmt::Display for LockPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockPriority::Critical => write!(f, "critical"),
            LockPriority::High => write!(f, "high"),
            LockPriority::Normal => write!(f, "normal"),
            LockPriority::Low => write!(f, "low"),
        }
    }
}

/// RAII guard for a priority-acquired database connection.
///
/// Holds a `MutexGuard<Connection>` and the priority at which it was
/// acquired. The lock is released when the guard is dropped.
///
/// Implements `Deref<Target = Connection>` and `DerefMut` so it can
/// be used like a regular `Connection` reference.
///
/// CNS span: `cns.storage.lock` with `priority` and `wait_time_ms` fields.
pub struct PriorityLockGuard<'a> {
    guard: MutexGuard<'a, Connection>,
    priority: LockPriority,
    wait_time: Duration,
}

impl<'a> PriorityLockGuard<'a> {
    /// The priority at which this lock was acquired.
    pub fn priority(&self) -> LockPriority {
        self.priority
    }

    /// How long the acquisition waited for the lock.
    pub fn wait_time(&self) -> Duration {
        self.wait_time
    }
}

impl<'a> Deref for PriorityLockGuard<'a> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a> DerefMut for PriorityLockGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Database {
    /// Acquire the database connection with a given priority.
    ///
    /// Currently all priorities use the same `Arc<Mutex<Connection>>`, so
    /// acquisition is FIFO. The priority is recorded for CNS observability
    /// and future priority-queue scheduling.
    ///
    /// # CNS span
    ///
    /// Emits `cns.storage.lock` with:
    /// - `priority`: the lock priority level
    /// - `wait_time_ms`: time spent waiting for the lock
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError::SqlCipher` if the lock is poisoned.
    pub fn acquire(
        &self,
        priority: LockPriority,
    ) -> Result<PriorityLockGuard<'_>, crate::database::DatabaseError> {
        let start = std::time::Instant::now();

        // Lock the inner Mutex<Connection> via Deref on Arc
        let guard = self.conn.lock().map_err(|e| {
            crate::database::DatabaseError::SqlCipher(format!(
                "Lock poisoned (priority={}): {}",
                priority, e
            ))
        })?;

        let wait_time = start.elapsed();

        tracing::trace!(
            target: "cns.storage.lock",
            priority = %priority,
            wait_time_ms = wait_time.as_millis() as u64,
            "Storage lock acquired"
        );

        // If Critical priority had to wait, emit a warning
        if priority.is_critical() && wait_time > Duration::from_millis(100) {
            tracing::warn!(
                target: "cns.storage.lock",
                priority = %priority,
                wait_time_ms = wait_time.as_millis() as u64,
                "Critical-priority lock waited >100ms"
            );
        }

        Ok(PriorityLockGuard {
            guard,
            priority,
            wait_time,
        })
    }

    /// Acquire the database connection with default (Normal) priority.
    ///
    /// Convenience method for routine operations that don't need
    /// priority scheduling.
    pub fn acquire_default(&self) -> Result<PriorityLockGuard<'_>, crate::database::DatabaseError> {
        self.acquire(LockPriority::Normal)
    }
}

