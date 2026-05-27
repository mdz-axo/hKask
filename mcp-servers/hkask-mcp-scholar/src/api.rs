use async_trait::async_trait;
use super::store::ScholarStore;
use super::types::*;
use std::sync::Arc;

#[async_trait]
pub trait ScholarApi: Send + Sync {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError>;
    async fn get_papers_batch(
        &self,
        ids: &[&str],
        fields: Option<&str>,
    ) -> Result<Vec<Option<Paper>>, ScholarError>;
    async fn search_papers(
        &self,
        query: &str,
        limit: Option<u32>,
        offset: Option<u32>,
        fields: Option<&str>,
    ) -> Result<SearchResult, ScholarError>;
    async fn list_citations(
        &self,
        paper_id: &str,
        offset: Option<u32>,
        limit: Option<u32>,
        fields: Option<&str>,
    ) -> Result<Vec<CitationEntry>, ScholarError>;
    async fn list_references(
        &self,
        paper_id: &str,
        offset: Option<u32>,
        limit: Option<u32>,
        fields: Option<&str>,
    ) -> Result<Vec<ReferenceEntry>, ScholarError>;
    async fn get_author(
        &self,
        author_id: &str,
        fields: Option<&str>,
    ) -> Result<Author, ScholarError>;
    async fn recommend(
        &self,
        positive_ids: &[&str],
        negative_ids: &[&str],
    ) -> Result<Vec<Paper>, ScholarError>;
}

pub struct HttpScholarApi {
    pub client: reqwest::Client,
    #[allow(dead_code)]
    pub api_key: Option<String>,
}

impl HttpScholarApi {
    pub fn new(api_key: Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "hkask-mcp-scholar".parse().unwrap(),
        );
        if let Some(ref key) = api_key {
            headers.insert("x-api-key", key.parse().unwrap());
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");
        Self { client, api_key }
    }

    fn fields_or_default<'a>(&'a self, fields: Option<&'a str>) -> &'a str {
        fields.unwrap_or(DEFAULT_FIELDS)
    }
}

pub fn parse_paper(v: &serde_json::Value) -> Option<Paper> {
    let paper_id = v.get("paperId")?.as_str()?.to_string();
    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());
    if title.as_ref().is_none_or(|t| t.is_empty()) && v.get("title").is_none() {
        return None;
    }
    let authors = v.get("authors").and_then(|a| a.as_array()).map(|arr| {
        arr.iter()
            .map(|a| AuthorBrief {
                author_id: a
                    .get("authorId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                name: a
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
            .collect::<Vec<_>>()
    });

    Some(Paper {
        paper_id,
        title,
        abstract_text: v
            .get("abstract")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        year: v.get("year").and_then(|v| v.as_i64()).map(|v| v as i32),
        citation_count: v.get("citationCount").and_then(|v| v.as_i64()),
        reference_count: v.get("referenceCount").and_then(|v| v.as_i64()),
        url: v.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        authors,
        venue: v
            .get("venue")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        publication_date: v
            .get("publicationDate")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        external_ids: v.get("externalIds").cloned(),
    })
}

pub fn classify_s2_error(status: reqwest::StatusCode, body: &str) -> ScholarError {
    match status.as_u16() {
        401 | 403 => ScholarError::Unavailable(format!("S2 auth error: {status}")),
        404 => ScholarError::NotFound("S2 resource not found".to_string()),
        429 => ScholarError::RateLimited("S2 rate limited".to_string()),
        502 | 503 => ScholarError::Unavailable(format!("S2 unavailable: {status}")),
        _ if status.is_server_error() => {
            ScholarError::Unavailable(format!("S2 server error: {status}"))
        }
        _ => ScholarError::HttpError(format!(
            "S2 API error {status}: {}",
            body.chars().take(200).collect::<String>()
        )),
    }
}

#[async_trait]
impl ScholarApi for HttpScholarApi {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError> {
        let f = self.fields_or_default(fields);
        let url = format!("{S2_API_BASE}/paper/{paper_id}?fields={f}");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ScholarError::Unavailable(format!("S2 request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        parse_paper(&v).ok_or_else(|| ScholarError::Internal("Failed to parse paper".to_string()))
    }

    async fn get_papers_batch(
        &self,
        ids: &[&str],
        fields: Option<&str>,
    ) -> Result<Vec<Option<Paper>>, ScholarError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        if ids.len() > BATCH_MAX {
            return Err(ScholarError::BadArgs(format!(
                "Batch size {} exceeds maximum of {BATCH_MAX}",
                ids.len()
            )));
        }
        let f = self.fields_or_default(fields);
        let url = format!("{S2_API_BASE}/paper/batch?fields={f}");
        let payload = serde_json::json!({ "ids": ids });
        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ScholarError::Unavailable(format!("S2 batch request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let arr: Vec<serde_json::Value> = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        Ok(arr.iter().map(parse_paper).collect())
    }

    async fn search_papers(
        &self,
        query: &str,
        limit: Option<u32>,
        offset: Option<u32>,
        fields: Option<&str>,
    ) -> Result<SearchResult, ScholarError> {
        let f = self.fields_or_default(fields);
        let limit = limit.unwrap_or(10).min(100);
        let offset = offset.unwrap_or(0);
        let url = format!(
            "{S2_API_BASE}/paper/search?query={}&limit={limit}&offset={offset}&fields={f}",
            urlencoding::encode(query)
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ScholarError::Unavailable(format!("S2 search failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        let total = v.get("total").and_then(|v| v.as_i64());
        let papers = v
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(parse_paper)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(SearchResult {
            total,
            offset: Some(offset as i32),
            papers,
        })
    }

    async fn list_citations(
        &self,
        paper_id: &str,
        offset: Option<u32>,
        limit: Option<u32>,
        fields: Option<&str>,
    ) -> Result<Vec<CitationEntry>, ScholarError> {
        let f = self.fields_or_default(fields);
        let limit = limit.unwrap_or(100).min(500);
        let offset = offset.unwrap_or(0);
        let url = format!(
            "{S2_API_BASE}/paper/{paper_id}/citations?fields={f}&offset={offset}&limit={limit}"
        );
        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                ScholarError::Unavailable(format!("S2 citations request failed: {e}"))
            })?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        v.get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let citing_paper = parse_paper(item.get("citingPaper")?)?;
                        Some(CitationEntry {
                            paper: citing_paper,
                            contexts: item.get("contexts").and_then(|c| c.as_array()).map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            }),
                            intents: item.get("intents").and_then(|i| i.as_array()).map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            }),
                        })
                    })
                    .collect()
            })
            .ok_or_else(|| ScholarError::Internal("Failed to parse citations".to_string()))
    }

    async fn list_references(
        &self,
        paper_id: &str,
        offset: Option<u32>,
        limit: Option<u32>,
        fields: Option<&str>,
    ) -> Result<Vec<ReferenceEntry>, ScholarError> {
        let f = self.fields_or_default(fields);
        let limit = limit.unwrap_or(100).min(500);
        let offset = offset.unwrap_or(0);
        let url = format!(
            "{S2_API_BASE}/paper/{paper_id}/references?fields={f}&offset={offset}&limit={limit}"
        );
        let resp =
            self.client.get(&url).send().await.map_err(|e| {
                ScholarError::Unavailable(format!("S2 references request failed: {e}"))
            })?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        v.get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let cited_paper = parse_paper(item.get("citedPaper")?)?;
                        Some(ReferenceEntry {
                            paper: cited_paper,
                            contexts: item.get("contexts").and_then(|c| c.as_array()).map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            }),
                            intents: item.get("intents").and_then(|i| i.as_array()).map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            }),
                        })
                    })
                    .collect()
            })
            .ok_or_else(|| ScholarError::Internal("Failed to parse references".to_string()))
    }

    async fn get_author(
        &self,
        author_id: &str,
        fields: Option<&str>,
    ) -> Result<Author, ScholarError> {
        let default_fields = "authorId,name,paperCount,citationCount,hIndex,url";
        let f = fields.unwrap_or(default_fields);
        let url = format!("{S2_API_BASE}/author/{author_id}?fields={f}");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ScholarError::Unavailable(format!("S2 author request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        Ok(Author {
            author_id: v
                .get("authorId")
                .and_then(|v| v.as_str())
                .unwrap_or(author_id)
                .to_string(),
            name: v
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            paper_count: v.get("paperCount").and_then(|v| v.as_i64()),
            citation_count: v.get("citationCount").and_then(|v| v.as_i64()),
            h_index: v.get("hIndex").and_then(|v| v.as_i64()).map(|v| v as i32),
            url: v.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    async fn recommend(
        &self,
        positive_ids: &[&str],
        negative_ids: &[&str],
    ) -> Result<Vec<Paper>, ScholarError> {
        let url = format!("{S2_API_BASE}/recommendations/v1/papers/");
        let mut payload = serde_json::json!({ "positivePaperIds": positive_ids });
        if !negative_ids.is_empty() {
            payload["negativePaperIds"] = serde_json::json!(negative_ids);
        }
        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ScholarError::Unavailable(format!("S2 recommend request failed: {e}")))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(classify_s2_error(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ScholarError::Internal(format!("Parse error: {e}")))?;
        Ok(v.get("recommendedPapers")
            .and_then(|d| d.as_array())
            .map(|arr| arr.iter().filter_map(parse_paper).collect())
            .unwrap_or_default())
    }
}

pub struct PersistingScholarApi {
    pub inner: Box<dyn ScholarApi>,
    pub store: Arc<ScholarStore>,
}

impl PersistingScholarApi {
    fn persist_paper(&self, paper: &Paper) {
        if let Err(e) = self.store.store_paper(paper) {
            tracing::warn!(paper_id = %paper.paper_id, error = %e, "Failed to persist paper to local store");
        }
        if let Some(ref authors) = paper.authors {
            for author in authors {
                if let (Some(aid), Some(name)) = (&author.author_id, &author.name) {
                    let _ = self.store.conn.lock().unwrap().execute(
                        "INSERT OR IGNORE INTO authors (author_id, name) VALUES (?1, ?2)",
                        rusqlite::params![aid, name],
                    );
                }
            }
        }
    }
}

#[async_trait]
impl ScholarApi for PersistingScholarApi {
    async fn get_paper(&self, paper_id: &str, fields: Option<&str>) -> Result<Paper, ScholarError> {
        let result = self.inner.get_paper(paper_id, fields).await?;
        self.persist_paper(&result);
        Ok(result)
    }

    async fn get_papers_batch(
        &self,
        ids: &[&str],
        fields: Option<&str>,
    ) -> Result<Vec<Option<Paper>>, ScholarError> {
        let results = self.inner.get_papers_batch(ids, fields).await?;
        for paper in results.iter().flatten() {
            self.persist_paper(paper);
        }
        Ok(results)
    }

    async fn search_papers(
        &self,
        query: &str,
        limit: Option<u32>,
        offset: Option<u32>,
        fields: Option<&str>,
    ) -> Result<SearchResult, ScholarError> {
        let result = self
            .inner
            .search_papers(query, limit, offset, fields)
            .await?;
        for paper in &result.papers {
            self.persist_paper(paper);
        }
        Ok(result)
    }

    async fn list_citations(
        &self,
        paper_id: &str,
        offset: Option<u32>,
        limit: Option<u32>,
        fields: Option<&str>,
    ) -> Result<Vec<CitationEntry>, ScholarError> {
        let result = self
            .inner
            .list_citations(paper_id, offset, limit, fields)
            .await?;
        for entry in &result {
            self.persist_paper(&entry.paper);
            let _ = self.store.store_citation(&entry.paper.paper_id, paper_id);
        }
        Ok(result)
    }

    async fn list_references(
        &self,
        paper_id: &str,
        offset: Option<u32>,
        limit: Option<u32>,
        fields: Option<&str>,
    ) -> Result<Vec<ReferenceEntry>, ScholarError> {
        let result = self
            .inner
            .list_references(paper_id, offset, limit, fields)
            .await?;
        for entry in &result {
            self.persist_paper(&entry.paper);
            let _ = self.store.store_citation(paper_id, &entry.paper.paper_id);
        }
        Ok(result)
    }

    async fn get_author(
        &self,
        author_id: &str,
        fields: Option<&str>,
    ) -> Result<Author, ScholarError> {
        let result = self.inner.get_author(author_id, fields).await?;
        if let Err(e) = self.store.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO authors (author_id, name, paper_count, citation_count, h_index, url) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![result.author_id, result.name, result.paper_count, result.citation_count, result.h_index, result.url],
        ) {
            tracing::warn!(author_id = %result.author_id, error = %e, "Failed to persist author");
        }
        Ok(result)
    }

    async fn recommend(
        &self,
        positive_ids: &[&str],
        negative_ids: &[&str],
    ) -> Result<Vec<Paper>, ScholarError> {
        let result = self.inner.recommend(positive_ids, negative_ids).await?;
        for paper in &result {
            self.persist_paper(paper);
        }
        Ok(result)
    }
}
