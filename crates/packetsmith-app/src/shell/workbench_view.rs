//! Native HTTP workbench: Postman-grade API client studio.
//! Features an activity rail, hierarchical collections sidebar, unified URL composer,
//! query param sync, auth managers, header toggles, JSON beautifier, and response studio.

use super::components::*;
use super::environment_input::TextInput;
use super::environment_view::EnvironmentView;
use super::theme;
use super::typography;
use super::AppState;
use super::icons::{icon, IconKind};
use gpui::{
    anchored, deferred, div, point, prelude::*, px, rgb, rgba, Anchor, Animation, AnimationExt, App,
    Context, Entity, FocusHandle, Focusable, Role, SharedString, Window,
};
use std::time::Duration;
use ps_http::{HeaderEntry, HttpBody, HttpMethod, HttpRequest, HttpResponse, QueryParam, UrlSyncEngine};

gpui::actions!(
    request_workbench,
    [
        SendRequest,
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
}

struct HistoryItem {
    method: String,
    url: String,
    status_code: Option<u16>,
    duration_ms: Option<u64>,
}

struct Draft {
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
}


fn mono_input(
    value: &str,
    placeholder: &str,
    compact: bool,
    cx: &mut Context<WorkbenchView>,
) -> Entity<TextInput> {
    cx.new(|cx| {
        let mut inp = TextInput::new(value, placeholder, false, cx).mono();
        if compact {
            inp = inp.compact();
        }
        inp
    })
}

fn secret_input(value: &str, placeholder: &str, cx: &mut Context<WorkbenchView>) -> Entity<TextInput> {
    cx.new(|cx| TextInput::new(value, placeholder, true, cx).mono().compact())
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
                    &format!("{modifier}-w"),
                    CloseTab,
                    Some("RequestWorkbench"),
                ),
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

        let sidebar_filter = cx.new(|cx| TextInput::new("", "Filter requests...", false, cx).compact());
        let command_palette_query = cx.new(|cx| TextInput::new("", "Type a command or jump to request...", false, cx));

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
        };

        view.add_draft("Untitled Request".into(), "GET", "https://jsonplaceholder.typicode.com/posts/1", cx);
        view
    }

    fn add_draft(&mut self, title: String, method: &str, url: &str, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;

        let (_, initial_params) = UrlSyncEngine::parse_url(url);
        let param_rows: Vec<ParamRow> = initial_params
            .into_iter()
            .map(|p| ParamRow {
                key: mono_input(&p.key, "Key", true, cx),
                value: mono_input(&p.value, "Value", true, cx),
                description: cx.new(|cx| TextInput::new("", "Description", false, cx).compact()),
                enabled: p.enabled,
            })
            .collect();

        self.drafts.push(Draft {
            id,
            generation: 0,
            title,
            method: method.to_uppercase(),
            url: mono_input(url, "https://api.example.com/v1/resource", false, cx),
            params: param_rows,
            auth_type: AuthType::None,
            bearer_token: mono_input("", "Token or {{token}}", false, cx),
            basic_username: cx.new(|cx| TextInput::new("", "Username", false, cx).compact()),
            basic_password: secret_input("", "Password", cx),
            api_key_name: mono_input("X-API-Key", "Key Name", true, cx),
            api_key_value: secret_input("", "Key Value", cx),
            headers: vec![
                HeaderRow {
                    key: mono_input("Accept", "Header", true, cx),
                    value: mono_input("application/json", "Value", true, cx),
                    description: cx.new(|cx| TextInput::new("", "Description", false, cx).compact()),
                    enabled: true,
                },
            ],
            body_type: BodyType::None,
            body: mono_input("{\n  \"example\": \"value\"\n}", "Paste JSON or payload here", false, cx),
            response: None,
            response_text: String::new(),
            message: "Ready to send".into(),
            running: None,
            sub_tab: RequestSubTab::Params,
            response_sub_tab: ResponseSubTab::Body,
            is_dirty: false,
        });

        self.active = self.drafts.len() - 1;
        self.activity_mode = ActivityMode::Collections;
        self.method_selector_open = false;
        cx.notify();
    }

    fn close_tab_at(&mut self, index: usize, cx: &mut Context<Self>) {
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
        if self.active >= self.drafts.len() {
            return;
        }
        let draft = &mut self.drafts[self.active];
        let raw_url = draft.url.read(cx).value();
        let (base, _) = UrlSyncEngine::parse_url(&raw_url);
        let params: Vec<QueryParam> = draft
            .params
            .iter()
            .map(|p| QueryParam {
                key: p.key.read(cx).value(),
                value: p.value.read(cx).value(),
                enabled: p.enabled,
                description: None,
            })
            .collect();
        let new_url = UrlSyncEngine::build_url(&base, &params);
        draft.url.update(cx, |u, _| u.set_text(new_url));
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
        if self.active >= self.drafts.len() {
            return;
        }
        let draft = &mut self.drafts[self.active];
        if draft.running.is_some() {
            return;
        }

        let parsed_method = draft.method.parse::<HttpMethod>().unwrap_or(HttpMethod::Get);
        let url_val = draft.url.read(cx).value();
        let mut request = HttpRequest::new(parsed_method, url_val.trim());

        // 1. Headers from Table
        let mut headers: Vec<HeaderEntry> = draft
            .headers
            .iter()
            .filter(|h| h.enabled)
            .map(|h| HeaderEntry {
                name: h.key.read(cx).value(),
                value: h.value.read(cx).value(),
                enabled: true,
                is_secret: false,
                description: None,
            })
            .collect();

        // 2. Auth Injection
        match draft.auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                let token = draft.bearer_token.read(cx).value();
                if !token.is_empty() {
                    headers.push(HeaderEntry {
                        name: "Authorization".into(),
                        value: format!("Bearer {}", token.trim()),
                        enabled: true,
                        is_secret: true,
                        description: Some("Bearer Token".into()),
                    });
                }
            }
            AuthType::Basic => {
                let user = draft.basic_username.read(cx).value();
                let pass = draft.basic_password.read(cx).value();
                let raw_cred = format!("{user}:{pass}");
                let encoded = simple_base64(raw_cred.as_bytes());
                headers.push(HeaderEntry {
                    name: "Authorization".into(),
                    value: format!("Basic {encoded}"),
                    enabled: true,
                    is_secret: true,
                    description: Some("Basic Authentication".into()),
                });
            }
            AuthType::ApiKey => {
                let key = draft.api_key_name.read(cx).value();
                let val = draft.api_key_value.read(cx).value();
                if !key.is_empty() && !val.is_empty() {
                    headers.push(HeaderEntry {
                        name: key.trim().into(),
                        value: val.trim().into(),
                        enabled: true,
                        is_secret: true,
                        description: Some("API Key".into()),
                    });
                }
            }
        }

        // 3. Body Injection
        let body_str = draft.body.read(cx).value();
        match draft.body_type {
            BodyType::None => {}
            BodyType::Json => {
                if !body_str.is_empty() {
                    if !headers.iter().any(|h| h.name.eq_ignore_ascii_case("content-type")) {
                        headers.push(HeaderEntry {
                            name: "Content-Type".into(),
                            value: "application/json".into(),
                            enabled: true,
                            is_secret: false,
                            description: None,
                        });
                    }
                    request.body = HttpBody::Json { json_content: body_str };
                }
            }
            BodyType::FormData | BodyType::Raw => {
                if !body_str.is_empty() {
                    request.body = HttpBody::Raw {
                        content: body_str,
                        content_type: "text/plain".into(),
                    };
                }
            }
        }

        request.headers = headers;

        let resolver = self
            .environments
            .read(cx)
            .ws()
            .variable_ui
            .resolver()
            .clone();

        let id = draft.id;
        draft.generation += 1;
        let generation = draft.generation;
        draft.message = "Sending…".into();
        draft.response = None;
        draft.response_text.clear();

        let history_method = draft.method.clone();
        let history_url = url_val.clone();

        let task = self
            .runtime
            .spawn(ps_http::desktop::send_desktop_request(request, resolver));
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                            ps_domain::ProtocolRequest::Http(http) => {
                                Some((doc.name.clone(), http.method.clone(), http.url.clone()))
                            }
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
            .filter(|(name, method, url)| {
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
            .w(px(50.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .py_3()
            .gap_3()
            .bg(rgb(theme::ACTIVITY_BAR))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .child(
                // Logo Icon
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(34.))
                    .h(px(34.))
                    .rounded_md()
                    .bg(rgb(theme::SURFACE_ELEVATED))
                    .border_1()
                    .border_color(rgb(theme::ACCENT))
                    .text_color(rgb(theme::ACCENT))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_size(px(14.))
                    .child("PS"),
            )
            .child(div().h(px(10.)))
            // Mode: Collections
            .child(styled_icon_button(
                "nav-collections",
                IconKind::Folder,
                None::<&str>,
                ButtonVariant::Ghost,
                ButtonSize::Medium,
                self.activity_mode == ActivityMode::Collections,
                cx,
                |s, _, cx| {
                    s.activity_mode = ActivityMode::Collections;
                    cx.notify();
                },
            ))
            // Mode: Environments
            .child(styled_icon_button(
                "nav-environments",
                IconKind::Globe,
                None::<&str>,
                ButtonVariant::Ghost,
                ButtonSize::Medium,
                self.activity_mode == ActivityMode::Environments,
                cx,
                |s, _, cx| {
                    s.activity_mode = ActivityMode::Environments;
                    cx.notify();
                },
            ))
            // Mode: History
            .child(styled_icon_button(
                "nav-history",
                IconKind::Clock,
                None::<&str>,
                ButtonVariant::Ghost,
                ButtonSize::Medium,
                self.activity_mode == ActivityMode::History,
                cx,
                |s, _, cx| {
                    s.activity_mode = ActivityMode::History;
                    cx.notify();
                },
            ))
            .child(div().flex_1())
            // Command Palette Quick Trigger
            .child(styled_icon_button(
                "nav-cmd",
                IconKind::Terminal,
                None::<&str>,
                ButtonVariant::Ghost,
                ButtonSize::Medium,
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
            .w(px(260.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(rgb(theme::SIDEBAR))
            .border_r_1()
            .border_color(rgb(theme::BORDER));

        match self.activity_mode {
            ActivityMode::Collections => {
                sidebar = sidebar
                    .child(
                        section_header(
                            "COLLECTIONS",
                            Some(filtered_saved.len()),
                            Some(styled_button(
                                "sidebar-new-request",
                                "+ New",
                                ButtonVariant::Primary,
                                ButtonSize::Small,
                                false,
                                cx,
                                |s, _, cx| s.add_draft("Untitled Request".into(), "GET", "", cx),
                            )),
                        ),
                    )
                    .child(
                        div().px_3().py_2().child(self.sidebar_filter.clone()),
                    );

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
                            .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child("No saved requests"))
                            .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child("Open a workspace or click + New."))
                            .child(styled_button(
                                "sample-apis",
                                "Load Sample APIs",
                                ButtonVariant::Secondary,
                                ButtonSize::Small,
                                false,
                                cx,
                                |s, _, cx| {
                                    s.add_draft("List Users (JSONPlaceholder)".into(), "GET", "https://jsonplaceholder.typicode.com/users", cx);
                                    s.add_draft("Create Post (JSONPlaceholder)".into(), "POST", "https://jsonplaceholder.typicode.com/posts", cx);
                                    s.add_draft("Random Cat Fact".into(), "GET", "https://catfact.ninja/fact", cx);
                                },
                            )),
                    );
                }

                for (index, (name, method, url)) in filtered_saved.into_iter().enumerate() {
                    let req_name = name.clone();
                    let req_method = method.clone();
                    let req_url = url.clone();
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
                                s.add_draft(req_name.clone(), &req_method, &req_url, cx);
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
                sidebar = sidebar.child(section_header("HISTORY", Some(self.history.len()), None));
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
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1p5()
                            .rounded_md()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .child(method_badge(&h_method))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(11.5))
                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                    .child(h_url.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .when_some(item.duration_ms, |this, ms| {
                                        this.child(
                                            div()
                                                .text_size(px(10.))
                                                .text_color(rgb(theme::MUTED))
                                                .child(format!("{ms}ms")),
                                        )
                                    })
                                    .child(
                                        div()
                                            .text_size(px(10.5))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(rgb(if item.status_code.map(|c| c < 400).unwrap_or(false) {
                                                theme::SUCCESS
                                            } else {
                                                theme::DANGER
                                            }))
                                            .child(status_str),
                                    ),
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
            .h(px(46.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .pl(if cfg!(target_os = "macos") { px(82.) } else { px(16.) })
            .pr_4()
            .gap_3()
            .bg(rgb(theme::HEADER))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            // Left: Breadcrumbs
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_size(px(12.5))
                    .child(div().text_color(rgb(theme::MUTED)).child("Workspace  /"))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(theme::TEXT))
                            .child(
                                self.drafts
                                    .get(self.active)
                                    .map(|d| d.title.clone())
                                    .unwrap_or_else(|| "Untitled Request".into()),
                            ),
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
                    .w(px(280.))
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(theme::SURFACE))
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .hover(|s| s.border_color(rgb(theme::BORDER_FOCUS)))
                    .child(icon(IconKind::Search, px(13.), rgb(theme::MUTED)))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.5))
                            .text_color(rgb(theme::MUTED))
                            .child("Search or jump to..."),
                    )
                    .child(shortcut_badge("⌘K"))
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

        // If in full environment studio mode:
        if self.activity_mode == ActivityMode::Environments {
            return div()
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(theme::CANVAS))
                .text_color(rgb(theme::TEXT))
                .font_family(typography::UI_FONT)
                .track_focus(&self.focus)
                .child(top_header)
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(activity_bar)
                        .child(self.environments.clone()),
                );
        }

        // -------------------------------------------------------------------
        // 4. Request Tabs Strip
        // -------------------------------------------------------------------
        let mut tab_strip = div()
            .id("workbench-tabs")
            .h(px(38.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_1()
            .px_3()
            .bg(rgb(theme::HEADER))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
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
                    .px_3()
                    .h(px(32.))
                    .rounded_t_md()
                    .bg(rgb(if active {
                        theme::SURFACE_ELEVATED
                    } else {
                        theme::HEADER
                    }))
                    .border_t_2()
                    .border_color(rgb(if active {
                        theme::ACCENT
                    } else {
                        0x00000000
                    }))
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .child(method_badge(&method))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(if active {
                                gpui::FontWeight::SEMIBOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(rgb(if active {
                                theme::TEXT
                            } else {
                                theme::MUTED
                            }))
                            .child(title),
                    )
                    .when(draft.is_dirty, |el| {
                        el.child(div().text_size(px(8.)).text_color(rgb(theme::ACCENT)).child("●"))
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
                            cx.notify();
                        }
                    })),
            );
        }

        tab_strip = tab_strip.child(
            div()
                .pl_1()
                .child(styled_icon_button(
                    "add-tab-btn",
                    IconKind::Plus,
                    None::<&str>,
                    ButtonVariant::Ghost,
                    ButtonSize::Small,
                    false,
                    cx,
                    |s, _, cx| s.add_draft("Untitled Request".into(), "GET", "", cx),
                )),
        );

        // -------------------------------------------------------------------
        // 5. Unified URL Composer Bar
        // -------------------------------------------------------------------
        let draft = &self.drafts[self.active];
        let current_method = draft.method.clone();

        let composer_bar = div()
            .px_6()
            .pt_4()
            .pb_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
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
                                    .px_3()
                                    .h(px(34.))
                                    .rounded_md()
                                    .bg(rgb(theme::method_bg_color(&current_method)))
                                    .border_1()
                                    .border_color(rgba((theme::method_color(&current_method) << 8) | 0x44))
                                    .text_color(rgb(theme::method_color(&current_method)))
                                    .font_family(typography::MONO_FONT)
                                    .text_size(px(12.5))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(current_method.clone())
                                    .child(icon(IconKind::ChevronDown, px(11.), rgb(theme::method_color(&current_method))))
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
                                                    .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                                                        s.method_selector_open = false;
                                                        cx.notify();
                                                    }))
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
                                                    .children(methods.into_iter().map(|(m, desc)| {
                                                        let is_selected = m == current;
                                                        let m_str = m.to_string();
                                                        div()
                                                            .id(SharedString::from(format!("dropdown-method-{m}")))
                                                            .role(Role::Button)
                                                            .cursor_pointer()
                                                            .flex()
                                                            .items_center()
                                                            .gap_2p5()
                                                            .px_2p5()
                                                            .py_1p5()
                                                            .rounded_md()
                                                            .bg(rgb(if is_selected { theme::HOVER } else { 0x00000000 }))
                                                            .hover(|s| s.bg(rgb(theme::HOVER)))
                                                            .child(method_badge(m))
                                                            .child(
                                                                div()
                                                                    .flex_1()
                                                                    .min_w_0()
                                                                    .text_size(px(11.))
                                                                    .text_color(rgb(theme::TEXT_SECONDARY))
                                                                    .child(desc),
                                                            )
                                                            .when(is_selected, |row| {
                                                                row.child(icon(IconKind::Check, px(13.), rgb(theme::ACCENT)))
                                                            })
                                                            .on_click(cx.listener(move |s, _, _, cx| {
                                                                s.drafts[s.active].method = m_str.clone();
                                                                s.drafts[s.active].is_dirty = true;
                                                                s.method_selector_open = false;
                                                                cx.notify();
                                                            }))
                                                    })),
                                            ),
                                    )
                                    .priority(100),
                                )
                            }),
                    )
                    // URL Input field
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(draft.url.clone()),
                    )
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
                    .h(px(2.5))
                    .rounded_full()
                    .bg(rgb(theme::SURFACE))
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(px(220.))
                            .rounded_full()
                            .bg(rgb(theme::ACCENT))
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
            .border_color(rgb(theme::BORDER))
            .child(styled_button(
                "subtab-params",
                format!("Params ({})", draft.params.len()),
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
                format!("Headers ({})", draft.headers.len()),
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
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(theme::MUTED))
                    .child("⌘+Enter to send"),
            );

        // -------------------------------------------------------------------
        // 7. Request Config Tab Content Area
        // -------------------------------------------------------------------
        let mut config_content = div()
            .id("config-content")
            .h(px(180.))
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
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme::MUTED))
                        .child(div().w(px(30.)).child(""))
                        .child(div().w(px(180.)).child("KEY"))
                        .child(div().flex_1().child("VALUE"))
                        .child(div().flex_1().child("DESCRIPTION"))
                        .child(div().w(px(30.)).child("")),
                );

                for (idx, row) in draft.params.iter().enumerate() {
                    let enabled = row.enabled;
                    params_col = params_col.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(30.))
                                    .child(styled_button(
                                        format!("toggle-param-{idx}"),
                                        if enabled { "☑" } else { "☐" },
                                        ButtonVariant::Ghost,
                                        ButtonSize::Small,
                                        enabled,
                                        cx,
                                        move |s, _, cx| {
                                            if s.active < s.drafts.len() && idx < s.drafts[s.active].params.len() {
                                                s.drafts[s.active].params[idx].enabled = !enabled;
                                                s.sync_params_to_url(cx);
                                            }
                                        },
                                    )),
                            )
                            .child(div().w(px(180.)).child(row.key.clone()))
                            .child(div().flex_1().child(row.value.clone()))
                            .child(div().flex_1().child(row.description.clone()))
                            .child(
                                div()
                                    .w(px(30.))
                                    .child(styled_button(
                                        format!("remove-param-{idx}"),
                                        "✕",
                                        ButtonVariant::Ghost,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        move |s, _, cx| {
                                            if s.active < s.drafts.len() && idx < s.drafts[s.active].params.len() {
                                                s.drafts[s.active].params.remove(idx);
                                                s.sync_params_to_url(cx);
                                            }
                                        },
                                    )),
                            ),
                    );
                }

                params_col = params_col.child(
                    div().pt_1().child(styled_button(
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
                                description: cx.new(|cx| TextInput::new("", "Description", false, cx).compact()),
                                enabled: true,
                            });
                            cx.notify();
                        },
                    )),
                );

                config_content = config_content.child(params_col);
            }
            RequestSubTab::Auth => {
                let auth_type = draft.auth_type;
                let mut auth_col = div().flex().flex_col().gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_size(px(12.)).text_color(rgb(theme::MUTED)).child("Type:"))
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
                                .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child("TOKEN"))
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
                                        .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child("USERNAME"))
                                        .child(draft.basic_username.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child("PASSWORD"))
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
                                        .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child("HEADER KEY"))
                                        .child(draft.api_key_name.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(div().text_size(px(11.)).text_color(rgb(theme::MUTED)).child("VALUE"))
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
                        .child(div().flex_1().child("VALUE"))
                        .child(div().flex_1().child("DESCRIPTION"))
                        .child(div().w(px(30.)).child("")),
                );

                for (idx, row) in draft.headers.iter().enumerate() {
                    let enabled = row.enabled;
                    headers_col = headers_col.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(30.))
                                    .child(styled_button(
                                        format!("toggle-header-{idx}"),
                                        if enabled { "☑" } else { "☐" },
                                        ButtonVariant::Ghost,
                                        ButtonSize::Small,
                                        enabled,
                                        cx,
                                        move |s, _, cx| {
                                            if s.active < s.drafts.len() && idx < s.drafts[s.active].headers.len() {
                                                s.drafts[s.active].headers[idx].enabled = !enabled;
                                                cx.notify();
                                            }
                                        },
                                    )),
                            )
                            .child(div().w(px(220.)).child(row.key.clone()))
                            .child(div().flex_1().child(row.value.clone()))
                            .child(div().flex_1().child(row.description.clone()))
                            .child(
                                div()
                                    .w(px(30.))
                                    .child(styled_button(
                                        format!("remove-header-{idx}"),
                                        "✕",
                                        ButtonVariant::Ghost,
                                        ButtonSize::Small,
                                        false,
                                        cx,
                                        move |s, _, cx| {
                                            if s.active < s.drafts.len() && idx < s.drafts[s.active].headers.len() {
                                                s.drafts[s.active].headers.remove(idx);
                                                cx.notify();
                                            }
                                        },
                                    )),
                            ),
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
                                    description: cx.new(|cx| TextInput::new("", "Description", false, cx).compact()),
                                    enabled: true,
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
                                    description: cx.new(|cx| TextInput::new("", "", false, cx).compact()),
                                    enabled: true,
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
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(styled_button(
                                "toggle-ssl",
                                "SSL Verification: Enabled",
                                ButtonVariant::Secondary,
                                ButtonSize::Small,
                                true,
                                cx,
                                |_, _, _| {},
                            ))
                            .child(styled_button(
                                "toggle-redirects",
                                "Follow Redirects: Enabled",
                                ButtonVariant::Secondary,
                                ButtonSize::Small,
                                true,
                                cx,
                                |_, _, _| {},
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(rgb(theme::MUTED))
                            .child("Request timeout is set to 30,000 ms by default."),
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
            .py_2()
            .flex()
            .items_center()
            .gap_3()
            .bg(rgb(theme::HEADER))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_size(px(12.5))
                    .text_color(rgb(theme::TEXT))
                    .child("Response"),
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
                .child(metric_chip_icon(IconKind::Zap, format!("{} ms", resp.duration_ms)))
                .child(metric_chip_icon(IconKind::HardDrive, format!("{} B", resp.size_bytes)))
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
                            .child(if draft.response_text.is_empty() {
                                "Empty response body".into()
                            } else {
                                draft.response_text.clone()
                            }),
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
                                .child(div().w(px(220.)).font_weight(gpui::FontWeight::BOLD).text_color(rgb(theme::MUTED)).child(k))
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
                        "Ready to Send",
                        if draft.running.is_some() {
                            "Executing request... waiting for network response."
                        } else {
                            "Enter a URL above and press Send (⌘+Enter) to view the response."
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
        let status_bar = div()
            .h(px(28.))
            .flex_shrink_0()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .bg(rgb(theme::ACTIVITY_BAR))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .text_size(px(11.))
            .text_color(rgb(theme::MUTED))
            // Live pulsing beacon dot
            .child(
                div()
                    .w(px(7.))
                    .h(px(7.))
                    .rounded_full()
                    .bg(rgb(theme::SUCCESS))
                    .with_animation(
                        "status-beacon-pulse",
                        Animation::new(Duration::from_millis(1800))
                            .repeat()
                            .with_easing(gpui::ease_in_out),
                        |this, delta| {
                            let alpha = 0.45 + 0.55 * (delta * std::f32::consts::PI).sin();
                            this.opacity(alpha)
                        },
                    ),
            )
            .child("Local Engine")
            .child(div().text_color(rgb(theme::BORDER)).child("|"))
            .child(format!("Environment: {environment_name}"))
            .child(div().text_color(rgb(theme::BORDER)).child("|"))
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
            .child(div().text_color(rgb(theme::BORDER)).child("|"))
            .child(shortcut_badge("⌘K Commands"))
            .child(shortcut_badge("⌘↵ Send"));

        // -------------------------------------------------------------------
        // 10. Assemble Root Layout
        // -------------------------------------------------------------------
        let mut root = div()
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
            .on_action(cx.listener(|s, _: &NewRequest, _, cx| {
                s.add_draft("Untitled Request".into(), "GET", "", cx)
            }))
            .on_action(cx.listener(|s, _: &CloseTab, _, cx| {
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
                s.drafts[s.active].message = "GitHub: github.com/Binary-Brawlers/PacketSmith".into();
                cx.notify();
            }))
            .child(top_header)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(activity_bar)
                    .child(sidebar)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(tab_strip)
                            .child(composer_bar)
                            .child(request_tabs)
                            .child(config_content)
                            .child(response_panel),
                    ),
            )
            .child(status_bar);

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
                                    .child(div().font_weight(gpui::FontWeight::BOLD).text_size(px(13.)).child("Command Palette"))
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
                                        "Send Request (⌘+Enter)",
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
                                        "New Request Tab (⌘+T)",
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
                            .child("Variables available in {{var_name}} expressions for this request."),
                    )
                    .child(
                        styled_button(
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
                        ),
                    ),
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

fn simple_base64(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() { input[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] as usize } else { 0 };

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

    #[test]
    fn preview_bounds_unicode_and_formats_small_json() {
        assert!(preview(br#"{"ok":true}"#).contains('\n'));
        let large = "é".repeat(PREVIEW_CHARS + 1);
        assert!(preview(large.as_bytes()).contains("Preview truncated"));
    }

    #[test]
    fn test_simple_base64() {
        assert_eq!(simple_base64(b"hello"), "aGVsbG8=");
        assert_eq!(simple_base64(b"admin:secret"), "YWRtaW46c2VjcmV0");
    }
}
