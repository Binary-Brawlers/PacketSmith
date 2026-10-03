//! Safe file-backed request editing, independent of the native view.
use ps_domain::{AuthConfig, HttpRequestPayload, ProtocolRequest, RequestDocument, ResourceId};
use ps_workspace::{CollectionManager, WorkspaceError};

#[derive(Clone, PartialEq, Eq)]
pub struct RequestEdit {
    pub http: HttpRequestPayload,
    pub auth: AuthConfig,
}

impl std::fmt::Debug for RequestEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestEdit").finish_non_exhaustive()
    }
}

pub fn reference_input(value: &str) -> String {
    if value.contains("{{") {
        value.into()
    } else {
        format!("{{{{vault:{value}}}}}")
    }
}

/// Display saved names as references, while preserving the original config when
/// the controls are unchanged (including the provider's named-variable fallback).
pub fn auth_for_editor(source: &AuthConfig) -> AuthConfig {
    match source {
        AuthConfig::Bearer { token_secret_ref } => AuthConfig::Bearer {
            token_secret_ref: reference_input(token_secret_ref),
        },
        AuthConfig::Basic {
            username,
            password_secret_ref,
        } => AuthConfig::Basic {
            username: username.clone(),
            password_secret_ref: password_secret_ref.as_deref().map(reference_input),
        },
        AuthConfig::ApiKey {
            key,
            value_secret_ref,
            location,
        } => AuthConfig::ApiKey {
            key: key.clone(),
            value_secret_ref: reference_input(value_secret_ref),
            location: *location,
        },
        _ => source.clone(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RequestSaveError {
    #[error("Use variable references in credential fields and sensitive headers before saving.")]
    PlaintextCredential,
    #[error("This request changed on disk. Reopen it before saving; your draft is retained.")]
    Conflict,
    #[error("Enter a request name before saving.")]
    MissingName,
    #[error("Could not save the request. Check the workspace and file permissions.")]
    Workspace(#[from] WorkspaceError),
}

/// Accept only an entire valid, unescaped reference, never surrounding literals.
pub fn is_reference(value: &str) -> bool {
    let value = value.trim();
    let parsed = ps_variable::parse_template(value);
    parsed.diagnostics.is_empty()
        && parsed.references.len() == 1
        && parsed.references[0].valid
        && !parsed.references[0].escaped
        && parsed.references[0].range == (0..value.len())
}

impl RequestEdit {
    pub fn validate_for_source(&self, source: &RequestDocument) -> Result<(), RequestSaveError> {
        self.validate()?;
        if self.auth == source.auth {
            return Ok(());
        }
        let safe = match &self.auth {
            AuthConfig::Bearer { token_secret_ref } => is_reference(token_secret_ref),
            AuthConfig::Basic {
                password_secret_ref,
                ..
            } => password_secret_ref
                .as_deref()
                .is_none_or(|value| value.is_empty() || is_reference(value)),
            AuthConfig::ApiKey {
                value_secret_ref, ..
            } => is_reference(value_secret_ref),
            AuthConfig::None | AuthConfig::Inherit => true,
            AuthConfig::OAuth2 {
                token_secret_ref, ..
            } => token_secret_ref.as_deref().is_none_or(is_reference),
        };
        if safe {
            Ok(())
        } else {
            Err(RequestSaveError::PlaintextCredential)
        }
    }

    /// Known sensitive headers must contain references. Authorization may keep
    /// its public scheme; disabled rows receive the same protection as enabled ones.
    pub fn validate(&self) -> Result<(), RequestSaveError> {
        for header in &self.http.headers {
            let sensitive = header.is_secret
                || ps_request_engine::redact_sensitive_header(&header.name, "value")
                    == "[REDACTED]";
            let value = header.value.trim();
            let reference = is_reference(value)
                || (header.name.eq_ignore_ascii_case("authorization")
                    && value.split_once(' ').is_some_and(|(scheme, credential)| {
                        (scheme.eq_ignore_ascii_case("bearer")
                            || scheme.eq_ignore_ascii_case("basic"))
                            && is_reference(credential)
                    }));
            if sensitive && !value.is_empty() && !reference {
                return Err(RequestSaveError::PlaintextCredential);
            }
        }
        Ok(())
    }

    /// Preserve IDs, scripts, settings, examples, tags and timestamps from the source.
    pub fn apply(&self, source: &RequestDocument) -> RequestDocument {
        let mut document = source.clone();
        document.protocol = ProtocolRequest::Http(self.http.clone());
        document.auth = self.auth.clone();
        document
    }
}

/// Compare the full on-disk document to the version opened by the editor before
/// overwriting it. Deleted or malformed files also leave the draft intact.
pub fn save_existing(
    manager: &mut CollectionManager,
    source: &RequestDocument,
    edit: &RequestEdit,
) -> Result<RequestDocument, RequestSaveError> {
    edit.validate_for_source(source)?;
    let path = manager.get_full_path(&source.id)?;
    let contents = std::fs::read_to_string(path).map_err(|_| RequestSaveError::Conflict)?;
    let current: RequestDocument = ps_workspace::parse_resource_from_yaml(&contents)
        .map_err(|_| RequestSaveError::Conflict)?;
    if current != *source {
        return Err(RequestSaveError::Conflict);
    }
    manager.save_request(edit.apply(source))?;
    Ok(manager.requests()[&source.id].clone())
}

pub fn save_new(
    manager: &mut CollectionManager,
    collection_id: ResourceId,
    source: &RequestDocument,
    name: &str,
    edit: &RequestEdit,
) -> Result<RequestDocument, RequestSaveError> {
    edit.validate_for_source(source)?;
    if name.trim().is_empty() {
        return Err(RequestSaveError::MissingName);
    }
    let mut document = edit.apply(source);
    document.name = name.trim().into();
    Ok(manager.create_request_document(collection_id, None, document)?)
}

/// Raw credentials are allowed for session-only sends. Resolve templates before
/// Basic encoding, and store literal bytes in a private copy of the resolver.
pub fn prepare_session_auth(
    mut auth: AuthConfig,
    mut resolver: ps_variable::VariableResolver,
) -> Result<(AuthConfig, ps_variable::VariableResolver), ps_http::auth::AuthError> {
    let credential = match &mut auth {
        AuthConfig::Bearer { token_secret_ref } => Some(token_secret_ref),
        AuthConfig::Basic {
            password_secret_ref,
            ..
        } => password_secret_ref.as_mut(),
        AuthConfig::ApiKey {
            value_secret_ref, ..
        } => Some(value_secret_ref),
        _ => None,
    };
    if let Some(value) = credential {
        if !is_reference(value) {
            let resolved = resolver.resolve_template(value);
            if !resolved.diagnostics.is_empty() {
                return Err(ps_http::auth::AuthError::Unresolved);
            }
            let name = format!("session_{}", ResourceId::new());
            resolver.insert(
                ps_domain::VariableScope::Vault,
                ps_variable::VariableDefinition::new(
                    &name,
                    resolved.value,
                    ps_variable::VariableSource::new(
                        ps_domain::VariableScope::Vault,
                        "Session credential",
                    ),
                )
                .secret(),
            );
            *value = name;
        }
    }
    Ok((auth, resolver))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_domain::{HeaderEntry, HttpBody, QueryParam};

    struct Workspace(std::path::PathBuf);
    impl Workspace {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("ps-request-save-{}", ResourceId::new())))
        }
    }
    impl Drop for Workspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn edit() -> RequestEdit {
        RequestEdit {
            http: HttpRequestPayload {
                method: "POST".into(),
                url: "{{host}}/items?item=1&item=2".into(),
                params: vec![
                    QueryParam {
                        key: "item".into(),
                        value: "1".into(),
                        enabled: true,
                        description: Some("First item".into()),
                    },
                    QueryParam {
                        key: "item".into(),
                        value: "2".into(),
                        enabled: true,
                        description: None,
                    },
                    QueryParam {
                        key: "disabled".into(),
                        value: "3".into(),
                        enabled: false,
                        description: None,
                    },
                ],
                headers: vec![
                    HeaderEntry {
                        name: "Accept".into(),
                        value: "application/json".into(),
                        enabled: true,
                        is_secret: false,
                        description: Some("Response format".into()),
                    },
                    HeaderEntry {
                        name: "Authorization".into(),
                        value: "Bearer {{token}}".into(),
                        enabled: false,
                        is_secret: true,
                        description: None,
                    },
                ],
                body: HttpBody::Json {
                    json_content: "{\"name\":\"{{name}}\"}".into(),
                },
            },
            auth: AuthConfig::Basic {
                username: "{{username}}".into(),
                password_secret_ref: Some("{{password}}".into()),
            },
        }
    }

    #[test]
    fn save_open_edit_round_trip_preserves_identity_metadata_and_rows() {
        let workspace = Workspace::new();
        let mut manager = CollectionManager::scan(&workspace.0).unwrap();
        let collection = manager.create_collection("Examples", None).unwrap();
        let mut source = RequestDocument::new(
            "Draft",
            ProtocolRequest::Http(HttpRequestPayload::new("GET", "")),
        );
        source.tags.push("important".into());
        source.description = Some("Keep this description".into());
        source.scripts.pre_request = Some("// retain script".into());
        source.settings.timeout_ms = 1234;
        let mut edit = edit();
        let saved = save_new(&mut manager, collection.id, &source, "Create item", &edit).unwrap();
        let rescanned = CollectionManager::scan(&workspace.0).unwrap();
        assert_eq!(rescanned.requests()[&source.id], saved);
        assert_eq!(saved.protocol, ProtocolRequest::Http(edit.http.clone()));
        assert_eq!(saved.auth, edit.auth);
        assert_eq!(saved.description, source.description);
        assert_eq!(saved.tags, source.tags);
        assert_eq!(saved.scripts, source.scripts);
        assert_eq!(saved.settings, source.settings);
        assert_eq!(saved.created_at, source.created_at);
        let path = manager.get_full_path(&source.id).unwrap();
        edit.http.method = "PUT".into();
        edit.http.body = HttpBody::Raw {
            content: "<item/>".into(),
            content_type: "application/xml".into(),
        };
        let updated = save_existing(&mut manager, &saved, &edit).unwrap();
        assert_eq!(manager.get_full_path(&source.id).unwrap(), path);
        assert_eq!(updated.id, source.id);
        assert_eq!(
            CollectionManager::scan(&workspace.0).unwrap().requests()[&source.id],
            updated
        );
        assert_eq!(manager.requests().len(), 1);
        assert_eq!(
            manager.collections()[&collection.id].item_order,
            [source.id]
        );
    }

    #[test]
    fn external_edits_and_deletions_do_not_get_overwritten() {
        let workspace = Workspace::new();
        let mut manager = CollectionManager::scan(&workspace.0).unwrap();
        let collection = manager.create_collection("Examples", None).unwrap();
        let source = RequestDocument::new(
            "Draft",
            ProtocolRequest::Http(HttpRequestPayload::new("GET", "")),
        );
        let edit = edit();
        let saved = save_new(&mut manager, collection.id, &source, "Request", &edit).unwrap();
        let path = manager.get_full_path(&saved.id).unwrap();
        let mut external = saved.clone();
        external.description = Some("External edit".into());
        let yaml = ps_workspace::serialize_resource_to_yaml(&external).unwrap();
        std::fs::write(&path, &yaml).unwrap();
        assert!(matches!(
            save_existing(&mut manager, &saved, &edit),
            Err(RequestSaveError::Conflict)
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), yaml);
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(
            save_existing(&mut manager, &saved, &edit),
            Err(RequestSaveError::Conflict)
        ));
        assert!(!path.exists());
    }

    #[test]
    fn sensitive_headers_are_rejected_before_any_file_write() {
        let workspace = Workspace::new();
        let mut manager = CollectionManager::scan(&workspace.0).unwrap();
        let collection = manager.create_collection("Examples", None).unwrap();
        let source = RequestDocument::new(
            "Draft",
            ProtocolRequest::Http(HttpRequestPayload::new("GET", "")),
        );
        for name in ["Authorization", "Cookie", "X-API-Key", "custom"] {
            let mut edit = edit();
            edit.http.headers = vec![HeaderEntry {
                name: name.into(),
                value: "private-credential".into(),
                enabled: false,
                is_secret: name == "custom",
                description: None,
            }];
            let error =
                save_new(&mut manager, collection.id, &source, "Request", &edit).unwrap_err();
            assert!(matches!(error, RequestSaveError::PlaintextCredential));
            assert!(!error.to_string().contains("private-credential"));
            assert!(!format!("{edit:?}").contains("private-credential"));
            assert!(manager.requests().is_empty());
        }
    }

    #[test]
    fn references_cannot_smuggle_literals_or_escaped_credentials() {
        for value in [
            "private{{token}}",
            "{{token}}private",
            r"\{{token}}",
            "{{bad name}}",
            "{{unterminated",
            "{{one}}{{two}}",
            "",
        ] {
            assert!(!is_reference(value), "{value}");
        }
        for value in ["{{token}}", " {{vault:AUTH_TOKEN}} "] {
            assert!(is_reference(value));
        }
    }

    #[test]
    fn auth_saves_keep_existing_names_but_reject_new_plaintext_credentials() {
        let mut source = RequestDocument::new(
            "Request",
            ProtocolRequest::Http(HttpRequestPayload::new("GET", "")),
        );
        let mut edit = edit();
        for auth in [
            AuthConfig::Bearer {
                token_secret_ref: "private-token".into(),
            },
            AuthConfig::Basic {
                username: "admin".into(),
                password_secret_ref: Some("private-password".into()),
            },
            AuthConfig::ApiKey {
                key: "X-Key".into(),
                value_secret_ref: "private-key".into(),
                location: ps_domain::ApiKeyLocation::Header,
            },
        ] {
            edit.auth = auth;
            assert!(matches!(
                edit.validate_for_source(&source),
                Err(RequestSaveError::PlaintextCredential)
            ));
        }
        source.auth = AuthConfig::Bearer {
            token_secret_ref: "SAVED_SECRET_NAME".into(),
        };
        edit.auth = source.auth.clone();
        assert!(edit.validate_for_source(&source).is_ok());
        assert_eq!(
            auth_for_editor(&source.auth),
            AuthConfig::Bearer {
                token_secret_ref: "{{vault:SAVED_SECRET_NAME}}".into()
            }
        );
    }

    #[test]
    fn session_basic_auth_resolves_before_encoding_without_mutating_the_shared_resolver() {
        let resolver =
            ps_variable::VariableResolver::new().with_globals(std::collections::HashMap::from([
                ("user".into(), "admin".into()),
                ("password".into(), "secret".into()),
            ]));
        let original = resolver.clone();
        for password in ["secret", "{{password}}"] {
            let (auth, scoped) = prepare_session_auth(
                AuthConfig::Basic {
                    username: "{{user}}".into(),
                    password_secret_ref: Some(password.into()),
                },
                resolver.clone(),
            )
            .unwrap();
            let applied = ps_http::auth::apply_auth(&auth, &scoped).unwrap();
            assert_eq!(applied.headers()[0].wire_value(), "Basic YWRtaW46c2VjcmV0");
            assert!(!format!("{:?}", applied.headers()).contains("YWRtaW46c2VjcmV0"));
        }
        assert_eq!(
            resolver.resolve_var("password"),
            original.resolve_var("password")
        );
        let (auth, scoped) = prepare_session_auth(
            AuthConfig::Bearer {
                token_secret_ref: "{{missing}}".into(),
            },
            resolver,
        )
        .unwrap();
        assert!(ps_http::auth::apply_auth(&auth, &scoped).is_err());
    }
}
