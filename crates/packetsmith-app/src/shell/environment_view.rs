//! Native environment workspace: selector, variable table, editor and comparison.
use super::{
    environment_input::{self, TextInput},
    AppState, WorkspaceState,
};
use gpui::{prelude::*, *};
use ps_domain::{ResourceId, VariableEntry, VariableType};

actions!(environment_workspace, [NextEnvironment]);

struct RowEditor {
    old_key: Option<String>,
    key: Entity<TextInput>,
    default: Entity<TextInput>,
    current: Entity<TextInput>,
    description: Entity<TextInput>,
    kind: VariableType,
    secret: bool,
    enabled: bool,
    preserve_default: bool,
}

pub struct EnvironmentView {
    state: AppState,
    focus: FocusHandle,
    workspace_path: Entity<TextInput>,
    name: Entity<TextInput>,
    transfer_path: Entity<TextInput>,
    editor: Option<RowEditor>,
    compare: Option<ResourceId>,
    deleting: Option<ResourceId>,
    message: String,
}

impl std::fmt::Debug for EnvironmentView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentView").finish_non_exhaustive()
    }
}

fn input(
    value: &str,
    placeholder: &str,
    secret: bool,
    cx: &mut Context<EnvironmentView>,
) -> Entity<TextInput> {
    cx.new(|cx| TextInput::new(value, placeholder, secret, cx))
}

impl EnvironmentView {
    pub fn new(mut state: AppState, cx: &mut Context<Self>) -> Self {
        let path = std::env::current_dir().unwrap_or_default();
        let mut ws = WorkspaceState::new(path.clone(), "Workspace");
        let message = match ws.scan_resources() {
            Ok(()) => "Select an environment or create one.".into(),
            Err(_) => {
                "Could not load this workspace. Choose a workspace directory and open it.".into()
            }
        };
        state.active_workspace = Some(ws);
        let name = state
            .active_workspace
            .as_ref()
            .and_then(|w| {
                w.active_environment_id
                    .and_then(|id| w.environments.get(id).ok())
            })
            .map(|d| d.name.clone())
            .unwrap_or_default();
        Self {
            state,
            focus: cx.focus_handle(),
            workspace_path: input(&path.to_string_lossy(), "Workspace directory", false, cx),
            name: input(&name, "Environment name", false, cx),
            transfer_path: input("", "Native YAML file path", false, cx),
            editor: None,
            compare: None,
            deleting: None,
            message,
        }
    }

    pub fn bind_keys(cx: &mut App) {
        environment_input::bind_keys(cx);
        cx.bind_keys([
            KeyBinding::new("cmd-shift-e", NextEnvironment, Some("Environments")),
            KeyBinding::new("ctrl-shift-e", NextEnvironment, Some("Environments")),
        ]);
    }

    fn ws(&self) -> &WorkspaceState {
        self.state
            .active_workspace
            .as_ref()
            .expect("workspace initialized")
    }
    fn ws_mut(&mut self) -> &mut WorkspaceState {
        self.state
            .active_workspace
            .as_mut()
            .expect("workspace initialized")
    }
    fn selected(&self) -> Result<ResourceId, String> {
        self.ws()
            .active_environment_id
            .ok_or_else(|| "Select an environment first.".into())
    }
    fn report<T>(
        &mut self,
        result: Result<T, impl std::fmt::Display>,
        success: &str,
        cx: &mut Context<Self>,
    ) {
        self.message = match result {
            Ok(_) => success.into(),
            Err(error) => error.to_string(),
        };
        cx.notify();
    }
    fn sync_name(&mut self, cx: &mut Context<Self>) {
        let name = self
            .selected()
            .ok()
            .and_then(|id| self.ws().environments.get(id).ok())
            .map(|doc| doc.name.clone())
            .unwrap_or_default();
        self.name = input(&name, "Environment name", false, cx);
        self.deleting = None;
    }
    fn can_leave(&mut self, cx: &mut Context<Self>) -> bool {
        if self.editor.is_some() {
            self.message =
                "Save or cancel the variable editor before switching environments.".into();
            cx.notify();
            false
        } else {
            true
        }
    }
    fn switch(&mut self, id: Option<ResourceId>, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        let result = self.ws_mut().select_environment(id);
        self.report(result, "Environment selected.", cx);
        self.sync_name(cx);
    }
    fn next(&mut self, _: &NextEnvironment, _: &mut Window, cx: &mut Context<Self>) {
        if !self.can_leave(cx) {
            return;
        }
        let result = self.ws_mut().next_environment();
        self.report(result, "Environment switched.", cx);
        self.sync_name(cx);
    }
    fn edit(&mut self, entry: Option<VariableEntry>, cx: &mut Context<Self>) {
        if self.editor.is_some() {
            self.message = "Save or cancel the current variable first.".into();
            cx.notify();
            return;
        }
        let old_key = entry.as_ref().map(|v| v.key.clone());
        let v = entry.unwrap_or_else(|| VariableEntry::new("", ""));
        let secret = v.is_secret || v.value_type == VariableType::SecretReference;
        self.editor = Some(RowEditor {
            old_key,
            key: input(&v.key, "Variable key", false, cx),
            default: input(
                if secret { "" } else { &v.value },
                "Shared default",
                secret,
                cx,
            ),
            current: input("", "New local override (empty is allowed)", secret, cx),
            description: input(
                v.description.as_deref().unwrap_or(""),
                "Description",
                false,
                cx,
            ),
            kind: v.value_type,
            secret: v.is_secret,
            enabled: v.enabled,
            preserve_default: true,
        });
        cx.notify();
    }
    fn save_row(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let Ok(id) = self.selected() else {
            return;
        };
        let old_key = editor.old_key.clone();
        let mut value = editor.default.read(cx).value();
        // An untouched masked default must never replace a stored reference.
        if editor.preserve_default
            && value.is_empty()
            && editor.kind == VariableType::SecretReference
        {
            if let Some(old) = old_key.as_ref().and_then(|key| {
                self.ws()
                    .environments
                    .get(id)
                    .ok()?
                    .variables
                    .iter()
                    .find(|v| &v.key == key)
            }) {
                if old.value_type == editor.kind {
                    value = old.value.clone();
                }
            }
        }
        if editor.secret && editor.kind != VariableType::SecretReference && !value.is_empty() {
            self.message = "Raw secrets belong in Current/local value. Shared defaults support vault references only.".into();
            cx.notify();
            return;
        }
        let description = editor.description.read(cx).value();
        let v = VariableEntry {
            key: editor.key.read(cx).value(),
            value,
            value_type: editor.kind,
            is_secret: editor.secret,
            enabled: editor.enabled,
            description: (!description.is_empty()).then_some(description),
        };
        let result = self
            .ws_mut()
            .save_environment_variable(id, old_key.as_deref(), v);
        if result.is_ok() {
            self.editor = None;
        }
        self.report(result, "Variable saved.", cx);
    }
    fn current(&mut self, reset: bool, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let Some(key) = editor.old_key.clone() else {
            self.message = "Save the new variable before setting its current value.".into();
            cx.notify();
            return;
        };
        let Ok(id) = self.selected() else {
            return;
        };
        let stored = self
            .ws()
            .environments
            .get(id)
            .ok()
            .and_then(|d| d.variables.iter().find(|v| v.key == key));
        if stored.is_none_or(|v| {
            v.is_secret != editor.secret
                || v.value_type != editor.kind
                || v.key != editor.key.read(cx).value()
        }) {
            self.message =
                "Save key, type, and secret changes before applying a local value.".into();
            cx.notify();
            return;
        }
        let value = if reset {
            None
        } else {
            Some(editor.current.read(cx).value())
        };
        let (kind, secret) = (editor.kind, editor.secret);
        let result = self
            .ws_mut()
            .set_environment_current_checked(id, &key, value, kind, secret);
        if result.is_ok() {
            if let Some(editor) = &self.editor {
                editor.current.update(cx, |v, _| v.reset());
            }
        }
        self.report(
            result,
            if reset {
                "Using the default value."
            } else {
                "Local override applied."
            },
            cx,
        );
    }
    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let label: SharedString = label.into();
        let action = std::rc::Rc::new(action);
        let keyboard_action = action.clone();
        div()
            .id(id.into())
            .role(Role::Button)
            .aria_label(label.clone())
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgb(0x27272a))
            .border_1()
            .border_color(rgb(0x3f3f46))
            .focusable()
            .tab_index(0)
            .focus(|s| s.border_color(rgb(0x818cf8)))
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0x3f3f46)))
            .child(label)
            .on_click(cx.listener(move |this, _, window, cx| action(this, window, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    keyboard_action(this, window, cx);
                    cx.stop_propagation();
                }
            }))
    }
}

impl Render for EnvironmentView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let docs: Vec<_> = self
            .ws()
            .environments
            .list()
            .iter()
            .map(|d| (d.id, d.name.clone()))
            .collect();
        let active = self.ws().active_environment_id;
        let mut sidebar = div()
            .w(px(220.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .border_r_1()
            .border_color(rgb(0x3f3f46))
            .child(div().text_lg().child("Environments"))
            .child(self.button(
                "none",
                if active.is_none() {
                    "● No environment"
                } else {
                    "No environment"
                },
                cx,
                |s, _, cx| s.switch(None, cx),
            ));
        for (id, name) in &docs {
            let id = *id;
            sidebar = sidebar.child(self.button(
                format!("env-{id}"),
                format!("{}{}", if active == Some(id) { "● " } else { "" }, name),
                cx,
                move |s, _, cx| s.switch(Some(id), cx),
            ));
        }
        let mut body = div().id("environment-body").flex_1().min_w_0().overflow_y_scroll().p_4().flex().flex_col().gap_3()
            .child(div().text_xl().child("Environment variables"))
            .child(div().text_color(rgb(0xa1a1aa)).child("Shared defaults travel with your workspace. Secret current values stay in this session."))
            .child(self.name.clone())
            .child(div().flex().flex_wrap().gap_2()
                .child(self.button("create", "Create", cx, |s, _, cx| {
                    if !s.can_leave(cx) { return; }
                    let name = s.name.read(cx).value(); let result = s.ws_mut().create_environment(&name);
                    s.report(result, "Environment created.", cx); s.sync_name(cx);
                }))
                .child(self.button("rename", "Rename", cx, |s, _, cx| {
                    if let Ok(id) = s.selected() { let name = s.name.read(cx).value(); let result = s.ws_mut().rename_environment(id, &name); s.report(result, "Environment renamed.", cx); } else { s.message = "Select an environment to rename.".into(); cx.notify(); }
                }))
                .child(self.button("clone", "Duplicate / clone", cx, |s, _, cx| {
                    if !s.can_leave(cx) { return; }
                    if let Ok(id) = s.selected() { let name = s.name.read(cx).value(); let result = s.ws_mut().clone_environment(id, &format!("{name} copy")); s.report(result, "Cloned shared defaults; local values were excluded.", cx); s.sync_name(cx); } else { s.message = "Select an environment to clone.".into(); cx.notify(); }
                }))
                .child(self.button("delete", "Delete", cx, |s, _, cx| { if s.can_leave(cx) { s.deleting = s.selected().ok(); if s.deleting.is_none() { s.message = "Select an environment to delete.".into(); } cx.notify(); } })));
        if let Some(id) = self.deleting {
            body = body.child(
                div()
                    .flex()
                    .gap_2()
                    .child("Delete this environment and its local values?")
                    .child(self.button(
                        "confirm-delete",
                        "Delete environment",
                        cx,
                        move |s, _, cx| {
                            let result = s.ws_mut().delete_environment(id);
                            s.report(result, "Environment deleted.", cx);
                            s.sync_name(cx);
                        },
                    ))
                    .child(self.button("cancel-delete", "Cancel", cx, |s, _, cx| {
                        s.deleting = None;
                        cx.notify();
                    })),
            );
        }
        if let Some(id) = active {
            let rows = self.ws().environments.rows(id).unwrap_or_default();
            body = body
                .child(self.button("add-variable", "Add variable", cx, |s, _, cx| {
                    s.edit(None, cx)
                }))
                .child(
                    div().flex().gap_2().text_color(rgb(0xa1a1aa)).children(
                        ["Key / type", "Default", "Current", "Description"]
                            .map(|text| div().flex_1().child(text)),
                    ),
                );
            if rows.is_empty() {
                body = body.child("No variables yet. Add a key and its default value.");
            }
            for row in rows {
                let key = row.key.clone();
                let label = format!(
                    "{} · {:?}{}{}",
                    row.key,
                    row.value_type,
                    if row.is_secret { " · secret" } else { "" },
                    if row.enabled { "" } else { " · disabled" }
                );
                body = body.child(
                    div()
                        .id(SharedString::from(format!("row-{key}")))
                        .flex()
                        .gap_2()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(0x3f3f46))
                        .child(div().flex_1().min_w_0().child(label))
                        .child(div().flex_1().min_w_0().child(row.default_value))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(row.current_value.unwrap_or_else(|| "Use default".into())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(row.description.unwrap_or_default()),
                        )
                        .child(
                            self.button(format!("edit-{key}"), "Edit", cx, move |s, _, cx| {
                                let entry = s
                                    .ws()
                                    .environments
                                    .get(id)
                                    .ok()
                                    .and_then(|d| d.variables.iter().find(|v| v.key == key))
                                    .cloned();
                                s.edit(entry, cx);
                            }),
                        ),
                );
            }
        }
        if let Some(editor) = &self.editor {
            let key = editor.key.clone();
            let default = editor.default.clone();
            let current = editor.current.clone();
            let description = editor.description.clone();
            let kind = editor.kind;
            let secret = editor.secret;
            let enabled = editor.enabled;
            let mut panel = div().flex().flex_col().gap_2().p_3().bg(rgb(0x202024))
                .child("Edit variable — Save commits shared fields; Apply current commits only the local override.")
                .child("Key").child(key).child("Initial/default value (blank preserves an existing secret reference)").child(default)
                .child(self.button("clear-default", "Clear default", cx, |s, _, cx| { if let Some(e) = &mut s.editor { e.preserve_default = false; e.default.update(cx, |v, _| v.reset()); } cx.notify(); }))
                .child("Description").child(description)
                .child(div().flex().flex_wrap().gap_2()
                    .child(self.button("type", format!("Type: {kind:?} (change)"), cx, |s, _, cx| { if let Some(e) = &mut s.editor { e.kind = match e.kind { VariableType::String => VariableType::Number, VariableType::Number => VariableType::Boolean, VariableType::Boolean => VariableType::Json, VariableType::Json => VariableType::SecretReference, VariableType::SecretReference => VariableType::String }; let mask = e.secret || e.kind == VariableType::SecretReference; e.default.update(cx, |v, _| v.password = mask); e.current.update(cx, |v, _| v.password = mask); } cx.notify(); }))
                    .child(self.button("secret", format!("Secret: {secret}"), cx, |s, _, cx| { if let Some(e) = &mut s.editor { e.secret = !e.secret; let mask = e.secret || e.kind == VariableType::SecretReference; e.default.update(cx, |v, _| v.password = mask); e.current.update(cx, |v, _| v.password = mask); } cx.notify(); }))
                    .child(self.button("enabled", format!("Enabled: {enabled}"), cx, |s, _, cx| { if let Some(e) = &mut s.editor { e.enabled = !e.enabled; } cx.notify(); })))
                .child(div().flex().gap_2().child(self.button("save-row", "Save variable", cx, |s, _, cx| s.save_row(cx)))
                    .child(self.button("cancel-row", "Cancel editor", cx, |s, _, cx| { s.editor = None; cx.notify(); })))
                .child("Current/local value").child(current)
                .child(div().flex().gap_2().child(self.button("set-current", "Apply current", cx, |s, _, cx| s.current(false, cx)))
                    .child(self.button("reset-current", "Use default", cx, |s, _, cx| s.current(true, cx))));
            if let Some(key) = self.editor.as_ref().and_then(|e| e.old_key.clone()) {
                panel = panel.child(self.button(
                    "remove-variable",
                    "Delete variable",
                    cx,
                    move |s, _, cx| {
                        if let Ok(id) = s.selected() {
                            let result = s.ws_mut().delete_environment_variable(id, &key);
                            if result.is_ok() {
                                s.editor = None;
                            }
                            s.report(result, "Variable deleted.", cx);
                        }
                    },
                ));
            }
            body = body.child(panel);
        }
        body = body.child(div().text_lg().child("Import / export native YAML")).child(self.transfer_path.clone())
            .child(div().flex().gap_2()
                .child(self.button("import", "Import file", cx, |s, _, cx| {
                    if !s.can_leave(cx) { return; }
                    let path = s.transfer_path.read(cx).value();
                    let result = std::fs::read_to_string(path).map_err(|_| "Could not read the import file.".to_owned())
                        .and_then(|yaml| s.ws_mut().import_environment(&yaml).map_err(|_| "Invalid environment file; check its schema, keys, and declared types.".into()));
                    s.report(result, "Imported with a new identity; raw secret defaults were excluded.", cx); s.sync_name(cx);
                }))
                .child(self.button("export", "Export to new file", cx, |s, _, cx| {
                    use std::io::Write;
                    let result = (|| -> Result<(), String> {
                        let yaml = s.ws().environments.export(s.selected()?).map_err(|_| "Could not serialize the environment.")?;
                        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(s.transfer_path.read(cx).value()).map_err(|_| "Cannot create export file. Choose a new path; existing files are preserved.")?;
                        file.write_all(yaml.as_bytes()).and_then(|_| file.sync_all()).map_err(|_| "Could not finish writing the export file.")
                            .map_err(str::to_owned)
                    })(); s.report(result, "Exported shared defaults without local overrides or raw secrets.", cx);
                })));
        body = body.child(div().text_lg().child("Compare / check production values"))
            .child("Select a reference. The active environment is the target; enabled reference keys are required.");
        let mut comparison = div().flex().flex_wrap().gap_2();
        for (id, name) in docs {
            comparison = comparison.child(self.button(
                format!("compare-{id}"),
                format!(
                    "{}{}",
                    if self.compare == Some(id) { "● " } else { "" },
                    name
                ),
                cx,
                move |s, _, cx| {
                    s.compare = Some(id);
                    cx.notify();
                },
            ));
        }
        body = body.child(comparison);
        if let (Some(target), Some(reference)) = (active, self.compare) {
            if let Ok(differences) = self.ws().environments.diff(reference, target) {
                body = body.child(format!(
                    "{} differences (secret contents are not compared)",
                    differences.len()
                ));
                for difference in differences {
                    let display = |row: Option<ps_workspace::EnvironmentRow>| {
                        row.map(|r| {
                            format!(
                                "default={} · current={} · {:?} · enabled={} · secret={} · {}",
                                r.default_value,
                                r.current_value.unwrap_or_else(|| "Use default".into()),
                                r.value_type,
                                r.enabled,
                                r.is_secret,
                                r.description.unwrap_or_default()
                            )
                        })
                        .unwrap_or_else(|| "Absent".into())
                    };
                    body = body.child(format!(
                        "{}: reference [{}] → target [{}]",
                        difference.key,
                        display(difference.left),
                        display(difference.right)
                    ));
                }
                if let Ok(missing) = self
                    .ws()
                    .environments
                    .missing_values_against(target, reference)
                {
                    body = body.child(if missing.is_empty() { "No missing literal values. Vault availability and unresolved templates are checked separately.".into() } else { format!("Missing production values: {}", missing.join(", ")) });
                }
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x18181b))
            .text_color(rgb(0xf4f4f5))
            .text_size(px(13.))
            .key_context("Environments")
            .track_focus(&self.focus)
            .tab_group()
            .tab_stop(false)
            .on_action(cx.listener(Self::next))
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
                    .child("PacketSmith")
                    .child(format!(
                        "Active: {}",
                        self.selected()
                            .ok()
                            .and_then(|id| self.ws().environments.get(id).ok())
                            .map(|d| d.name.as_str())
                            .unwrap_or("None")
                    ))
                    .child(div().flex_1().child(self.workspace_path.clone()))
                    .child(
                        self.button("open-workspace", "Open workspace", cx, |s, _, cx| {
                            if !s.can_leave(cx) {
                                return;
                            }
                            let path = std::path::PathBuf::from(s.workspace_path.read(cx).value());
                            if !path.is_dir() {
                                s.message = "Choose an existing workspace directory.".into();
                                cx.notify();
                                return;
                            }
                            let mut ws = WorkspaceState::new(path, "Workspace");
                            let result = ws.scan_resources();
                            if result.is_ok() {
                                s.state.active_workspace = Some(ws);
                                s.compare = None;
                                s.sync_name(cx);
                            }
                            s.report(
                                result.map_err(|_| {
                                    "Could not load workspace resources or local state."
                                }),
                                "Workspace opened.",
                                cx,
                            );
                        }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar.id("environment-list").overflow_y_scroll())
                    .child(body),
            )
            .child(
                div()
                    .p_3()
                    .border_t_1()
                    .border_color(rgb(0x3f3f46))
                    .child(format!(
                        "{}   |   Quick switch: Cmd/Ctrl+Shift+E",
                        self.message
                    )),
            )
    }
}

impl Focusable for EnvironmentView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
