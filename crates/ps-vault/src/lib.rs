//! Local OS-backed secrets. Call blocking storage operations off the UI thread.
//! No workspace serialization, logging, or plaintext fallback is provided.
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};
use zeroize::Zeroizing;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VaultError {
    #[error("Secret was not found")]
    NotFound,
    #[error("Secure storage is unavailable or access was denied")]
    Unavailable,
    #[error("Invalid secret name, record, or domain policy")]
    InvalidRecord,
    #[error("Secret is not permitted for this destination")]
    DomainDenied,
}

/// Intentionally neither serializable nor printable; expose only for deliberate use.
pub struct SecretValue(Zeroizing<String>);
impl SecretValue {
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}

/// Storage adapters must preserve confidentiality and never include payloads in errors.
pub trait SecretStore: fmt::Debug + Send + Sync {
    fn read(&self, name: &str) -> Result<SecretValue, VaultError>;
    fn write(&self, name: &str, value: &SecretValue) -> Result<(), VaultError>;
    fn delete(&self, name: &str) -> Result<(), VaultError>;
}

/// Separate namespace for each stable workspace ID; never use a filesystem path.
#[derive(Debug)]
pub struct OsSecretStore {
    service: String,
}
impl OsSecretStore {
    pub fn new(workspace_id: &str) -> Result<Self, VaultError> {
        if workspace_id.is_empty()
            || !workspace_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(VaultError::InvalidRecord);
        }
        Ok(Self {
            service: format!("dev.packetsmith.vault.v1.{workspace_id}"),
        })
    }
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn entry(&self, name: &str) -> Result<keyring::Entry, VaultError> {
        validate_name(name)?;
        keyring::Entry::new(&self.service, name).map_err(|_| VaultError::Unavailable)
    }
}
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn storage_error(error: keyring::Error) -> VaultError {
    match error {
        keyring::Error::NoEntry => VaultError::NotFound,
        _ => VaultError::Unavailable,
    }
}
#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
impl SecretStore for OsSecretStore {
    fn read(&self, name: &str) -> Result<SecretValue, VaultError> {
        self.entry(name)?
            .get_password()
            .map(SecretValue::new)
            .map_err(storage_error)
    }
    fn write(&self, name: &str, value: &SecretValue) -> Result<(), VaultError> {
        self.entry(name)?
            .set_password(value.expose())
            .map_err(storage_error)
    }
    fn delete(&self, name: &str) -> Result<(), VaultError> {
        self.entry(name)?.delete_credential().map_err(storage_error)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    value: Zeroizing<String>,
    allowed_domains: Vec<String>,
    tags: Vec<String>,
}

/// Metadata and value are stored together in a single secure credential update.
#[derive(Debug)]
pub struct Vault<S: SecretStore> {
    store: S,
}
impl<S: SecretStore> Vault<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Creates or replaces a secret. Empty policies deny all execution destinations.
    pub fn put(
        &self,
        name: &str,
        value: SecretValue,
        allowed_domains: Vec<String>,
        tags: Vec<String>,
    ) -> Result<(), VaultError> {
        validate_name(name)?;
        let allowed_domains = allowed_domains
            .iter()
            .map(|domain| canonical_domain(domain))
            .collect::<Result<_, _>>()?;
        let record = Record {
            version: 1,
            value: value.0,
            allowed_domains,
            tags,
        };
        let encoded = SecretValue::new(
            serde_json::to_string(&record).map_err(|_| VaultError::InvalidRecord)?,
        );
        // Windows generic credentials impose a 2560-byte blob limit. Keep the
        // portable record below that limit even with UTF-16 backend encoding.
        if encoded.expose().encode_utf16().count() > 1200 {
            return Err(VaultError::InvalidRecord);
        }
        self.store.write(name, &encoded)
    }
    fn record(&self, name: &str) -> Result<Record, VaultError> {
        validate_name(name)?;
        let raw = self.store.read(name)?;
        let record: Record =
            serde_json::from_str(raw.expose()).map_err(|_| VaultError::InvalidRecord)?;
        if record.version != 1 {
            return Err(VaultError::InvalidRecord);
        }
        for domain in &record.allowed_domains {
            canonical_domain(domain)?;
        }
        Ok(record)
    }
    /// Deliberate reveal; callers must not persist or log the returned value.
    pub fn reveal(&self, name: &str) -> Result<SecretValue, VaultError> {
        Ok(SecretValue(self.record(name)?.value))
    }
    pub fn delete(&self, name: &str) -> Result<(), VaultError> {
        validate_name(name)?;
        self.store.delete(name)
    }
    /// Exact-host HTTPS policy. No suffix, wildcard, or HTTP matching.
    pub fn resolve(&self, name: &str, destination: &str) -> Result<SecretValue, VaultError> {
        let url = url::Url::parse(destination).map_err(|_| VaultError::DomainDenied)?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(VaultError::DomainDenied);
        }
        let host = url.host_str().ok_or(VaultError::DomainDenied)?;
        let record = self.record(name)?;
        if !record.allowed_domains.iter().any(|domain| domain == host) {
            return Err(VaultError::DomainDenied);
        }
        Ok(SecretValue(record.value))
    }
    /// Produces a fresh request-scoped resolver, never a global/editor cache.
    /// The caller must disable redirects or reauthorize every redirect target.
    pub fn resolver(
        &self,
        names: &[&str],
        destination: &str,
        mut base: ps_variable::VariableResolver,
    ) -> Result<ps_variable::VariableResolver, VaultError> {
        base.clear_scope(ps_variable::VariableScope::Vault);
        let mut values = HashMap::new();
        for name in names {
            values.insert(
                (*name).to_owned(),
                self.resolve(name, destination)?.expose().to_owned(),
            );
        }
        Ok(base.with_vault(values))
    }
}
fn validate_name(name: &str) -> Result<(), VaultError> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        return Err(VaultError::InvalidRecord);
    }
    Ok(())
}
fn canonical_domain(domain: &str) -> Result<String, VaultError> {
    if domain.is_empty()
        || domain.contains(['/', ':', '@', '?', '#', '*', '%', '\\'])
        || domain.trim() != domain
        || domain.ends_with('.')
    {
        return Err(VaultError::InvalidRecord);
    }
    let url =
        url::Url::parse(&format!("https://{domain}")).map_err(|_| VaultError::InvalidRecord)?;
    url.host_str()
        .map(str::to_owned)
        .ok_or(VaultError::InvalidRecord)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Memory(Mutex<HashMap<String, String>>);
    impl fmt::Debug for Memory {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Memory([REDACTED])")
        }
    }
    impl SecretStore for Memory {
        fn read(&self, name: &str) -> Result<SecretValue, VaultError> {
            self.0
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(SecretValue::new)
                .ok_or(VaultError::NotFound)
        }
        fn write(&self, name: &str, value: &SecretValue) -> Result<(), VaultError> {
            self.0
                .lock()
                .unwrap()
                .insert(name.into(), value.expose().into());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), VaultError> {
            self.0
                .lock()
                .unwrap()
                .remove(name)
                .map(|_| ())
                .ok_or(VaultError::NotFound)
        }
    }
    #[test]
    fn lifecycle_and_redaction() {
        let vault = Vault::new(Memory::default());
        vault
            .put(
                "token",
                SecretValue::new("super-secret".into()),
                vec!["API.Example.com".into()],
                vec![],
            )
            .unwrap();
        assert_eq!(vault.reveal("token").unwrap().expose(), "super-secret");
        assert!(!format!("{vault:?}").contains("super-secret"));
        let resolver = vault
            .resolver(
                &["token"],
                "https://api.example.com/path",
                ps_variable::VariableResolver::new(),
            )
            .unwrap();
        let result = resolver.resolve_template("Bearer {{vault:token}}");
        assert_eq!(result.value, "Bearer super-secret");
        assert!(!format!("{result:?}").contains("super-secret"));
        vault
            .put(
                "token",
                SecretValue::new("replacement".into()),
                vec![],
                vec![],
            )
            .unwrap();
        assert_eq!(vault.reveal("token").unwrap().expose(), "replacement");
        assert_eq!(
            vault
                .resolve("token", "https://api.example.com")
                .unwrap_err(),
            VaultError::DomainDenied
        );
        vault.delete("token").unwrap();
        assert_eq!(vault.reveal("token").unwrap_err(), VaultError::NotFound);
    }
    #[test]
    fn destination_policy_fails_closed() {
        let vault = Vault::new(Memory::default());
        vault
            .put(
                "token",
                SecretValue::new("secret".into()),
                vec!["api.example.com".into()],
                vec![],
            )
            .unwrap();
        for target in [
            "http://api.example.com",
            "https://api.example.com.evil.test",
            "https://evilapi.example.com",
            "https://sub.api.example.com",
            "https://api.example.com@evil.test",
            "https://user@api.example.com",
            "https://api.example.com.",
        ] {
            assert_eq!(
                vault.resolve("token", target).unwrap_err(),
                VaultError::DomainDenied
            );
        }
        for domain in [
            "*.example.com",
            "example.com/path",
            "example.com:443",
            "",
            "example.com\\evil",
        ] {
            assert_eq!(canonical_domain(domain), Err(VaultError::InvalidRecord));
        }
    }
    #[test]
    fn credentials_are_literal_and_stale_vault_scope_is_removed() {
        let vault = Vault::new(Memory::default());
        vault
            .put(
                "token",
                SecretValue::new("{{missing}}".into()),
                vec!["example.com".into()],
                vec![],
            )
            .unwrap();
        let base = ps_variable::VariableResolver::new()
            .with_vault(HashMap::from([("stale".into(), "old-secret".into())]));
        let resolver = vault
            .resolver(&["token"], "https://example.com", base)
            .unwrap();
        assert_eq!(
            resolver.resolve_template("{{vault:token}}").value,
            "{{missing}}"
        );
        assert!(resolver.resolve_var("vault:stale").is_none());
    }
    #[test]
    fn malformed_record_errors_do_not_expose_contents() {
        let store = Memory::default();
        store
            .write("token", &SecretValue::new("plaintext-secret".into()))
            .unwrap();
        let vault = Vault::new(store);
        assert_eq!(
            vault.reveal("token").unwrap_err(),
            VaultError::InvalidRecord
        );
    }
}
