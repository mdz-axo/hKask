use hkask_memory::{EpisodicMemory, MemoryConfig, SemanticMemory};
use hkask_storage::{SqliteStorage, StorageConfig, StorageEngine};
use tempfile::TempDir;

#[test]
fn test_storage_memory_integration_basic() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage_config = StorageConfig::with_path(db_path.to_str().unwrap());
    let storage = SqliteStorage::new(&storage_config).unwrap();

    let memory_config = MemoryConfig::default();
    let semantic = SemanticMemory::new(memory_config.clone()).unwrap();
    let episodic = EpisodicMemory::new(memory_config).unwrap();

    assert!(storage.is_initialized());
    assert!(semantic.is_ok());
    assert!(episodic.is_ok());
}

#[test]
fn test_storage_memory_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage_config = StorageConfig::with_path(db_path.to_str().unwrap());
    let _storage = SqliteStorage::new(&storage_config).unwrap();

    assert!(db_path.exists());
}

#[test]
fn test_memory_config_compatibility() {
    let config = MemoryConfig::default();

    let semantic = SemanticMemory::new(config.clone());
    let episodic = EpisodicMemory::new(config);

    assert!(semantic.is_ok());
    assert!(episodic.is_ok());
}
