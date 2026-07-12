//! ResearchDataBridge — trait for web search, feeds, and extraction in the TUI.

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct FeedInfo {
    pub id: String,
    pub title: String,
    pub unread: usize,
}

#[derive(Debug, Clone)]
pub struct ExtractResult {
    pub url: String,
    pub content: String,
    pub format: String,
}

pub trait ResearchDataBridge: Send + Sync {
    fn search(&self, query: &str) -> Vec<SearchResult>;
    fn feed_list(&self) -> Vec<FeedInfo>;
    fn extract(&self, url: &str) -> Option<ExtractResult>;
    fn last_query(&self) -> Option<String>;
}

pub struct MockResearchBridge {
    pub results: Vec<SearchResult>,
    pub feeds: Vec<FeedInfo>,
    pub last: Option<String>,
}
impl MockResearchBridge {
    pub fn new() -> Self {
        Self {
            results: vec![],
            feeds: vec![],
            last: None,
        }
    }
    pub fn with_sample() -> Self {
        Self {
            results: vec![SearchResult {
                title: "Example Result".into(),
                url: "https://example.com".into(),
                snippet: "A sample search result.".into(),
            }],
            feeds: vec![FeedInfo {
                id: "feed/1".into(),
                title: "arXiv CS.AI".into(),
                unread: 5,
            }],
            last: Some("example query".into()),
        }
    }
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}
impl ResearchDataBridge for MockResearchBridge {
    fn search(&self, _: &str) -> Vec<SearchResult> {
        self.results.clone()
    }
    fn feed_list(&self) -> Vec<FeedInfo> {
        self.feeds.clone()
    }
    fn extract(&self, _: &str) -> Option<ExtractResult> {
        None
    }
    fn last_query(&self) -> Option<String> {
        self.last.clone()
    }
}
