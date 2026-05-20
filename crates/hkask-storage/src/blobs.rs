//! Blob storage with BLAKE3 hashing

use hkask_types::{Visibility, WebID};
use rusqlite::Connection;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlobError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

/// Stored blob
#[derive(Debug, Clone)]
pub struct Blob {
    pub id: String,
    pub content_type: String,
    pub size: usize,
    pub blake3_hash: String,
    pub data: Vec<u8>,
    pub visibility: Visibility,
    pub owner_webid: WebID,
}

impl Blob {
    pub fn new(data: Vec<u8>, content_type: &str, owner_webid: WebID) -> Self {
        let hash = blake3::hash(&data);
        Self {
            id: hash.to_string(),
            content_type: content_type.to_string(),
            size: data.len(),
            blake3_hash: hash.to_string(),
            data,
            visibility: Visibility::Private,
            owner_webid,
        }
    }

    pub fn verify(&self) -> bool {
        blake3::hash(&self.data).to_string() == self.blake3_hash
    }
}

/// Blob store for binary data
pub struct BlobStore {
    #[allow(dead_code)]
    conn: Rc<Connection>,
}

impl BlobStore {
    pub fn new(conn: Rc<Connection>) -> Self {
        Self { conn }
    }
}

