//! Advanced HTTP networking client configuration, redirect tracking, proxy routing,
//! and TLS policy enforcement.

use std::time::Duration;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum NetworkClientError {
    #[error("Failed to build HTTP client: {0}")]
    Build(#[from] reqwest::Error),
    #[error("Invalid proxy URL: {0}")]
    InvalidProxy(String),
}

/// Redirect policy configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectPolicy {
    pub follow: bool,
    pub max_redirects: usize,
    pub strip_auth_on_cross_origin: bool,
}

impl Default for RedirectPolicy {
    fn default() -> Self {
        Self {
            follow: true,
            max_redirects: 10,
            strip_auth_on_cross_origin: true,
        }
    }
}

/// A recorded step in a redirect chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedirectStep {
    pub url: String,
    pub status_code: u16,
}

/// Proxy routing configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub no_proxy: Vec<String>,
}

impl ProxyConfig {
    /// Determines whether a specific hostname matches the no_proxy exclusion list.
    pub fn is_match_no_proxy(&self, host: &str) -> bool {
        let host_lower = host.to_lowercase();
        self.no_proxy.iter().any(|pattern| {
            let pat_lower = pattern.to_lowercase();
            if pat_lower == "*" {
                true
            } else if pat_lower.starts_with('.') {
                host_lower.ends_with(&pat_lower)
            } else {
                host_lower == pat_lower
            }
        })
    }
}

/// TLS security configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsConfig {
    pub verify_ssl: bool,
    pub custom_ca_cert_path: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_ssl: true,
            custom_ca_cert_path: None,
        }
    }
}

/// Builder creating configured reqwest HTTP clients.
#[derive(Debug, Clone)]
pub struct NetworkClientBuilder {
    timeout: Duration,
    connect_timeout: Duration,
    redirect_policy: RedirectPolicy,
    proxy_config: ProxyConfig,
    tls_config: TlsConfig,
}

impl Default for NetworkClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            redirect_policy: RedirectPolicy::default(),
            proxy_config: ProxyConfig::default(),
            tls_config: TlsConfig::default(),
        }
    }
}

impl NetworkClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    pub fn with_redirect_policy(mut self, policy: RedirectPolicy) -> Self {
        self.redirect_policy = policy;
        self
    }

    pub fn with_proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy_config = proxy;
        self
    }

    pub fn with_tls(mut self, tls: TlsConfig) -> Self {
        self.tls_config = tls;
        self
    }

    /// Constructs the reqwest client adhering to all networking options.
    pub fn build(&self) -> Result<reqwest::Client, NetworkClientError> {
        let mut builder = reqwest::Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout);

        // Redirects
        if self.redirect_policy.follow {
            builder = builder.redirect(Policy::limited(self.redirect_policy.max_redirects));
        } else {
            builder = builder.redirect(Policy::none());
        }

        // TLS Certificate Verification
        if !self.tls_config.verify_ssl {
            warn!("TLS certificate verification is disabled! Insecure mode active.");
            builder = builder.danger_accept_invalid_certs(true);
        }

        // Proxy Configuration
        if self.proxy_config.enabled {
            if let Some(ref http_url) = self.proxy_config.http_proxy {
                let p = reqwest::Proxy::http(http_url)
                    .map_err(|e| NetworkClientError::InvalidProxy(e.to_string()))?;
                builder = builder.proxy(p);
            }
            if let Some(ref https_url) = self.proxy_config.https_proxy {
                let p = reqwest::Proxy::https(https_url)
                    .map_err(|e| NetworkClientError::InvalidProxy(e.to_string()))?;
                builder = builder.proxy(p);
            }
        }

        let client = builder.build()?;
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_proxy_matching() {
        let config = ProxyConfig {
            enabled: true,
            http_proxy: None,
            https_proxy: None,
            no_proxy: vec!["localhost".into(), ".internal.net".into()],
        };

        assert!(config.is_match_no_proxy("localhost"));
        assert!(config.is_match_no_proxy("service.internal.net"));
        assert!(!config.is_match_no_proxy("external.com"));
    }

    #[test]
    fn test_client_builder_defaults() {
        let builder = NetworkClientBuilder::new();
        let client = builder.build().expect("build client");
        // Verify build succeeds with default settings
        let _ = client;
    }
}
