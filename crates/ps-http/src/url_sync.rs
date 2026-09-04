//! Bidirectional URL and query parameter synchronization, path variable detection,
//! and bulk key-value editor formatting.

use crate::{HeaderEntry, QueryParam};

/// Synchronizes between a raw URL string and structured query parameters table rows.
#[derive(Debug, Clone, Default)]
pub struct UrlSyncEngine;

impl UrlSyncEngine {
    /// Parses a raw URL into its base path (scheme + authority + path) and a list of query parameters.
    pub fn parse_url(raw_url: &str) -> (String, Vec<QueryParam>) {
        if let Some(query_idx) = raw_url.find('?') {
            let base_url = raw_url[..query_idx].to_string();
            let query_str = &raw_url[query_idx + 1..];
            let params = Self::parse_query_string(query_str);
            (base_url, params)
        } else {
            (raw_url.to_string(), Vec::new())
        }
    }

    /// Reconstructs a full URL by joining base URL with enabled query parameters.
    pub fn build_url(base_url: &str, params: &[QueryParam]) -> String {
        let enabled_params: Vec<_> = params.iter().filter(|p| p.enabled).collect();
        if enabled_params.is_empty() {
            return base_url.to_string();
        }

        let mut query_parts = Vec::new();
        for param in enabled_params {
            let encoded_key = Self::percent_encode(&param.key);
            let encoded_val = Self::percent_encode(&param.value);
            if encoded_val.is_empty() {
                query_parts.push(encoded_key);
            } else {
                query_parts.push(format!("{}={}", encoded_key, encoded_val));
            }
        }

        let separator = if base_url.contains('?') { "&" } else { "?" };
        format!("{}{}{}", base_url, separator, query_parts.join("&"))
    }

    /// Parses a raw query string into `QueryParam` structs.
    pub fn parse_query_string(query_str: &str) -> Vec<QueryParam> {
        let mut params = Vec::new();
        if query_str.is_empty() {
            return params;
        }

        for pair in query_str.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = if let Some(eq_idx) = pair.find('=') {
                (&pair[..eq_idx], &pair[eq_idx + 1..])
            } else {
                (pair, "")
            };

            params.push(QueryParam {
                key: Self::percent_decode(key),
                value: Self::percent_decode(value),
                enabled: true,
                description: None,
            });
        }
        params
    }

    /// Detects named path variables like `:id` or `{{id}}` in the URL string.
    pub fn detect_path_variables(url_str: &str) -> Vec<String> {
        let mut vars = Vec::new();
        // 1. Detect :param
        for segment in url_str.split('/') {
            if let Some(param) = segment.strip_prefix(':') {
                let clean_param = param.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if !clean_param.is_empty() && !vars.contains(&clean_param.to_string()) {
                    vars.push(clean_param.to_string());
                }
            }
        }
        // 2. Detect {{param}}
        let mut chars = url_str.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next();
                let mut var_name = String::new();
                while let Some(inner) = chars.next() {
                    if inner == '}' && chars.peek() == Some(&'}') {
                        chars.next();
                        let trimmed = var_name.trim().to_string();
                        if !trimmed.is_empty() && !vars.contains(&trimmed) {
                            vars.push(trimmed);
                        }
                        break;
                    }
                    var_name.push(inner);
                }
            }
        }
        vars
    }

    /// Bulk edit parser: converts `key: value` or `key=value` multi-line text into key-value pairs.
    pub fn parse_bulk_text(text: &str) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }
            if let Some(colon_idx) = trimmed.find(':') {
                let key = trimmed[..colon_idx].trim().to_string();
                let val = trimmed[colon_idx + 1..].trim().to_string();
                entries.push((key, val));
            } else if let Some(eq_idx) = trimmed.find('=') {
                let key = trimmed[..eq_idx].trim().to_string();
                let val = trimmed[eq_idx + 1..].trim().to_string();
                entries.push((key, val));
            } else {
                entries.push((trimmed.to_string(), String::new()));
            }
        }
        entries
    }

    /// Bulk edit serializer: converts key-value pairs into formatted `key: value` multi-line text.
    pub fn serialize_bulk_text<I, K, V>(items: I) -> String
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut lines = Vec::new();
        for (k, v) in items {
            let key = k.as_ref().trim();
            let val = v.as_ref().trim();
            if !key.is_empty() {
                lines.push(format!("{}: {}", key, val));
            }
        }
        lines.join("\n")
    }

    pub fn percent_encode(input: &str) -> String {
        url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
    }

    pub fn percent_decode(input: &str) -> String {
        url::form_urlencoded::parse(input.as_bytes())
            .map(|(k, _)| k.into_owned())
            .next()
            .unwrap_or_else(|| input.to_string())
    }
}

/// Standard header presets for quick configuration.
pub struct HeaderPresets;

impl HeaderPresets {
    pub fn json() -> Vec<HeaderEntry> {
        vec![
            HeaderEntry {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
                enabled: true,
                is_secret: false,
                description: Some("Request body format".into()),
            },
            HeaderEntry {
                name: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
                is_secret: false,
                description: Some("Expected response format".into()),
            },
        ]
    }

    pub fn form_urlencoded() -> Vec<HeaderEntry> {
        vec![HeaderEntry {
            name: "Content-Type".to_string(),
            value: "application/x-www-form-urlencoded".to_string(),
            enabled: true,
            is_secret: false,
            description: Some("URL-encoded form body".into()),
        }]
    }

    pub fn xml() -> Vec<HeaderEntry> {
        vec![
            HeaderEntry {
                name: "Content-Type".to_string(),
                value: "application/xml".to_string(),
                enabled: true,
                is_secret: false,
                description: Some("XML payload format".into()),
            },
            HeaderEntry {
                name: "Accept".to_string(),
                value: "application/xml".to_string(),
                enabled: true,
                is_secret: false,
                description: Some("Expected XML response".into()),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_build_url() {
        let raw = "https://api.example.com/v1/search?query=rust&limit=25";
        let (base, params) = UrlSyncEngine::parse_url(raw);
        assert_eq!(base, "https://api.example.com/v1/search");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].key, "query");
        assert_eq!(params[0].value, "rust");

        let reconstructed = UrlSyncEngine::build_url(&base, &params);
        assert_eq!(reconstructed, raw);
    }

    #[test]
    fn test_detect_path_variables() {
        let vars = UrlSyncEngine::detect_path_variables("https://api.example.com/users/:userId/orders/{{orderId}}");
        assert_eq!(vars, vec!["userId".to_string(), "orderId".to_string()]);
    }

    #[test]
    fn test_bulk_parse_and_serialize() {
        let text = "Accept: application/json\nAuthorization: Bearer xyz\n";
        let parsed = UrlSyncEngine::parse_bulk_text(text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "Accept");
        assert_eq!(parsed[0].1, "application/json");

        let serialized = UrlSyncEngine::serialize_bulk_text(&parsed);
        assert_eq!(serialized, "Accept: application/json\nAuthorization: Bearer xyz");
    }
}
