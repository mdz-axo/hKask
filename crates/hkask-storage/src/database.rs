//! Database connection

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub struct Database;

impl Database {
    pub fn new(_path: &str) -> Result<Self, DatabaseError> {
        Ok(Self)
    }

    pub fn in_memory() -> Result<Self, DatabaseError> {
        Ok(Self)
    }
}
