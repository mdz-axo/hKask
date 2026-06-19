//! Lock acquisition helpers — standard lock-poisoning to InfrastructureError
//!
//! Every lock acquisition across hKask MUST go through these helpers.
//! Never use `.expect()` or raw `.lock().map_err()` on production paths.
//!
//! # Why not `From<PoisonError>`?
//!
//! `InfrastructureError` already implements `From<Poison`Error<T>`>` (hkask-types),
//! so `lock.lock()?` works when the caller's error type has `#[from] InfrastructureError`.
//! The helpers below provide a named, self-documenting call site and keep the
//! `?` ergonomics intact while also supporting explicit `.map_err()` chains
//! for crate-local error types.
use hkask_types::InfrastructureError;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
/// Acquire a `Mutex` lock, mapping poison to `InfrastructureError::LockPoisoned`.
///
/// This is the standard way to acquire `Mutex` locks across hKask — never use
/// `.expect()` or raw `.lock().map_err()` on production paths.
///
/// # Example
///
/// ```ignore
/// use hkask_storage::lock_helpers::lock_mutex;
/// let guard = lock_mutex(&self.conn)?;
/// ```
/// Lock a std::sync::Mutex, mapping poison errors to InfrastructureError.
///
/// expect: "The system enforces OCAP boundaries on storage access"
/// \[P4\] Motivating: Clear Boundaries — lock helper wraps Mutex to provide structured error
/// post: returns Ok(MutexGuard) if lock acquired
/// post: returns Err(LockPoisoned) if mutex is poisoned
pub fn lock_mutex<T>(lock: &Mutex<T>) -> Result<MutexGuard<'_, T>, InfrastructureError> {
    lock.lock().map_err(|_| InfrastructureError::LockPoisoned)
}
/// Acquire a read lock on an `RwLock`, mapping poison to `InfrastructureError::LockPoisoned`.
///
/// # Example
///
/// ```ignore
/// use hkask_storage::lock_helpers::read_rwlock;
/// let guard = read_rwlock(&self.cache)?;
/// ```
/// Acquire a read lock on a std::sync::RwLock.
///
/// expect: "The system enforces OCAP boundaries on storage access"
/// \[P4\] Motivating: Clear Boundaries — lock helper wraps RwLock read guard
/// post: returns Ok(RwLockReadGuard) if lock acquired
/// post: returns Err(LockPoisoned) if lock is poisoned
pub fn read_rwlock<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<'_, T>, InfrastructureError> {
    lock.read().map_err(|_| InfrastructureError::LockPoisoned)
}
/// Acquire a write lock on an `RwLock`, mapping poison to `InfrastructureError::LockPoisoned`.
///
/// # Example
///
/// ```ignore
/// use hkask_storage::lock_helpers::write_rwlock;
/// let mut guard = write_rwlock(&self.cache)?;
/// ```
/// Acquire a write lock on a std::sync::RwLock.
///
/// expect: "The system enforces OCAP boundaries on storage access"
/// \[P4\] Motivating: Clear Boundaries — lock helper wraps RwLock write guard
/// post: returns Ok(RwLockWriteGuard) if lock acquired
/// post: returns Err(LockPoisoned) if lock is poisoned
pub fn write_rwlock<T>(lock: &RwLock<T>) -> Result<RwLockWriteGuard<'_, T>, InfrastructureError> {
    lock.write().map_err(|_| InfrastructureError::LockPoisoned)
}
