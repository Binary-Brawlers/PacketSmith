//! Shared desktop palette: neutral charcoal surfaces and a restrained blue accent.
use gpui::{rgb, Background};

pub const CANVAS: u32 = 0x181a1f;
pub const ACTIVITY_BAR: u32 = 0x14161a;
pub const SIDEBAR: u32 = 0x1c1e24;
pub const HEADER: u32 = 0x1c1e24;
pub const SURFACE: u32 = 0x20232a;
pub const SURFACE_ELEVATED: u32 = 0x262930;
pub const HOVER: u32 = 0x2d3039;
pub const ACTIVE: u32 = 0x303b53;
pub const BORDER: u32 = 0x363a43;
pub const BORDER_SUBTLE: u32 = 0x2b2e36;
pub const BORDER_FOCUS: u32 = 0x729bff;
pub const BORDER_GLOW: u32 = 0x729bff55;
pub const TEXT: u32 = 0xe9ebf0;
pub const TEXT_SECONDARY: u32 = 0xb6bcc8;
pub const MUTED: u32 = 0x9098a8;
pub const MUTED_DARK: u32 = 0x697181;
pub const ACCENT: u32 = 0x4c78f5;
pub const ACCENT_DEEP: u32 = 0x426ae0;
pub const ACCENT_HOVER: u32 = 0x638cff;
pub const ACCENT_LIGHT: u32 = 0x91b2ff;
pub const ACCENT_BG: u32 = 0x25334d;
pub const SPOTLIGHT: u32 = 0x79c9dc;
pub const SPOTLIGHT_BG: u32 = 0x20343d;
pub const INK: u32 = 0xffffff;
pub const DANGER: u32 = 0xf18b91;
pub const DANGER_BG: u32 = 0x3d282f;
pub const WARNING: u32 = 0xe9c078;
pub const WARNING_BG: u32 = 0x393226;
pub const SUCCESS: u32 = 0x77d4ab;
pub const SUCCESS_BG: u32 = 0x22382f;

// Solid primary actions keep the visual hierarchy quiet and predictable.
pub fn brand_gradient() -> Background {
    rgb(ACCENT).into()
}
pub fn brand_gradient_hover() -> Background {
    rgb(ACCENT_HOVER).into()
}
pub fn brand_gradient_active() -> Background {
    rgb(ACCENT_DEEP).into()
}
pub fn card_sheen() -> Background {
    rgb(SURFACE_ELEVATED).into()
}

pub const METHOD_GET: u32 = SUCCESS;
pub const METHOD_GET_BG: u32 = SUCCESS_BG;
pub const METHOD_POST: u32 = 0xe9c078;
pub const METHOD_POST_BG: u32 = WARNING_BG;
pub const METHOD_PUT: u32 = 0x91b2ff;
pub const METHOD_PUT_BG: u32 = ACCENT_BG;
pub const METHOD_PATCH: u32 = 0xc2a0ec;
pub const METHOD_PATCH_BG: u32 = 0x342b43;
pub const METHOD_DELETE: u32 = DANGER;
pub const METHOD_DELETE_BG: u32 = DANGER_BG;
pub const METHOD_HEAD: u32 = SPOTLIGHT;
pub const METHOD_HEAD_BG: u32 = SPOTLIGHT_BG;
pub const METHOD_OPTIONS: u32 = 0xdca0c3;
pub const METHOD_OPTIONS_BG: u32 = 0x392b36;

/// Returns the primary text color for a given HTTP method.
pub fn method_color(method: &str) -> u32 {
    match method.trim().to_uppercase().as_str() {
        "GET" => METHOD_GET,
        "POST" => METHOD_POST,
        "PUT" => METHOD_PUT,
        "PATCH" => METHOD_PATCH,
        "DELETE" => METHOD_DELETE,
        "HEAD" => METHOD_HEAD,
        "OPTIONS" => METHOD_OPTIONS,
        _ => MUTED,
    }
}

/// Returns the translucent pill background color for a given HTTP method.
pub fn method_bg_color(method: &str) -> u32 {
    match method.trim().to_uppercase().as_str() {
        "GET" => METHOD_GET_BG,
        "POST" => METHOD_POST_BG,
        "PUT" => METHOD_PUT_BG,
        "PATCH" => METHOD_PATCH_BG,
        "DELETE" => METHOD_DELETE_BG,
        "HEAD" => METHOD_HEAD_BG,
        "OPTIONS" => METHOD_OPTIONS_BG,
        _ => SURFACE,
    }
}

/// Returns the text color for an HTTP response status code.
pub fn status_color(status: u16) -> u32 {
    match status {
        200..=299 => SUCCESS,
        300..=399 => METHOD_POST,
        400..=499 => WARNING,
        500..=599 => DANGER,
        _ => MUTED,
    }
}

/// Returns the background tint for an HTTP response status code badge.
pub fn status_bg_color(status: u16) -> u32 {
    match status {
        200..=299 => SUCCESS_BG,
        300..=399 => METHOD_POST_BG,
        400..=499 => WARNING_BG,
        500..=599 => DANGER_BG,
        _ => SURFACE,
    }
}
