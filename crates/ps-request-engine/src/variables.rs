//! Backwards-compatible re-export of PacketSmith's dedicated variable engine.

pub use ps_variable::*;

/// Redacts sensitive header values (Authorization, Cookie, X-Api-Key).
pub fn redact_sensitive_header(name: &str, value: &str) -> String {
    let lower = name.to_lowercase();
    if lower == "authorization"
        || lower == "cookie"
        || lower == "set-cookie"
        || lower.contains("key")
        || lower.contains("secret")
        || lower.contains("token")
    {
        "[REDACTED]".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_variable_scope_precedence() {
        let mut globals = HashMap::new();
        globals.insert("host".into(), "api.global.com".into());
        globals.insert("version".into(), "v1".into());

        let mut envs = HashMap::new();
        envs.insert("host".into(), "api.staging.com".into());

        let mut reqs = HashMap::new();
        reqs.insert("version".into(), "v2".into());

        let resolver = VariableResolver::new()
            .with_globals(globals)
            .with_environment(envs)
            .with_request(reqs);

        let url = resolver.interpolate("https://{{host}}/{{version}}/users");
        // Environment overrides global ("api.staging.com"), request overrides global ("v2")
        assert_eq!(url, "https://api.staging.com/v2/users");
    }

    #[test]
    fn test_dynamic_variables() {
        let resolver = VariableResolver::new();
        let interpolated = resolver.interpolate("ID: {{$guid}}, Time: {{$timestamp}}");
        assert!(interpolated.starts_with("ID: "));
        assert!(interpolated.contains(", Time: "));
    }

    #[test]
    fn test_sensitive_header_redaction() {
        assert_eq!(
            redact_sensitive_header("Authorization", "Bearer secret_123"),
            "[REDACTED]"
        );
        assert_eq!(
            redact_sensitive_header("Content-Type", "application/json"),
            "application/json"
        );
    }
}
