//! Temporary Storage Port Adapter
//!
//! This module provides temporary directory and database helpers
//! implementing storage port traits for testing.

use std::path::PathBuf;
use tempfile::TempDir;

/// Temporary database fixture
pub struct TestDb {
    _temp_dir: TempDir,
    pub path: PathBuf,
}

impl TestDb {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("test.db");
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Default for TestDb {
    fn default() -> Self {
        Self::new().expect("Failed to create test database")
    }
}

/// Temporary directory fixture
pub struct TestDir {
    _temp_dir: TempDir,
    pub path: PathBuf,
}

impl TestDir {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Default for TestDir {
    fn default() -> Self {
        Self::new().expect("Failed to create test directory")
    }
}

/// Temporary git repository fixture
pub struct TestGitRepo {
    _temp_dir: TempDir,
    pub path: PathBuf,
}

impl TestGitRepo {
    pub fn new() -> Result<Self, std::io::Error> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().to_path_buf();
        
        // Initialize git repository
        std::process::Command::new("git")
            .arg("init")
            .current_dir(&path)
            .output()?;
        
        Ok(Self {
            _temp_dir: temp_dir,
            path,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Default for TestGitRepo {
    fn default() -> Self {
        Self::new().expect("Failed to create test git repository")
    }
}
