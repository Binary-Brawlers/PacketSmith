//! Bounded HTTP response execution shared by native request editors.
use crate::{HttpBody, HttpRequest, HttpResponse, NetworkClientBuilder, RedirectPolicy};
use ps_domain::AuthConfig;
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
    send_desktop_request_with_auth(request, resolver, &AuthConfig::None).await
}

fn auth_error(error: crate::auth::AuthError) -> ExecutionError {
    match error {
        crate::auth::AuthError::MissingCredential | crate::auth::AuthError::Unresolved => {
            ExecutionError::Protocol(
                "Resolve missing auth credentials or variables before sending.".into(),
            )
        }
        crate::auth::AuthError::Unsupported => {
            ExecutionError::Protocol("This authentication method is not supported yet.".into())
        }
        crate::auth::AuthError::InvalidConfig => {
            ExecutionError::Protocol("The authentication configuration is invalid.".into())
        }
    }
}

/// Auth-aware variant. Credentials resolve through `resolver` (vault names and
/// `{{templates}}`); failures block the send instead of going out unauthenticated.
/// Auth wire values are never re-resolved as templates: vault bytes are opaque
/// and must stay literal.
pub async fn send_desktop_request_with_auth(
    request: HttpRequest,
    resolver: VariableResolver,
    auth: &AuthConfig,
) -> Result<HttpResponse, ExecutionError> {
    let applied = crate::auth::apply_auth(auth, &resolver).map_err(auth_error)?;
    let started = Instant::now();
    let client = NetworkClientBuilder::new()
        .with_redirect_policy(RedirectPolicy {
            follow: false,
            ..Default::default()
        })
        .build()
        .map_err(|_| ExecutionError::Internal("Could not initialize the HTTP client.".into()))?;
    let builder = build_request(&client, &request, &resolver, &applied)?;
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
                v.push('\n');
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
/// Shared preparation for file-backed and interactive HTTP requests.
pub(crate) fn build_request(
    client: &reqwest::Client,
    request: &HttpRequest,
    resolver: &VariableResolver,
    applied: &crate::auth::AppliedAuth,
) -> Result<reqwest::RequestBuilder, ExecutionError> {
    let mut resolved = request.clone();
    resolved.raw_url = resolve(resolver, &request.raw_url)?;
    for param in &mut resolved.params {
        if param.enabled {
            param.key = resolve(resolver, &param.key)?;
            param.value = resolve(resolver, &param.value)?;
        }
    }
    let mut url = resolved
        .build_resolved_url()
        .map_err(|_| ExecutionError::Protocol("Enter a valid HTTP or HTTPS URL.".into()))?;
    if !applied.query_params().is_empty() {
        url.query_pairs_mut().extend_pairs(
            applied
                .query_params()
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str())),
        );
    }
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ExecutionError::Protocol(
            "Enter a valid HTTP or HTTPS URL.".into(),
        ));
    }
    let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
        .map_err(|_| ExecutionError::Protocol("Enter a valid HTTP method.".into()))?;
    let mut builder = client.request(method, url);
    let mut user_header_names: Vec<String> = Vec::new();
    let mut cookie_values: Vec<String> = Vec::new();
    for header in request.headers.iter().filter(|h| h.enabled) {
        let name = resolve(resolver, &header.name)?;
        let value = resolve(resolver, &header.value)?;
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| ExecutionError::Protocol("A header name is invalid.".into()))?;
        if name == reqwest::header::COOKIE {
            // Defer: user + auth cookies merge into a single Cookie header below.
            cookie_values.push(value);
            if !user_header_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case("cookie"))
            {
                user_header_names.push("cookie".to_string());
            }
            continue;
        }
        user_header_names.push(name.to_string());
        let value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| ExecutionError::Protocol("A header value is invalid.".into()))?;
        builder = builder.header(name, value);
    }
    // Generated auth applies after explicit headers; an explicit header with the
    // same name wins (no duplicate Authorization headers). Auth wire values are
    // final and bypass template re-resolution so vault bytes stay literal.
    for header in applied.headers() {
        if user_header_names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&header.name))
        {
            continue;
        }
        let name = reqwest::header::HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|_| ExecutionError::Protocol("An auth header name is invalid.".into()))?;
        if name == reqwest::header::COOKIE {
            cookie_values.push(header.wire_value().to_owned());
            continue;
        }
        user_header_names.push(name.to_string());
        let value = reqwest::header::HeaderValue::from_str(header.wire_value())
            .map_err(|_| ExecutionError::Protocol("An auth header value is invalid.".into()))?;
        builder = builder.header(name, value);
    }
    for (k, v) in applied.cookies() {
        cookie_values.push(format!("{k}={v}"));
    }
    if !cookie_values.is_empty() {
        builder = builder.header(reqwest::header::COOKIE, cookie_values.join("; "));
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
            builder = builder.body(resolve(resolver, content)?);
        }
        HttpBody::Json { json_content } => {
            let content = resolve(resolver, json_content)?;
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
    Ok(builder)
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
    use ps_domain::ApiKeyLocation;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serves one request, captures the raw HTTP wire, and replies 200 OK.
    async fn serve_once_capture_wire() -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let count = socket.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(header_end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers =
                        String::from_utf8_lossy(&bytes[..header_end]).to_ascii_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .and_then(|length| length.parse::<usize>().ok())
                        .unwrap_or(0);
                    if bytes.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
                .await
                .unwrap();
            String::from_utf8(bytes).unwrap()
        });
        (address, server)
    }

    #[tokio::test]
    async fn persisted_payload_sends_complete_body_headers_and_query_without_duplicates() {
        let (address, server) = serve_once_capture_wire().await;
        let mut payload = ps_domain::HttpRequestPayload::new(
            "POST",
            format!("http://{address}/items?item={{{{item}}}}&item=two"),
        );
        payload.params = vec![
            crate::QueryParam {
                key: "item".into(),
                value: "{{item}}".into(),
                enabled: true,
                description: None,
            },
            crate::QueryParam {
                key: "item".into(),
                value: "two".into(),
                enabled: true,
                description: None,
            },
            crate::QueryParam {
                key: "skip".into(),
                value: "skip".into(),
                enabled: false,
                description: None,
            },
        ];
        payload.headers = vec![crate::HeaderEntry {
            name: "X-Test".into(),
            value: "{{item}}".into(),
            enabled: true,
            is_secret: false,
            description: Some("Keep description".into()),
        }];
        payload.body = HttpBody::Json {
            json_content: "{\"value\":\"{{item}}\"}".into(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let reopened = serde_json::from_str(&json).unwrap();
        let resolver =
            VariableResolver::new().with_globals(HashMap::from([("item".into(), "one".into())]));
        let response = send_desktop_request(HttpRequest::from_payload(&reopened), resolver)
            .await
            .unwrap();
        assert_eq!(response.status_code, 200);
        let wire = server.await.unwrap();
        assert!(
            wire.starts_with("POST /items?item=one&item=two HTTP/1.1"),
            "{wire}"
        );
        assert!(wire.to_ascii_lowercase().contains("x-test: one"));
        assert!(wire
            .to_ascii_lowercase()
            .contains("content-type: application/json"));
        assert!(wire.ends_with("{\"value\":\"one\"}"));
        assert!(!wire.contains("skip"));
    }

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

    #[tokio::test]
    async fn bearer_auth_sends_vault_backed_authorization_header() {
        let (address, server) = serve_once_capture_wire().await;
        let request = HttpRequest::new(crate::HttpMethod::Get, format!("http://{address}/secure"));
        let resolver = VariableResolver::new().with_vault(HashMap::from([(
            "AUTH_TOKEN".into(),
            "desktop-secret".into(),
        )]));
        let auth = AuthConfig::Bearer {
            token_secret_ref: "AUTH_TOKEN".into(),
        };
        let response = send_desktop_request_with_auth(request, resolver, &auth)
            .await
            .unwrap();
        assert_eq!(response.status_code, 200);
        let wire = server.await.unwrap().to_ascii_lowercase();
        assert!(wire.contains("authorization: bearer desktop-secret"));
    }

    #[tokio::test]
    async fn basic_auth_sends_rfc7617_header() {
        let (address, server) = serve_once_capture_wire().await;
        let request = HttpRequest::new(crate::HttpMethod::Get, format!("http://{address}/secure"));
        let resolver =
            VariableResolver::new().with_vault(HashMap::from([("PASS".into(), "secret".into())]));
        let auth = AuthConfig::Basic {
            username: "admin".into(),
            password_secret_ref: Some("PASS".into()),
        };
        send_desktop_request_with_auth(request, resolver, &auth)
            .await
            .unwrap();
        let wire = server.await.unwrap().to_ascii_lowercase();
        // "admin:secret" base64 is YWRtaW46c2VjcmV0 (lowercased on the wire here).
        assert!(wire.contains("authorization: basic ywrtaw46c2vjcmv0"));
    }

    #[tokio::test]
    async fn api_key_query_appends_to_request_line() {
        let (address, server) = serve_once_capture_wire().await;
        let request = HttpRequest::new(crate::HttpMethod::Get, format!("http://{address}/items"));
        let resolver = VariableResolver::new()
            .with_vault(HashMap::from([("K".into(), "query-secret".into())]));
        let auth = AuthConfig::ApiKey {
            key: "api_key".into(),
            value_secret_ref: "K".into(),
            location: ApiKeyLocation::Query,
        };
        send_desktop_request_with_auth(request, resolver, &auth)
            .await
            .unwrap();
        let wire = server.await.unwrap();
        assert!(wire.starts_with("GET /items?api_key=query-secret HTTP/1.1"));
    }

    #[tokio::test]
    async fn api_key_cookie_merges_with_explicit_cookie_header() {
        let (address, server) = serve_once_capture_wire().await;
        let mut request =
            HttpRequest::new(crate::HttpMethod::Get, format!("http://{address}/items"));
        request.headers.push(crate::HeaderEntry {
            name: "Cookie".into(),
            value: "theme=light".into(),
            enabled: true,
            is_secret: false,
            description: None,
        });
        let resolver = VariableResolver::new()
            .with_vault(HashMap::from([("K".into(), "cookie-secret".into())]));
        let auth = AuthConfig::ApiKey {
            key: "session".into(),
            value_secret_ref: "K".into(),
            location: ApiKeyLocation::Cookie,
        };
        send_desktop_request_with_auth(request, resolver, &auth)
            .await
            .unwrap();
        let wire = server.await.unwrap().to_ascii_lowercase();
        assert!(wire.contains("cookie: theme=light; session=cookie-secret"));
    }

    #[tokio::test]
    async fn explicit_authorization_header_wins_over_generated_auth() {
        let (address, server) = serve_once_capture_wire().await;
        let mut request =
            HttpRequest::new(crate::HttpMethod::Get, format!("http://{address}/items"));
        request.headers.push(crate::HeaderEntry {
            name: "Authorization".into(),
            value: "Bearer explicit".into(),
            enabled: true,
            is_secret: true,
            description: None,
        });
        let resolver = VariableResolver::new().with_vault(HashMap::from([(
            "AUTH_TOKEN".into(),
            "generated-secret".into(),
        )]));
        let auth = AuthConfig::Bearer {
            token_secret_ref: "AUTH_TOKEN".into(),
        };
        send_desktop_request_with_auth(request, resolver, &auth)
            .await
            .unwrap();
        let wire = server.await.unwrap().to_ascii_lowercase();
        assert!(wire.contains("authorization: bearer explicit"));
        assert!(!wire.contains("generated-secret"));
    }

    #[tokio::test]
    async fn unresolved_auth_fails_before_network_without_leaking() {
        let request = HttpRequest::new(crate::HttpMethod::Get, "http://127.0.0.1:1/unused");
        let resolver = VariableResolver::new();
        let auth = AuthConfig::Bearer {
            token_secret_ref: "MISSING_TOKEN".into(),
        };
        let error = send_desktop_request_with_auth(request, resolver, &auth)
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("MISSING_TOKEN"));
    }
}
