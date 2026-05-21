// Auto-extracted inline tests for hkask-storage
// Extracted: Thu May 21 00:22:26 PDT 2026

// === From blobs.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_new() {
        let data = b"Hello!".to_vec();
        let owner = WebID::new();
        let blob = Blob::new(data.clone(), "text/plain", owner);
        assert!(blob.verify());
    }
}

// === From database.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_in_memory_database() {
        let db = Database::in_memory().unwrap();
        let conn = db.conn_arc();
        let locked_conn = conn.lock().unwrap();
        let mut stmt = locked_conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"triples".to_string()));
        assert!(tables.contains(&"embeddings".to_string()));
    }

    #[test]
    fn test_encrypted_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_encrypted.db");
        let passphrase = "test_passphrase_123";
        let db = Database::open(db_path.to_str().unwrap(), passphrase).unwrap();

        let conn = db.conn_arc();
        let locked_conn = conn.lock().unwrap();
        let mut stmt = locked_conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"triples".to_string()));
    }
}

// === From embeddings.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_new() {
        let vector = vec![0.1, 0.2, 0.3];
        let embedding = Embedding::new(vector.clone(), "test-model");
        assert_eq!(embedding.dimensions, 3);
    }
}

// === From git_cas.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let cas = GitCas::open(temp_dir.path()).unwrap();

        let content = b"Hello, Git!";
        let hash = cas.store(content).unwrap();
        let retrieved = cas.retrieve(&hash).unwrap();
        assert_eq!(retrieved, content);
    }
}

// === From triples.rs ===
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_triple_new() {
        let owner = WebID::new();
        let triple = Triple::new("entity1", "attribute1", json!("value1"), owner);
        assert_eq!(triple.entity, "entity1");
        assert_eq!(triple.confidence, 1.0);
        assert!(triple.is_semantic());
    }
}
