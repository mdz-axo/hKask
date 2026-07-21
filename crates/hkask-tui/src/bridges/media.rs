//! MediaDataBridge — trait for media gallery data in the TUI.
//!
//! Provides the Media window with live gallery status and image listing
//! from hkask-mcp-media / GalleryStore.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Gallery status snapshot.
#[derive(Debug, Clone)]
pub struct GalleryStatus {
    pub active: bool,
    pub gallery_id: Option<String>,
    pub image_count: usize,
    pub root_path: Option<String>,
}

/// Summary of a single image in the gallery.
#[derive(Debug, Clone)]
pub struct ImageSummary {
    pub index: usize,
    pub path: String,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub tags: Vec<String>,
}

/// Trait for querying media subsystem state.
pub trait MediaDataBridge: Send + Sync {
    fn gallery_status(&self) -> GalleryStatus;
    fn recent_images(&self, limit: usize) -> Vec<ImageSummary>;
    fn tagged_images(&self, tag: &str, limit: usize) -> Vec<ImageSummary>;
}

/// Mock implementation for TUI development and testing.
pub struct MockMediaBridge {
    pub gallery: GalleryStatus,
    pub images: Vec<ImageSummary>,
    query_count: AtomicUsize,
}

impl MockMediaBridge {
    pub fn new() -> Self {
        Self {
            gallery: GalleryStatus {
                active: false,
                gallery_id: None,
                image_count: 0,
                root_path: None,
            },
            images: Vec::new(),
            query_count: AtomicUsize::new(0),
        }
    }

    pub fn with_gallery(root: &str, count: usize) -> Self {
        let mut images = Vec::new();
        for i in 0..count.min(12) {
            images.push(ImageSummary {
                index: i,
                path: format!("{}/img_{:04}.jpg", root, i + 1),
                format: "JPEG".into(),
                width: 1920,
                height: 1080,
                tags: vec![format!("tag_{}", i % 3), "color:blue".into()],
            });
        }
        Self {
            gallery: GalleryStatus {
                active: true,
                gallery_id: Some("gallery-1".into()),
                image_count: count,
                root_path: Some(root.into()),
            },
            images,
            query_count: AtomicUsize::new(0),
        }
    }

    pub fn query_count(&self) -> usize {
        self.query_count.load(Ordering::Relaxed)
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl MediaDataBridge for MockMediaBridge {
    fn gallery_status(&self) -> GalleryStatus {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.gallery.clone()
    }
    fn recent_images(&self, limit: usize) -> Vec<ImageSummary> {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.images.iter().take(limit).cloned().collect()
    }
    fn tagged_images(&self, _tag: &str, _limit: usize) -> Vec<ImageSummary> {
        Vec::new()
    }
}
