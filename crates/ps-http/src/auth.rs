//! Authentication providers for HTTP execution.
//!
//! Covers checklist 21.1 (architecture) and the 21.2 basic set:
//! No Auth, Inherit, API Key, Bearer, Basic. OAuth 2.0, Digest, AWS SigV4,
//! Hawk, NTLM, and plugin providers are explicitly out of scope and return
//! [`AuthError::Unsupported`] instead of failing silently.
//!
//! Design notes:
//! - `AuthConfig` shapes live in `ps-domain`; this module resolves inheritance
//!   and applies credentials to outgoing requests.
//! - Secret references (`token_secret_ref`, `password_secret_ref`,
//!   `value_secret_ref`) are vault secret *names* (see
//!   `docs/format/workspace_format.md`), resolved through the shared
//!   [`VariableResolver`]. `{{template}}` references are resolved as templates
//!   so environment/collection/request variables keep working.
//! - Nothing secret ever appears in errors or `Debug` output. Wire values are
//!   only exposed through explicit accessors; logs and UI must use the
//!   redacted preview.

use std::fmt;

use ps_domain::{ApiKeyLocation, AuthConfig};
use ps_request_engine::{redact_sensitive_header, VariableResolver};
use thiserror::Error;

/// Secret-safe authentication failures. Messages never include credential values.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    #[error("Authentication credential is missing")]
    MissingCredential,
    #[error("Authentication references an unresolved variable or vault secret")]
    Unresolved,
    #[error("Authentication configuration is invalid")]
    InvalidConfig,
    #[error("Authentication method is not supported yet")]
    Unsupported,
}

/// One applied credential header with its log-safe preview.
#[derive(Clone, PartialEq, Eq)]
pub struct AppliedHeader {
    pub name: String,
    /// Real wire value. Never log this directly; use [`AppliedHeader::redacted`].
    value: String,
}

impl fmt::Debug for AppliedHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppliedHeader")
            .field("name", &self.name)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

impl AppliedHeader {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    /// Real wire value for the HTTP client only.
    pub fn wire_value(&self) -> &str {
        &self.value
    }

    /// Log/UI-safe preview (`[REDACTED]` for sensitive headers).
    pub fn redacted(&self) -> String {
        redact_sensitive_header(&self.name, &self.value)
    }
}

/// Credentials resolved from an [`AuthConfig`], ready to attach to a request.
///
/// The struct intentionally hides wire values behind [`AppliedAuth::headers`]
/// accessors; its `Debug` impl only shows redacted previews.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct AppliedAuth {
    headers: Vec<AppliedHeader>,
    query_params: Vec<(String, String)>,
    cookies: Vec<(String, String)>,
}

impl AppliedAuth {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.headers.is_empty() && self.query_params.is_empty() && self.cookies.is_empty()
    }

    /// Wire headers for the HTTP client. Callers must not log these values.
    pub fn headers(&self) -> &[AppliedHeader] {
        &self.headers
    }

    /// Query pairs to append (ApiKey in query). Values are secrets; do not log.
    pub fn query_params(&self) -> &[(String, String)] {
        &self.query_params
    }

    /// Cookie pairs to merge into the `Cookie` header. Values are secrets.
    pub fn cookies(&self) -> &[(String, String)] {
        &self.cookies
    }

    /// Log/UI-safe header preview (`name: [REDACTED]` for sensitive headers).
    pub fn redacted_preview(&self) -> Vec<(String, String)> {
        self.headers
            .iter()
            .map(|h| (h.name.clone(), h.redacted()))
            .collect()
    }

    fn push_header(&mut self, name: String, value: String) {
        // Never emit duplicate Authorization headers: an explicit caller header
        // wins over generated auth (documented override policy).
        if name.eq_ignore_ascii_case("authorization")
            && self
                .headers
                .iter()
                .any(|h| h.name.eq_ignore_ascii_case("authorization"))
        {
            return;
        }
        self.headers.push(AppliedHeader::new(name, value));
    }
}

impl fmt::Debug for AppliedAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppliedAuth")
            .field("headers", &self.redacted_preview())
            .field("query_count", &self.query_params.len())
            .field("cookie_count", &self.cookies.len())
            .finish()
    }
}

/// Pluggable credential source. Implement this to add providers (OAuth 2.0,
/// plugin providers, …) without changing call sites.
pub trait AuthProvider: fmt::Debug + Send + Sync {
    /// Stable provider id, e.g. `"bearer"`, `"basic"`, `"api-key"`, `"none"`.
    fn id(&self) -> &'static str;
    /// Resolve credentials against `resolver`. Returns secret-safe errors.
    fn apply(
        &self,
        config: &AuthConfig,
        resolver: &VariableResolver,
    ) -> Result<AppliedAuth, AuthError>;
}

#[derive(Debug, Default)]
pub struct NoAuthProvider;
#[derive(Debug, Default)]
pub struct BearerProvider;
#[derive(Debug, Default)]
pub struct BasicProvider;
#[derive(Debug, Default)]
pub struct ApiKeyProvider;

impl AuthProvider for NoAuthProvider {
    fn id(&self) -> &'static str {
        "none"
    }

    fn apply(
        &self,
        _config: &AuthConfig,
        _resolver: &VariableResolver,
    ) -> Result<AppliedAuth, AuthError> {
        Ok(AppliedAuth::empty())
    }
}

impl AuthProvider for BearerProvider {
    fn id(&self) -> &'static str {
        "bearer"
    }

    fn apply(
        &self,
        config: &AuthConfig,
        resolver: &VariableResolver,
    ) -> Result<AppliedAuth, AuthError> {
        let secret_ref = match config {
            AuthConfig::Bearer { token_secret_ref } => token_secret_ref.as_str(),
            _ => return Err(AuthError::InvalidConfig),
        };
        let token = resolve_secret(resolver, secret_ref)?;
        if token.is_empty() {
            return Err(AuthError::MissingCredential);
        }
        let mut out = AppliedAuth::empty();
        out.push_header("Authorization".into(), format!("Bearer {token}"));
        Ok(out)
    }
}

impl AuthProvider for BasicProvider {
    fn id(&self) -> &'static str {
        "basic"
    }

    fn apply(
        &self,
        config: &AuthConfig,
        resolver: &VariableResolver,
    ) -> Result<AppliedAuth, AuthError> {
        let (username_raw, password_ref) = match config {
            AuthConfig::Basic {
                username,
                password_secret_ref,
            } => (username.as_str(), password_secret_ref.as_deref()),
            _ => return Err(AuthError::InvalidConfig),
        };
        let username = resolve_plain_template(resolver, username_raw)?;
        let password = match password_ref {
            None => String::new(),
            Some(secret_ref) if secret_ref.trim().is_empty() => String::new(),
            Some(secret_ref) => resolve_secret(resolver, secret_ref)?,
        };
        let mut out = AppliedAuth::empty();
        out.push_header(
            "Authorization".into(),
            format!(
                "Basic {}",
                base64_encode(format!("{username}:{password}").as_bytes())
            ),
        );
        Ok(out)
    }
}

impl AuthProvider for ApiKeyProvider {
    fn id(&self) -> &'static str {
        "api-key"
    }

    fn apply(
        &self,
        config: &AuthConfig,
        resolver: &VariableResolver,
    ) -> Result<AppliedAuth, AuthError> {
        let (key_raw, value_ref, location) = match config {
            AuthConfig::ApiKey {
                key,
                value_secret_ref,
                location,
            } => (key.as_str(), value_secret_ref.as_str(), *location),
            _ => return Err(AuthError::InvalidConfig),
        };
        let key = resolve_plain_template(resolver, key_raw)?;
        validate_key_name(&key)?;
        let value = resolve_secret(resolver, value_ref)?;
        if value.is_empty() {
            return Err(AuthError::MissingCredential);
        }
        let mut out = AppliedAuth::empty();
        match location {
            ApiKeyLocation::Header => out.push_header(key, value),
            ApiKeyLocation::Query => out.query_params.push((key, value)),
            ApiKeyLocation::Cookie => out.cookies.push((key, value)),
        }
        Ok(out)
    }
}

/// Resolve the effective auth for a request given its folder/collection chain.
///
/// `Inherit` walks upward (request → folder → collection). An explicit `None`
/// stops inheritance. A terminal `Inherit` with no parent resolves to `None`.
pub fn resolve_effective_auth(
    request: &AuthConfig,
    folder: Option<&AuthConfig>,
    collection: &AuthConfig,
) -> AuthConfig {
    if *request != AuthConfig::Inherit {
        return request.clone();
    }
    if let Some(folder_auth) = folder {
        if *folder_auth != AuthConfig::Inherit {
            return folder_auth.clone();
        }
    }
    if *collection != AuthConfig::Inherit {
        return collection.clone();
    }
    AuthConfig::None
}

/// Apply an already-resolved [`AuthConfig`] (see [`resolve_effective_auth`]).
///
/// `Inherit` without a parent resolves to no auth. Anything beyond
/// None/Basic/Bearer/ApiKey returns [`AuthError::Unsupported`].
pub fn apply_auth(
    config: &AuthConfig,
    resolver: &VariableResolver,
) -> Result<AppliedAuth, AuthError> {
    match config {
        AuthConfig::None | AuthConfig::Inherit => Ok(AppliedAuth::empty()),
        AuthConfig::Basic { .. } => BasicProvider.apply(config, resolver),
        AuthConfig::Bearer { .. } => BearerProvider.apply(config, resolver),
        AuthConfig::ApiKey { .. } => ApiKeyProvider.apply(config, resolver),
        AuthConfig::OAuth2 { .. } => Err(AuthError::Unsupported),
    }
}

/// Resolve a vault secret reference or `{{template}}` to its secret value.
///
/// Lookup order: `vault:<ref>` scope, then plain variable lookup, then template
/// resolution when the ref itself contains `{{...}}`. Failures are generic so
/// secret names never leak into diagnostics beyond the fact of being unresolved.
fn resolve_secret(resolver: &VariableResolver, secret_ref: &str) -> Result<String, AuthError> {
    let trimmed = secret_ref.trim();
    if trimmed.is_empty() {
        return Err(AuthError::MissingCredential);
    }
    if let Some(value) = resolver.resolve_var(&format!("vault:{trimmed}")) {
        return Ok(value);
    }
    // `{{vault:NAME}}` / `{{variable}}` style references.
    if trimmed.contains("{{") {
        let result = resolver.resolve_template(trimmed);
        if result.diagnostics.is_empty() {
            return Ok(result.value);
        }
        return Err(AuthError::Unresolved);
    }
    if let Some(value) = resolver.resolve_var(trimmed) {
        return Ok(value);
    }
    Err(AuthError::Unresolved)
}

/// Resolve a non-secret identifier (username, API key name) as a template.
fn resolve_plain_template(resolver: &VariableResolver, raw: &str) -> Result<String, AuthError> {
    let result = resolver.resolve_template(raw);
    if !result.diagnostics.is_empty() {
        return Err(AuthError::Unresolved);
    }
    let value = result.value.trim().to_owned();
    if value.is_empty() {
        return Err(AuthError::MissingCredential);
    }
    Ok(value)
}

fn validate_key_name(key: &str) -> Result<(), AuthError> {
    if key.len() > 256
        || key
            .bytes()
            .any(|b| !(0x21..=0x7e).contains(&b) || b == b':' || b == b' ')
        || key.contains(['\r', '\n', ';', '=', '&'])
    {
        return Err(AuthError::InvalidConfig);
    }
    Ok(())
}

/// Minimal RFC 4648 base64 encoding (standard alphabet, `=` padding).
/// Kept dependency-free; Basic auth is the only caller.
fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as usize
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARSET[(triple >> 18) & 63] as char);
        out.push(CHARSET[(triple >> 12) & 63] as char);
        if i + 1 < input.len() {
            out.push(CHARSET[(triple >> 6) & 63] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(CHARSET[triple & 63] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn resolver_with(vault: &[(&str, &str)], vars: &[(&str, &str)]) -> VariableResolver {
        VariableResolver::new()
            .with_vault(HashMap::from_iter(
                vault.iter().map(|(k, v)| ((*k).into(), (*v).into())),
            ))
            .with_globals(HashMap::from_iter(
                vars.iter().map(|(k, v)| ((*k).into(), (*v).into())),
            ))
    }

    #[test]
    fn inheritance_walks_request_folder_collection() {
        let collection = AuthConfig::Bearer {
            token_secret_ref: "C".into(),
        };
        let folder = AuthConfig::Basic {
            username: "u".into(),
            password_secret_ref: None,
        };
        let request = AuthConfig::ApiKey {
            key: "X-Key".into(),
            value_secret_ref: "K".into(),
            location: ApiKeyLocation::Header,
        };
        // Explicit request wins.
        assert_eq!(
            resolve_effective_auth(&request, Some(&folder), &collection),
            request
        );
        // Request inherits folder.
        assert_eq!(
            resolve_effective_auth(&AuthConfig::Inherit, Some(&folder), &collection),
            folder
        );
        // Folder inherits collection.
        assert_eq!(
            resolve_effective_auth(
                &AuthConfig::Inherit,
                Some(&AuthConfig::Inherit),
                &collection
            ),
            collection
        );
        // Explicit None stops inheritance.
        assert_eq!(
            resolve_effective_auth(&AuthConfig::None, Some(&folder), &collection),
            AuthConfig::None
        );
        // Terminal inherit resolves to None.
        assert_eq!(
            resolve_effective_auth(&AuthConfig::Inherit, None, &AuthConfig::Inherit),
            AuthConfig::None
        );
    }

    #[test]
    fn bearer_applies_and_redacts() {
        let resolver = resolver_with(&[("AUTH_TOKEN", "super-secret")], &[]);
        let applied = apply_auth(
            &AuthConfig::Bearer {
                token_secret_ref: "AUTH_TOKEN".into(),
            },
            &resolver,
        )
        .unwrap();
        assert_eq!(applied.headers()[0].wire_value(), "Bearer super-secret");
        assert_eq!(
            applied.redacted_preview(),
            vec![("Authorization".to_string(), "[REDACTED]".to_string())]
        );
        let debug = format!("{applied:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("super-secret"));
    }

    #[test]
    fn bearer_supports_template_reference() {
        let resolver = resolver_with(&[("AUTH_TOKEN", "templated-secret")], &[]);
        let applied = apply_auth(
            &AuthConfig::Bearer {
                token_secret_ref: "{{vault:AUTH_TOKEN}}".into(),
            },
            &resolver,
        )
        .unwrap();
        assert_eq!(applied.headers()[0].wire_value(), "Bearer templated-secret");
    }

    #[test]
    fn basic_encodes_rfc7617_vector() {
        let resolver = resolver_with(&[("PASS", "secret")], &[]);
        let applied = apply_auth(
            &AuthConfig::Basic {
                username: "admin".into(),
                password_secret_ref: Some("PASS".into()),
            },
            &resolver,
        )
        .unwrap();
        // "admin:secret" base64 (checks padding + alphabet, not just prefix).
        assert_eq!(applied.headers()[0].wire_value(), "Basic YWRtaW46c2VjcmV0");
        assert!(!format!("{applied:?}").contains("secret"));
    }

    #[test]
    fn basic_username_supports_variables_and_empty_password() {
        let resolver = resolver_with(&[], &[("user", "alice")]);
        let applied = apply_auth(
            &AuthConfig::Basic {
                username: "{{user}}".into(),
                password_secret_ref: None,
            },
            &resolver,
        )
        .unwrap();
        // "alice:" base64.
        assert_eq!(applied.headers()[0].wire_value(), "Basic YWxpY2U6");
    }

    #[test]
    fn api_key_supports_all_locations() {
        let resolver = resolver_with(&[("K", "key-secret")], &[]);
        let header = apply_auth(
            &AuthConfig::ApiKey {
                key: "X-API-Key".into(),
                value_secret_ref: "K".into(),
                location: ApiKeyLocation::Header,
            },
            &resolver,
        )
        .unwrap();
        assert_eq!(header.headers()[0].wire_value(), "key-secret");
        assert!(header.query_params().is_empty());

        let query = apply_auth(
            &AuthConfig::ApiKey {
                key: "api_key".into(),
                value_secret_ref: "K".into(),
                location: ApiKeyLocation::Query,
            },
            &resolver,
        )
        .unwrap();
        assert!(query.headers().is_empty());
        assert_eq!(
            query.query_params(),
            &[("api_key".to_string(), "key-secret".to_string())]
        );

        let cookie = apply_auth(
            &AuthConfig::ApiKey {
                key: "session".into(),
                value_secret_ref: "K".into(),
                location: ApiKeyLocation::Cookie,
            },
            &resolver,
        )
        .unwrap();
        assert!(cookie.headers().is_empty());
        assert_eq!(
            cookie.cookies(),
            &[("session".to_string(), "key-secret".to_string())]
        );
        assert!(!format!("{cookie:?}").contains("key-secret"));
    }

    #[test]
    fn auth_errors_never_contain_secrets() {
        let resolver = resolver_with(&[("TOKEN", "do-not-leak")], &[]);
        for config in [
            AuthConfig::Bearer {
                token_secret_ref: "MISSING".into(),
            },
            AuthConfig::Bearer {
                token_secret_ref: "".into(),
            },
            AuthConfig::Bearer {
                token_secret_ref: "{{missing}}".into(),
            },
            AuthConfig::OAuth2 {
                flow: "auth_code".into(),
                token_secret_ref: None,
            },
        ] {
            let err = apply_auth(&config, &resolver).unwrap_err();
            assert!(!err.to_string().contains("do-not-leak"));
            assert!(!format!("{err:?}").contains("do-not-leak"));
        }
        // OAuth2 is explicitly unsupported, not silently ignored.
        assert_eq!(
            apply_auth(
                &AuthConfig::OAuth2 {
                    flow: "auth_code".into(),
                    token_secret_ref: None,
                },
                &resolver,
            )
            .unwrap_err(),
            AuthError::Unsupported
        );
    }

    #[test]
    fn providers_expose_stable_ids() {
        let resolver = VariableResolver::new();
        assert_eq!(NoAuthProvider.id(), "none");
        assert_eq!(BearerProvider.id(), "bearer");
        assert_eq!(BasicProvider.id(), "basic");
        assert_eq!(ApiKeyProvider.id(), "api-key");
        assert!(NoAuthProvider
            .apply(&AuthConfig::None, &resolver)
            .unwrap()
            .is_empty());
    }
}
