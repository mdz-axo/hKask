//! CompaniesDataBridge — trait for company data in the TUI.
//!
//! Provides the Companies window with live company profiles, people,
//! and relationship data from hkask-mcp-companies / Firecrawl.

use std::sync::Arc;

/// Summary of a company for TUI display.
#[derive(Debug, Clone)]
pub struct CompanySummary {
    pub name: String,
    pub domain: String,
    pub industry: Option<String>,
    pub size: Option<String>,
    pub location: Option<String>,
}

/// Summary of a person associated with a company.
#[derive(Debug, Clone)]
pub struct PersonSummary {
    pub name: String,
    pub role: Option<String>,
    pub company: String,
}

/// Trait for querying companies subsystem state.
pub trait CompaniesDataBridge: Send + Sync {
    /// Search companies by name/domain keyword.
    fn search(&self, query: &str) -> Vec<CompanySummary>;
    /// Get the last searched company name.
    fn last_searched(&self) -> Option<String>;
    /// Get people associated with the last searched company.
    fn people(&self) -> Vec<PersonSummary>;
}

/// Mock implementation for TUI development and testing.
pub struct MockCompaniesBridge {
    pub results: Vec<CompanySummary>,
    pub last_query: Option<String>,
    pub people: Vec<PersonSummary>,
}

impl MockCompaniesBridge {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            last_query: None,
            people: Vec::new(),
        }
    }

    pub fn with_sample() -> Self {
        Self {
            results: vec![
                CompanySummary {
                    name: "Acme Corp".into(),
                    domain: "acme.com".into(),
                    industry: Some("Technology".into()),
                    size: Some("500-1000".into()),
                    location: Some("San Francisco, CA".into()),
                },
                CompanySummary {
                    name: "Globex Inc".into(),
                    domain: "globex.com".into(),
                    industry: Some("Manufacturing".into()),
                    size: Some("1000+".into()),
                    location: Some("Springfield, USA".into()),
                },
            ],
            last_query: Some("acme".into()),
            people: vec![
                PersonSummary {
                    name: "Jane Smith".into(),
                    role: Some("CEO".into()),
                    company: "Acme Corp".into(),
                },
                PersonSummary {
                    name: "John Doe".into(),
                    role: Some("CTO".into()),
                    company: "Acme Corp".into(),
                },
            ],
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl CompaniesDataBridge for MockCompaniesBridge {
    fn search(&self, _query: &str) -> Vec<CompanySummary> {
        self.results.clone()
    }
    fn last_searched(&self) -> Option<String> {
        self.last_query.clone()
    }
    fn people(&self) -> Vec<PersonSummary> {
        self.people.clone()
    }
}
