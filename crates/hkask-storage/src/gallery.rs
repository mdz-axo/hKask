//! Gallery storage — SQLite-backed image gallery index.
//!
//! The gallery is a lens over the filesystem, not a copy of it.
//! Images are indexed by path + hash; tags are AI-generated metadata.
//! Schema is flat — one join maximum (image → tags).
//!
//! Tables:
//! - `galleries`: root_path, policy_mode
//! - `images`: path, hash, dimensions, gallery_id
//! - `tags`: image_id, tag_type, value, confidence

use crate::{Store, now_rfc3339};
use hkask_types::InfrastructureError;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GalleryStoreError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

    #[error("Gallery not found: {0}")]
    NotFound(String),

    #[error("Image not found: {0}")]
    ImageNotFound(String),

    #[error("Invalid policy mode: {0}")]
    InvalidMode(String),

    #[error("Gallery already exists at path: {0}")]
    AlreadyExists(String),
}

impl_from_rusqlite!(GalleryStoreError, Infra);

/// Gallery policy mode — three states, no gray zone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum GalleryMode {
    /// Files are read-only, never modified.
    ReadOnly,
    /// Files can be edited; copies written as new images.
    CopyOnWrite,
    /// Files may be edited in-place; original data may be lost.
    Destructive,
}

impl FromStr for GalleryMode {
    type Err = GalleryStoreError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "read-only" => Ok(Self::ReadOnly),
            "copy-on-write" => Ok(Self::CopyOnWrite),
            "destructive" => Ok(Self::Destructive),
            other => Err(GalleryStoreError::InvalidMode(other.to_string())),
        }
    }
}

impl GalleryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::CopyOnWrite => "copy-on-write",
            Self::Destructive => "destructive",
        }
    }
}

/// A gallery record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryRecord {
    pub id: String,
    pub root_path: String,
    pub mode: String,
    pub image_count: u32,
    pub total_size_bytes: u64,
    pub created_at: String,
    pub updated_at: String,
}

/// An indexed image entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRecord {
    pub id: String,
    pub gallery_id: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub hash: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub size_bytes: u64,
    pub added_at: String,
}

/// A tag on an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRecord {
    pub id: String,
    pub image_id: String,
    pub tag_type: String,
    pub value: String,
    pub confidence: f64,
    pub model_used: String,
    pub created_at: String,
}

define_store!(GalleryStore);

impl GalleryStore {
    /// Initialize gallery tables in the database.
    pub fn init_tables(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS galleries (
                id TEXT PRIMARY KEY,
                root_path TEXT NOT NULL UNIQUE,
                mode TEXT NOT NULL DEFAULT 'read-only',
                image_count INTEGER NOT NULL DEFAULT 0,
                total_size_bytes INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS gallery_images (
                id TEXT PRIMARY KEY,
                gallery_id TEXT NOT NULL REFERENCES galleries(id) ON DELETE CASCADE,
                relative_path TEXT NOT NULL,
                absolute_path TEXT NOT NULL,
                hash TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                format TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                added_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_gallery_images_gallery
                ON gallery_images(gallery_id);
            CREATE INDEX IF NOT EXISTS idx_gallery_images_hash
                ON gallery_images(hash);

            CREATE TABLE IF NOT EXISTS gallery_tags (
                id TEXT PRIMARY KEY,
                image_id TEXT NOT NULL REFERENCES gallery_images(id) ON DELETE CASCADE,
                tag_type TEXT NOT NULL,
                value TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 1.0,
                model_used TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_gallery_tags_image
                ON gallery_tags(image_id);
            CREATE INDEX IF NOT EXISTS idx_gallery_tags_type
                ON gallery_tags(tag_type);",
        )
    }

    /// Create a new gallery. Returns the gallery record.
    ///
    /// REQ: media-gallery-create-01
    pub fn create(
        &self,
        root_path: &str,
        mode: GalleryMode,
    ) -> std::result::Result<GalleryRecord, GalleryStoreError> {
        let conn = self.lock_conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_rfc3339();

        // Check for existing gallery at this path
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM galleries WHERE root_path = ?1",
                [root_path],
                |row| row.get(0),
            )
            .optional()?;

        if existing.is_some() {
            return Err(GalleryStoreError::AlreadyExists(root_path.to_string()));
        }

        conn.execute(
            "INSERT INTO galleries (id, root_path, mode, image_count, total_size_bytes, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, 0, ?4, ?4)",
            rusqlite::params![id, root_path, mode.as_str(), now],
        )?;

        Ok(GalleryRecord {
            id,
            root_path: root_path.to_string(),
            mode: mode.as_str().to_string(),
            image_count: 0,
            total_size_bytes: 0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Add an image to the gallery index.
    ///
    /// REQ: media-gallery-scan-01
    #[allow(clippy::too_many_arguments)]
    pub fn add_image(
        &self,
        gallery_id: &str,
        relative_path: &str,
        absolute_path: &str,
        hash: &str,
        width: u32,
        height: u32,
        format: &str,
        size_bytes: u64,
    ) -> std::result::Result<ImageRecord, GalleryStoreError> {
        let conn = self.lock_conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_rfc3339();

        conn.execute(
            "INSERT INTO gallery_images (id, gallery_id, relative_path, absolute_path, hash, width, height, format, size_bytes, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id, gallery_id, relative_path, absolute_path, hash,
                width, height, format, size_bytes as i64, now
            ],
        )?;

        // Update gallery counts
        conn.execute(
            "UPDATE galleries SET image_count = image_count + 1, total_size_bytes = total_size_bytes + ?1, updated_at = ?2
             WHERE id = ?3",
            rusqlite::params![size_bytes as i64, now, gallery_id],
        )?;

        Ok(ImageRecord {
            id,
            gallery_id: gallery_id.to_string(),
            relative_path: relative_path.to_string(),
            absolute_path: absolute_path.to_string(),
            hash: hash.to_string(),
            width,
            height,
            format: format.to_string(),
            size_bytes,
            added_at: now,
        })
    }

    /// Get an image by index (0-based position in gallery) or by hash.
    ///
    /// REQ: media-gallery-get-image-01
    pub fn get_image(
        &self,
        gallery_id: &str,
        index: Option<usize>,
        hash: Option<&str>,
    ) -> std::result::Result<ImageRecord, GalleryStoreError> {
        let conn = self.lock_conn()?;

        let row = if let Some(h) = hash {
            conn.query_row(
                "SELECT id, gallery_id, relative_path, absolute_path, hash, width, height, format, size_bytes, added_at
                 FROM gallery_images WHERE gallery_id = ?1 AND hash = ?2",
                rusqlite::params![gallery_id, h],
                Self::image_from_row,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    GalleryStoreError::ImageNotFound(format!("hash={}", h))
                }
                other => GalleryStoreError::from(other),
            })?
        } else if let Some(idx) = index {
            conn.query_row(
                "SELECT id, gallery_id, relative_path, absolute_path, hash, width, height, format, size_bytes, added_at
                 FROM gallery_images WHERE gallery_id = ?1
                 ORDER BY added_at ASC LIMIT 1 OFFSET ?2",
                rusqlite::params![gallery_id, idx as i64],
                Self::image_from_row,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    GalleryStoreError::ImageNotFound(format!("index={}", idx))
                }
                other => GalleryStoreError::from(other),
            })?
        } else {
            return Err(GalleryStoreError::ImageNotFound(
                "Must provide either index or hash".to_string(),
            ));
        };

        Ok(row)
    }

    /// Tag an image with AI-generated metadata.
    ///
    /// REQ: media-gallery-tag-image-01
    pub fn tag_image(
        &self,
        image_id: &str,
        tag_type: &str,
        value: &str,
        confidence: f64,
        model_used: &str,
    ) -> std::result::Result<TagRecord, GalleryStoreError> {
        let conn = self.lock_conn()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_rfc3339();

        conn.execute(
            "INSERT INTO gallery_tags (id, image_id, tag_type, value, confidence, model_used, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, image_id, tag_type, value, confidence, model_used, now],
        )?;

        Ok(TagRecord {
            id,
            image_id: image_id.to_string(),
            tag_type: tag_type.to_string(),
            value: value.to_string(),
            confidence,
            model_used: model_used.to_string(),
            created_at: now,
        })
    }

    /// Get all tags for an image.
    ///
    /// REQ: media-gallery-get-tags-01
    pub fn get_tags(
        &self,
        image_id: &str,
    ) -> std::result::Result<Vec<TagRecord>, GalleryStoreError> {
        let conn = self.lock_conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, image_id, tag_type, value, confidence, model_used, created_at
             FROM gallery_tags WHERE image_id = ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt
            .query_map([image_id], Self::tag_from_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Get gallery record by ID.
    pub fn get_gallery(
        &self,
        gallery_id: &str,
    ) -> std::result::Result<GalleryRecord, GalleryStoreError> {
        let conn = self.lock_conn()?;

        conn.query_row(
            "SELECT id, root_path, mode, image_count, total_size_bytes, created_at, updated_at
             FROM galleries WHERE id = ?1",
            [gallery_id],
            |row| {
                Ok(GalleryRecord {
                    id: row.get(0)?,
                    root_path: row.get(1)?,
                    mode: row.get(2)?,
                    image_count: row.get(3)?,
                    total_size_bytes: row.get::<_, i64>(4)? as u64,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                GalleryStoreError::NotFound(gallery_id.to_string())
            }
            other => GalleryStoreError::from(other),
        })
    }

    // ── Row mappers ──

    fn image_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImageRecord> {
        Ok(ImageRecord {
            id: row.get(0)?,
            gallery_id: row.get(1)?,
            relative_path: row.get(2)?,
            absolute_path: row.get(3)?,
            hash: row.get(4)?,
            width: row.get(5)?,
            height: row.get(6)?,
            format: row.get(7)?,
            size_bytes: row.get::<_, i64>(8)? as u64,
            added_at: row.get(9)?,
        })
    }

    fn tag_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TagRecord> {
        Ok(TagRecord {
            id: row.get(0)?,
            image_id: row.get(1)?,
            tag_type: row.get(2)?,
            value: row.get(3)?,
            confidence: row.get(4)?,
            model_used: row.get(5)?,
            created_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_memory_db;

    fn setup() -> GalleryStore {
        let db = in_memory_db();
        let conn = db.conn.lock().unwrap();
        GalleryStore::init_tables(&conn).unwrap();
        drop(conn);
        GalleryStore::new(db.conn)
    }

    /// REQ: media-gallery-create-01 — create gallery returns valid record
    #[test]
    fn create_gallery_returns_record() {
        let store = setup();
        let record = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();
        assert_eq!(record.root_path, "/tmp/test-gallery");
        assert_eq!(record.mode, "read-only");
        assert_eq!(record.image_count, 0);
    }

    /// REQ: media-gallery-create-02 — duplicate path is rejected
    #[test]
    fn create_duplicate_path_rejected() {
        let store = setup();
        store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();
        let result = store.create("/tmp/test-gallery", GalleryMode::CopyOnWrite);
        assert!(result.is_err());
    }

    /// REQ: media-gallery-scan-01 — add_image stores record
    #[test]
    fn add_image_stores_record() {
        let store = setup();
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();

        let img = store
            .add_image(
                &gallery.id,
                "photo.jpg",
                "/tmp/test-gallery/photo.jpg",
                "abc123def",
                1920,
                1080,
                "jpg",
                500_000,
            )
            .unwrap();

        assert_eq!(img.relative_path, "photo.jpg");
        assert_eq!(img.width, 1920);
        assert_eq!(img.height, 1080);
    }

    /// REQ: media-gallery-get-image-01 — get by index
    #[test]
    fn get_image_by_index() {
        let store = setup();
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();

        store
            .add_image(
                &gallery.id,
                "first.jpg",
                "/tmp/test-gallery/first.jpg",
                "hash1",
                100,
                100,
                "jpg",
                1000,
            )
            .unwrap();
        store
            .add_image(
                &gallery.id,
                "second.jpg",
                "/tmp/test-gallery/second.jpg",
                "hash2",
                200,
                200,
                "jpg",
                2000,
            )
            .unwrap();

        let img = store.get_image(&gallery.id, Some(0), None).unwrap();
        assert_eq!(img.relative_path, "first.jpg");

        let img = store.get_image(&gallery.id, Some(1), None).unwrap();
        assert_eq!(img.relative_path, "second.jpg");
    }

    /// REQ: media-gallery-get-image-02 — get by hash
    #[test]
    fn get_image_by_hash() {
        let store = setup();
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();

        store
            .add_image(
                &gallery.id,
                "photo.jpg",
                "/tmp/test-gallery/photo.jpg",
                "abc123",
                1920,
                1080,
                "jpg",
                500_000,
            )
            .unwrap();

        let img = store.get_image(&gallery.id, None, Some("abc123")).unwrap();
        assert_eq!(img.hash, "abc123");
    }

    /// REQ: media-gallery-tag-image-01 — tag_image stores tag
    #[test]
    fn tag_image_stores_tag() {
        let store = setup();
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();
        let img = store
            .add_image(
                &gallery.id,
                "photo.jpg",
                "/tmp/test-gallery/photo.jpg",
                "hash1",
                100,
                100,
                "jpg",
                1000,
            )
            .unwrap();

        let tag = store
            .tag_image(
                &img.id,
                "face",
                "young adult male",
                0.95,
                "llama-3.2-vision",
            )
            .unwrap();

        assert_eq!(tag.tag_type, "face");
        assert_eq!(tag.value, "young adult male");
        assert_eq!(tag.confidence, 0.95);
    }

    /// REQ: media-gallery-get-tags-01 — get_tags returns all tags
    #[test]
    fn get_tags_returns_all() {
        let store = setup();
        let gallery = store
            .create("/tmp/test-gallery", GalleryMode::ReadOnly)
            .unwrap();
        let img = store
            .add_image(
                &gallery.id,
                "photo.jpg",
                "/tmp/test-gallery/photo.jpg",
                "hash1",
                100,
                100,
                "jpg",
                1000,
            )
            .unwrap();

        store
            .tag_image(&img.id, "face", "person A", 0.9, "llama")
            .unwrap();
        store
            .tag_image(&img.id, "object", "car", 0.85, "llama")
            .unwrap();

        let tags = store.get_tags(&img.id).unwrap();
        assert_eq!(tags.len(), 2);
    }
}
