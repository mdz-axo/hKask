//! DiscoveryService — Academic author corpus discovery pipeline.
//!
//! Searches Semantic Scholar and arXiv for an author's works, extracts
//! content, caches to disk, and generates a corpus.yaml ready for
//! `EmbedService::embed_corpus()`.
//!
//! # REQ: P3 (Generative Space) — full parameter exposure, no hidden settings.
//!
//! ## Pipeline
//! 1. Search Semantic Scholar → paper metadata (titles, abstracts, PDF links)
//! 2. Search arXiv → preprint metadata
//! 3. Extract content from discovered URLs → cache to .cache/{slug}.txt
//! 4. Generate corpus.yaml from discovered works
//! 5. Return config path for replica_build

use crate::embed::{CorpusConfig, Work};
use crate::error::ServiceError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const USER_AGENT: &str = "hkask-discovery/0.27";
const SEMANTIC_SCHOLAR_API: &str = "https://api.semanticscholar.org/graph/v1/paper/search";
const ARXIV_API: &str = "https://export.arxiv.org/api/query";

// ── Request / Result types ──────────────────────────────────────────────────

/// Parameters for corpus discovery.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverRequest {
    /// Full name of the academic author (e.g., "David Dunning")
    pub author_name: String,
    /// Maximum number of works to include
    #[serde(default = "default_max_works")]
    pub max_works: usize,
    /// Directory for caching extracted content
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// Directory to write the generated corpus.yaml
    pub output_dir: Option<String>,
    /// SerpAPI key for YouTube transcript + web search (skips these sources if absent)
    #[serde(default)]
    pub serpapi_key: Option<String>,
    /// Whether to search for YouTube transcripts
    #[serde(default = "default_true")]
    pub include_transcripts: bool,
    /// Whether to search the web for institutional pages and interviews
    #[serde(default = "default_true")]
    pub include_web: bool,
    /// Curated mode: present web + YouTube results for user confirmation before including
    #[serde(default = "default_true")]
    pub curated: bool,
}

fn default_max_works() -> usize {
    20
}
fn default_cache_dir() -> String {
    "./.cache".to_string()
}
fn default_true() -> bool {
    true
}

/// Result of a discovery run.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoverResult {
    /// Author slug (e.g., "david-dunning")
    pub author_slug: String,
    /// Number of academic works discovered (Semantic Scholar + arXiv)
    pub works_found: usize,
    /// Number of works successfully cached
    pub works_cached: usize,
    /// Path to the generated corpus.yaml
    pub config_path: String,
    /// Sources used
    pub sources: Vec<String>,
    /// Web search candidates for curation (only populated when curated=true)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub web_candidates: Vec<DiscoveredWork>,
    /// YouTube transcript candidates for curation (only populated when curated=true)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub youtube_candidates: Vec<DiscoveredWork>,
}

/// A discovered work with metadata.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredWork {
    pub title: String,
    pub slug: String,
    pub url: String,
    pub year: Option<u16>,
    pub source: String,
    pub work_type: String,
}

// ── Service ─────────────────────────────────────────────────────────────────

pub struct DiscoveryService;

impl DiscoveryService {
    /// Run the full discovery pipeline and generate a corpus.yaml.
    pub async fn discover(req: &DiscoverRequest) -> Result<DiscoverResult, ServiceError> {
        let author_slug = slugify(&req.author_name);
        let output_dir = req
            .output_dir
            .clone()
            .unwrap_or_else(|| format!("./{}", author_slug));
        let output_path = PathBuf::from(&output_dir);
        let cache_dir = PathBuf::from(&req.cache_dir);

        // Ensure output and cache directories exist
        std::fs::create_dir_all(&output_path).map_err(|e| {
            ServiceError::Embed(format!(
                "Failed to create output directory '{}': {e}",
                output_path.display()
            ))
        })?;
        std::fs::create_dir_all(&cache_dir).map_err(|e| {
            ServiceError::Embed(format!(
                "Failed to create cache directory '{}': {e}",
                cache_dir.display()
            ))
        })?;

        let mut works: Vec<DiscoveredWork> = Vec::new();
        let mut sources: Vec<String> = Vec::new();

        // ── Phase 1: Semantic Scholar ──────────────────────────────────────
        match search_semantic_scholar(&req.author_name, req.max_works).await {
            Ok(papers) => {
                let count = papers.len();
                works.extend(papers);
                sources.push(format!("semantic_scholar ({count} papers)"));
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.discover",
                    error = %e,
                    "Semantic Scholar search failed — continuing with other sources"
                );
            }
        }

        // ── Phase 2: arXiv ─────────────────────────────────────────────────
        match search_arxiv(&req.author_name, req.max_works).await {
            Ok(preprints) => {
                let count = preprints.len();
                // Deduplicate: skip arXiv papers already found via Semantic Scholar
                let existing_urls: Vec<&str> = works.iter().map(|w| w.url.as_str()).collect();
                let new: Vec<DiscoveredWork> = preprints
                    .into_iter()
                    .filter(|w| !existing_urls.contains(&w.url.as_str()))
                    .collect();
                let added = new.len();
                works.extend(new);
                sources.push(format!("arxiv ({added} new, {count} total)"));
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.discover",
                    error = %e,
                    "arXiv search failed — continuing"
                );
            }
        }

        // ── Phase 3: Web search for institutional pages ───────────────────
        let mut web_candidates: Vec<DiscoveredWork> = Vec::new();
        if req.include_web
            && let Some(ref key) = req.serpapi_key
        {
            match search_web_serpapi(&req.author_name, key, 5).await {
                Ok(pages) => {
                    let existing_urls: Vec<&str> = works.iter().map(|w| w.url.as_str()).collect();
                    let new: Vec<DiscoveredWork> = pages
                        .into_iter()
                        .filter(|w| !existing_urls.contains(&w.url.as_str()))
                        .collect();
                    let added = new.len();
                    if req.curated {
                        web_candidates = new;
                        sources.push(format!(
                            "web_search ({added} candidates — awaiting curation)"
                        ));
                    } else {
                        works.extend(new);
                        sources.push(format!("web_search ({added} pages)"));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        error = %e,
                        "Web search failed — continuing"
                    );
                }
            }
        }

        // ── Phase 4: YouTube transcript discovery ─────────────────────────
        let mut youtube_candidates: Vec<DiscoveredWork> = Vec::new();
        if req.include_transcripts
            && let Some(ref key) = req.serpapi_key
        {
            match search_youtube_transcripts(&req.author_name, key, 5).await {
                Ok(transcripts) => {
                    let count = transcripts.len();
                    if req.curated {
                        youtube_candidates = transcripts;
                        sources.push(format!(
                            "youtube_transcripts ({count} candidates — awaiting curation)"
                        ));
                    } else {
                        works.extend(transcripts);
                        sources.push(format!("youtube_transcripts ({count} videos)"));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        error = %e,
                        "YouTube transcript search failed — continuing"
                    );
                }
            }
        }

        // Check after ALL sources have been tried
        if works.is_empty() && web_candidates.is_empty() && youtube_candidates.is_empty() {
            return Err(ServiceError::Embed(format!(
                "No works found for '{}' across any source",
                req.author_name
            )));
        }

        // ── Phase 5: Extract and cache content ────────────────────────────
        let mut cached = 0usize;
        for work in &works {
            let cache_path = cache_dir.join(format!("{}.txt", work.slug));
            if cache_path.exists() {
                cached += 1;
                continue;
            }

            match download_and_cache(&work.url, &cache_path).await {
                Ok(()) => cached += 1,
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.discover",
                        slug = %work.slug,
                        url = %work.url,
                        error = %e,
                        "Failed to download work — skipping"
                    );
                }
            }
        }

        // ── Phase 6: Generate corpus.yaml ──────────────────────────────────
        let config_path = generate_corpus_yaml(&author_slug, &works, &output_path)?;

        tracing::info!(
            target: "hkask.discover",
            author = %req.author_name,
            slug = %author_slug,
            works_found = works.len(),
            works_cached = cached,
            config = %config_path.display(),
            "Discovery complete"
        );

        Ok(DiscoverResult {
            author_slug,
            works_found: works.len(),
            works_cached: cached,
            config_path: config_path.to_string_lossy().to_string(),
            sources,
            web_candidates,
            youtube_candidates,
        })
    }
}

/// Generate a corpus.yaml from a list of discovered works.
///
/// Public so the CLI can regenerate the config after curation —
/// selected web/YouTube candidates are added to the works list
/// and a fresh corpus.yaml is written.
pub fn generate_corpus_yaml(
    author_slug: &str,
    works: &[DiscoveredWork],
    output_dir: &Path,
) -> Result<PathBuf, ServiceError> {
    let corpus_works: Vec<Work> = works
        .iter()
        .map(|w| Work {
            title: w.title.clone(),
            slug: w.slug.clone(),
            url: w.url.clone(),
        })
        .collect();

    let config = CorpusConfig {
        author: author_slug.to_string(),
        embedding: crate::embed::EmbeddingConfig {
            model: "DI/Qwen/Qwen3-Embedding-0.6B".to_string(),
            dim: 1024,
            batch_size: 64,
        },
        works: corpus_works,
        foundational_rules: vec![],
        chunking: crate::embed::ChunkingConfig {
            min_words: 50,
            max_words: 200,
            sentence_boundary: ".!? ".to_string(),
        },
        centroid_entity_ref: format!("style:{author_slug}:centroid"),
        validation: crate::embed::ValidationConfig {
            centroid_distance_max: 0.25,
            exemplar_count_min: 3,
            exemplar_count_max: 7,
        },
        budget: hkask_memory::salience::BudgetConfig::PerPage {
            per_100_pages: 3750,
        },
        entities: crate::embed::EntityConfig {
            characters: vec![],
            places: vec![],
            events: vec![],
            concepts: vec![],
        },
        methods: vec![],
    };

    let config_yaml = serde_yaml::to_string(&config)
        .map_err(|e| ServiceError::Embed(format!("Failed to serialize corpus config: {e}")))?;

    let config_path = output_dir.join("corpus.yaml");
    std::fs::write(&config_path, &config_yaml).map_err(|e| {
        ServiceError::Embed(format!(
            "Failed to write corpus.yaml to '{}': {e}",
            config_path.display()
        ))
    })?;

    Ok(config_path)
}

// ── API search helpers ──────────────────────────────────────────────────────

/// Search Semantic Scholar for papers by author name.
async fn search_semantic_scholar(
    author: &str,
    limit: usize,
) -> Result<Vec<DiscoveredWork>, ServiceError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ServiceError::Embed(format!("HTTP client build failed: {e}")))?;

    let params: Vec<(&str, String)> = vec![
        ("query", author.to_string()),
        ("limit", limit.to_string()),
        (
            "fields",
            "title,authors,year,externalIds,url,openAccessPdf,publicationTypes".to_string(),
        ),
    ];

    let resp = client
        .get(SEMANTIC_SCHOLAR_API)
        .query(&params)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("Semantic Scholar request failed: {e}")))?;

    let body = resp.text().await.unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    let papers = parsed["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|paper| {
                    let title = paper["title"].as_str()?.to_string();
                    let slug = slugify(&title);
                    let url = paper["openAccessPdf"]["url"]
                        .as_str()
                        .or_else(|| paper["url"].as_str())
                        .unwrap_or("")
                        .to_string();
                    let year = paper["year"].as_u64().map(|y| y as u16);
                    let source = paper["publicationTypes"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|t| t.as_str())
                        .unwrap_or("journal_article")
                        .to_string();

                    if title.is_empty() || url.is_empty() {
                        return None;
                    }

                    Some(DiscoveredWork {
                        title,
                        slug,
                        url,
                        year,
                        source,
                        work_type: "journal_article".to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(papers)
}

/// Search arXiv for preprints by author name.
async fn search_arxiv(author: &str, limit: usize) -> Result<Vec<DiscoveredWork>, ServiceError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ServiceError::Embed(format!("HTTP client build failed: {e}")))?;

    // Use arXiv's author search syntax
    let query = format!("au:{author}");
    let params: Vec<(&str, String)> = vec![
        ("search_query", query),
        ("max_results", limit.to_string()),
        ("sortBy", "relevance".to_string()),
    ];

    let resp = client
        .get(ARXIV_API)
        .query(&params)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("arXiv request failed: {e}")))?;

    let body = resp.text().await.unwrap_or_default();
    let papers = parse_arxiv_atom(&body);

    Ok(papers)
}

/// Parse arXiv Atom XML into DiscoveredWork structs.
fn parse_arxiv_atom(xml: &str) -> Vec<DiscoveredWork> {
    let mut results = Vec::new();

    for entry_str in xml.split("<entry>").skip(1) {
        let entry = match entry_str.split("</entry>").next() {
            Some(e) => e,
            None => continue,
        };

        let title = extract_xml_tag(entry, "title");
        let published = extract_xml_tag(entry, "published");

        // PDF link
        let pdf_url = entry
            .lines()
            .find(|line| line.contains("title=\"pdf\""))
            .and_then(|line| {
                let start = line.find("href=\"")? + 6;
                let end = line[start..].find('"')?;
                Some(line[start..start + end].to_string())
            });

        let arxiv_url = extract_xml_tag(entry, "id");
        let url = if !pdf_url.as_ref().is_none_or(|u| u.is_empty()) {
            pdf_url.unwrap_or(arxiv_url)
        } else {
            arxiv_url
        };

        if title.is_empty() || url.is_empty() {
            continue;
        }

        let year = if !published.is_empty() {
            published
                .split('T')
                .next()
                .and_then(|d| d[..4].parse::<u16>().ok())
        } else {
            None
        };

        let slug = slugify(&title);
        results.push(DiscoveredWork {
            title,
            slug,
            url,
            year,
            source: "arxiv".to_string(),
            work_type: "preprint".to_string(),
        });
    }

    results
}

/// Extract text between XML tags, decoding common entities.
fn extract_xml_tag(xml: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");

    let start = match xml.find(&open) {
        Some(pos) => pos + open.len(),
        None => return String::new(),
    };
    let end = match xml[start..].find(&close) {
        Some(pos) => start + pos,
        None => return String::new(),
    };

    let raw = xml[start..end].trim().replace('\n', " ");
    decode_xml_entities(&raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

// ── Web search (SerpAPI Google) ─────────────────────────────────────────────

const SERPAPI_BASE: &str = "https://serpapi.com/search";

/// Search the web via SerpAPI's Google engine for pages about the author.
async fn search_web_serpapi(
    author: &str,
    api_key: &str,
    limit: usize,
) -> Result<Vec<DiscoveredWork>, ServiceError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ServiceError::Embed(format!("HTTP client build failed: {e}")))?;

    let query = format!("{author} faculty page institutional profile interview");
    let params: Vec<(&str, String)> = vec![
        ("q", query),
        ("api_key", api_key.to_string()),
        ("engine", "google".to_string()),
        ("num", limit.to_string()),
    ];

    let resp = client
        .get(SERPAPI_BASE)
        .query(&params)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("SerpAPI web search failed: {e}")))?;

    let body = resp.text().await.unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    let results = parsed["organic_results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let title = item["title"].as_str()?.to_string();
                    let url = item["link"].as_str()?.to_string();
                    let slug = slugify(&title);
                    if title.is_empty() || url.is_empty() {
                        return None;
                    }
                    Some(DiscoveredWork {
                        title,
                        slug,
                        url,
                        year: None,
                        source: "web".to_string(),
                        work_type: "web_page".to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(results)
}

// ── YouTube transcript search (SerpAPI) ─────────────────────────────────────

/// Search YouTube for videos about the author and fetch their transcripts.
async fn search_youtube_transcripts(
    author: &str,
    api_key: &str,
    limit: usize,
) -> Result<Vec<DiscoveredWork>, ServiceError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ServiceError::Embed(format!("HTTP client build failed: {e}")))?;

    // Step 1: Search YouTube for videos
    let search_query = format!("{author} interview talk lecture");
    let params: Vec<(&str, String)> = vec![
        ("q", search_query),
        ("api_key", api_key.to_string()),
        ("engine", "youtube".to_string()),
        ("num", limit.to_string()),
    ];

    let resp = client
        .get(SERPAPI_BASE)
        .query(&params)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("SerpAPI YouTube search failed: {e}")))?;

    let body = resp.text().await.unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    // Extract video results
    let video_results = parsed["video_results"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|video| {
                    let title = video["title"].as_str()?.to_string();
                    let link = video["link"].as_str()?.to_string();
                    // Extract video ID from YouTube URL
                    let video_id = extract_youtube_id(&link)?;
                    Some((title, video_id))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if video_results.is_empty() {
        return Ok(vec![]);
    }

    // Step 2: Fetch transcript for each video
    let mut transcripts: Vec<DiscoveredWork> = Vec::new();
    for (title, video_id) in video_results {
        match fetch_youtube_transcript(&client, api_key, &video_id, &title).await {
            Ok(Some(work)) => transcripts.push(work),
            Ok(None) => {
                tracing::info!(
                    target: "hkask.discover",
                    video_id = %video_id,
                    title = %title,
                    "No transcript available for video — skipping"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "hkask.discover",
                    video_id = %video_id,
                    error = %e,
                    "Failed to fetch transcript — skipping"
                );
            }
        }
    }

    Ok(transcripts)
}

/// Extract a YouTube video ID from a URL.
fn extract_youtube_id(url: &str) -> Option<String> {
    // youtube.com/watch?v=VIDEO_ID
    if let Some(pos) = url.find("v=") {
        let after = &url[pos + 2..];
        let id: String = after.chars().take(11).collect();
        if id.len() == 11
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Some(id);
        }
    }
    // youtu.be/VIDEO_ID
    if let Some(pos) = url.find("youtu.be/") {
        let after = &url[pos + 9..];
        let id: String = after.chars().take(11).collect();
        if id.len() == 11
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Some(id);
        }
    }
    None
}

/// Fetch a YouTube transcript via SerpAPI's youtube_video_transcript engine.
/// Returns None if no transcript is available for the video.
async fn fetch_youtube_transcript(
    client: &reqwest::Client,
    api_key: &str,
    video_id: &str,
    title: &str,
) -> Result<Option<DiscoveredWork>, ServiceError> {
    let params: Vec<(&str, String)> = vec![
        ("v", video_id.to_string()),
        ("api_key", api_key.to_string()),
        ("engine", "youtube_video_transcript".to_string()),
    ];

    let resp = client
        .get(SERPAPI_BASE)
        .query(&params)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("SerpAPI transcript request failed: {e}")))?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ServiceError::Embed(format!(
            "SerpAPI transcript error {status} for video '{video_id}'"
        )));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    // Transcript segments: each has "snippet"
    let transcript_text = parsed["transcript"]
        .as_array()
        .map(|segments| {
            segments
                .iter()
                .filter_map(|seg| seg["snippet"].as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    if transcript_text.is_empty() {
        return Ok(None);
    }

    let video_url = format!("https://www.youtube.com/watch?v={video_id}");
    let video_title = parsed["title"].as_str().unwrap_or(title).to_string();
    let slug = slugify(&video_title);

    Ok(Some(DiscoveredWork {
        title: video_title,
        slug,
        url: video_url,
        year: None,
        source: "youtube_transcript".to_string(),
        work_type: "transcript".to_string(),
    }))
}

// ── Download + cache ────────────────────────────────────────────────────────

/// Download content from a URL and cache it to disk.
pub async fn download_and_cache(url: &str, cache_path: &Path) -> Result<(), ServiceError> {
    let resp = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| ServiceError::Embed(format!("HTTP client build failed: {e}")))?
        .get(url)
        .send()
        .await
        .map_err(|e| ServiceError::Embed(format!("HTTP request failed for '{url}': {e}")))?;

    if !resp.status().is_success() {
        return Err(ServiceError::Embed(format!(
            "HTTP {} for '{url}'",
            resp.status()
        )));
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ServiceError::Embed(format!("Failed to read response body: {e}")))?;

    // PDF → extract text
    let is_pdf = content_type.contains("application/pdf")
        || url.ends_with(".pdf")
        || bytes.starts_with(b"%PDF");

    let text = if is_pdf {
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join(format!("hkask-discover-{}.pdf", uuid::Uuid::new_v4()));
        std::fs::write(&tmp_path, &bytes)
            .map_err(|e| ServiceError::Embed(format!("Failed to write temp PDF: {e}")))?;
        let extracted = pdf_extract::extract_text(&tmp_path).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_path);
        extracted
    } else {
        let raw = String::from_utf8_lossy(&bytes).to_string();
        // Strip HTML if present
        if content_type.contains("text/html")
            || raw.starts_with("<!DOCTYPE")
            || raw.starts_with("<html")
        {
            crate::embed::strip_html_tags(&raw)
        } else {
            raw
        }
    };

    if text.split_whitespace().count() < 10 {
        return Err(ServiceError::Embed(format!(
            "Downloaded content from '{url}' is too short (likely paywalled or scanned PDF without OCR)"
        )));
    }

    std::fs::write(cache_path, &text)
        .map_err(|e| ServiceError::Embed(format!("Failed to write cache: {e}")))?;

    tracing::info!(
        target: "hkask.discover",
        path = %cache_path.display(),
        bytes = bytes.len(),
        words = text.split_whitespace().count(),
        "Cached work"
    );

    Ok(())
}

// ── Utilities ───────────────────────────────────────────────────────────────

/// Convert a name or title into a filesystem-safe slug.
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
