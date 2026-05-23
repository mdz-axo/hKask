//! Adapters for external services

pub mod http_adapter;

pub use http_adapter::{
    HttpAdapter, HttpAdapterError, HttpMethod, HttpRequest, HttpResponse, ReqwestHttpAdapter,
};
