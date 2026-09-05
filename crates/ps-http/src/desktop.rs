//! Bounded HTTP response execution shared by native request editors.
use crate::{HttpBody, HttpRequest, HttpResponse, NetworkClientBuilder, RedirectPolicy};
use ps_request_engine::{ExecutionError, VariableResolver};
use std::{collections::HashMap, time::Instant};

/// Maximum response retained by the interactive editor.
pub const DESKTOP_RESPONSE_LIMIT: usize = 2 * 1024 * 1024;

fn resolve(resolver: &VariableResolver, value: &str) -> Result<String, ExecutionError> {
    let result = resolver.resolve_template(value);
    if !result.diagnostics.is_empty() {
        return Err(ExecutionError::Protocol(
            "Resolve missing or invalid variables before sending.".into(),
        ));
    }
    Ok(result.value)
}

/// Dropping this future cancels network I/O. No raw URL, body or credential is
/// included in errors. Redirects are disabled until per-hop policy is integrated.
pub async fn send_desktop_request(
    request: HttpRequest,
    resolver: VariableResolver,
) -> Result<HttpResponse, ExecutionError> {
    let started = Instant::now();
    let mut resolved = request.clone();
    resolved.raw_url = resolve(&resolver, &request.raw_url)?;
    for param in &mut resolved.params {
        if param.enabled {
            param.key = resolve(&resolver, &param.key)?;
            param.value = resolve(&resolver, &param.value)?;
        }
    }
    let url = resolved
        .build_resolved_url()
        .map_err(|_| ExecutionError::Protocol("Enter a valid HTTP or HTTPS URL.".into()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ExecutionError::Protocol(
            "Enter a valid HTTP or HTTPS URL.".into(),
        ));
    }
    let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
        .map_err(|_| ExecutionError::Protocol("Enter a valid HTTP method.".into()))?;
    let client = NetworkClientBuilder::new()
        .with_redirect_policy(RedirectPolicy {
            follow: false,
            ..Default::default()
        })
        .build()
        .map_err(|_| ExecutionError::Internal("Could not initialize the HTTP client.".into()))?;
    let mut builder = client.request(method, url);
    for header in request.headers.iter().filter(|h| h.enabled) {
        let name = resolve(&resolver, &header.name)?;
        let value = resolve(&resolver, &header.value)?;
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| ExecutionError::Protocol("A header name is invalid.".into()))?;
        let value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| ExecutionError::Protocol("A header value is invalid.".into()))?;
        builder = builder.header(name, value);
    }
    match &request.body {
        HttpBody::None => {}
        HttpBody::Raw {
            content,
            content_type,
        } => {
            if !request
                .headers
                .iter()
                .any(|h| h.enabled && h.name.eq_ignore_ascii_case("content-type"))
            {
                builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
            }
            builder = builder.body(resolve(&resolver, content)?);
        }
        HttpBody::Json { json_content } => {
            let content = resolve(&resolver, json_content)?;
            serde_json::from_str::<serde_json::Value>(&content)
                .map_err(|_| ExecutionError::Protocol("Request body is not valid JSON.".into()))?;
            if !request
                .headers
                .iter()
                .any(|h| h.enabled && h.name.eq_ignore_ascii_case("content-type"))
            {
                builder = builder.header(reqwest::header::CONTENT_TYPE, "application/json");
            }
            builder = builder.body(content);
        }
        _ => {
            return Err(ExecutionError::Protocol(
                "This editor supports raw text and JSON bodies.".into(),
            ))
        }
    }
    let mut response = builder.send().await.map_err(network_error)?;
    let status = response.status();
    let version = format!("{:?}", response.version());
    let mut headers = HashMap::new();
    for (key, value) in response.headers() {
        let safe = ps_request_engine::redact_sensitive_header(
            key.as_str(),
            value.to_str().unwrap_or("[binary]"),
        );
        headers
            .entry(key.to_string())
            .and_modify(|v: &mut String| {
                v.push_str("\n");
                v.push_str(&safe);
            })
            .or_insert(safe);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        if chunk.len() > DESKTOP_RESPONSE_LIMIT.saturating_sub(body.len()) {
            return Err(ExecutionError::Protocol(
                "Response exceeds the editor's 2 MiB limit.".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(HttpResponse {
        status_code: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").into(),
        headers,
        size_bytes: body.len(),
        body_bytes: body,
        duration_ms: started.elapsed().as_millis() as u64,
        http_version: version,
    })
}
fn network_error(error: reqwest::Error) -> ExecutionError {
    if error.is_timeout() {
        ExecutionError::Timeout(30_000)
    } else {
        ExecutionError::Network(
            "Request failed. Check the connection, URL, and TLS certificate.".into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unresolved_variables_fail_without_exposing_values() {
        let resolver = VariableResolver::new();
        let error = resolve(&resolver, "https://{{missing}}/private-token").unwrap_err();
        assert!(!error.to_string().contains("private-token"));
    }
    #[tokio::test]
    async fn invalid_requests_fail_before_network_io() {
        for url in ["file:///tmp/test", "not a url", "https://{{missing}}"] {
            assert!(send_desktop_request(
                HttpRequest::new(crate::HttpMethod::Get, url),
                VariableResolver::new()
            )
            .await
            .is_err());
        }
    }
    #[tokio::test]
    async fn sends_headers_body_and_retains_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 1024];
            loop {
                let count = socket.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.ends_with(b"hello") {
                    break;
                }
            }
            let wire = String::from_utf8(bytes).unwrap();
            assert!(wire.starts_with("POST /echo?item=1&item=2 HTTP/1.1"));
            assert!(wire.to_ascii_lowercase().contains("x-test: resolved"));
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nSet-Cookie: token=private\r\nConnection: close\r\n\r\nOK").await.unwrap();
        });
        let mut request = HttpRequest::new(
            crate::HttpMethod::Post,
            format!("http://{address}/echo?item=1&item=2"),
        );
        request.headers.push(crate::HeaderEntry {
            name: "X-Test".into(),
            value: "{{value}}".into(),
            enabled: true,
            is_secret: false,
            description: None,
        });
        request.body = HttpBody::Raw {
            content: "hello".into(),
            content_type: "text/plain".into(),
        };
        let resolver = VariableResolver::new()
            .with_globals(HashMap::from([("value".into(), "resolved".into())]));
        let response = send_desktop_request(request, resolver).await.unwrap();
        assert_eq!(response.status_code, 200);
        assert_eq!(response.body_bytes, b"OK");
        assert_eq!(response.headers["set-cookie"], "[REDACTED]");
        server.await.unwrap();
    }
}
