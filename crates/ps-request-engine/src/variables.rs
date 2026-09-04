//! Variable interpolation and template resolution engine for PacketSmith.
//!
//! Evaluates `{{variable}}` placeholders across hierarchical variable scopes:
//! Request -> Folder -> Collection -> Environment -> Global.
//! Also evaluates dynamic system variables like `{{$guid}}` and `{{$timestamp}}`.

use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

/// Hierarchical variable resolver.
#[derive(Debug, Clone, Default)]
pub struct VariableResolver {
    global_vars: HashMap<String, String>,
    env_vars: HashMap<String, String>,
    collection_vars: HashMap<String, String>,
    folder_vars: HashMap<String, String>,
    request_vars: HashMap<String, String>,
    vault_secrets: HashMap<String, String>,
}

impl VariableResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_globals(mut self, vars: HashMap<String, String>) -> Self {
        self.global_vars = vars;
        self
    }

    pub fn with_environment(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars = vars;
        self
    }

    pub fn with_collection(mut self, vars: HashMap<String, String>) -> Self {
        self.collection_vars = vars;
        self
    }

    pub fn with_folder(mut self, vars: HashMap<String, String>) -> Self {
        self.folder_vars = vars;
        self
    }

    pub fn with_request(mut self, vars: HashMap<String, String>) -> Self {
        self.request_vars = vars;
        self
    }

    pub fn with_vault(mut self, secrets: HashMap<String, String>) -> Self {
        self.vault_secrets = secrets;
        self
    }

    /// Resolves a single variable name adhering to scope precedence:
    /// Request > Folder > Collection > Environment > Global.
    pub fn resolve_var(&self, name: &str) -> Option<String> {
        // 1. Dynamic system variables
        if let Some(val) = Self::resolve_dynamic(name) {
            return Some(val);
        }

        // 2. Vault secrets (prefixed with "vault:")
        if let Some(secret_key) = name.strip_prefix("vault:") {
            if let Some(val) = self.vault_secrets.get(secret_key) {
                return Some(val.clone());
            }
        }

        // 3. Hierarchical user scopes
        if let Some(val) = self.request_vars.get(name) {
            return Some(val.clone());
        }
        if let Some(val) = self.folder_vars.get(name) {
            return Some(val.clone());
        }
        if let Some(val) = self.collection_vars.get(name) {
            return Some(val.clone());
        }
        if let Some(val) = self.env_vars.get(name) {
            return Some(val.clone());
        }
        if let Some(val) = self.global_vars.get(name) {
            return Some(val.clone());
        }

        None
    }

    /// Interpolates all occurrences of `{{var_name}}` within the given template string.
    pub fn interpolate(&self, template: &str) -> String {
        let mut result = String::with_capacity(template.len());
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                let mut var_name = String::new();
                let mut closed = false;

                while let Some(inner) = chars.next() {
                    if inner == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        closed = true;
                        break;
                    }
                    var_name.push(inner);
                }

                if closed {
                    let trimmed = var_name.trim();
                    if let Some(replacement) = self.resolve_var(trimmed) {
                        result.push_str(&replacement);
                    } else {
                        // Preserve original placeholder if unresolved
                        result.push_str("{{");
                        result.push_str(&var_name);
                        result.push_str("}}");
                    }
                } else {
                    result.push_str("{{");
                    result.push_str(&var_name);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Handles dynamic build-in variables like `{{$guid}}` or `{{$timestamp}}`.
    fn resolve_dynamic(name: &str) -> Option<String> {
        match name {
            "$guid" => Some(Uuid::new_v4().to_string()),
            "$timestamp" => Some(Utc::now().timestamp().to_string()),
            "$isoTimestamp" => Some(Utc::now().to_rfc3339()),
            "$randomInt" => {
                // Pseudo-random 1..1000 based on time nanos
                let nanos = Utc::now().timestamp_subsec_nanos();
                Some(((nanos % 1000) + 1).to_string())
            }
            _ => None,
        }
    }
}

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
        assert_eq!(redact_sensitive_header("Authorization", "Bearer secret_123"), "[REDACTED]");
        assert_eq!(redact_sensitive_header("Content-Type", "application/json"), "application/json");
    }
}
