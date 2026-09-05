//! Reusable UI component library for the PacketSmith desktop application.
//!
//! Provides standard badges, method pills, buttons, tabs, table cells,
//! and card layouts adhering to the modern Obsidian/Slate design language.

use std::rc::Rc;
use gpui::{
    div, prelude::*, px, rgb, rgba, Context, KeyDownEvent, Role, SharedString, Window,
};
use super::theme;
use super::typography;

// ---------------------------------------------------------------------------
// 1. Badges & Micro-Pills
// ---------------------------------------------------------------------------

/// Renders a Postman-standard HTTP method pill (e.g. GET in emerald, POST in blue).
pub fn method_badge(method: &str) -> impl IntoElement {
    let method_upper = method.trim().to_uppercase();
    let text_color = theme::method_color(&method_upper);
    let bg_color = theme::method_bg_color(&method_upper);

    div()
        .flex()
        .items_center()
        .justify_center()
        .px_2()
        .py(px(1.5))
        .rounded_sm()
        .bg(rgb(bg_color))
        .border_1()
        .border_color(rgba((text_color << 8) | 0x33))
        .text_color(rgb(text_color))
        .font_family(typography::MONO_FONT)
        .text_size(px(10.5))
        .font_weight(gpui::FontWeight::BOLD)
        .child(method_upper)
}

/// Renders an HTTP response status badge (e.g. "200 OK" in emerald, "404 Not Found" in amber).
pub fn status_badge(status_code: u16, status_text: &str) -> impl IntoElement {
    let text_color = theme::status_color(status_code);
    let bg_color = theme::status_bg_color(status_code);

    div()
        .flex()
        .items_center()
        .gap_1p5()
        .px_2p5()
        .py_1()
        .rounded_md()
        .bg(rgb(bg_color))
        .border_1()
        .border_color(rgba((text_color << 8) | 0x44))
        .text_color(rgb(text_color))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(div().text_size(px(8.)).child("●"))
        .child(format!("{status_code} {status_text}"))
}

/// Renders a performance metric chip (e.g. "124 ms" or "3.2 KB") with a vector SVG icon.
pub fn metric_chip_icon(kind: super::icons::IconKind, label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(rgb(theme::SURFACE))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .font_family(typography::MONO_FONT)
        .text_size(px(11.))
        .child(super::icons::icon(kind, px(12.5), rgb(theme::MUTED)))
        .child(label.into())
}

/// Renders a performance metric chip (e.g. "⚡ 124 ms" or "💾 3.2 KB").
pub fn metric_chip(icon: &str, label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(rgb(theme::SURFACE))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .text_color(rgb(theme::TEXT_SECONDARY))
        .font_family(typography::MONO_FONT)
        .text_size(px(11.))
        .child(div().text_color(rgb(theme::MUTED)).child(icon.to_string()))
        .child(label.into())
}

/// Small count pill (e.g. for number of headers, params, or items).
pub fn count_pill(count: usize) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px_1p5()
        .py(px(0.5))
        .rounded_full()
        .bg(rgb(theme::SURFACE_ELEVATED))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .text_color(rgb(theme::MUTED))
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(count.to_string())
}

/// Small scope badge (e.g. "ENV", "GLOBAL", "VAULT").
pub fn scope_badge(scope: &str) -> impl IntoElement {
    let (bg, text) = match scope.to_uppercase().as_str() {
        "ENV" | "ENVIRONMENT" => (theme::METHOD_POST_BG, theme::METHOD_POST),
        "GLOBAL" => (theme::METHOD_PATCH_BG, theme::METHOD_PATCH),
        "VAULT" => (theme::WARNING_BG, theme::WARNING),
        _ => (theme::SURFACE_ELEVATED, theme::MUTED),
    };

    div()
        .px_1p5()
        .py(px(0.5))
        .rounded_sm()
        .bg(rgb(bg))
        .text_color(rgb(text))
        .font_family(typography::MONO_FONT)
        .text_size(px(9.5))
        .font_weight(gpui::FontWeight::BOLD)
        .child(scope.to_uppercase())
}

// ---------------------------------------------------------------------------
// 2. Button Component System
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

/// Builder for modern interactive buttons with hover, focus, and keyboard activation.
pub fn styled_button<V: 'static>(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
    selected: bool,
    cx: &mut Context<V>,
    action: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> gpui::AnyElement {
    let id: SharedString = id.into();
    let label: SharedString = label.into();
    let action = Rc::new(action);
    let keyboard_action = action.clone();

    let (py, px, text_size, rounded) = match size {
        ButtonSize::Small => (px(2.), px(8.), px(11.), px(4.)),
        ButtonSize::Medium => (px(5.), px(12.), px(12.5), px(6.)),
        ButtonSize::Large => (px(8.), px(16.), px(13.), px(6.)),
    };

    let (bg, text, border, hover_bg) = match variant {
        ButtonVariant::Primary => (
            theme::ACCENT,
            theme::INK,
            theme::ACCENT,
            theme::ACCENT_HOVER,
        ),
        ButtonVariant::Secondary => {
            if selected {
                (theme::ACTIVE, theme::TEXT, theme::BORDER_FOCUS, theme::HOVER)
            } else {
                (theme::SURFACE_ELEVATED, theme::TEXT, theme::BORDER, theme::HOVER)
            }
        }
        ButtonVariant::Ghost => {
            if selected {
                (theme::SURFACE_ELEVATED, theme::TEXT, theme::BORDER, theme::HOVER)
            } else {
                (0x00000000, theme::MUTED, 0x00000000, theme::HOVER)
            }
        }
        ButtonVariant::Danger => (
            theme::DANGER_BG,
            theme::DANGER,
            theme::DANGER_BG,
            0x5c1a1a,
        ),
        ButtonVariant::Tab => {
            if selected {
                (theme::SURFACE_ELEVATED, theme::TEXT, theme::BORDER, theme::HOVER)
            } else {
                (0x00000000, theme::MUTED, 0x00000000, theme::HOVER)
            }
        }
    };

    let font_weight = if variant == ButtonVariant::Primary || selected {
        gpui::FontWeight::SEMIBOLD
    } else {
        gpui::FontWeight::NORMAL
    };

    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label.clone())
        .focusable()
        .tab_index(0)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .gap_1p5()
        .px(px)
        .py(py)
        .rounded(rounded)
        .bg(rgb(bg))
        .border_1()
        .border_color(rgb(border))
        .text_color(rgb(text))
        .text_size(text_size)
        .font_weight(font_weight)
        .hover(move |s| s.bg(rgb(hover_bg)).text_color(rgb(if variant == ButtonVariant::Ghost && !selected { theme::TEXT } else { text })))
        .focus(|s| s.border_color(rgb(theme::BORDER_FOCUS)))
        .child(label)
        .on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                keyboard_action(view, window, cx);
                cx.stop_propagation();
            }
        }))
        .into_any_element()
}

/// Builder for modern interactive buttons with vector SVG icons.
#[allow(clippy::too_many_arguments)]
pub fn styled_icon_button<V: 'static>(
    id: impl Into<SharedString>,
    icon_kind: super::icons::IconKind,
    label: Option<impl Into<SharedString>>,
    variant: ButtonVariant,
    size: ButtonSize,
    selected: bool,
    cx: &mut Context<V>,
    action: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> gpui::AnyElement {
    let id: SharedString = id.into();
    let label_str: Option<SharedString> = label.map(|l| l.into());
    let action = Rc::new(action);
    let keyboard_action = action.clone();

    let (py, px, icon_size, text_size, rounded) = match size {
        ButtonSize::Small => (px(2.5), px(6.), px(13.), px(11.), px(4.)),
        ButtonSize::Medium => (px(5.), px(10.), px(15.), px(12.5), px(6.)),
        ButtonSize::Large => (px(8.), px(14.), px(17.), px(13.), px(6.)),
    };

    let (bg, text, border, hover_bg) = match variant {
        ButtonVariant::Primary => (
            theme::ACCENT,
            theme::INK,
            theme::ACCENT,
            theme::ACCENT_HOVER,
        ),
        ButtonVariant::Secondary => {
            if selected {
                (theme::ACTIVE, theme::TEXT, theme::BORDER_FOCUS, theme::HOVER)
            } else {
                (theme::SURFACE_ELEVATED, theme::TEXT, theme::BORDER, theme::HOVER)
            }
        }
        ButtonVariant::Ghost => {
            if selected {
                (theme::SURFACE_ELEVATED, theme::TEXT, theme::BORDER, theme::HOVER)
            } else {
                (0x00000000, theme::MUTED, 0x00000000, theme::HOVER)
            }
        }
        ButtonVariant::Danger => (
            theme::DANGER_BG,
            theme::DANGER,
            theme::DANGER_BG,
            0x5c1a1a,
        ),
        ButtonVariant::Tab => {
            if selected {
                (theme::SURFACE_ELEVATED, theme::TEXT, theme::BORDER, theme::HOVER)
            } else {
                (0x00000000, theme::MUTED, 0x00000000, theme::HOVER)
            }
        }
    };

    let font_weight = if variant == ButtonVariant::Primary || selected {
        gpui::FontWeight::SEMIBOLD
    } else {
        gpui::FontWeight::NORMAL
    };

    let mut btn = div()
        .id(id)
        .role(Role::Button)
        .focusable()
        .tab_index(0)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .gap_1p5()
        .px(px)
        .py(py)
        .rounded(rounded)
        .bg(rgb(bg))
        .border_1()
        .border_color(rgb(border))
        .text_color(rgb(text))
        .text_size(text_size)
        .font_weight(font_weight)
        .hover(move |s| s.bg(rgb(hover_bg)).text_color(rgb(if variant == ButtonVariant::Ghost && !selected { theme::TEXT } else { text })))
        .focus(|s| s.border_color(rgb(theme::BORDER_FOCUS)))
        .child(super::icons::icon(icon_kind, icon_size, rgb(text)));

    if let Some(ref l) = label_str {
        btn = btn.aria_label(l.clone()).child(l.clone());
    }

    btn.on_click(cx.listener(move |view, _, window, cx| action(view, window, cx)))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                keyboard_action(view, window, cx);
                cx.stop_propagation();
            }
        }))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// 3. Section Header & Empty State Cards
// ---------------------------------------------------------------------------

/// Section header with title, item count badge, and optional right-aligned action element.
pub fn section_header(
    title: impl Into<SharedString>,
    count: Option<usize>,
    right_element: Option<gpui::AnyElement>,
) -> impl IntoElement {
    let mut header = div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(rgb(theme::MUTED))
                        .child(title.into()),
                )
                .when_some(count, |el, c| el.child(count_pill(c))),
        );

    if let Some(right) = right_element {
        header = header.child(right);
    }

    header
}

/// Engaging empty state card with vector SVG icon, title, description, and action button.
#[allow(clippy::too_many_arguments)]
pub fn empty_state_card_icon<V: 'static>(
    icon_kind: super::icons::IconKind,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    action_label: Option<impl Into<SharedString>>,
    action_id: Option<impl Into<SharedString>>,
    cx: &mut Context<V>,
    action: Option<impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static>,
) -> impl IntoElement {
    let mut card = div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .rounded_lg()
        .bg(rgb(theme::SURFACE))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(52.))
                .h(px(52.))
                .rounded_full()
                .bg(rgb(theme::SURFACE_ELEVATED))
                .border_1()
                .border_color(rgb(theme::BORDER))
                .child(super::icons::icon(icon_kind, px(24.), rgb(theme::ACCENT))),
        )
        .child(
            div()
                .text_size(px(15.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(theme::TEXT))
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(12.5))
                .text_color(rgb(theme::MUTED))
                .child(description.into()),
        );

    if let (Some(label), Some(id), Some(action)) = (action_label, action_id, action) {
        card = card.child(
            div().pt_2().child(styled_button(
                id,
                label,
                ButtonVariant::Primary,
                ButtonSize::Medium,
                false,
                cx,
                action,
            )),
        );
    }

    card
}

/// Engaging empty state card with icon, title, description, and action button.
pub fn empty_state_card<V: 'static>(
    icon: &str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    action_label: Option<impl Into<SharedString>>,
    action_id: Option<impl Into<SharedString>>,
    cx: &mut Context<V>,
    action: Option<impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static>,
) -> impl IntoElement {
    let mut card = div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .p_8()
        .rounded_lg()
        .bg(rgb(theme::SURFACE))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(48.))
                .h(px(48.))
                .rounded_full()
                .bg(rgb(theme::SURFACE_ELEVATED))
                .border_1()
                .border_color(rgb(theme::BORDER))
                .text_size(px(22.))
                .text_color(rgb(theme::ACCENT))
                .child(icon.to_string()),
        )
        .child(
            div()
                .text_size(px(15.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(theme::TEXT))
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(12.5))
                .text_color(rgb(theme::MUTED))
                .child(description.into()),
        );

    if let (Some(label), Some(id), Some(action)) = (action_label, action_id, action) {
        card = card.child(
            div().pt_2().child(styled_button(
                id,
                label,
                ButtonVariant::Primary,
                ButtonSize::Medium,
                false,
                cx,
                action,
            )),
        );
    }

    card
}

/// Keyboard shortcut helper badge (e.g. "⌘↵" or "⌘T").
pub fn shortcut_badge(shortcut: &str) -> impl IntoElement {
    div()
        .px_1p5()
        .py(px(0.5))
        .rounded_sm()
        .bg(rgb(theme::SURFACE_ELEVATED))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .text_color(rgb(theme::MUTED))
        .font_family(typography::MONO_FONT)
        .text_size(px(10.))
        .child(shortcut.to_string())
}
