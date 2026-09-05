//! HTTP protocol models, URL parser, and execution client for PacketSmith.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::time::Instant;
use async_trait::async_trait;
use chrono::Utc;
use ps_domain::{ProtocolRequest, RequestDocument};
use ps_request_engine::{
    EventSink, ExecutionContext, ExecutionError, ExecutionEvent, ExecutionSummary, ProtocolExecutor,
    ResolutionResult,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

pub mod client;
pub mod response;
pub mod url_sync;

pub use client::*;
pub use response::*;
pub use url_sync::*;

#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Invalid URL: {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Unsupported protocol request passed to HTTP executor")]
    InvalidProtocol,
}

/// Standard and custom HTTP methods.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Custom(String),
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for HttpMethod {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "GET" => Self::Get,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "PATCH" => Self::Patch,
            "DELETE" => Self::Delete,
            "HEAD" => Self::Head,
            "OPTIONS" => Self::Options,
            _ => Self::Custom(s.to_string()),
        })
    }
}

/// Key-value query parameter with enable toggle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryParam {
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub description: Option<String>,
}

/// Key-value HTTP header with enable toggle and secret mask flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
    pub enabled: bool,
    pub is_secret: bool,
    pub description: Option<String>,
}

/// HTTP request body payload variants.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HttpBody {
    #[default]
    None,
    Raw {
        content: String,
        content_type: String,
    },
    Json {
        json_content: String,
    },
    FormUrlEncoded {
        fields: Vec<QueryParam>,
    },
    Multipart {
        fields: Vec<MultipartField>,
    },
    Binary {
        file_path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipartField {
    pub name: String,
    pub value: String,
    pub is_file: bool,
    pub content_type: Option<String>,
}

/// Comprehensive HTTP request representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub raw_url: String,
    pub params: Vec<QueryParam>,
    pub headers: Vec<HeaderEntry>,
    pub body: HttpBody,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            raw_url: url.into(),
            params: Vec::new(),
            headers: Vec::new(),
            body: HttpBody::None,
        }
    }

    /// Resolves the URL appending enabled query parameters.
    pub fn build_resolved_url(&self) -> Result<Url, url::ParseError> {
        let mut parsed = Url::parse(&self.raw_url)?;
        if !self.params.is_empty() {
            let mut query_pairs = parsed.query_pairs().into_owned().collect::<Vec<_>>();
            for param in &self.params {
                if param.enabled {
                    query_pairs.push((param.key.clone(), param.value.clone()));
                }
            }
            parsed.query_pairs_mut().clear().extend_pairs(query_pairs);
        }
        Ok(parsed)
    }
}

/// HTTP Response model capturing status, headers, body, and metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body_bytes: Vec<u8>,
    pub duration_ms: u64,
    pub size_bytes: usize,
    pub http_version: String,
}

/// HTTP Protocol Executor implementing the `ProtocolExecutor` trait.
#[derive(Debug, Default)]
pub struct HttpExecutor {
    client: reqwest::Client,
}

impl HttpExecutor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .build()
                .unwrap_or_default(),
        }
    }
}

fn resolve_request_url(ctx: &ExecutionContext, raw_url: &str) -> ResolutionResult {
    ctx.variable_resolver.resolve_template(raw_url)
}

#[async_trait]
impl ProtocolExecutor for HttpExecutor {
    async fn execute(
        &self,
        request: &RequestDocument,
        ctx: &ExecutionContext,
        event_sink: &EventSink,
    ) -> Result<ExecutionSummary, ExecutionError> {
        let http_payload = match &request.protocol {
            ProtocolRequest::Http(payload) => payload,
            _ => return Err(ExecutionError::Protocol("Non-HTTP protocol passed".into())),
        };

        event_sink
            .emit(ExecutionEvent::Preparing {
                timestamp: Utc::now(),
            })
            .await;

        event_sink
            .emit(ExecutionEvent::ResolvingVariables {
                timestamp: Utc::now(),
            })
            .await;

        let resolved_url = resolve_request_url(ctx, &http_payload.url);

        if ctx.cancellation_token.is_cancelled() {
            event_sink
                .emit(ExecutionEvent::Cancelled {
                    timestamp: Utc::now(),
                })
                .await;
            return Err(ExecutionError::Cancelled);
        }

        let start = Instant::now();
        event_sink
            .emit(ExecutionEvent::Connecting {
                // Event sinks feed logs and UI. Always use the redacted preview.
                url: resolved_url.display_value.clone(),
                timestamp: Utc::now(),
            })
            .await;

        let method = match reqwest::Method::from_bytes(http_payload.method.as_bytes()) {
            Ok(m) => m,
            Err(_) => reqwest::Method::GET,
        };

        let response_res = self.client.request(method, &resolved_url.value).send().await;

        match response_res {
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                let mut header_map = HashMap::new();
                for (k, v) in resp.headers() {
                    if let Ok(val) = v.to_str() {
                        let safe_val = ps_request_engine::redact_sensitive_header(k.as_str(), val);
                        header_map.insert(k.as_str().to_string(), safe_val);
                    }
                }

                event_sink
                    .emit(ExecutionEvent::HeadersReceived {
                        status_code,
                        headers: header_map,
                        timestamp: Utc::now(),
                    })
                    .await;

                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| ExecutionError::Network(e.to_string()))?;

                event_sink
                    .emit(ExecutionEvent::DownloadProgress {
                        bytes_received: bytes.len(),
                    })
                    .await;

                let duration_ms = start.elapsed().as_millis() as u64;

                let summary = ExecutionSummary {
                    run_id: ctx.run_id,
                    request_id: request.id,
                    status_code: Some(status_code),
                    duration_ms,
                    bytes_sent: 0,
                    bytes_received: bytes.len(),
                    completed_at: Utc::now(),
                };

                event_sink
                    .emit(ExecutionEvent::Completed {
                        summary: summary.clone(),
                    })
                    .await;

                Ok(summary)
            }
            Err(err) => {
                let err_msg = err.to_string();
                event_sink
                    .emit(ExecutionEvent::Failed {
                        error: err_msg.clone(),
                        timestamp: Utc::now(),
                    })
                    .await;
                Err(ExecutionError::Network(err_msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_parse() {
        assert_eq!("get".parse::<HttpMethod>().unwrap(), HttpMethod::Get);
        assert_eq!("POST".parse::<HttpMethod>().unwrap(), HttpMethod::Post);
        assert_eq!(
            "PURGE".parse::<HttpMethod>().unwrap(),
            HttpMethod::Custom("PURGE".to_string())
        );
    }

    #[test]
    fn test_url_query_resolution() {
        let mut req = HttpRequest::new(HttpMethod::Get, "https://api.example.com/v1/users");
        req.params.push(QueryParam {
            key: "limit".to_string(),
            value: "20".to_string(),
            enabled: true,
            description: None,
        });
        req.params.push(QueryParam {
            key: "ignored".to_string(),
            value: "1".to_string(),
            enabled: false,
            description: None,
        });

        let url = req.build_resolved_url().expect("Failed to build URL");
        assert_eq!(url.as_str(), "https://api.example.com/v1/users?limit=20");
    }

    #[test]
    fn test_resolved_url_has_a_secret_safe_event_value() {
        let context = ExecutionContext::new(
            HashMap::new(),
            HashMap::from([("token".into(), "do-not-log".into())]),
        );
        let resolved = resolve_request_url(
            &context,
            "https://api.example.com?token={{vault:token}}",
        );

        assert!(resolved.value.contains("do-not-log"));
        assert!(!resolved.display_value.contains("do-not-log"));
        assert!(!format!("{resolved:?}").contains("do-not-log"));
        assert!(!format!("{context:?}").contains("do-not-log"));
    }
}
