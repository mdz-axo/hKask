//! DocprocDataBridge — trait for document processing in the TUI.

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub index: usize,
    pub token_count: usize,
    pub preview: String,
}

#[derive(Debug, Clone)]
pub struct QAPair {
    pub question: String,
    pub answer: String,
    pub level: String,
}

pub trait DocprocDataBridge: Send + Sync {
    fn chunk_list(&self) -> Vec<ChunkInfo>;
    fn qa_list(&self) -> Vec<QAPair>;
    fn index_status(&self) -> (usize, usize);
}

pub struct MockDocprocBridge {
    pub chunks: Vec<ChunkInfo>,
    pub qas: Vec<QAPair>,
    pub indexed: usize,
    pub total: usize,
}
impl MockDocprocBridge {
    pub fn new() -> Self { Self { chunks: vec![], qas: vec![], indexed: 0, total: 0 } }
    pub fn with_sample() -> Self {
        Self {
            chunks: vec![
                ChunkInfo { index: 0, token_count: 256, preview: "The quick brown fox...".into() },
                ChunkInfo { index: 1, token_count: 512, preview: "Lorem ipsum dolor sit amet...".into() },
            ],
            qas: vec![
                QAPair { question: "What is the main topic?".into(), answer: "Document processing with LLMs.".into(), level: "knowledge".into() },
            ],
            indexed: 2,
            total: 2,
        }
    }
    pub fn arc(self) -> Arc<Self> { Arc::new(self) }
}
impl DocprocDataBridge for MockDocprocBridge {
    fn chunk_list(&self) -> Vec<ChunkInfo> { self.chunks.clone() }
    fn qa_list(&self) -> Vec<QAPair> { self.qas.clone() }
    fn index_status(&self) -> (usize, usize) { (self.indexed, self.total) }
}
