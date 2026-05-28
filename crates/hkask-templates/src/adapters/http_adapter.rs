//! HTTP Adapter Trait - Decouples HTTP client from domain logic
//!
//! Enables mocking for tests and swapping HTTP implementations.

use serde_json::Value;
use thiserror::Error;

/// HTTP adapter error
#[derive(Error, Debug)]
pub enum HttpAdapterError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Response parse error: {0}")]
    ParseError(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Value>,
    pub timeout_secs: u64,
}

impl HttpRequest {
    pub fn get(url: String) -> Self {
        Self {
            method: HttpMethod::Get,
            url,
            headers: Vec::new(),
            body: None,
            timeout_secs: 30,
        }
    }

    pub fn post(url: String, body: Value) -> Self {
        Self {
            method: HttpMethod::Post,
            url,
            headers: Vec::new(),
            body: Some(body),
            timeout_secs: 30,
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// HTTP methods
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

/// Reqwest HTTP adapter implementation
pub struct ReqwestHttpAdapter {
    client: reqwest::Client,
}

impl ReqwestHttpAdapter {
    pub fn new(timeout_secs: u64) -> Result<Self, HttpAdapterError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| HttpAdapterError::RequestFailed(e.to_string()))?;

        Ok(Self { client })
    }

    pub fn with_default_timeout() -> Result<Self, HttpAdapterError> {
        Self::new(30)
    }

    pub async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, HttpAdapterError> {
        let mut req = match request.method {
            HttpMethod::Get => self.client.get(&request.url),
            HttpMethod::Post => self.client.post(&request.url),
            HttpMethod::Put => self.client.put(&request.url),
            HttpMethod::Delete => self.client.delete(&request.url),
        };

        for (key, value) in request.headers {
            req = req.header(&key, value);
        }

        if let Some(body) = request.body {
            req = req.json(&body);
        }

        let response = req
            .send()
            .await
            .map_err(|e| HttpAdapterError::RequestFailed(e.to_string()))?;

        let status = response.status().as_u16();
        let headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|vs| (k.as_str().to_string(), vs.to_string()))
            })
            .collect();

        let body: Value = response
            .json()
            .await
            .map_err(|e| HttpAdapterError::ParseError(e.to_string()))?;

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    pub async fn get(&self, url: &str) -> Result<HttpResponse, HttpAdapterError> {
        self.execute(HttpRequest::get(url.to_string())).await
    }

    pub async fn post(&self, url: &str, body: Value) -> Result<HttpResponse, HttpAdapterError> {
        self.execute(HttpRequest::post(url.to_string(), body)).await
    }
}
