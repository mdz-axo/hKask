//! Blob storage

pub struct BlobStore;

impl BlobStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BlobStore {
    fn default() -> Self {
        Self::new()
    }
}
