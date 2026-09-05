//! Protocol-independent template parsing and variable resolution.
//!
//! The resolver keeps raw values separate from display-safe values, records the
//! source of every substitution, and leaves unresolved templates intact. This
//! lets desktop, CLI, and protocol crates share identical resolution behavior
//! without leaking vault-backed or nested secret values into diagnostics.

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use ps_domain::ResourceId;
pub use ps_domain::VariableScope;
use serde_json::Value;
use uuid::Uuid;

/// Text used anywhere a secret value would otherwise be displayed.
pub const SECRET_MASK: &str = "••••••";
const DEFAULT_MAX_DEPTH: usize = 32;

/// A location capable of owning a variable definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSource {
    pub scope: VariableScope,
    pub resource_id: Option<ResourceId>,
    pub label: String,
}

impl VariableSource {
    pub fn new(scope: VariableScope, label: impl Into<String>) -> Self {
        Self {
            scope,
            resource_id: None,
            label: label.into(),
        }
    }

    pub fn with_resource_id(mut self, resource_id: ResourceId) -> Self {
        self.resource_id = Some(resource_id);
        self
    }
}

/// A value available to the resolution engine.
#[derive(Clone, PartialEq, Eq)]
pub struct VariableDefinition {
    pub name: String,
    pub value: String,
    pub is_secret: bool,
    pub enabled: bool,
    pub source: VariableSource,
}

impl fmt::Debug for VariableDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableDefinition")
            .field("name", &self.name)
            .field(
                "value",
                &if self.is_secret {
                    SECRET_MASK
                } else {
                    &self.value
                },
            )
            .field("is_secret", &self.is_secret)
            .field("enabled", &self.enabled)
            .field("source", &self.source)
            .finish()
    }
}

impl VariableDefinition {
    pub fn new(name: impl Into<String>, value: impl Into<String>, source: VariableSource) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            is_secret: false,
            enabled: true,
            source,
        }
    }

    pub fn secret(mut self) -> Self {
        self.is_secret = true;
        self
    }
}

/// One template reference, using byte offsets into its source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableReference {
    pub name: String,
    pub range: Range<usize>,
    pub escaped: bool,
    pub valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableDiagnosticKind {
    InvalidName,
    UnterminatedReference,
    UnresolvedReference,
    CyclicReference,
    MaximumDepthExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDiagnostic {
    pub kind: VariableDiagnosticKind,
    pub range: Range<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedTemplate {
    pub references: Vec<VariableReference>,
    pub diagnostics: Vec<VariableDiagnostic>,
}

/// Parse `{{name}}` references without interpreting their values.
pub fn parse_template(input: &str) -> ParsedTemplate {
    let bytes = input.as_bytes();
    let mut parsed = ParsedTemplate::default();
    let mut index = 0;

    while index + 1 < bytes.len() {
        if bytes[index] != b'{' || bytes[index + 1] != b'{' {
            index += 1;
            continue;
        }

        let mut slash_count = 0;
        let mut before = index;
        while before > 0 && bytes[before - 1] == b'\\' {
            slash_count += 1;
            before -= 1;
        }
        let escaped = slash_count % 2 == 1;
        let reference_start = index;
        let content_start = index + 2;
        let mut close = content_start;
        while close + 1 < bytes.len() && !(bytes[close] == b'}' && bytes[close + 1] == b'}') {
            close += 1;
        }

        if close + 1 >= bytes.len() {
            parsed.diagnostics.push(VariableDiagnostic {
                kind: VariableDiagnosticKind::UnterminatedReference,
                range: reference_start..bytes.len(),
                message: "Variable reference is missing closing braces".into(),
            });
            break;
        }

        let end = close + 2;
        let name = input[content_start..close].trim().to_string();
        let valid = is_valid_variable_name(&name);
        if !valid && !escaped {
            parsed.diagnostics.push(VariableDiagnostic {
                kind: VariableDiagnosticKind::InvalidName,
                range: reference_start..end,
                message: if name.is_empty() {
                    "Variable name cannot be empty".into()
                } else {
                    format!("Variable name '{name}' contains unsupported characters")
                },
            });
        }
        parsed.references.push(VariableReference {
            name,
            range: reference_start..end,
            escaped,
            valid,
        });
        index = end;
    }

    parsed
}

/// Names support Unicode letters/numbers plus separators used by dynamic,
/// vault, and nested-object references.
pub fn is_valid_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphanumeric() || matches!(first, '_' | '$')) {
        return false;
    }
    chars.all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '$'))
}

/// Provenance returned for resolved variables and editor hover information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionProvenance {
    pub source: VariableSource,
    pub definition_name: String,
    pub object_path: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ResolvedVariable {
    pub name: String,
    pub value: String,
    pub display_value: String,
    pub is_secret: bool,
    pub provenance: ResolutionProvenance,
}

impl fmt::Debug for ResolvedVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedVariable")
            .field("name", &self.name)
            .field("display_value", &self.display_value)
            .field("is_secret", &self.is_secret)
            .field("provenance", &self.provenance)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionOccurrence {
    pub reference: VariableReference,
    pub output_range: Range<usize>,
    pub resolved: Option<ResolvedVariable>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ResolutionResult {
    /// Value intended for protocol execution. This may contain secrets.
    pub value: String,
    /// Safe representation intended for logs, previews, and event payloads.
    pub display_value: String,
    pub occurrences: Vec<ResolutionOccurrence>,
    pub diagnostics: Vec<VariableDiagnostic>,
}

impl fmt::Debug for ResolutionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolutionResult")
            .field("display_value", &self.display_value)
            .field("occurrences", &self.occurrences)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl ResolutionResult {
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableResolveError {
    Unresolved(String),
    Cycle(Vec<String>),
    MaximumDepth(usize),
}

impl fmt::Display for VariableResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unresolved(name) => write!(f, "Variable '{name}' is unresolved"),
            Self::Cycle(chain) => write!(f, "Cyclic variable reference: {}", chain.join(" -> ")),
            Self::MaximumDepth(depth) => {
                write!(
                    f,
                    "Variable resolution exceeded the maximum depth of {depth}"
                )
            }
        }
    }
}

/// Result supplied by a dynamic variable provider.
#[derive(Clone, PartialEq, Eq)]
pub struct GeneratedVariable {
    pub value: String,
    pub is_secret: bool,
}

impl fmt::Debug for GeneratedVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GeneratedVariable")
            .field(
                "value",
                &if self.is_secret {
                    SECRET_MASK
                } else {
                    &self.value
                },
            )
            .field("is_secret", &self.is_secret)
            .finish()
    }
}

impl GeneratedVariable {
    pub fn public(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            is_secret: false,
        }
    }
}

/// Extension point for built-in and plugin-provided dynamic variables.
pub trait DynamicVariableProvider: Send + Sync + fmt::Debug {
    fn generate(&self, name: &str) -> Option<GeneratedVariable>;
}

#[derive(Debug, Default)]
pub struct BuiltInDynamicVariables;

impl DynamicVariableProvider for BuiltInDynamicVariables {
    fn generate(&self, name: &str) -> Option<GeneratedVariable> {
        let random = Uuid::new_v4();
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).ok()?;
        let random_u64 = u64::from_be_bytes(bytes[0..8].try_into().expect("UUID prefix"));
        let value = match name {
            "$guid" | "$uuid" => random.to_string(),
            "$timestamp" => Utc::now().timestamp().to_string(),
            "$isoTimestamp" => Utc::now().to_rfc3339(),
            "$randomInt" => (random_u64 % 100_000).to_string(),
            "$randomString" => random.simple().to_string()[..12].to_string(),
            "$randomEmail" => format!("user-{}@example.test", &random.simple().to_string()[..12]),
            "$randomIp" => format!(
                "{}.{}.{}.{}",
                bytes[0].max(1),
                bytes[1],
                bytes[2],
                bytes[3].max(1)
            ),
            "$randomDate" => {
                let start = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
                    .expect("valid built-in date")
                    .with_timezone(&Utc);
                let days = (random_u64 % 18_263) as i64;
                (start + Duration::days(days)).date_naive().to_string()
            }
            "$randomBytes" => bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
            _ => return None,
        };
        Some(GeneratedVariable::public(value))
    }
}

/// Hierarchical variable resolver. Narrower scopes always win.
#[derive(Debug, Clone)]
pub struct VariableResolver {
    scopes: HashMap<VariableScope, HashMap<String, VariableDefinition>>,
    dynamic_providers: Vec<Arc<dyn DynamicVariableProvider>>,
    max_depth: usize,
}

impl Default for VariableResolver {
    fn default() -> Self {
        Self {
            scopes: HashMap::new(),
            dynamic_providers: vec![Arc::new(BuiltInDynamicVariables)],
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

impl VariableResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_globals(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Global, variables)
    }

    pub fn with_environment(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Environment, variables)
    }

    pub fn with_collection(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Collection, variables)
    }

    pub fn with_folder(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Folder, variables)
    }

    pub fn with_request(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Request, variables)
    }

    pub fn with_iteration(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Iteration, variables)
    }

    pub fn with_temporary(self, variables: HashMap<String, String>) -> Self {
        self.with_values(VariableScope::Ephemeral, variables)
    }

    pub fn with_vault(self, variables: HashMap<String, String>) -> Self {
        let definitions = variables
            .into_iter()
            .map(|(name, value)| {
                VariableDefinition::new(
                    name,
                    value,
                    VariableSource::new(VariableScope::Vault, "Vault"),
                )
                .secret()
            })
            .collect();
        self.with_definitions(VariableScope::Vault, definitions)
    }

    pub fn with_values(self, scope: VariableScope, variables: HashMap<String, String>) -> Self {
        let definitions = variables
            .into_iter()
            .map(|(name, value)| {
                VariableDefinition::new(name, value, VariableSource::new(scope, scope_label(scope)))
            })
            .collect();
        self.with_definitions(scope, definitions)
    }

    pub fn with_definitions(
        mut self,
        scope: VariableScope,
        definitions: Vec<VariableDefinition>,
    ) -> Self {
        for definition in definitions {
            self.insert(scope, definition);
        }
        self
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth.max(1);
        self
    }

    pub fn register_dynamic_provider(&mut self, provider: Arc<dyn DynamicVariableProvider>) {
        self.dynamic_providers.insert(0, provider);
    }

    pub fn insert(&mut self, scope: VariableScope, mut definition: VariableDefinition) {
        definition.source.scope = scope;
        self.scopes
            .entry(scope)
            .or_default()
            .insert(definition.name.clone(), definition);
    }

    /// Clear one scope without disturbing globals, request variables, or providers.
    pub fn clear_scope(&mut self, scope: VariableScope) {
        self.scopes.remove(&scope);
    }

    pub fn remove(&mut self, scope: VariableScope, name: &str) -> Option<VariableDefinition> {
        self.scopes.get_mut(&scope)?.remove(name)
    }

    pub fn resolve_var(&self, name: &str) -> Option<String> {
        self.resolve(name).ok().map(|resolved| resolved.value)
    }

    pub fn resolve(&self, name: &str) -> Result<ResolvedVariable, VariableResolveError> {
        self.resolve_name(name, &mut Vec::new(), 0)
    }

    pub fn interpolate(&self, template: &str) -> String {
        self.resolve_template(template).value
    }

    pub fn resolve_template(&self, template: &str) -> ResolutionResult {
        self.resolve_template_inner(template, &mut Vec::new(), 0)
    }

    fn resolve_template_inner(
        &self,
        template: &str,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> ResolutionResult {
        let parsed = parse_template(template);
        let mut value = String::with_capacity(template.len());
        let mut display_value = String::with_capacity(template.len());
        let mut diagnostics = parsed.diagnostics;
        let mut occurrences = Vec::new();
        let mut cursor = 0;

        for reference in parsed.references {
            let escaped_prefix = reference.range.start.saturating_sub(1);
            if reference.escaped {
                value.push_str(&template[cursor..escaped_prefix]);
                display_value.push_str(&template[cursor..escaped_prefix]);
                let literal = &template[reference.range.clone()];
                let output_start = value.len();
                value.push_str(literal);
                display_value.push_str(literal);
                let output_end = value.len();
                occurrences.push(ResolutionOccurrence {
                    reference: reference.clone(),
                    output_range: output_start..output_end,
                    resolved: None,
                });
                cursor = reference.range.end;
                continue;
            }

            value.push_str(&template[cursor..reference.range.start]);
            display_value.push_str(&template[cursor..reference.range.start]);
            let output_start = value.len();

            if reference.valid {
                match self.resolve_name(&reference.name, stack, depth) {
                    Ok(resolved) => {
                        value.push_str(&resolved.value);
                        display_value.push_str(&resolved.display_value);
                        let output_end = value.len();
                        occurrences.push(ResolutionOccurrence {
                            reference: reference.clone(),
                            output_range: output_start..output_end,
                            resolved: Some(resolved),
                        });
                    }
                    Err(error) => {
                        let raw = &template[reference.range.clone()];
                        value.push_str(raw);
                        display_value.push_str(raw);
                        diagnostics.push(diagnostic_for_error(&reference, error));
                        occurrences.push(ResolutionOccurrence {
                            reference: reference.clone(),
                            output_range: output_start..value.len(),
                            resolved: None,
                        });
                    }
                }
            } else {
                let raw = &template[reference.range.clone()];
                value.push_str(raw);
                display_value.push_str(raw);
                occurrences.push(ResolutionOccurrence {
                    reference: reference.clone(),
                    output_range: output_start..value.len(),
                    resolved: None,
                });
            }
            cursor = reference.range.end;
        }

        value.push_str(&template[cursor..]);
        display_value.push_str(&template[cursor..]);
        ResolutionResult {
            value,
            display_value,
            occurrences,
            diagnostics,
        }
    }

    fn resolve_name(
        &self,
        name: &str,
        stack: &mut Vec<String>,
        depth: usize,
    ) -> Result<ResolvedVariable, VariableResolveError> {
        if depth >= self.max_depth {
            return Err(VariableResolveError::MaximumDepth(self.max_depth));
        }
        if let Some(cycle_start) = stack.iter().position(|entry| entry == name) {
            let mut cycle = stack[cycle_start..].to_vec();
            cycle.push(name.to_string());
            return Err(VariableResolveError::Cycle(cycle));
        }

        if name.starts_with('$') {
            for provider in &self.dynamic_providers {
                if let Some(generated) = provider.generate(name) {
                    return Ok(ResolvedVariable {
                        name: name.to_string(),
                        display_value: if generated.is_secret {
                            SECRET_MASK.into()
                        } else {
                            generated.value.clone()
                        },
                        value: generated.value,
                        is_secret: generated.is_secret,
                        provenance: ResolutionProvenance {
                            source: VariableSource::new(VariableScope::BuiltIn, "Dynamic variable"),
                            definition_name: name.to_string(),
                            object_path: None,
                        },
                    });
                }
            }
        }

        let (definition, object_path) = self
            .find_definition(name)
            .ok_or_else(|| VariableResolveError::Unresolved(name.to_string()))?;

        // Vault bytes are opaque credentials, never another template to evaluate.
        if definition.source.scope == VariableScope::Vault {
            return Ok(ResolvedVariable {
                name: name.to_string(),
                value: definition.value.clone(),
                display_value: SECRET_MASK.into(),
                is_secret: true,
                provenance: ResolutionProvenance {
                    source: definition.source.clone(),
                    definition_name: definition.name.clone(),
                    object_path: None,
                },
            });
        }

        stack.push(name.to_string());
        let nested = self.resolve_template_inner(&definition.value, stack, depth + 1);
        stack.pop();
        if let Some(diagnostic) = nested.diagnostics.first() {
            return Err(match diagnostic.kind {
                VariableDiagnosticKind::CyclicReference => {
                    VariableResolveError::Cycle(vec![name.to_string()])
                }
                VariableDiagnosticKind::MaximumDepthExceeded => {
                    VariableResolveError::MaximumDepth(self.max_depth)
                }
                _ => VariableResolveError::Unresolved(name.to_string()),
            });
        }

        let value = if let Some(path) = object_path.as_deref() {
            resolve_json_path(&nested.value, path)
                .ok_or_else(|| VariableResolveError::Unresolved(name.to_string()))?
        } else {
            nested.value
        };
        let nested_secret = nested
            .occurrences
            .iter()
            .filter_map(|occurrence| occurrence.resolved.as_ref())
            .any(|resolved| resolved.is_secret);
        let is_secret = definition.is_secret || nested_secret;
        Ok(ResolvedVariable {
            name: name.to_string(),
            display_value: if is_secret {
                SECRET_MASK.into()
            } else {
                value.clone()
            },
            value,
            is_secret,
            provenance: ResolutionProvenance {
                source: definition.source.clone(),
                definition_name: definition.name.clone(),
                object_path,
            },
        })
    }

    fn find_definition(&self, name: &str) -> Option<(&VariableDefinition, Option<String>)> {
        let normalized_name = name.strip_prefix("vault:").unwrap_or(name);
        if name.starts_with("vault:") {
            return self
                .scope_definition(VariableScope::Vault, normalized_name)
                .map(|definition| (definition, None));
        }

        for scope in scope_precedence() {
            if let Some(definition) = self.scope_definition(scope, name) {
                return Some((definition, None));
            }
        }

        let dot_positions: Vec<usize> = name.match_indices('.').map(|(index, _)| index).collect();
        for split in dot_positions.into_iter().rev() {
            let base = &name[..split];
            let path = &name[(split + 1)..];
            for scope in scope_precedence() {
                if let Some(definition) = self.scope_definition(scope, base) {
                    return Some((definition, Some(path.to_string())));
                }
            }
        }
        None
    }

    fn scope_definition(&self, scope: VariableScope, name: &str) -> Option<&VariableDefinition> {
        self.scopes
            .get(&scope)?
            .get(name)
            .filter(|definition| definition.enabled)
    }
}

fn scope_precedence() -> [VariableScope; 7] {
    [
        VariableScope::Ephemeral,
        VariableScope::Iteration,
        VariableScope::Request,
        VariableScope::Folder,
        VariableScope::Collection,
        VariableScope::Environment,
        VariableScope::Global,
    ]
}

pub fn scope_label(scope: VariableScope) -> &'static str {
    match scope {
        VariableScope::Global => "Workspace",
        VariableScope::Environment => "Environment",
        VariableScope::Collection => "Collection",
        VariableScope::Folder => "Folder",
        VariableScope::Request => "Request",
        VariableScope::Iteration => "Iteration data",
        VariableScope::Ephemeral => "Temporary",
        VariableScope::Vault => "Vault",
        VariableScope::BuiltIn => "Dynamic variable",
    }
}

fn resolve_json_path(json: &str, path: &str) -> Option<String> {
    let value: Value = serde_json::from_str(json).ok()?;
    let mut current = &value;
    for segment in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(match current {
        Value::String(value) => value.clone(),
        other => other.to_string(),
    })
}

fn diagnostic_for_error(
    reference: &VariableReference,
    error: VariableResolveError,
) -> VariableDiagnostic {
    let kind = match error {
        VariableResolveError::Unresolved(_) => VariableDiagnosticKind::UnresolvedReference,
        VariableResolveError::Cycle(_) => VariableDiagnosticKind::CyclicReference,
        VariableResolveError::MaximumDepth(_) => VariableDiagnosticKind::MaximumDepthExceeded,
    };
    VariableDiagnostic {
        kind,
        range: reference.range.clone(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_detects_valid_escaped_and_invalid_references() {
        let input = r"/{{user.id}}/\{{literal}}/{{bad name}}/{{unterminated";
        let parsed = parse_template(input);
        assert_eq!(parsed.references.len(), 3);
        assert_eq!(parsed.references[0].name, "user.id");
        assert!(parsed.references[1].escaped);
        assert!(!parsed.references[2].valid);
        assert_eq!(parsed.diagnostics.len(), 2);
    }

    #[test]
    fn scope_precedence_and_nested_values_are_deterministic() {
        let resolver = VariableResolver::new()
            .with_globals(HashMap::from([
                ("host".into(), "global.test".into()),
                ("url".into(), "https://{{host}}".into()),
            ]))
            .with_environment(HashMap::from([("host".into(), "staging.test".into())]))
            .with_request(HashMap::from([("version".into(), "v2".into())]));

        let result = resolver.resolve_template("{{url}}/{{version}}");
        assert_eq!(result.value, "https://staging.test/v2");
        assert!(result.is_complete());
        assert_eq!(
            result.occurrences[1]
                .resolved
                .as_ref()
                .unwrap()
                .provenance
                .source
                .scope,
            VariableScope::Request
        );
    }

    #[test]
    fn every_runtime_scope_participates_in_precedence() {
        let key = "candidate".to_string();
        let mut resolver = VariableResolver::new()
            .with_globals(HashMap::from([(key.clone(), "global".into())]))
            .with_environment(HashMap::from([(key.clone(), "environment".into())]))
            .with_collection(HashMap::from([(key.clone(), "collection".into())]))
            .with_folder(HashMap::from([(key.clone(), "folder".into())]))
            .with_request(HashMap::from([(key.clone(), "request".into())]))
            .with_iteration(HashMap::from([(key.clone(), "iteration".into())]))
            .with_temporary(HashMap::from([(key.clone(), "temporary".into())]));

        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("temporary"));
        resolver.remove(VariableScope::Ephemeral, &key);
        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("iteration"));
        resolver.remove(VariableScope::Iteration, &key);
        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("request"));
        resolver.remove(VariableScope::Request, &key);
        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("folder"));
        resolver.remove(VariableScope::Folder, &key);
        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("collection"));
        resolver.remove(VariableScope::Collection, &key);
        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("environment"));
        resolver.remove(VariableScope::Environment, &key);
        assert_eq!(resolver.resolve_var(&key).as_deref(), Some("global"));
    }

    #[test]
    fn unresolved_and_cyclic_references_remain_visible() {
        let resolver = VariableResolver::new().with_globals(HashMap::from([
            ("a".into(), "{{b}}".into()),
            ("b".into(), "{{a}}".into()),
        ]));
        let unresolved = resolver.resolve_template("/{{missing}}");
        assert_eq!(unresolved.value, "/{{missing}}");
        assert_eq!(
            unresolved.diagnostics[0].kind,
            VariableDiagnosticKind::UnresolvedReference
        );

        let cyclic = resolver.resolve_template("{{a}}");
        assert_eq!(cyclic.value, "{{a}}");
        assert_eq!(
            cyclic.diagnostics[0].kind,
            VariableDiagnosticKind::CyclicReference
        );
    }

    #[test]
    fn secret_values_are_separate_from_display_values() {
        let resolver = VariableResolver::new()
            .with_vault(HashMap::from([("token".into(), "super-secret".into())]));
        let result = resolver.resolve_template("Bearer {{vault:token}}");
        assert_eq!(result.value, "Bearer super-secret");
        assert_eq!(result.display_value, format!("Bearer {SECRET_MASK}"));
        assert!(result.occurrences[0].resolved.as_ref().unwrap().is_secret);
    }

    #[test]
    fn secret_taint_propagates_through_nested_variables() {
        let resolver = VariableResolver::new()
            .with_globals(HashMap::from([(
                "authorization".into(),
                "Bearer {{vault:token}}".into(),
            )]))
            .with_vault(HashMap::from([("token".into(), "super-secret".into())]));
        let result = resolver.resolve_template("{{authorization}}");
        assert_eq!(result.value, "Bearer super-secret");
        assert_eq!(result.display_value, SECRET_MASK);
        assert!(!format!("{result:?}").contains("super-secret"));
    }

    #[test]
    fn object_paths_and_escaped_templates_are_supported() {
        let resolver = VariableResolver::new().with_globals(HashMap::from([(
            "user".into(),
            r#"{"profile":{"id":42}}"#.into(),
        )]));
        let result = resolver.resolve_template(r"\{{literal}}/{{user.profile.id}}");
        assert_eq!(result.value, "{{literal}}/42");
    }

    #[test]
    fn all_dynamic_variables_resolve() {
        let resolver = VariableResolver::new();
        for name in [
            "$uuid",
            "$timestamp",
            "$isoTimestamp",
            "$randomInt",
            "$randomString",
            "$randomEmail",
            "$randomIp",
            "$randomDate",
            "$randomBytes",
        ] {
            assert!(resolver.resolve(name).is_ok(), "{name} should resolve");
        }
    }

    #[derive(Debug)]
    struct CustomDynamicProvider;

    impl DynamicVariableProvider for CustomDynamicProvider {
        fn generate(&self, name: &str) -> Option<GeneratedVariable> {
            (name == "$ticket").then(|| GeneratedVariable::public("PS-42"))
        }
    }

    #[test]
    fn custom_dynamic_provider_extends_the_registry() {
        let mut resolver = VariableResolver::new();
        resolver.register_dynamic_provider(Arc::new(CustomDynamicProvider));
        assert_eq!(resolver.resolve_var("$ticket").as_deref(), Some("PS-42"));
    }
}
