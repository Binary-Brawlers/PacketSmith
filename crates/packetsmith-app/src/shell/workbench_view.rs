//! Native HTTP workbench: Postman-grade API client studio.
//! Features an activity rail, hierarchical collections sidebar, unified URL composer,
//! query param sync, auth managers, header toggles, JSON beautifier, and response studio.

use super::components::*;
use super::environment_input::TextInput;
use super::environment_view::EnvironmentView;
use super::icons::{icon, IconKind};
use super::requests::{self, reference_input, RequestEdit};
use super::theme;
use super::typography;
use super::AppState;
use gpui::{
    anchored, deferred, div, point, prelude::*, px, rgb, rgba, Anchor, Animation, AnimationExt,
    App, Context, Entity, FocusHandle, Focusable, Role, SharedString, Window,
};
use ps_domain::{
    ApiKeyLocation, AuthConfig, HttpRequestPayload, ProtocolRequest, RequestDocument, ResourceId,
};
use ps_http::{HeaderEntry, HttpBody, HttpRequest, HttpResponse, QueryParam, UrlSyncEngine};
use std::path::PathBuf;
use std::time::Duration;

gpui::actions!(
    request_workbench,
    [
        SendRequest,
        SaveRequest,
        DismissDialog,
        NewRequest,
        CloseTab,
        CommandPalette,
        BeautifyJson,
        SelectCollections,
        SelectEnvironments,
        SelectHistory,
        OpenDocs,
        OpenGitHub,
    ]
);

const PREVIEW_CHARS: usize = 32_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityMode {
    Collections,
    Environments,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestSubTab {
    Params,
    Auth,
    Headers,
    Body,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseSubTab {
    Body,
    Headers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthType {
    Saved,
    None,
    Bearer,
    Basic,
    ApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType {
    None,
    Json,
    FormData,
    Raw,
}

struct ParamRow {
    key: Entity<TextInput>,
    value: Entity<TextInput>,
    description: Entity<TextInput>,
    enabled: bool,
}

struct HeaderRow {
    key: Entity<TextInput>,
    value: Entity<TextInput>,
    description: Entity<TextInput>,
    enabled: bool,
    is_secret: bool,
}

struct HistoryItem {
    method: String,
    url: String,
    status_code: Option<u16>,
    duration_ms: Option<u64>,
}

struct Draft {
    document: RequestDocument,
    workspace_path: Option<PathBuf>,
    saved_edit: Option<RequestEdit>,
    last_url: String,
    last_params: Vec<QueryParam>,
    raw_content_type: String,
    api_key_location: ApiKeyLocation,
    id: usize,
    generation: usize,
    title: String,
    method: String,
    url: Entity<TextInput>,
    params: Vec<ParamRow>,
    auth_type: AuthType,
    bearer_token: Entity<TextInput>,
    basic_username: Entity<TextInput>,
    basic_password: Entity<TextInput>,
    api_key_name: Entity<TextInput>,
    api_key_value: Entity<TextInput>,
    headers: Vec<HeaderRow>,
    body_type: BodyType,
    body: Entity<TextInput>,
    response: Option<HttpResponse>,
    response_text: String,
    message: String,
    running: Option<tokio::task::AbortHandle>,
    sub_tab: RequestSubTab,
    response_sub_tab: ResponseSubTab,
    is_dirty: bool,
}

impl Draft {
    fn param_values(&self, cx: &App) -> Vec<QueryParam> {
        self.params
            .iter()
            .map(|p| QueryParam {
                key: p.key.read(cx).value(),
                value: p.value.read(cx).value(),
                enabled: p.enabled,
                description: nonempty(p.description.read(cx).value()),
            })
            .collect()
    }

    fn edit(&self, cx: &App) -> RequestEdit {
        let auth = match self.auth_type {
            AuthType::Saved => self.document.auth.clone(),
            AuthType::None => AuthConfig::None,
            AuthType::Bearer => AuthConfig::Bearer {
                token_secret_ref: self.bearer_token.read(cx).value(),
            },
            AuthType::Basic => AuthConfig::Basic {
                username: self.basic_username.read(cx).value(),
                password_secret_ref: nonempty(self.basic_password.read(cx).value()),
            },
            AuthType::ApiKey => AuthConfig::ApiKey {
                key: self.api_key_name.read(cx).value(),
                value_secret_ref: self.api_key_value.read(cx).value(),
                location: self.api_key_location,
            },
        };
        let auth = if auth == requests::auth_for_editor(&self.document.auth) {
            self.document.auth.clone()
        } else {
            auth
        };
        let body = match self.body_type {
            BodyType::None => HttpBody::None,
            BodyType::Json => HttpBody::Json {
                json_content: self.body.read(cx).value(),
            },
            BodyType::Raw | BodyType::FormData => HttpBody::Raw {
                content: self.body.read(cx).value(),
                content_type: self.raw_content_type.clone(),
            },
        };
        RequestEdit {
            auth,
            http: HttpRequestPayload {
                method: self.method.clone(),
                url: self.url.read(cx).value(),
                params: self.param_values(cx),
                headers: self
                    .headers
                    .iter()
                    .map(|h| HeaderEntry {
                        name: h.key.read(cx).value(),
                        value: h.value.read(cx).value(),
                        enabled: h.enabled,
                        is_secret: h.is_secret,
                        description: nonempty(h.description.read(cx).value()),
                    })
                    .collect(),
                body,
            },
        }
    }
}

fn nonempty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
impl Drop for Draft {
    fn drop(&mut self) {
        if let Some(task) = self.running.take() {
            task.abort();
        }
    }
}

pub struct WorkbenchView {
    focus: FocusHandle,
    environments: Entity<EnvironmentView>,
    activity_mode: ActivityMode,
    drafts: Vec<Draft>,
    active: usize,
    next_id: usize,
    runtime: tokio::runtime::Handle,
    sidebar_filter: Entity<TextInput>,
    show_env_quick_look: bool,
    show_command_palette: bool,
    command_palette_query: Entity<TextInput>,
    history: Vec<HistoryItem>,
    method_selector_open: bool,
    saving: bool,
    save_dialog: Option<usize>,
    save_name: Entity<TextInput>,
    save_collection: Option<ResourceId>,
    closing: Option<usize>,
    focus_dialog: bool,
    focus_workbench: bool,
}

fn mono_input(
    value: &str,
    placeholder: &str,
    compact: bool,
    cx: &mut Context<WorkbenchView>,
) -> Entity<TextInput> {
    let input = cx.new(|cx| {
        let mut inp = TextInput::new(value, placeholder, false, cx).mono();
        if compact {
            inp = inp.compact();
        }
        inp
    });
    cx.observe(&input, |_, _, cx| cx.notify()).detach();
    input
}

fn secret_input(
    value: &str,
    placeholder: &str,
    cx: &mut Context<WorkbenchView>,
) -> Entity<TextInput> {
    let input = cx.new(|cx| {
        TextInput::new(value, placeholder, true, cx)
            .mono()
            .compact()
    });
    cx.observe(&input, |_, _, cx| cx.notify()).detach();
    input
}

impl std::fmt::Debug for WorkbenchView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkbenchView")
            .field("draft_count", &self.drafts.len())
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl WorkbenchView {
    pub fn bind_keys(cx: &mut App) {
        cx.bind_keys([gpui::KeyBinding::new(
            "escape",
            DismissDialog,
            Some("RequestWorkbench"),
        )]);
        for modifier in ["cmd", "ctrl"] {
            cx.bind_keys([
                gpui::KeyBinding::new(
                    &format!("{modifier}-enter"),
                    SendRequest,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(
                    &format!("{modifier}-t"),
                    NewRequest,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(
                    &format!("{modifier}-s"),
                    SaveRequest,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(&format!("{modifier}-w"), CloseTab, Some("RequestWorkbench")),
                gpui::KeyBinding::new(
                    &format!("{modifier}-k"),
                    CommandPalette,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(
                    &format!("{modifier}-b"),
                    BeautifyJson,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(
                    &format!("{modifier}-1"),
                    SelectCollections,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(
                    &format!("{modifier}-2"),
                    SelectEnvironments,
                    Some("RequestWorkbench"),
                ),
                gpui::KeyBinding::new(
                    &format!("{modifier}-3"),
                    SelectHistory,
                    Some("RequestWorkbench"),
                ),
            ]);
        }
    }

    pub fn new(state: AppState, cx: &mut Context<Self>) -> Self {
        let environments = cx.new(|cx| EnvironmentView::new(state, cx));
        cx.observe(&environments, |_, _, cx| cx.notify()).detach();

        let sidebar_filter =
            cx.new(|cx| TextInput::new("", "Filter requests...", false, cx).compact());
        let command_palette_query =
            cx.new(|cx| TextInput::new("", "Type a command or jump to request...", false, cx));

        let save_name = mono_input("", "Request name", false, cx);
        let mut view = Self {
            focus: cx.focus_handle(),
            environments,
            activity_mode: ActivityMode::Collections,
            drafts: Vec::new(),
            active: 0,
            next_id: 1,
            runtime: tokio::runtime::Handle::current(),
            sidebar_filter,
            show_env_quick_look: false,
            show_command_palette: false,
            command_palette_query,
            history: Vec::new(),
            method_selector_open: false,
            saving: false,
            save_dialog: None,
            save_name,
            save_collection: None,
            closing: None,
            focus_dialog: false,
            focus_workbench: true,
        };

        view.add_draft(
            "Untitled Request".into(),
            "GET",
            "https://jsonplaceholder.typicode.com/posts/1",
            cx,
        );
        view
    }

    fn add_draft(&mut self, title: String, method: &str, url: &str, cx: &mut Context<Self>) {
        let mut document = RequestDocument::new(
            title,
            ProtocolRequest::Http(HttpRequestPayload::new(method, url)),
        );
        document.auth = AuthConfig::None;
        if let ProtocolRequest::Http(http) = &mut document.protocol {
            http.headers.push(HeaderEntry {
                name: "Accept".into(),
                value: "application/json".into(),
                enabled: true,
                is_secret: false,
                description: None,
            });
        }
        self.open_document(document, None, cx);
    }

    fn param_row(param: &QueryParam, cx: &mut Context<Self>) -> ParamRow {
        ParamRow {
            key: mono_input(&param.key, "Key", true, cx),
            value: mono_input(&param.value, "Value", true, cx),
            description: mono_input(
                param.description.as_deref().unwrap_or(""),
                "Description",
                true,
                cx,
            ),
            enabled: param.enabled,
        }
    }

    fn open_document(
        &mut self,
        document: RequestDocument,
        workspace_path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self
            .drafts
            .iter()
            .position(|d| d.document.id == document.id && d.workspace_path == workspace_path)
        {
            self.active = index;
            self.focus_workbench = true;
            self.activity_mode = ActivityMode::Collections;
            cx.notify();
            return;
        }
        let ProtocolRequest::Http(http) = &document.protocol else {
            return;
        };
        let (body_type, body, raw_content_type) = match &http.body {
            HttpBody::None => (BodyType::None, "".to_string(), "text/plain".to_string()),
            HttpBody::Json { json_content } => (
                BodyType::Json,
                json_content.clone(),
                "application/json".to_string(),
            ),
            HttpBody::Raw {
                content,
                content_type,
            } => (BodyType::Raw, content.clone(), content_type.clone()),
            _ => {
                self.drafts[self.active].message =
                    "This body format is not supported by the desktop editor yet.".into();
                cx.notify();
                return;
            }
        };
        let id = self.next_id;
        self.next_id += 1;
        let params = if http.params.is_empty() {
            UrlSyncEngine::parse_url(&http.url).1
        } else {
            http.params.clone()
        };
        let (auth_type, bearer, username, password, key, value, location) = match &document.auth {
            AuthConfig::None => (
                AuthType::None,
                String::new(),
                String::new(),
                String::new(),
                "X-API-Key".into(),
                String::new(),
                ApiKeyLocation::Header,
            ),
            AuthConfig::Bearer { token_secret_ref } => (
                AuthType::Bearer,
                reference_input(token_secret_ref),
                String::new(),
                String::new(),
                "X-API-Key".into(),
                String::new(),
                ApiKeyLocation::Header,
            ),
            AuthConfig::Basic {
                username,
                password_secret_ref,
            } => (
                AuthType::Basic,
                String::new(),
                username.clone(),
                password_secret_ref
                    .as_deref()
                    .map(reference_input)
                    .unwrap_or_default(),
                "X-API-Key".into(),
                String::new(),
                ApiKeyLocation::Header,
            ),
            AuthConfig::ApiKey {
                key,
                value_secret_ref,
                location,
            } => (
                AuthType::ApiKey,
                String::new(),
                String::new(),
                String::new(),
                key.clone(),
                reference_input(value_secret_ref),
                *location,
            ),
            _ => (
                AuthType::Saved,
                String::new(),
                String::new(),
                String::new(),
                "X-API-Key".into(),
                String::new(),
                ApiKeyLocation::Header,
            ),
        };
        let draft = Draft {
            id,
            generation: 0,
            title: document.name.clone(),
            method: http.method.clone(),
            url: mono_input(&http.url, "https://api.example.com/v1/resource", false, cx),
            params: params.iter().map(|p| Self::param_row(p, cx)).collect(),
            last_url: http.url.clone(),
            last_params: params,
            raw_content_type,
            api_key_location: location,
            auth_type,
            bearer_token: secret_input(&bearer, "Token or {{token}}", cx),
            basic_username: mono_input(&username, "Username", true, cx),
            basic_password: secret_input(&password, "Password or {{password}}", cx),
            api_key_name: mono_input(&key, "Key Name", true, cx),
            api_key_value: secret_input(&value, "Key Value or {{api_key}}", cx),
            headers: http
                .headers
                .iter()
                .map(|h| HeaderRow {
                    key: mono_input(&h.name, "Header", true, cx),
                    value: if h.is_secret {
                        secret_input(&h.value, "Value", cx)
                    } else {
                        mono_input(&h.value, "Value", true, cx)
                    },
                    description: mono_input(
                        h.description.as_deref().unwrap_or(""),
                        "Description",
                        true,
                        cx,
                    ),
                    enabled: h.enabled,
                    is_secret: h.is_secret,
                })
                .collect(),
            body_type,
            body: mono_input(&body, "Paste JSON or payload here", false, cx),
            response: None,
            response_text: String::new(),
            message: "Ready to send".into(),
            running: None,
            sub_tab: RequestSubTab::Params,
            response_sub_tab: ResponseSubTab::Body,
            is_dirty: false,
            saved_edit: None,
            document,
            workspace_path,
        };
        self.drafts.push(draft);
        self.active = self.drafts.len() - 1;
        self.focus_workbench = true;
        self.drafts[self.active].saved_edit = Some(self.drafts[self.active].edit(cx));
        self.activity_mode = ActivityMode::Collections;
        self.method_selector_open = false;
        cx.notify();
    }

    fn refresh_drafts(&mut self, cx: &mut Context<Self>) {
        for draft in &mut self.drafts {
            for header in &draft.headers {
                let name = header.key.read(cx).value();
                let secret = header.is_secret
                    || ps_request_engine::redact_sensitive_header(&name, "value") == "[REDACTED]";
                if header.value.read(cx).password != secret {
                    header.value.update(cx, |input, cx| {
                        input.password = secret;
                        cx.notify();
                    });
                }
            }
            let url = draft.url.read(cx).value();
            let params = draft.param_values(cx);
            if url != draft.last_url {
                let mut parsed = UrlSyncEngine::parse_url(&url).1;
                // Keep disabled rows, and metadata on matching enabled rows.
                for (index, param) in parsed.iter_mut().enumerate() {
                    if let Some(old) = params.iter().filter(|p| p.enabled).nth(index) {
                        if old.key == param.key && old.value == param.value {
                            param.description = old.description.clone();
                        }
                    }
                }
                parsed.extend(params.into_iter().filter(|p| !p.enabled));
                draft.params = parsed.iter().map(|p| Self::param_row(p, cx)).collect();
                draft.last_params = parsed;
                draft.last_url = url;
            } else if params != draft.last_params {
                let base = UrlSyncEngine::parse_url(&url).0;
                let updated = UrlSyncEngine::build_url(&base, &params);
                draft.url.update(cx, |input, cx| {
                    input.set_text(updated.clone());
                    cx.notify();
                });
                draft.last_url = updated;
                draft.last_params = params;
            }
            draft.is_dirty = draft.saved_edit.as_ref() != Some(&draft.edit(cx));
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        if let Some(id) = self.save_dialog {
            self.persist_draft(
                id,
                Some((self.save_collection, self.save_name.read(cx).value())),
                cx,
            );
            return;
        }
        self.refresh_drafts(cx);
        let draft = &self.drafts[self.active];
        let ws = self.environments.read(cx).ws();
        if draft
            .workspace_path
            .as_ref()
            .is_some_and(|path| path != &ws.workspace_path)
        {
            self.drafts[self.active].message =
                "Reopen this request's workspace before saving.".into();
            cx.notify();
            return;
        }
        if draft.workspace_path.is_some() {
            self.persist_draft(draft.id, None, cx);
        } else {
            self.save_dialog = Some(draft.id);
            self.focus_dialog = true;
            self.save_collection = ws.collection_manager.as_ref().and_then(|m| {
                let mut collections: Vec<_> = m.collections().values().collect();
                collections.sort_by(|a, b| {
                    a.name
                        .cmp(&b.name)
                        .then(a.id.to_string().cmp(&b.id.to_string()))
                });
                collections.first().map(|c| c.id)
            });
            self.save_name.update(cx, |input, cx| {
                input.set_text(draft.title.clone());
                cx.notify();
            });
            cx.notify();
        }
    }

    fn persist_draft(
        &mut self,
        id: usize,
        destination: Option<(Option<ResourceId>, String)>,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        self.refresh_drafts(cx);
        let Some(draft) = self.drafts.iter().find(|d| d.id == id) else {
            return;
        };
        let edit = draft.edit(cx);
        if edit.validate_for_source(&draft.document).is_err() {
            self.drafts.iter_mut().find(|d| d.id == id).unwrap().message =
                requests::RequestSaveError::PlaintextCredential.to_string();
            cx.notify();
            return;
        }
        let source = draft.document.clone();
        let saved = draft.workspace_path.is_some();
        let workspace_path = self.environments.read(cx).ws().workspace_path.clone();
        if self.environments.read(cx).ws().collection_manager.is_none() {
            self.drafts[self.active].message =
                "Open a workspace under Environments before saving.".into();
            cx.notify();
            return;
        }
        self.saving = true;
        let saved_edit = edit.clone();
        let path_to_scan = workspace_path.clone();
        let task = self.runtime.spawn_blocking(move || {
            let mut manager = ps_workspace::CollectionManager::scan(path_to_scan)?;
            let result = if saved {
                requests::save_existing(&mut manager, &source, &edit)
            } else {
                let (collection, name) = destination.expect("new request destination");
                if name.trim().is_empty() {
                    return Err(requests::RequestSaveError::MissingName);
                }
                let collection = match collection {
                    Some(id) => id,
                    None => manager.create_collection("Requests", None)?.id,
                };
                requests::save_new(&mut manager, collection, &source, &name, &edit)
            };
            result.map(|document| (manager, document))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                view.saving = false;
                match result {
                    Ok(Ok((manager, document))) => {
                        if view.environments.read(cx).ws().workspace_path == workspace_path {
                            view.environments.update(cx, |env, cx| {
                                env.ws_mut().resource_tree =
                                    Some(ps_workspace::ResourceTree::from_manager(&manager));
                                env.ws_mut().collection_manager = Some(manager);
                                cx.notify();
                            });
                        }
                        if let Some(draft) = view.drafts.iter_mut().find(|d| d.id == id) {
                            draft.title = document.name.clone();
                            draft.document = document;
                            draft.workspace_path = Some(workspace_path);
                            draft.saved_edit = Some(saved_edit);
                            draft.message = "Request saved.".into();
                            draft.is_dirty = draft.saved_edit.as_ref() != Some(&draft.edit(cx));
                            if view.closing == Some(id) && !draft.is_dirty {
                                view.closing = None;
                                let index = view.drafts.iter().position(|d| d.id == id).unwrap();
                                view.remove_tab_at(index, cx);
                            }
                        }
                        view.save_dialog = None;
                    }
                    error => {
                        if let Some(draft) = view.drafts.iter_mut().find(|d| d.id == id) {
                            draft.message = match error {
                                Ok(Err(e)) => e.to_string(),
                                _ => "Could not save the request. Your draft is retained.".into(),
                            };
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn close_tab_at(&mut self, index: usize, cx: &mut Context<Self>) {
        self.refresh_drafts(cx);
        if let Some(draft) = self.drafts.get(index) {
            if draft.is_dirty {
                self.closing = Some(draft.id);
                self.focus_dialog = true;
                cx.notify();
                return;
            }
        }
        self.remove_tab_at(index, cx);
    }

    fn remove_tab_at(&mut self, index: usize, cx: &mut Context<Self>) {
        self.focus_workbench = true;
        if self.drafts.is_empty() {
            self.add_draft("Untitled Request".into(), "GET", "", cx);
            self.active = 0;
            cx.notify();
            return;
        }
        if index >= self.drafts.len() {
            return;
        }

        self.drafts.remove(index);
        if self.drafts.is_empty() {
            self.add_draft("Untitled Request".into(), "GET", "", cx);
            self.active = 0;
        } else if index < self.active {
            self.active = self.active.saturating_sub(1);
        } else if self.active >= self.drafts.len() {
            self.active = self.drafts.len() - 1;
        }
        cx.notify();
    }

    fn sync_params_to_url(&mut self, cx: &mut Context<Self>) {
        self.refresh_drafts(cx);
        cx.notify();
    }

    fn beautify_json_body(&mut self, cx: &mut Context<Self>) {
        if self.active >= self.drafts.len() {
            return;
        }
        let draft = &mut self.drafts[self.active];
        let current_text = draft.body.read(cx).value();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&current_text) {
            if let Ok(formatted) = serde_json::to_string_pretty(&value) {
                draft.body.update(cx, |b, _| b.set_text(formatted));
                cx.notify();
            }
        }
    }

    fn send(&mut self, cx: &mut Context<Self>) {
        if self.save_dialog.is_some() || self.closing.is_some() {
            return;
        }
        self.refresh_drafts(cx);
        if self.active >= self.drafts.len() {
            return;
        }
        let draft = &mut self.drafts[self.active];
        if draft.running.is_some() {
            return;
        }

        let edit = draft.edit(cx);
        let url_val = edit.http.url.clone();
        let request = HttpRequest::from_payload(&edit.http);
        let ws = self.environments.read(cx).ws();
        let resolver = ws.variable_ui.resolver().clone();
        let auth = if matches!(edit.auth, AuthConfig::Inherit) {
            let mut chain = Vec::new();
            if let Some(manager) = &ws.collection_manager {
                let mut parent = manager.get_resource_parent(&draft.document.id);
                while let Some(current) = parent {
                    match current {
                        ps_workspace::ResourceParent::Folder(id) => {
                            if let Some(folder) = manager.folders().get(&id) {
                                chain.push(folder.auth.clone());
                            }
                            parent = manager.get_resource_parent(&id);
                        }
                        ps_workspace::ResourceParent::Collection(id) => {
                            if let Some(collection) = manager.collections().get(&id) {
                                chain.push(collection.auth.clone());
                            }
                            break;
                        }
                    }
                }
            }
            chain
                .into_iter()
                .find(|auth| !matches!(auth, AuthConfig::Inherit))
                .unwrap_or(AuthConfig::None)
        } else {
            edit.auth
        };
        let prepared = if draft.document.auth == auth || draft.auth_type == AuthType::Saved {
            Ok((auth, resolver))
        } else {
            requests::prepare_session_auth(auth, resolver)
        };
        let (auth, resolver) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                draft.message = error.to_string();
                cx.notify();
                return;
            }
        };

        let id = draft.id;
        draft.generation += 1;
        let generation = draft.generation;
        draft.message = "Sending…".into();
        draft.response = None;
        draft.response_text.clear();

        let history_method = draft.method.clone();
        let history_url = url_val.clone();

        let task = self.runtime.spawn(async move {
            ps_http::desktop::send_desktop_request_with_auth(request, resolver, &auth).await
        });
        draft.running = Some(task.abort_handle());

        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                if let Some(draft) = view.drafts.iter_mut().find(|draft| draft.id == id) {
                    if draft.generation != generation || draft.running.take().is_none() {
                        return;
                    }
                    match result {
                        Ok(Ok(response)) => {
                            view.history.insert(
                                0,
                                HistoryItem {
                                    method: history_method,
                                    url: history_url,
                                    status_code: Some(response.status_code),
                                    duration_ms: Some(response.duration_ms),
                                },
                            );
                            if view.history.len() > 50 {
                                view.history.truncate(50);
                            }
                            draft.message = format!(
                                "{} {} · {} ms · {} bytes · {}",
                                response.status_code,
                                response.status_text,
                                response.duration_ms,
                                response.size_bytes,
                                response.http_version
                            );
                            draft.response_text = preview(&response.body_bytes);
                            draft.response = Some(response);
                        }
                        Ok(Err(error)) => {
                            view.history.insert(
                                0,
                                HistoryItem {
                                    method: history_method,
                                    url: history_url,
                                    status_code: None,
                                    duration_ms: None,
                                },
                            );
                            draft.message = error.to_string();
                        }
                        Err(_) => draft.message = "Request cancelled or interrupted.".into(),
                    }
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }
}

impl Render for WorkbenchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.refresh_drafts(cx);
        if self.focus_workbench && self.save_dialog.is_none() && self.closing.is_none() {
            window.focus(&self.focus, cx);
            self.focus_workbench = false;
        }
        if self.focus_dialog {
            let focus = if self.save_dialog.is_some() {
                self.save_name.focus_handle(cx)
            } else {
                self.focus.clone()
            };
            window.focus(&focus, cx);
            self.focus_dialog = false;
        }
        if !self.drafts.is_empty() && self.active >= self.drafts.len() {
            self.active = self.drafts.len() - 1;
        }

        let (environment_name, is_env_none, saved) = {
            let env_view = self.environments.read(cx);
            let ws = env_view.ws();
            let name = ws
                .active_environment_id
                .and_then(|id| ws.environments.get(id).ok())
                .map(|doc| doc.name.clone())
                .unwrap_or_else(|| "No environment".into());
            let is_none = ws.active_environment_id.is_none();
            let mut list = ws
                .collection_manager
                .as_ref()
                .map(|manager| {
                    manager
                        .requests()
                        .values()
                        .filter_map(|doc| match &doc.protocol {
                            ps_domain::ProtocolRequest::Http(http) => Some((
                                doc.name.clone(),
                                http.method.clone(),
                                http.url.clone(),
                                doc.clone(),
                            )),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            list.sort_by(|a, b| a.0.cmp(&b.0));
            (name, is_none, list)
        };

        let filter_query = self.sidebar_filter.read(cx).value().to_lowercase();
        let filtered_saved: Vec<_> = saved
            .into_iter()
            .filter(|(name, method, url, _)| {
                if filter_query.is_empty() {
                    true
                } else {
                    name.to_lowercase().contains(&filter_query)
                        || method.to_lowercase().contains(&filter_query)
                        || url.to_lowercase().contains(&filter_query)
                }
            })
            .collect();

        // -------------------------------------------------------------------
        // 1. Left Activity Bar (48px rail)
        // -------------------------------------------------------------------
        let activity_bar = div()
            .w(px(88.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .py_4()
            .gap_2()
            .bg(rgb(theme::ACTIVITY_BAR))
            .border_r_1()
            .border_color(rgb(theme::BORDER_SUBTLE))
            .child(
                div()
                    .w(px(34.))
                    .h(px(34.))
                    .rounded_lg()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(theme::ACCENT_BG))
                    .child(icon(IconKind::Network, px(20.), rgb(theme::ACCENT_LIGHT))),
            )
            .child(div().h(px(12.)))
            .child(navigation_item(
                "nav-collections",
                IconKind::Folder,
                "Requests",
                self.activity_mode == ActivityMode::Collections,
                cx,
                |s, _, cx| {
                    s.activity_mode = ActivityMode::Collections;
                    cx.notify();
                },
            ))
            .child(navigation_item(
                "nav-environments",
                IconKind::Globe,
                "Environments",
                self.activity_mode == ActivityMode::Environments,
                cx,
                |s, _, cx| {
                    s.activity_mode = ActivityMode::Environments;
                    cx.notify();
                },
            ))
            .child(navigation_item(
                "nav-history",
                IconKind::Clock,
                "History",
                self.activity_mode == ActivityMode::History,
                cx,
                |s, _, cx| {
                    s.activity_mode = ActivityMode::History;
                    cx.notify();
                },
            ))
            .child(div().flex_1())
            .child(navigation_item(
                "nav-cmd",
                IconKind::Terminal,
                "Commands",
                false,
                cx,
                |s, _, cx| {
                    s.show_command_palette = !s.show_command_palette;
                    cx.notify();
                },
            ));

        // -------------------------------------------------------------------
        // 2. Secondary Sidebar (260px)
        // -------------------------------------------------------------------
        let mut sidebar = div()
            .w(px(232.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(rgb(theme::SIDEBAR))
            .border_r_1()
            .border_color(rgb(theme::BORDER_SUBTLE));

        match self.activity_mode {
            ActivityMode::Collections => {
                sidebar = sidebar
                    .child(section_header(
                        "Collections",
                        Some(filtered_saved.len()),
                        Some(styled_button(
                            "sidebar-new-request",
                            "+ New",
                            ButtonVariant::Ghost,
                            ButtonSize::Small,
                            false,
                            cx,
                            |s, _, cx| s.add_draft("Untitled Request".into(), "GET", "", cx),
                        )),
                    ))
                    .child(div().px_3().py_2().child(self.sidebar_filter.clone()));

                let mut list = div()
                    .id("saved-requests")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .gap_1();

                if filtered_saved.is_empty() {
                    list = list.child(
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_center()
                            .gap_2()
                            .text_center()
                            .child(icon(IconKind::Folder, px(24.), rgb(theme::MUTED)))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child("No saved requests"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::MUTED))
                                    .child("Keep your API requests together in a local workspace."),
                            )
                            .child(styled_button(
                                "collections-open-workspace",
                                "Open workspace",
                                ButtonVariant::Secondary,
                                ButtonSize::Small,
                                false,
                                cx,
                                |s, _, cx| {
                                    s.activity_mode = ActivityMode::Environments;
                                    cx.notify();
                                },
                            ))
                            .child(styled_button(
                                "sample-apis",
                                "Try sample requests",
                                ButtonVariant::Secondary,
                                ButtonSize::Small,
                                false,
                                cx,
                                |s, _, cx| {
                                    s.add_draft(
                                        "List Users (JSONPlaceholder)".into(),
                                        "GET",
                                        "https://jsonplaceholder.typicode.com/users",
                                        cx,
                                    );
                                    s.add_draft(
                                        "Create Post (JSONPlaceholder)".into(),
                                        "POST",
                                        "https://jsonplaceholder.typicode.com/posts",
                                        cx,
                                    );
                                    s.add_draft(
                                        "Random Cat Fact".into(),
                                        "GET",
                                        "https://catfact.ninja/fact",
                                        cx,
                                    );
                                },
                            )),
                    );
                }

                for (index, (name, method, _url, document)) in
                    filtered_saved.into_iter().enumerate()
                {
                    let req_name = name.clone();
                    let req_method = method.clone();
                    let workspace_path = self.environments.read(cx).ws().workspace_path.clone();
                    list = list.child(
                        div()
                            .id(SharedString::from(format!("saved-row-{index}")))
                            .role(Role::Button)
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1p5()
                            .rounded_md()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .child(method_badge(&req_method))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(req_name.clone()),
                            )
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.open_document(document.clone(), Some(workspace_path.clone()), cx);
                            })),
                    );
                }

                sidebar = sidebar.child(list);
            }
            ActivityMode::Environments => {
                sidebar = sidebar
                    .child(section_header("ENVIRONMENTS", None, None))
                    .child(
                        div()
                            .flex_1()
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(styled_button(
                                "env-none",
                                "● No Environment",
                                ButtonVariant::Secondary,
                                ButtonSize::Medium,
                                is_env_none,
                                cx,
                                |s, _, cx| {
                                    s.environments.update(cx, |env, cx| {
                                        env.ws_mut().select_environment(None).ok();
                                        cx.notify();
                                    });
                                },
                            ))
                            .child(styled_button(
                                "open-env-studio",
                                "Open Environment Studio  →",
                                ButtonVariant::Primary,
                                ButtonSize::Medium,
                                false,
                                cx,
                                |s, _, cx| {
                                    s.activity_mode = ActivityMode::Environments;
                                    cx.notify();
                                },
                            )),
                    );
            }
            ActivityMode::History => {
                sidebar = sidebar.child(section_header("History", Some(self.history.len()), None));
                let mut hist_list = div()
                    .id("hist-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_2()
                    .py_2()
                    .flex()
                    .flex_col()
                    .gap_1();

                if self.history.is_empty() {
                    hist_list = hist_list.child(
                        div()
                            .p_4()
                            .text_center()
                            .text_size(px(12.))
                            .text_color(rgb(theme::MUTED))
                            .child("No requests sent yet in this session."),
                    );
                }

                for (idx, item) in self.history.iter().enumerate() {
                    let h_method = item.method.clone();
                    let h_url = item.url.clone();
                    let status_str = item
                        .status_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "ERR".into());

                    hist_list = hist_list.child(
                        div()
                            .id(SharedString::from(format!("hist-{idx}")))
                            .role(Role::Button)
                            .aria_label(h_url.clone())
                            .cursor_pointer()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .px_3()
                            .py_3()
                            .rounded_md()
                            .border_b_1()
                            .border_color(rgb(theme::BORDER_SUBTLE))
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(method_badge(&h_method))
                                    .child(div().flex_1())
                                    .when_some(item.duration_ms, |el, ms| {
                                        el.child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::MUTED))
                                                .child(format!("{ms} ms")),
                                        )
                                    })
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(rgb(item
                                                .status_code
                                                .map(theme::status_color)
                                                .unwrap_or(theme::DANGER)))
                                            .child(status_str),
                                    ),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .font_family(typography::MONO_FONT)
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(h_url.clone()),
                            )
                            .on_click(cx.listener(move |s, _, _, cx| {
                                s.add_draft("Restored Request".into(), &h_method, &h_url, cx);
                            })),
                    );
                }

                sidebar = sidebar.child(hist_list);
            }
        }

        // -------------------------------------------------------------------
        // 3. Top Navigation Header Bar
        // -------------------------------------------------------------------
        let top_header = div()
            .h(px(52.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .pl(if cfg!(target_os = "macos") {
                px(82.)
            } else {
                px(16.)
            })
            .pr_4()
            .gap_3()
            .bg(rgb(theme::HEADER))
            .border_b_1()
            .border_color(rgb(theme::BORDER_SUBTLE))
            // Left: Breadcrumbs
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_size(px(12.5))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme::TEXT))
                            .child("PacketSmith"),
                    )
                    .child(div().px_2().text_color(rgb(theme::MUTED_DARK)).child("/"))
                    .child(div().text_color(rgb(theme::MUTED)).child("Workspace"))
                    .child(icon(
                        IconKind::ChevronRight,
                        px(12.),
                        rgb(theme::MUTED_DARK),
                    ))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme::TEXT))
                            .child(match self.activity_mode {
                                ActivityMode::Collections => "Requests",
                                ActivityMode::Environments => "Environments",
                                ActivityMode::History => "History",
                            }),
                    ),
            )
            .child(div().flex_1())
            // Center: Command Search Bar Pill
            .child(
                div()
                    .id("cmd-palette-pill")
                    .role(Role::Button)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w(px(240.))
                    .px_3()
                    .py_1p5()
                    .rounded_lg()
                    .bg(rgb(theme::CANVAS))
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .shadow_xs()
                    .hover(|s| s.border_color(rgb(theme::ACCENT)))
                    .child(icon(IconKind::Search, px(13.), rgb(theme::MUTED)))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.5))
                            .text_color(rgb(theme::MUTED))
                            .child("Search or jump to..."),
                    )
                    .child(platform_shortcut("⌘K", "Ctrl K"))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.show_command_palette = !s.show_command_palette;
                        cx.notify();
                    })),
            )
            .child(div().flex_1())
            // Right: Environment Selector & Eye Button
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(styled_icon_button(
                        "top-env-btn",
                        IconKind::Globe,
                        Some(environment_name.clone()),
                        ButtonVariant::Secondary,
                        ButtonSize::Small,
                        false,
                        cx,
                        |s, _, cx| {
                            s.activity_mode = ActivityMode::Environments;
                            cx.notify();
                        },
                    ))
                    .child(styled_icon_button(
                        "top-quick-look-btn",
                        IconKind::Eye,
                        None::<&str>,
                        ButtonVariant::Ghost,
                        ButtonSize::Small,
                        self.show_env_quick_look,
                        cx,
                        |s, _, cx| {
                            s.show_env_quick_look = !s.show_env_quick_look;
                            cx.notify();
                        },
                    )),
            );

        // -------------------------------------------------------------------
        // 4. Request Tabs Strip
        // -------------------------------------------------------------------
        let mut tab_strip = div()
            .id("workbench-tabs")
            .h(px(44.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_0()
            .px_3()
            .pt_1()
            .border_b_1()
            .border_color(rgb(theme::BORDER_SUBTLE))
            .bg(rgb(theme::SIDEBAR))
            .overflow_x_scroll();

        for (idx, draft) in self.drafts.iter().enumerate() {
            let active = idx == self.active;
            let tab_id = draft.id;
            let method = draft.method.clone();
            let title = draft.title.clone();

            tab_strip = tab_strip.child(
                div()
                    .id(SharedString::from(format!("tab-item-{tab_id}")))
                    .role(Role::Tab)
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3p5()
                    .h(px(36.))
                    .rounded_t_md()
                    .bg(if active { rgb(theme::CANVAS) } else { rgba(0) })
                    .border_t_2()
                    .border_color(if active { rgb(theme::ACCENT) } else { rgba(0) })
                    .hover(|s| {
                        s.bg(rgb(if active {
                            theme::SURFACE_ELEVATED
                        } else {
                            theme::HOVER
                        }))
                    })
                    .child(method_badge(&method))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(if active {
                                gpui::FontWeight::SEMIBOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(rgb(if active { theme::TEXT } else { theme::MUTED }))
                            .child(title),
                    )
                    .when(draft.is_dirty, |el| {
                        el.child(
                            div()
                                .text_size(px(8.))
                                .text_color(rgb(theme::ACCENT))
                                .child("●"),
                        )
                    })
                    // Close tab button (✕)
                    .child(
                        div()
                            .id(SharedString::from(format!("close-tab-{tab_id}")))
                            .role(Role::Button)
                            .px_1()
                            .py(px(0.5))
                            .rounded_sm()
                            .text_size(px(10.))
                            .text_color(rgb(theme::MUTED))
                            .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(theme::DANGER)))
                            .child(icon(IconKind::Close, px(10.), rgb(theme::MUTED)))
                            .on_click(cx.listener(move |s, _, _, cx| {
                                cx.stop_propagation();
                                s.close_tab_at(idx, cx);
                            })),
                    )
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if idx < s.drafts.len() {
                            s.active = idx;
                            s.focus_workbench = true;
                            cx.notify();
                        }
                    })),
            );
        }

        tab_strip = tab_strip.child(div().pl_1().child(styled_icon_button(
            "add-tab-btn",
            IconKind::Plus,
            None::<&str>,
            ButtonVariant::Ghost,
            ButtonSize::Small,
            false,
            cx,
            |s, _, cx| s.add_draft("Untitled Request".into(), "GET", "", cx),
        )));

        // -------------------------------------------------------------------
        // 5. Unified URL Composer Bar
        // -------------------------------------------------------------------
        let draft = &self.drafts[self.active];
        let current_method = draft.method.clone();

        let composer_bar = div()
            .px_5()
            .pt_5()
            .pb_3()
            .flex()
            .flex_col()
            .gap_2p5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .p_1p5()
                    .rounded_lg()
                    .bg(rgb(theme::SURFACE))
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    // Method selector dropdown container
                    .child(
                        div()
                            .relative()
                            .child(
                                div()
                                    .id("method-selector-pill")
                                    .role(Role::Button)
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .px_3p5()
                                    .h(px(38.))
                                    .rounded_lg()
                                    .bg(rgb(theme::method_bg_color(&current_method)))
                                    .border_1()
                                    .border_color(rgba(
                                        (theme::method_color(&current_method) << 8) | 0x66,
                                    ))
                                    .text_color(rgb(theme::method_color(&current_method)))
                                    .font_family(typography::MONO_FONT)
                                    .text_size(px(12.5))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(current_method.clone())
                                    .child(icon(
                                        IconKind::ChevronDown,
                                        px(11.),
                                        rgb(theme::method_color(&current_method)),
                                    ))
                                    .on_click(cx.listener(|s, _, _, cx| {
                                        s.method_selector_open = !s.method_selector_open;
                                        cx.notify();
                                    })),
                            )
                            .when(self.method_selector_open, |el| {
                                let methods = [
                                    ("GET", "Retrieve data from a server"),
                                    ("POST", "Submit payload to create resource"),
                                    ("PUT", "Replace resource representation"),
                                    ("PATCH", "Apply partial modifications"),
                                    ("DELETE", "Remove specified resource"),
                                    ("HEAD", "Same as GET without response body"),
                                    ("OPTIONS", "Inspect communication options"),
                                ];
                                let current = current_method.clone();

                                el.child(
                                    deferred(
                                        anchored()
                                            .anchor(Anchor::TopLeft)
                                            .offset(point(px(0.), px(38.)))
                                            .snap_to_window_with_margin(px(8.))
                                            .child(
                                                div()
                                                    .id("method-dropdown-menu")
                                                    .occlude()
                                                    .on_mouse_down_out(cx.listener(
                                                        |s, _, _, cx| {
                                                            s.method_selector_open = false;
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .w(px(290.))
                                                    .rounded_lg()
                                                    .bg(rgb(theme::SURFACE_ELEVATED))
                                                    .border_1()
                                                    .border_color(rgb(theme::BORDER))
                                                    .shadow_lg()
                                                    .p_1p5()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_0p5()
                                                    .children(methods.into_iter().map(
                                                        |(m, desc)| {
                                                            let is_selected = m == current;
                                                            let m_str = m.to_string();
                                                            div()
                                                                .id(SharedString::from(format!(
                                                                    "dropdown-method-{m}"
                                                                )))
                                                                .role(Role::Button)
                                                                .cursor_pointer()
                                                                .flex()
                                                                .items_center()
                                                                .gap_2p5()
                                                                .px_2p5()
                                                                .py_1p5()
                                                                .rounded_md()
                                                                .bg(rgb(if is_selected {
                                                                    theme::HOVER
                                                                } else {
                                                                    0x00000000
                                                                }))
                                                                .hover(|s| s.bg(rgb(theme::HOVER)))
                                                                .child(method_badge(m))
                                                                .child(
                                                                    div()
                                                                        .flex_1()
                                                                        .min_w_0()
                                                                        .text_size(px(11.))
                                                                        .text_color(rgb(
                                                                            theme::TEXT_SECONDARY,
                                                                        ))
                                                                        .child(desc),
                                                                )
                                                                .when(is_selected, |row| {
                                                                    row.child(icon(
                                                                        IconKind::Check,
                                                                        px(13.),
                                                                        rgb(theme::ACCENT),
                                                                    ))
                                                                })
                                                                .on_click(cx.listener(
                                                                    move |s, _, _, cx| {
                                                                        s.drafts[s.active].method =
                                                                            m_str.clone();
                                                                        s.method_selector_open =
                                                                            false;
                                                                        cx.notify();
                                                                    },
                                                                ))
                                                        },
                                                    )),
                                            ),
                                    )
                                    .priority(100),
                                )
                            }),
                    )
                    // URL Input field
                    .child(div().flex_1().min_w_0().child(draft.url.clone()))
                    .child(styled_button(
                        "btn-save-req",
                        if self.saving { "Saving…" } else { "Save" },
                        ButtonVariant::Secondary,
                        ButtonSize::Large,
                        false,
                        cx,
                        |s, _, cx| s.save(cx),
                    ))
                    // Primary Action: Send Button
                    .child(if draft.running.is_some() {
                        styled_icon_button(
                            "btn-cancel-req",
                            IconKind::Stop,
                            Some("Cancel"),
                            ButtonVariant::Danger,
                            ButtonSize::Large,
                            false,
                            cx,
                            |s, _, cx| {
                                if let Some(task) = s.drafts[s.active].running.take() {
                                    task.abort();
                                }
                                s.drafts[s.active].message = "Request cancelled.".into();
                                cx.notify();
                            },
                        )
                    } else {
                        styled_icon_button(
                            "btn-send-req",
                            IconKind::Play,
                            Some("Send"),
                            ButtonVariant::Primary,
                            ButtonSize::Large,
                            false,
                            cx,
                            |s, _, cx| s.send(cx),
                        )
                    }),
            );

        // Smooth in-flight scanning animation bar
        let composer_bar = if draft.running.is_some() {
            composer_bar.child(
                div()
                    .w_full()
                    .h(px(3.))
                    .mx_2()
                    .rounded_full()
                    .bg(rgb(theme::SURFACE_ELEVATED))
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(px(220.))
                            .rounded_full()
                            .bg(theme::brand_gradient())
                            .with_animation(
                                "in-flight-progress",
                                Animation::new(Duration::from_millis(1100))
                                    .repeat()
                                    .with_easing(gpui::ease_in_out),
                                |this, delta| {
                                    let offset = (delta * 900.0) - 150.0;
                                    this.left(px(offset)).relative()
                                },
                            ),
                    ),
            )
        } else {
            composer_bar
        };

        // -------------------------------------------------------------------
        // 6. Request Configuration Tabs (Params, Auth, Headers, Body, Settings)
        // -------------------------------------------------------------------
        let draft = &self.drafts[self.active];
        let sub_tab = draft.sub_tab;

        let request_tabs = div()
            .px_6()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(theme::BORDER_SUBTLE))
            .child(styled_button(
                "subtab-params",
                format!("Params · {}", draft.params.len()),
                ButtonVariant::Tab,
                ButtonSize::Medium,
                sub_tab == RequestSubTab::Params,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].sub_tab = RequestSubTab::Params;
                    cx.notify();
                },
            ))
            .child(styled_button(
                "subtab-auth",
                match draft.auth_type {
                    AuthType::Saved => "Auth (Saved)",
                    AuthType::None => "Auth",
                    AuthType::Bearer => "Auth (Bearer)",
                    AuthType::Basic => "Auth (Basic)",
                    AuthType::ApiKey => "Auth (API Key)",
                },
                ButtonVariant::Tab,
                ButtonSize::Medium,
                sub_tab == RequestSubTab::Auth,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].sub_tab = RequestSubTab::Auth;
                    cx.notify();
                },
            ))
            .child(styled_button(
                "subtab-headers",
                format!("Headers · {}", draft.headers.len()),
                ButtonVariant::Tab,
                ButtonSize::Medium,
                sub_tab == RequestSubTab::Headers,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].sub_tab = RequestSubTab::Headers;
                    cx.notify();
                },
            ))
            .child(styled_button(
                "subtab-body",
                match draft.body_type {
                    BodyType::None => "Body",
                    BodyType::Json => "Body (JSON)",
                    BodyType::FormData => "Body (Form)",
                    BodyType::Raw => "Body (Raw)",
                },
                ButtonVariant::Tab,
                ButtonSize::Medium,
                sub_tab == RequestSubTab::Body,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].sub_tab = RequestSubTab::Body;
                    cx.notify();
                },
            ))
            .child(styled_button(
                "subtab-settings",
                "Settings",
                ButtonVariant::Tab,
                ButtonSize::Medium,
                sub_tab == RequestSubTab::Settings,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].sub_tab = RequestSubTab::Settings;
                    cx.notify();
                },
            ))
            .child(div().flex_1())
            .child(platform_shortcut("⌘↵ Send", "Ctrl Enter"));

        // -------------------------------------------------------------------
        // 7. Request Config Tab Content Area
        // -------------------------------------------------------------------
        let mut config_content = div()
            .id("config-content")
            .h(px(210.))
            .flex_shrink_0()
            .overflow_y_scroll()
            .px_6()
            .py_3();

        match sub_tab {
            RequestSubTab::Params => {
                let mut params_col = div().flex().flex_col().gap_2();
                params_col = params_col.child(
                    div()
                        .flex()
                        .gap_2()
                        .px_2()
                        .py_2()
                        .bg(rgb(theme::SURFACE))
                        .rounded_sm()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme::MUTED))
                        .child(div().w(px(30.)).child(""))
                        .child(div().w(px(180.)).child("Key"))
                        .child(div().flex_1().child("Value"))
                        .child(div().flex_1().child("Description"))
                        .child(div().w(px(30.)).child("")),
                );

                for (idx, row) in draft.params.iter().enumerate() {
                    let enabled = row.enabled;
                    params_col = params_col.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(30.)).child(styled_button(
                                format!("toggle-param-{idx}"),
                                if enabled { "☑" } else { "☐" },
                                ButtonVariant::Ghost,
                                ButtonSize::Small,
                                enabled,
                                cx,
                                move |s, _, cx| {
                                    if s.active < s.drafts.len()
                                        && idx < s.drafts[s.active].params.len()
                                    {
                                        s.drafts[s.active].params[idx].enabled = !enabled;
                                        s.sync_params_to_url(cx);
                                    }
                                },
                            )))
                            .child(div().w(px(180.)).child(row.key.clone()))
                            .child(div().flex_1().child(row.value.clone()))
                            .child(div().flex_1().child(row.description.clone()))
                            .child(div().w(px(30.)).child(styled_button(
                                format!("remove-param-{idx}"),
                                "✕",
                                ButtonVariant::Ghost,
                                ButtonSize::Small,
                                false,
                                cx,
                                move |s, _, cx| {
                                    if s.active < s.drafts.len()
                                        && idx < s.drafts[s.active].params.len()
                                    {
                                        s.drafts[s.active].params.remove(idx);
                                        s.sync_params_to_url(cx);
                                    }
                                },
                            ))),
                    );
                }

                if draft.params.is_empty() {
                    params_col = params_col.child(
                        div()
                            .py_3()
                            .px_2()
                            .text_size(px(12.))
                            .text_color(rgb(theme::MUTED))
                            .child(
                                "No query parameters. Add a key and value to refine this request.",
                            ),
                    );
                }
                params_col = params_col.child(div().pt_1().flex().child(styled_button(
                    "add-param-btn",
                    "+ Add Query Parameter",
                    ButtonVariant::Secondary,
                    ButtonSize::Small,
                    false,
                    cx,
                    |s, _, cx| {
                        s.drafts[s.active].params.push(ParamRow {
                            key: mono_input("", "Key", true, cx),
                            value: mono_input("", "Value", true, cx),
                            description: mono_input("", "Description", true, cx),
                            enabled: true,
                        });
                        cx.notify();
                    },
                )));

                config_content = config_content.child(params_col);
            }
            RequestSubTab::Auth => {
                let auth_type = draft.auth_type;
                let mut auth_col = div().flex().flex_col().gap_3().child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(theme::MUTED))
                                .child("Type:"),
                        )
                        .child(styled_button(
                            "auth-none",
                            "No Auth",
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            auth_type == AuthType::None,
                            cx,
                            |s, _, cx| {
                                s.drafts[s.active].auth_type = AuthType::None;
                                cx.notify();
                            },
                        ))
                        .child(styled_button(
                            "auth-bearer",
                            "Bearer Token",
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            auth_type == AuthType::Bearer,
                            cx,
                            |s, _, cx| {
                                s.drafts[s.active].auth_type = AuthType::Bearer;
                                cx.notify();
                            },
                        ))
                        .child(styled_button(
                            "auth-basic",
                            "Basic Auth",
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            auth_type == AuthType::Basic,
                            cx,
                            |s, _, cx| {
                                s.drafts[s.active].auth_type = AuthType::Basic;
                                cx.notify();
                            },
                        ))
                        .child(styled_button(
                            "auth-apikey",
                            "API Key",
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            auth_type == AuthType::ApiKey,
                            cx,
                            |s, _, cx| {
                                s.drafts[s.active].auth_type = AuthType::ApiKey;
                                cx.notify();
                            },
                        )),
                );

                match auth_type {
                    AuthType::Saved => {
                        auth_col = auth_col.child(
                            "Saved authentication is preserved. Select an auth type to replace it.",
                        );
                    }
                    AuthType::None => {
                        auth_col = auth_col.child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(theme::MUTED))
                                .child("This request does not use authorization."),
                        );
                    }
                    AuthType::Bearer => {
                        auth_col = auth_col.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(theme::MUTED))
                                        .child("TOKEN"),
                                )
                                .child(draft.bearer_token.clone()),
                        );
                    }
                    AuthType::Basic => {
                        auth_col = auth_col.child(
                            div()
                                .flex()
                                .gap_3()
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::MUTED))
                                                .child("USERNAME"),
                                        )
                                        .child(draft.basic_username.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::MUTED))
                                                .child("PASSWORD"),
                                        )
                                        .child(draft.basic_password.clone()),
                                ),
                        );
                    }
                    AuthType::ApiKey => {
                        auth_col = auth_col.child(
                            div()
                                .flex()
                                .gap_3()
                                .child(
                                    div()
                                        .w(px(200.))
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::MUTED))
                                                .child("HEADER KEY"),
                                        )
                                        .child(draft.api_key_name.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::MUTED))
                                                .child("Value"),
                                        )
                                        .child(draft.api_key_value.clone()),
                                ),
                        );
                    }
                }

                config_content = config_content.child(auth_col);
            }
            RequestSubTab::Headers => {
                let mut headers_col = div().flex().flex_col().gap_2();
                headers_col = headers_col.child(
                    div()
                        .flex()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme::MUTED))
                        .child(div().w(px(30.)).child(""))
                        .child(div().w(px(220.)).child("HEADER"))
                        .child(div().flex_1().child("Value"))
                        .child(div().flex_1().child("Description"))
                        .child(div().w(px(30.)).child("")),
                );

                for (idx, row) in draft.headers.iter().enumerate() {
                    let enabled = row.enabled;
                    headers_col = headers_col.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(30.)).child(styled_button(
                                format!("toggle-header-{idx}"),
                                if enabled { "☑" } else { "☐" },
                                ButtonVariant::Ghost,
                                ButtonSize::Small,
                                enabled,
                                cx,
                                move |s, _, cx| {
                                    if s.active < s.drafts.len()
                                        && idx < s.drafts[s.active].headers.len()
                                    {
                                        s.drafts[s.active].headers[idx].enabled = !enabled;
                                        cx.notify();
                                    }
                                },
                            )))
                            .child(div().w(px(220.)).child(row.key.clone()))
                            .child(div().flex_1().child(row.value.clone()))
                            .child(div().flex_1().child(row.description.clone()))
                            .child(div().w(px(30.)).child(styled_button(
                                format!("remove-header-{idx}"),
                                "✕",
                                ButtonVariant::Ghost,
                                ButtonSize::Small,
                                false,
                                cx,
                                move |s, _, cx| {
                                    if s.active < s.drafts.len()
                                        && idx < s.drafts[s.active].headers.len()
                                    {
                                        s.drafts[s.active].headers.remove(idx);
                                        cx.notify();
                                    }
                                },
                            ))),
                    );
                }

                headers_col = headers_col.child(
                    div()
                        .flex()
                        .gap_2()
                        .pt_1()
                        .child(styled_button(
                            "add-header-btn",
                            "+ Add Header",
                            ButtonVariant::Secondary,
                            ButtonSize::Small,
                            false,
                            cx,
                            |s, _, cx| {
                                s.drafts[s.active].headers.push(HeaderRow {
                                    key: mono_input("", "Header", true, cx),
                                    value: mono_input("", "Value", true, cx),
                                    description: mono_input("", "Description", true, cx),
                                    enabled: true,
                                    is_secret: false,
                                });
                                cx.notify();
                            },
                        ))
                        .child(styled_button(
                            "add-json-preset",
                            "Add JSON Headers",
                            ButtonVariant::Ghost,
                            ButtonSize::Small,
                            false,
                            cx,
                            |s, _, cx| {
                                s.drafts[s.active].headers.push(HeaderRow {
                                    key: mono_input("Content-Type", "Header", true, cx),
                                    value: mono_input("application/json", "Value", true, cx),
                                    description: mono_input("", "Description", true, cx),
                                    enabled: true,
                                    is_secret: false,
                                });
                                cx.notify();
                            },
                        )),
                );

                config_content = config_content.child(headers_col);
            }
            RequestSubTab::Body => {
                let body_type = draft.body_type;
                let mut body_col = div().flex().flex_col().gap_2().child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(styled_button(
                                    "body-none",
                                    "none",
                                    ButtonVariant::Secondary,
                                    ButtonSize::Small,
                                    body_type == BodyType::None,
                                    cx,
                                    |s, _, cx| {
                                        s.drafts[s.active].body_type = BodyType::None;
                                        cx.notify();
                                    },
                                ))
                                .child(styled_button(
                                    "body-json",
                                    "JSON",
                                    ButtonVariant::Secondary,
                                    ButtonSize::Small,
                                    body_type == BodyType::Json,
                                    cx,
                                    |s, _, cx| {
                                        s.drafts[s.active].body_type = BodyType::Json;
                                        cx.notify();
                                    },
                                ))
                                .child(styled_button(
                                    "body-raw",
                                    "Raw text",
                                    ButtonVariant::Secondary,
                                    ButtonSize::Small,
                                    body_type == BodyType::Raw,
                                    cx,
                                    |s, _, cx| {
                                        s.drafts[s.active].body_type = BodyType::Raw;
                                        cx.notify();
                                    },
                                )),
                        )
                        .when(body_type == BodyType::Json, |el| {
                            el.child(styled_icon_button(
                                "beautify-json-btn",
                                IconKind::Sparkles,
                                Some("Beautify JSON"),
                                ButtonVariant::Ghost,
                                ButtonSize::Small,
                                false,
                                cx,
                                |s, _, cx| s.beautify_json_body(cx),
                            ))
                        }),
                );

                if body_type != BodyType::None {
                    body_col = body_col.child(draft.body.clone());
                } else {
                    body_col = body_col.child(
                        div()
                            .py_4()
                            .text_size(px(12.))
                            .text_color(rgb(theme::MUTED))
                            .child("This request does not have a body."),
                    );
                }

                config_content = config_content.child(body_col);
            }
            RequestSubTab::Settings => {
                let settings_col = div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Request defaults"),
                    )
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child("TLS certificate verification is enabled."),
                    )
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_SECONDARY))
                            .child("Redirects are disabled. Request timeout: 30 seconds."),
                    );

                config_content = config_content.child(settings_col);
            }
        }

        // -------------------------------------------------------------------
        // 8. Response Studio
        // -------------------------------------------------------------------
        let response = draft.response.as_ref();
        let resp_sub_tab = draft.response_sub_tab;

        let mut response_panel = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .border_t_1()
            .border_color(rgb(theme::BORDER));

        let mut response_header = div()
            .px_6()
            .py_2p5()
            .flex()
            .items_center()
            .gap_2p5()
            .flex_wrap()
            .flex_shrink_0()
            .bg(rgb(theme::HEADER))
            .border_b_1()
            .border_color(rgb(theme::BORDER_SUBTLE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon(IconKind::Code, px(15.), rgb(theme::MUTED)))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_size(px(13.))
                            .text_color(rgb(theme::TEXT))
                            .child("Response"),
                    ),
            )
            .child(styled_button(
                "resp-tab-body",
                "Body",
                ButtonVariant::Tab,
                ButtonSize::Small,
                resp_sub_tab == ResponseSubTab::Body,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].response_sub_tab = ResponseSubTab::Body;
                    cx.notify();
                },
            ))
            .child(styled_button(
                "resp-tab-headers",
                format!(
                    "Headers ({})",
                    response.map(|r| r.headers.len()).unwrap_or(0)
                ),
                ButtonVariant::Tab,
                ButtonSize::Small,
                resp_sub_tab == ResponseSubTab::Headers,
                cx,
                |s, _, cx| {
                    s.drafts[s.active].response_sub_tab = ResponseSubTab::Headers;
                    cx.notify();
                },
            ));

        if let Some(resp) = response {
            response_header = response_header
                .child(div().flex_1())
                .child(status_badge(resp.status_code, &resp.status_text))
                .child(metric_chip_icon(
                    IconKind::Zap,
                    format!("{} ms", resp.duration_ms),
                ))
                .child(metric_chip_icon(
                    IconKind::HardDrive,
                    format!("{} B", resp.size_bytes),
                ))
                .child(styled_icon_button(
                    "copy-response-btn",
                    IconKind::Copy,
                    Some("Copy"),
                    ButtonVariant::Ghost,
                    ButtonSize::Small,
                    false,
                    cx,
                    |s, _, cx| {
                        if let Some(r) = &s.drafts[s.active].response {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                String::from_utf8_lossy(&r.body_bytes).into_owned(),
                            ));
                        }
                    },
                ));

            response_panel = response_panel.child(response_header);

            match resp_sub_tab {
                ResponseSubTab::Body => {
                    response_panel = response_panel.child(
                        div()
                            .id("response-content-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p_6()
                            .font_family(typography::MONO_FONT)
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .when(draft.response_text.is_empty(), |el| {
                                el.child("Empty response body")
                            })
                            .children(draft.response_text.lines().enumerate().map(
                                |(index, line)| {
                                    div()
                                        .flex()
                                        .gap_4()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .w(px(28.))
                                                .flex_shrink_0()
                                                .text_right()
                                                .text_color(rgb(theme::MUTED_DARK))
                                                .child((index + 1).to_string()),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_color(rgb(
                                                    if line.trim_start().starts_with('"') {
                                                        theme::ACCENT_LIGHT
                                                    } else {
                                                        theme::TEXT_SECONDARY
                                                    },
                                                ))
                                                .child(line.to_owned()),
                                        )
                                },
                            )),
                    );
                }
                ResponseSubTab::Headers => {
                    let mut rows = resp
                        .headers
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>();
                    rows.sort_by(|a, b| a.0.cmp(&b.0));

                    let mut headers_table = div()
                        .id("headers-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p_6()
                        .flex()
                        .flex_col()
                        .gap_1();

                    for (k, v) in rows {
                        headers_table = headers_table.child(
                            div()
                                .flex()
                                .gap_4()
                                .py_1()
                                .border_b_1()
                                .border_color(rgb(theme::BORDER_SUBTLE))
                                .font_family(typography::MONO_FONT)
                                .text_size(px(11.5))
                                .child(
                                    div()
                                        .w(px(220.))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(rgb(theme::MUTED))
                                        .child(k),
                                )
                                .child(div().flex_1().text_color(rgb(theme::TEXT)).child(v)),
                        );
                    }
                    response_panel = response_panel.child(headers_table);
                }
            }
        } else {
            response_panel = response_panel.child(response_header).child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .p_8()
                    .child(empty_state_card_icon(
                        IconKind::Play,
                        "Your response will appear here",
                        if draft.running.is_some() {
                            "Executing request... waiting for network response."
                        } else {
                            "Send a request to inspect its status, headers, and response body."
                        },
                        None::<&str>,
                        None::<&str>,
                        cx,
                        None::<fn(&mut WorkbenchView, &mut Window, &mut Context<WorkbenchView>)>,
                    )),
            );
        }

        // -------------------------------------------------------------------
        // 9. Bottom Status Bar (28px)
        // -------------------------------------------------------------------
        let request_notice = div()
            .px_6()
            .py_2()
            .bg(rgb(theme::WARNING_BG))
            .text_size(px(12.))
            .text_color(rgb(theme::WARNING))
            .child(draft.message.clone());
        let show_request_notice = draft.running.is_none()
            && draft.message != "Ready to send"
            && draft.response.as_ref().is_none_or(|response| {
                !draft.message.starts_with(&response.status_code.to_string())
            });

        let status_bar = div()
            .h(px(32.))
            .flex_shrink_0()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .bg(rgb(theme::ACTIVITY_BAR))
            .border_t_1()
            .border_color(rgb(theme::BORDER_SUBTLE))
            .text_size(px(11.))
            .text_color(rgb(theme::MUTED))
            // Live pulsing beacon dot
            .child(
                div()
                    .w(px(7.))
                    .h(px(7.))
                    .rounded_full()
                    .bg(rgb(theme::SUCCESS)),
            )
            .child("Local workspace")
            .child(div().text_color(rgb(theme::BORDER_SUBTLE)).child("|"))
            .child(format!("Environment: {environment_name}"))
            .child(div().text_color(rgb(theme::BORDER_SUBTLE)).child("|"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(icon(IconKind::Lock, px(11.), rgb(theme::SUCCESS)))
                    .child("SSL: Verified"),
            )
            .child(div().flex_1())
            .child(format!("{} open tabs", self.drafts.len()))
            .child(div().text_color(rgb(theme::BORDER_SUBTLE)).child("|"))
            .child(platform_shortcut("⌘K Commands", "Ctrl K Commands"))
            .child(platform_shortcut("⌘↵ Send", "Ctrl Enter Send"));

        // -------------------------------------------------------------------
        // 10. Assemble Root Layout
        // -------------------------------------------------------------------
        let mut root = div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::CANVAS))
            .text_color(rgb(theme::TEXT))
            .font_family(typography::UI_FONT)
            .text_size(px(13.))
            .track_focus(&self.focus)
            .key_context("RequestWorkbench")
            .on_action(cx.listener(|s, _: &SendRequest, _, cx| s.send(cx)))
            .on_action(cx.listener(|s, _: &SaveRequest, _, cx| s.save(cx)))
            .on_action(cx.listener(|s, _: &DismissDialog, _, cx| {
                if !s.saving {
                    s.save_dialog = None;
                    s.closing = None;
                }
                s.show_command_palette = false;
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &NewRequest, _, cx| {
                if s.save_dialog.is_some() || s.closing.is_some() {
                    return;
                }
                s.add_draft("Untitled Request".into(), "GET", "", cx)
            }))
            .on_action(cx.listener(|s, _: &CloseTab, _, cx| {
                if s.save_dialog.is_some() || s.closing.is_some() {
                    return;
                }
                let active = s.active;
                s.close_tab_at(active, cx);
            }))
            .on_action(cx.listener(|s, _: &CommandPalette, _, cx| {
                s.show_command_palette = !s.show_command_palette;
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &BeautifyJson, _, cx| {
                s.beautify_json_body(cx);
            }))
            .on_action(cx.listener(|s, _: &SelectCollections, _, cx| {
                s.activity_mode = ActivityMode::Collections;
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &SelectEnvironments, _, cx| {
                s.activity_mode = ActivityMode::Environments;
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &SelectHistory, _, cx| {
                s.activity_mode = ActivityMode::History;
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &OpenDocs, _, cx| {
                s.drafts[s.active].message = "Documentation: docs.packetsmith.dev".into();
                cx.notify();
            }))
            .on_action(cx.listener(|s, _: &OpenGitHub, _, cx| {
                s.drafts[s.active].message =
                    "GitHub: github.com/Binary-Brawlers/PacketSmith".into();
                cx.notify();
            }))
            .child(top_header)
            .child(div().flex().flex_1().min_h_0().child(activity_bar).child(
                if self.activity_mode == ActivityMode::Environments {
                    div()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .child(self.environments.clone())
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .child(sidebar)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .child(tab_strip)
                                .child(composer_bar)
                                .child(request_tabs)
                                .child(config_content)
                                .when(show_request_notice, |el| el.child(request_notice))
                                .child(response_panel),
                        )
                        .into_any_element()
                },
            ))
            .child(status_bar);

        if let Some(id) = self.save_dialog {
            let mut collections = self
                .environments
                .read(cx)
                .ws()
                .collection_manager
                .as_ref()
                .map(|m| {
                    m.collections()
                        .values()
                        .map(|c| (c.id, c.name.clone()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            collections.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.to_string().cmp(&b.0.to_string())));
            let mut destinations = div().flex().flex_col().gap_1();
            if collections.is_empty() {
                destinations =
                    destinations.child("A Requests collection will be created in this workspace.");
            } else {
                for (collection_id, name) in collections {
                    destinations = destinations.child(styled_button(
                        format!("save-collection-{collection_id}"),
                        name,
                        ButtonVariant::Secondary,
                        ButtonSize::Small,
                        self.save_collection == Some(collection_id),
                        cx,
                        move |s, _, cx| {
                            if !s.saving {
                                s.save_collection = Some(collection_id);
                                cx.notify();
                            }
                        },
                    ));
                }
            }
            let message = self
                .drafts
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.message.clone())
                .unwrap_or_default();
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .occlude()
                    .bg(rgba(0x00000088))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(480.))
                            .p_5()
                            .rounded_lg()
                            .bg(rgb(theme::SURFACE_ELEVATED))
                            .border_1()
                            .border_color(rgb(theme::BORDER))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child("Save request")
                            .child(self.save_name.clone())
                            .child("Collection")
                            .child(destinations)
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::MUTED))
                                    .child(message),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(styled_button(
                                        "cancel-save",
                                        "Cancel",
                                        ButtonVariant::Ghost,
                                        ButtonSize::Medium,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            if !s.saving {
                                                s.save_dialog = None;
                                                s.closing = None;
                                                cx.notify();
                                            }
                                        },
                                    ))
                                    .child(styled_button(
                                        "confirm-save",
                                        if self.saving { "Saving…" } else { "Save" },
                                        ButtonVariant::Primary,
                                        ButtonSize::Medium,
                                        false,
                                        cx,
                                        move |s, _, cx| {
                                            s.persist_draft(
                                                id,
                                                Some((
                                                    s.save_collection,
                                                    s.save_name.read(cx).value(),
                                                )),
                                                cx,
                                            );
                                        },
                                    )),
                            ),
                    ),
            );
        } else if let Some(id) = self.closing {
            let title = self
                .drafts
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.title.clone())
                .unwrap_or_default();
            let message = self
                .drafts
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.message.clone())
                .unwrap_or_default();
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .occlude()
                    .bg(rgba(0x00000088))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(480.))
                            .p_5()
                            .rounded_lg()
                            .bg(rgb(theme::SURFACE_ELEVATED))
                            .border_1()
                            .border_color(rgb(theme::BORDER))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(format!("Save changes to {title}?"))
                            .child("This tab has unsaved changes.")
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::MUTED))
                                    .child(message),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap_2()
                                    .child(styled_button(
                                        "cancel-close",
                                        "Cancel",
                                        ButtonVariant::Ghost,
                                        ButtonSize::Medium,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            if !s.saving {
                                                s.closing = None;
                                                cx.notify();
                                            }
                                        },
                                    ))
                                    .child(styled_button(
                                        "discard-close",
                                        "Discard",
                                        ButtonVariant::Danger,
                                        ButtonSize::Medium,
                                        false,
                                        cx,
                                        move |s, _, cx| {
                                            if !s.saving {
                                                s.closing = None;
                                                if let Some(index) =
                                                    s.drafts.iter().position(|d| d.id == id)
                                                {
                                                    s.remove_tab_at(index, cx);
                                                }
                                            }
                                        },
                                    ))
                                    .child(styled_button(
                                        "save-close",
                                        if self.saving { "Saving…" } else { "Save" },
                                        ButtonVariant::Primary,
                                        ButtonSize::Medium,
                                        false,
                                        cx,
                                        move |s, _, cx| {
                                            if let Some(index) =
                                                s.drafts.iter().position(|d| d.id == id)
                                            {
                                                s.active = index;
                                                s.save(cx);
                                            }
                                        },
                                    )),
                            ),
                    ),
            );
        }

        // Command Palette Modal Overlay
        if self.show_command_palette {
            root = root.child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x00000088))
                    .flex()
                    .items_start()
                    .justify_center()
                    .pt_20()
                    .with_animation(
                        "cmd-palette-fade",
                        Animation::new(Duration::from_millis(150)).with_easing(gpui::ease_in_out),
                        |this, delta| this.opacity(delta),
                    )
                    .child(
                        div()
                            .w(px(520.))
                            .rounded_lg()
                            .bg(rgb(theme::SURFACE_ELEVATED))
                            .border_1()
                            .border_color(rgb(theme::BORDER_FOCUS))
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_size(px(13.))
                                            .child("Command Palette"),
                                    )
                                    .child(styled_icon_button(
                                        "close-palette-btn",
                                        IconKind::Close,
                                        None::<&str>,
                                        ButtonVariant::Ghost,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.show_command_palette = false;
                                            cx.notify();
                                        },
                                    )),
                            )
                            .child(self.command_palette_query.clone())
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(styled_button(
                                        "cmd-send",
                                        if cfg!(target_os = "macos") {
                                            "Send request (⌘Enter)"
                                        } else {
                                            "Send request (Ctrl Enter)"
                                        },
                                        ButtonVariant::Secondary,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.show_command_palette = false;
                                            s.send(cx);
                                        },
                                    ))
                                    .child(styled_button(
                                        "cmd-new",
                                        if cfg!(target_os = "macos") {
                                            "New request (⌘T)"
                                        } else {
                                            "New request (Ctrl T)"
                                        },
                                        ButtonVariant::Secondary,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.show_command_palette = false;
                                            s.add_draft("Untitled Request".into(), "GET", "", cx);
                                        },
                                    ))
                                    .child(styled_button(
                                        "cmd-beautify",
                                        "Beautify JSON Body",
                                        ButtonVariant::Secondary,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.show_command_palette = false;
                                            s.beautify_json_body(cx);
                                        },
                                    ))
                                    .child(styled_button(
                                        "cmd-env",
                                        "Switch to Environment Studio",
                                        ButtonVariant::Secondary,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        |s, _, cx| {
                                            s.show_command_palette = false;
                                            s.activity_mode = ActivityMode::Environments;
                                            cx.notify();
                                        },
                                    )),
                            ),
                    ),
            );
        }

        // Environment Quick Look Popover
        if self.show_env_quick_look {
            root = root.child(
                div()
                    .absolute()
                    .top(px(48.))
                    .right(px(16.))
                    .w(px(340.))
                    .rounded_lg()
                    .bg(rgb(theme::SURFACE_ELEVATED))
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_size(px(12.5))
                                    .child(format!("Active: {environment_name}")),
                            )
                            .child(styled_icon_button(
                                "close-quick-look-btn",
                                IconKind::Close,
                                None::<&str>,
                                ButtonVariant::Ghost,
                                ButtonSize::Small,
                                false,
                                cx,
                                |s, _, cx| {
                                    s.show_env_quick_look = false;
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(rgb(theme::MUTED))
                            .child(
                                "Variables available in {{var_name}} expressions for this request.",
                            ),
                    )
                    .child(styled_button(
                        "manage-env-btn",
                        "Manage in Environment Studio  →",
                        ButtonVariant::Primary,
                        ButtonSize::Small,
                        false,
                        cx,
                        |s, _, cx| {
                            s.show_env_quick_look = false;
                            s.activity_mode = ActivityMode::Environments;
                            cx.notify();
                        },
                    )),
            );
        }

        root
    }
}

impl Focusable for WorkbenchView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn preview(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    let formatted = if bytes.len() <= PREVIEW_CHARS {
        serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok())
            .unwrap_or_else(|| raw.into_owned())
    } else {
        raw.into_owned()
    };
    let mut chars = formatted.chars();
    let mut preview: String = chars.by_ref().take(PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        preview.push_str("\n\n[Preview truncated. Copy response for the complete retained body.]");
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_bounds_unicode_and_formats_small_json() {
        assert!(preview(br#"{"ok":true}"#).contains('\n'));
        let large = "é".repeat(PREVIEW_CHARS + 1);
        assert!(preview(large.as_bytes()).contains("Preview truncated"));
    }
}
