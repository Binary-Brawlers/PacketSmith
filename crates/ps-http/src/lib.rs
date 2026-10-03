//! HTTP protocol models, URL parser, and execution client for PacketSmith.

use async_trait::async_trait;
use chrono::Utc;
use ps_domain::{ProtocolRequest, RequestDocument};
use ps_request_engine::{
    EventSink, ExecutionContext, ExecutionError, ExecutionEvent, ExecutionSummary,
    ProtocolExecutor, ResolutionResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::time::Instant;
use thiserror::Error;
use url::Url;

pub mod auth;
pub mod client;
pub mod desktop;
pub mod response;
pub mod url_sync;

pub use auth::*;
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

pub use ps_domain::{HeaderEntry, HttpBody, MultipartField, QueryParam};

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

    /// A persisted parameter table replaces the URL query, retaining disabled rows
    /// without sending them or duplicating the enabled pairs mirrored in the URL.
    pub fn from_payload(payload: &ps_domain::HttpRequestPayload) -> Self {
        Self {
            method: payload.method.parse().unwrap(),
            raw_url: if payload.params.is_empty() { payload.url.clone() } else { UrlSyncEngine::parse_url(&payload.url).0 },
            params: payload.params.clone(),
            headers: payload.headers.clone(),
            body: payload.body.clone(),
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
            client: reqwest::Client::builder().build().unwrap_or_default(),
        }
    }
}

fn resolve_request_url(ctx: &ExecutionContext, raw_url: &str) -> ResolutionResult {
    ctx.variable_resolver.resolve_template(raw_url)
}

/// Secret-safe mapping: messages never include credential values.
fn auth_to_protocol_error(error: auth::AuthError) -> ExecutionError {
    match error {
        auth::AuthError::MissingCredential | auth::AuthError::Unresolved => {
            ExecutionError::Protocol("Resolve missing auth credentials before sending.".into())
        }
        auth::AuthError::Unsupported => {
            ExecutionError::Protocol("This authentication method is not supported yet.".into())
        }
        auth::AuthError::InvalidConfig => {
            ExecutionError::Protocol("The authentication configuration is invalid.".into())
        }
    }
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

        // Auth is resolved through the shared variable/vault resolver so
        // `token_secret_ref` vault names and `{{templates}}` keep working.
        // Fail closed: never send without credentials that failed to resolve.
        let applied_auth = auth::apply_auth(&request.auth, &ctx.variable_resolver)
            .map_err(auth_to_protocol_error)?;

        if ctx.cancellation_token.is_cancelled() {
            event_sink
                .emit(ExecutionEvent::Cancelled {
                    timestamp: Utc::now(),
                })
                .await;
            return Err(ExecutionError::Cancelled);
        }

        // Attach auth query pairs before sending; the event/log URL stays redacted.
        let mut display_url = resolved_url.display_value.clone();
        if !applied_auth.query_params().is_empty() {
            if let Ok(mut parsed) = Url::parse(&display_url) {
                let mut pairs: Vec<(String, String)> = parsed
                    .query_pairs()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect();
                for (key, _) in applied_auth.query_params() {
                    if !pairs.iter().any(|(k, _)| k == key) {
                        pairs.push((key.clone(), ps_variable::SECRET_MASK.to_owned()));
                    }
                }
                parsed.query_pairs_mut().clear().extend_pairs(&pairs);
                display_url = parsed.to_string();
            }
        }

        let http_request = HttpRequest::from_payload(http_payload);
        let request_builder = desktop::build_request(&self.client, &http_request, &ctx.variable_resolver, &applied_auth)?;

        let start = Instant::now();
        event_sink
            .emit(ExecutionEvent::Connecting {
                // Event sinks feed logs and UI. Always use the redacted preview.
                url: display_url,
                timestamp: Utc::now(),
            })
            .await;

        let response_res = request_builder.send().await;

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
        let resolved =
            resolve_request_url(&context, "https://api.example.com?token={{vault:token}}");

        assert!(resolved.value.contains("do-not-log"));
        assert!(!resolved.display_value.contains("do-not-log"));
        assert!(!format!("{resolved:?}").contains("do-not-log"));
        assert!(!format!("{context:?}").contains("do-not-log"));
    }

    #[tokio::test]
    async fn test_executor_sends_bearer_auth_with_redacted_events() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::sync::mpsc;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let count = socket.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
            String::from_utf8(bytes).unwrap()
        });

        let mut doc = RequestDocument::new(
            "Secure",
            ProtocolRequest::Http(ps_domain::HttpRequestPayload::new(
                "GET".to_string(),
                format!("http://{address}/secure"),
            )),
        );
        doc.auth = ps_domain::AuthConfig::Bearer {
            token_secret_ref: "TOKEN".into(),
        };
        let ctx = ExecutionContext::new(
            HashMap::new(),
            HashMap::from([("TOKEN".into(), "executor-secret".into())]),
        );
        let (tx, mut rx) = mpsc::channel(32);
        let sink = EventSink::new(tx);
        let summary = HttpExecutor::new()
            .execute(&doc, &ctx, &sink)
            .await
            .unwrap();
        assert_eq!(summary.status_code, Some(200));

        let wire = server.await.unwrap().to_ascii_lowercase();
        assert!(wire.contains("authorization: bearer executor-secret"));

        // Connecting events must use the redacted URL; nothing secret leaks.
        let mut saw_connecting = false;
        while let Ok(event) = rx.try_recv() {
            if let ExecutionEvent::Connecting { url, .. } = &event {
                saw_connecting = true;
                assert!(!url.contains("executor-secret"));
            }
            assert!(!format!("{event:?}").contains("executor-secret"));
        }
        assert!(saw_connecting);
    }

    #[tokio::test]
    async fn test_executor_fails_closed_on_unresolved_auth() {
        use tokio::sync::mpsc;

        let mut doc = RequestDocument::new(
            "Secure",
            ProtocolRequest::Http(ps_domain::HttpRequestPayload::new(
                "GET".to_string(),
                "http://127.0.0.1:1/unused".to_string(),
            )),
        );
        doc.auth = ps_domain::AuthConfig::Bearer {
            token_secret_ref: "MISSING".into(),
        };
        let ctx = ExecutionContext::new(HashMap::new(), HashMap::new());
        let (tx, _rx) = mpsc::channel(32);
        let sink = EventSink::new(tx);
        let error = HttpExecutor::new()
            .execute(&doc, &ctx, &sink)
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("MISSING"));
    }
}
