//! Editor-facing variable annotations, hover details, and navigation actions.
//!
//! This module deliberately exposes only display-safe values. Raw resolved
//! values remain inside `ps-variable` and protocol execution paths.

use std::ops::Range;

use ps_domain::{ResourceId, VariableScope};
use ps_variable::{
    is_valid_variable_name, parse_template, scope_label, ResolutionProvenance, VariableDefinition,
    VariableDiagnostic, VariableResolver, VariableSource, SECRET_MASK,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableHighlightState {
    Resolved,
    Dynamic,
    Secret,
    Unresolved,
    Invalid,
}

/// Inline decoration data consumed by URL, header, and body editors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableHighlight {
    pub name: String,
    pub range: Range<usize>,
    pub state: VariableHighlightState,
    pub accessibility_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableEditorAnalysis {
    pub highlights: Vec<VariableHighlight>,
    pub diagnostics: Vec<VariableDiagnostic>,
}

/// Compact hover content shown for a template reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableHoverInfo {
    pub name: String,
    pub display_value: String,
    pub source_label: String,
    pub scope_label: String,
    pub is_secret: bool,
    pub can_jump_to_definition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDefinitionTarget {
    pub name: String,
    pub scope: VariableScope,
    pub source_label: String,
    pub resource_id: Option<ResourceId>,
}

/// A searchable text field belonging to a request, collection, or environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableTextResource {
    pub resource_id: ResourceId,
    pub resource_label: String,
    pub field: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableUsage {
    pub resource_id: ResourceId,
    pub resource_label: String,
    pub field: String,
    pub range: Range<usize>,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VariableUiError {
    #[error("'{0}' is not a valid variable name")]
    InvalidName(String),
    #[error("Variable '{0}' is already resolved; create-from-unresolved was not applied")]
    AlreadyResolved(String),
}

#[derive(Debug, Clone, Default)]
pub struct VariableUiController {
    resolver: VariableResolver,
}

impl VariableUiController {
    pub fn new(resolver: VariableResolver) -> Self {
        Self { resolver }
    }

    pub fn resolver(&self) -> &VariableResolver {
        &self.resolver
    }

    pub fn resolver_mut(&mut self) -> &mut VariableResolver {
        &mut self.resolver
    }

    /// Builds calm, semantic editor decorations. Escaped references are literals
    /// and are intentionally omitted from highlighting and navigation.
    pub fn analyze(&self, text: &str) -> VariableEditorAnalysis {
        let parsed = parse_template(text);
        let highlights = parsed
            .references
            .iter()
            .filter(|reference| !reference.escaped)
            .map(|reference| {
                let (state, accessibility_label) = if !reference.valid {
                    (
                        VariableHighlightState::Invalid,
                        format!("Invalid variable reference {}", reference.name),
                    )
                } else {
                    match self.resolver.resolve(&reference.name) {
                        Ok(resolved) if resolved.is_secret => (
                            VariableHighlightState::Secret,
                            format!(
                                "Secret variable {}, value hidden, from {}",
                                reference.name, resolved.provenance.source.label
                            ),
                        ),
                        Ok(resolved)
                            if resolved.provenance.source.scope == VariableScope::BuiltIn =>
                        {
                            (
                                VariableHighlightState::Dynamic,
                                format!("Dynamic variable {}", reference.name),
                            )
                        }
                        Ok(resolved) => (
                            VariableHighlightState::Resolved,
                            format!(
                                "Resolved variable {}, from {}",
                                reference.name, resolved.provenance.source.label
                            ),
                        ),
                        Err(_) => (
                            VariableHighlightState::Unresolved,
                            format!("Unresolved variable {}", reference.name),
                        ),
                    }
                };
                VariableHighlight {
                    name: reference.name.clone(),
                    range: reference.range.clone(),
                    state,
                    accessibility_label,
                }
            })
            .collect();

        let diagnostics = self.resolver.resolve_template(text).diagnostics;
        VariableEditorAnalysis {
            highlights,
            diagnostics,
        }
    }

    pub fn hover_at(&self, text: &str, byte_offset: usize) -> Option<VariableHoverInfo> {
        let parsed = parse_template(text);
        let reference = parsed.references.iter().find(|reference| {
            !reference.escaped
                && reference.range.start <= byte_offset
                && byte_offset < reference.range.end
        })?;

        match self.resolver.resolve(&reference.name) {
            Ok(resolved) => Some(VariableHoverInfo {
                name: reference.name.clone(),
                display_value: if resolved.is_secret {
                    SECRET_MASK.into()
                } else {
                    resolved.display_value
                },
                source_label: resolved.provenance.source.label.clone(),
                scope_label: scope_label(resolved.provenance.source.scope).into(),
                is_secret: resolved.is_secret,
                can_jump_to_definition: jump_target(&resolved.provenance).is_some(),
            }),
            Err(_) => Some(VariableHoverInfo {
                name: reference.name.clone(),
                display_value: "Unresolved".into(),
                source_label: "No matching definition".into(),
                scope_label: "Unresolved".into(),
                is_secret: false,
                can_jump_to_definition: false,
            }),
        }
    }

    pub fn jump_to_variable(
        &self,
        text: &str,
        byte_offset: usize,
    ) -> Option<VariableDefinitionTarget> {
        let reference = parse_template(text)
            .references
            .into_iter()
            .find(|reference| {
                !reference.escaped
                    && reference.range.start <= byte_offset
                    && byte_offset < reference.range.end
            })?;
        let resolved = self.resolver.resolve(&reference.name).ok()?;
        jump_target(&resolved.provenance)
    }

    pub fn create_from_unresolved(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        source: VariableSource,
        is_secret: bool,
    ) -> Result<(), VariableUiError> {
        let name = name.into();
        if !is_valid_variable_name(&name) {
            return Err(VariableUiError::InvalidName(name));
        }
        if self.resolver.resolve(&name).is_ok() {
            return Err(VariableUiError::AlreadyResolved(name));
        }
        let scope = source.scope;
        let mut definition = VariableDefinition::new(name, value, source);
        definition.is_secret = is_secret;
        self.resolver.insert(scope, definition);
        Ok(())
    }

    pub fn find_usages(
        &self,
        name: &str,
        resources: &[VariableTextResource],
    ) -> Vec<VariableUsage> {
        let mut usages: Vec<_> = resources
            .iter()
            .flat_map(|resource| {
                parse_template(&resource.text)
                    .references
                    .into_iter()
                    .filter(|reference| !reference.escaped && reference.name == name)
                    .map(|reference| VariableUsage {
                        resource_id: resource.resource_id,
                        resource_label: resource.resource_label.clone(),
                        field: resource.field.clone(),
                        range: reference.range,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        usages.sort_by(|a, b| {
            a.resource_label
                .cmp(&b.resource_label)
                .then_with(|| a.field.cmp(&b.field))
                .then_with(|| a.range.start.cmp(&b.range.start))
        });
        usages
    }
}

fn jump_target(provenance: &ResolutionProvenance) -> Option<VariableDefinitionTarget> {
    if provenance.source.scope == VariableScope::BuiltIn {
        return None;
    }
    Some(VariableDefinitionTarget {
        name: provenance.definition_name.clone(),
        scope: provenance.source.scope,
        source_label: provenance.source.label.clone(),
        resource_id: provenance.source.resource_id,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn controller() -> VariableUiController {
        VariableUiController::new(
            VariableResolver::new()
                .with_globals(HashMap::from([("host".into(), "api.test".into())]))
                .with_vault(HashMap::from([("token".into(), "secret-value".into())])),
        )
    }

    #[test]
    fn analysis_distinguishes_resolved_secret_dynamic_and_unresolved_tokens() {
        let analysis =
            controller().analyze("{{host}} {{vault:token}} {{$uuid}} {{missing}} \\{{literal}}");
        assert_eq!(analysis.highlights.len(), 4);
        assert_eq!(
            analysis.highlights[0].state,
            VariableHighlightState::Resolved
        );
        assert_eq!(analysis.highlights[1].state, VariableHighlightState::Secret);
        assert_eq!(
            analysis.highlights[2].state,
            VariableHighlightState::Dynamic
        );
        assert_eq!(
            analysis.highlights[3].state,
            VariableHighlightState::Unresolved
        );
    }

    #[test]
    fn secret_hover_never_contains_the_raw_value() {
        let hover = controller().hover_at("Bearer {{vault:token}}", 12).unwrap();
        assert!(hover.is_secret);
        assert_eq!(hover.display_value, SECRET_MASK);
        assert!(!format!("{hover:?}").contains("secret-value"));
    }

    #[test]
    fn create_jump_and_find_usages_form_one_editor_workflow() {
        let resource_id = ResourceId::new();
        let mut controller = controller();
        controller
            .create_from_unresolved(
                "account_id",
                "42",
                VariableSource::new(VariableScope::Request, "Get account")
                    .with_resource_id(resource_id),
                false,
            )
            .unwrap();

        let target = controller
            .jump_to_variable("/accounts/{{account_id}}", 15)
            .unwrap();
        assert_eq!(target.resource_id, Some(resource_id));

        let usages = controller.find_usages(
            "account_id",
            &[VariableTextResource {
                resource_id,
                resource_label: "Get account".into(),
                field: "URL".into(),
                text: "/accounts/{{account_id}}?owner={{account_id}}".into(),
            }],
        );
        assert_eq!(usages.len(), 2);
    }
}
