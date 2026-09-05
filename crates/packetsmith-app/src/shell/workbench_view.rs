//! Native HTTP workbench. Drafts and responses remain in memory for this slice.
use super::{environment_input::TextInput, environment_view::EnvironmentView, AppState};
use gpui::{
    div, prelude::*, px, rgb, App, Context, Entity, FocusHandle, Focusable, KeyDownEvent, Role,
    SharedString, Window,
};
use ps_http::{HeaderEntry, HttpBody, HttpMethod, HttpRequest, HttpResponse};

gpui::actions!(request_workbench, [SendRequest, NewRequest]);

const PREVIEW_CHARS: usize = 32_000;

struct Draft {
    id: usize,
    generation: usize,
    title: String,
    method: Entity<TextInput>,
    url: Entity<TextInput>,
    headers: Vec<(Entity<TextInput>, Entity<TextInput>)>,
    body: Entity<TextInput>,
    json: bool,
    response: Option<HttpResponse>,
    response_text: String,
    message: String,
    running: Option<tokio::task::AbortHandle>,
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
    show_environments: bool,
    drafts: Vec<Draft>,
    active: usize,
    next_id: usize,
    headers_tab: bool,
    runtime: tokio::runtime::Handle,
}

fn input(value: &str, placeholder: &str, cx: &mut Context<WorkbenchView>) -> Entity<TextInput> {
    cx.new(|cx| TextInput::new(value, placeholder, false, cx))
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
            ]);
        }
    }
    pub fn new(state: AppState, cx: &mut Context<Self>) -> Self {
        let environments = cx.new(|cx| EnvironmentView::new(state, cx));
        cx.observe(&environments, |_, _, cx| cx.notify()).detach();
        let mut view = Self {
            focus: cx.focus_handle(),
            environments,
            show_environments: false,
            drafts: Vec::new(),
            active: 0,
            next_id: 1,
            headers_tab: false,
            runtime: tokio::runtime::Handle::current(),
        };
        view.add_draft("Untitled request".into(), "GET", "", cx);
        view
    }
    fn add_draft(&mut self, title: String, method: &str, url: &str, cx: &mut Context<Self>) {
        let id = self.next_id;
        self.next_id += 1;
        self.drafts.push(Draft {
            id,
            generation: 0,
            title,
            method: input(method, "Method", cx),
            url: input(url, "https://api.example.com/resource", cx),
            headers: vec![],
            body: input("", "Paste raw text or JSON body", cx),
            json: false,
            response: None,
            response_text: String::new(),
            message: "Ready to send".into(),
            running: None,
        });
        self.active = self.drafts.len() - 1;
        self.show_environments = false;
        cx.notify();
    }
    fn send(&mut self, cx: &mut Context<Self>) {
        let draft = &mut self.drafts[self.active];
        if draft.running.is_some() {
            return;
        }
        let mut request = HttpRequest::new(
            draft
                .method
                .read(cx)
                .value()
                .trim()
                .parse::<HttpMethod>()
                .unwrap(),
            draft.url.read(cx).value().trim(),
        );
        request.headers = draft
            .headers
            .iter()
            .map(|(name, value)| HeaderEntry {
                name: name.read(cx).value(),
                value: value.read(cx).value(),
                enabled: true,
                is_secret: false,
                description: None,
            })
            .collect();
        let body = draft.body.read(cx).value();
        if !body.is_empty() {
            request.body = if draft.json {
                HttpBody::Json { json_content: body }
            } else {
                HttpBody::Raw {
                    content: body,
                    content_type: "text/plain".into(),
                }
            };
        }
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
        let task = self
            .runtime
            .spawn(ps_http::desktop::send_desktop_request(request, resolver));
        draft.running = Some(task.abort_handle());
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |view, cx| {
                if let Some(draft) = view.drafts.iter_mut().find(|draft| draft.id == id) {
                    // Cancel clears this handle; a late completion must not replace it.
                    if draft.generation != generation || draft.running.take().is_none() {
                        return;
                    }
                    match result {
                        Ok(Ok(response)) => {
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
                        Ok(Err(error)) => draft.message = error.to_string(),
                        Err(_) => draft.message = "Request cancelled or interrupted.".into(),
                    }
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }
    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let label = label.into();
        let action = std::rc::Rc::new(action);
        let keyboard = action.clone();
        div()
            .id(id.into())
            .role(Role::Button)
            .aria_label(label.clone())
            .focusable()
            .tab_index(0)
            .px_3()
            .py_2()
            .rounded_md()
            .cursor_pointer()
            .bg(rgb(0x27272a))
            .border_1()
            .border_color(rgb(0x3f3f46))
            .hover(|s| s.bg(rgb(0x3f3f46)))
            .focus(|s| s.border_color(rgb(0x818cf8)))
            .child(label)
            .on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    keyboard(view, window, cx);
                    cx.stop_propagation();
                }
            }))
    }
}

impl Render for WorkbenchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = self.environments.read(cx).ws();
        let environment = ws
            .active_environment_id
            .and_then(|id| ws.environments.get(id).ok())
            .map(|doc| doc.name.clone())
            .unwrap_or_else(|| "No environment".into());
        let mut saved = ws
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
        saved.sort_by(|a, b| a.0.cmp(&b.0));
        let root = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x18181b))
            .text_color(rgb(0xf4f4f5))
            .font_family(super::typography::UI_FONT)
            .text_size(px(14.))
            .track_focus(&self.focus)
            .key_context("RequestWorkbench")
            .on_action(cx.listener(|s, _: &SendRequest, _, cx| {
                if !s.show_environments {
                    s.send(cx);
                }
            }))
            .on_action(cx.listener(|s, _: &NewRequest, _, cx| {
                s.add_draft("Untitled request".into(), "GET", "", cx)
            }))
            .tab_group()
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_3()
                    .border_b_1()
                    .border_color(rgb(0x3f3f46))
                    .child(div().text_lg().child("PacketSmith"))
                    .child(self.button("requests", "Requests", cx, |s, _, cx| {
                        s.show_environments = false;
                        cx.notify();
                    }))
                    .child(self.button("environments", "Environments", cx, |s, _, cx| {
                        s.show_environments = true;
                        cx.notify();
                    }))
                    .child(div().flex_1())
                    .child(
                        self.button("active-environment", environment, cx, |s, _, cx| {
                            s.show_environments = true;
                            cx.notify();
                        }),
                    ),
            );
        if self.show_environments {
            return root.child(div().flex_1().min_h_0().child(self.environments.clone()));
        }
        let mut sidebar = div()
            .id("saved-requests")
            .w(px(230.))
            .flex_shrink_0()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .border_r_1()
            .border_color(rgb(0x3f3f46))
            .overflow_y_scroll()
            .child(div().text_color(rgb(0xa1a1aa)).child("WORKSPACE REQUESTS"))
            .child(self.button("new-request", "+ New request", cx, |s, _, cx| {
                s.add_draft("Untitled request".into(), "GET", "", cx)
            }));
        if saved.is_empty() {
            sidebar = sidebar.child(div().mt_3().text_color(rgb(0xa1a1aa)).child("No saved HTTP requests. Open a workspace from Environments, or start a new request."));
        }
        for (index, (name, method, url)) in saved.into_iter().enumerate() {
            sidebar = sidebar.child(self.button(
                format!("saved-{index}"),
                format!("{method}  {name}"),
                cx,
                move |s, _, cx| s.add_draft(name.clone(), &method, &url, cx),
            ));
        }
        let mut tabs = div()
            .id("request-tabs")
            .flex()
            .gap_2()
            .p_3()
            .overflow_x_scroll();
        for (index, draft) in self.drafts.iter().enumerate() {
            let label = format!(
                "{}{}",
                if index == self.active { "● " } else { "" },
                draft.title
            );
            tabs =
                tabs.child(
                    self.button(format!("tab-{}", draft.id), label, cx, move |s, _, cx| {
                        s.active = index;
                        cx.notify();
                    }),
                );
        }
        let draft = &self.drafts[self.active];
        let mut editor = div()
            .id("request-editor")
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .overflow_y_scroll()
            .max_h(px(340.))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(105.)).child(draft.method.clone()))
                    .child(div().flex_1().child(draft.url.clone()))
                    .child(if draft.running.is_some() {
                        self.button("cancel", "Cancel", cx, |s, _, cx| {
                            let draft = &mut s.drafts[s.active];
                            if let Some(task) = draft.running.take() {
                                task.abort();
                            }
                            draft.message = "Request cancelled.".into();
                            cx.notify();
                        })
                        .into_any_element()
                    } else {
                        self.button("send", "Send request", cx, |s, _, cx| s.send(cx))
                            .into_any_element()
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child("Headers")
                    .child(self.button("add-header", "+ Add header", cx, |s, _, cx| {
                        let row = (input("", "Header name", cx), input("", "Header value", cx));
                        s.drafts[s.active].headers.push(row);
                        cx.notify();
                    })),
            );
        for (index, (name, value)) in draft.headers.iter().enumerate() {
            editor = editor.child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().w(px(210.)).child(name.clone()))
                    .child(div().flex_1().child(value.clone()))
                    .child(self.button(
                        format!("remove-header-{index}"),
                        "Remove",
                        cx,
                        move |s, _, cx| {
                            s.drafts[s.active].headers.remove(index);
                            cx.notify();
                        },
                    )),
            );
        }
        editor = editor
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child("Body")
                    .child(self.button(
                        "body-mode",
                        if draft.json { "JSON" } else { "Raw text" },
                        cx,
                        |s, _, cx| {
                            s.drafts[s.active].json = !s.drafts[s.active].json;
                            cx.notify();
                        },
                    )),
            )
            .child(draft.body.clone());
        let response = draft.response.as_ref();
        let headers = response
            .map(|r| {
                let mut rows = r
                    .headers
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>();
                rows.sort();
                rows.join("\n")
            })
            .unwrap_or_default();
        let body = if self.headers_tab {
            headers
        } else {
            draft.response_text.clone()
        };
        let response_panel = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .border_t_1()
            .border_color(rgb(0x3f3f46))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .child("Response")
                    .child(self.button("response-body", "Body", cx, |s, _, cx| {
                        s.headers_tab = false;
                        cx.notify();
                    }))
                    .child(self.button("response-headers", "Headers", cx, |s, _, cx| {
                        s.headers_tab = true;
                        cx.notify();
                    }))
                    .child(div().flex_1())
                    .child(
                        self.button("copy-response", "Copy response", cx, |s, _, cx| {
                            if let Some(response) = &s.drafts[s.active].response {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    String::from_utf8_lossy(&response.body_bytes).into_owned(),
                                ));
                            }
                        }),
                    ),
            )
            .child(
                div()
                    .px_4()
                    .pb_3()
                    .text_color(rgb(0xa1a1aa))
                    .child(draft.message.clone()),
            )
            .child(
                div()
                    .id("response-content")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_4()
                    .child(if response.is_some() {
                        if body.is_empty() {
                            "Empty response".into()
                        } else {
                            body
                        }
                    } else {
                        "Enter a URL and send a request to see the response here.".into()
                    }),
            );
        root.child(
            div().flex().flex_1().min_h_0().child(sidebar).child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(tabs)
                    .child(editor)
                    .child(response_panel),
            ),
        )
        .child(
            div()
                .px_4()
                .py_2()
                .border_t_1()
                .border_color(rgb(0x3f3f46))
                .text_color(rgb(0xa1a1aa))
                .child(
                    "Drafts are session-only · Response limit: 2 MiB · Redirects are not followed",
                ),
        )
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
