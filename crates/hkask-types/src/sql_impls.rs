//! FromSql/ToSql implementations for hKask domain types
//!
//! These impls live behind the `sql` feature flag so that `hkask-types`
//! doesn't force a rusqlite dependency on downstream crates that don't
//! need database support. `hkask-storage` enables this feature.
//!
//! With these impls, stores can write:
//!     let id: TripleID = row.get(0)?;
//! instead of:
//!     let id_str: String = row.get(0)?;
//!     let id = TripleID::from_str(&id_str)
//!         .map_err(|e| InfrastructureError::Database(...))?;
//!
//! (Fowler C3: Replace Primitive with Object — eliminates manual
//! parse/serialize boilerplate at every DB column boundary.)
//!
//! Note: DateTime<Utc> and Option<T> impls live in hkask-storage
//! (newtype wrappers) because Rust's orphan rules forbid implementing
//! foreign traits for foreign types.

use crate::agent_def::AgentKind;
use crate::goal::GoalState;
use crate::id::{BotID, EventID, GoalID, PodID, TemplateID, TripleID, UserID, WebID};
use crate::visibility::Visibility;
use rusqlite::ToSql;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use std::str::FromStr;

// ── ID types (generic Id<T>) ──────────────────────────────────────────

macro_rules! impl_id_sql {
    ($id_type:ty) => {
        impl FromSql for $id_type {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                let s = String::column_result(value)?;
                <$id_type>::from_str(&s).map_err(|e| {
                    FromSqlError::Other(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid ID format: {e}"),
                    )))
                })
            }
        }

        impl ToSql for $id_type {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::Owned(self.as_uuid().to_string().into()))
            }
        }
    };
}

impl_id_sql!(TripleID);
impl_id_sql!(GoalID);
impl_id_sql!(EventID);
impl_id_sql!(BotID);
impl_id_sql!(TemplateID);
impl_id_sql!(PodID);
impl_id_sql!(UserID);

// WebID is a separate struct (not Id<T>), needs its own impl

impl FromSql for WebID {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = String::column_result(value)?;
        WebID::from_str(&s).map_err(|e| {
            FromSqlError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid WebID format: {e}"),
            )))
        })
    }
}

impl ToSql for WebID {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(self.0.to_string().into()))
    }
}

// ── Enum types ──────────────────────────────────────────────────────────

impl FromSql for Visibility {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = String::column_result(value)?;
        Visibility::parse_str(&s).ok_or_else(|| {
            FromSqlError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid visibility: {s}"),
            )))
        })
    }
}

impl ToSql for Visibility {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

impl FromSql for GoalState {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = String::column_result(value)?;
        GoalState::parse_str(&s).ok_or_else(|| {
            FromSqlError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid goal state: {s}"),
            )))
        })
    }
}

impl ToSql for GoalState {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

impl FromSql for AgentKind {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = String::column_result(value)?;
        AgentKind::parse(&s).ok_or_else(|| {
            FromSqlError::Other(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid agent kind: {s}"),
            )))
        })
    }
}

impl ToSql for AgentKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webid_round_trip() {
        let id = WebID::new();
        let s = id.to_string();
        let parsed: WebID = WebID::from_str(&s).unwrap();
        assert_eq!(id, parsed);
        // Also verify FromSql/ToSql impls exist and compile
        let _ = id.to_sql();
    }

    #[test]
    fn triple_id_round_trip() {
        let id = TripleID::new();
        let s = id.to_string();
        let parsed: TripleID = TripleID::from_str(&s).unwrap();
        assert_eq!(id, parsed);
        let _ = id.to_sql();
    }

    #[test]
    fn visibility_round_trip() {
        for vis in [Visibility::Private, Visibility::Public, Visibility::Shared] {
            let s = vis.as_str();
            let parsed = Visibility::parse_str(s).unwrap();
            assert_eq!(vis, parsed);
            let _ = vis.to_sql();
        }
    }

    #[test]
    fn goal_state_round_trip() {
        for state in [
            GoalState::Pending,
            GoalState::Active,
            GoalState::Completed,
            GoalState::Blocked,
            GoalState::Abandoned,
        ] {
            let s = state.as_str();
            let parsed = GoalState::parse_str(s).unwrap();
            assert_eq!(state, parsed);
            let _ = state.to_sql();
        }
    }

    #[test]
    fn agent_kind_round_trip() {
        for kind in [AgentKind::Bot, AgentKind::Replicant] {
            let s = kind.as_str();
            let parsed = AgentKind::parse(s).unwrap();
            assert_eq!(kind, parsed);
            let _ = kind.to_sql();
        }
    }

    #[test]
    fn invalid_webid_returns_error() {
        assert!(WebID::from_str("not-a-uuid").is_err());
    }
}
