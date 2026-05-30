//! Blob storage with BLAKE3 hashing

use hkask_types::{InfrastructureError, Visibility, WebID};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlobError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
}

impl From<rusqlite::Error> for BlobError {
    fn from(e: rusqlite::Error) -> Self {
        BlobError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

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
