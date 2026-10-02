//! Aurora design language for PacketSmith.
//! Deep space-navy canvas, layered luminous surfaces, and a vivid
//! violet-to-blue gradient brand inspired by Linear, Raycast, and Stripe.

use gpui::{linear_color_stop, linear_gradient, rgba, Background};

// --- Surfaces & Canvas ---
pub const CANVAS: u32 = 0x0a0f1e; // Deep space-navy canvas
pub const ACTIVITY_BAR: u32 = 0x070c17; // Narrow leftmost navigation rail
pub const SIDEBAR: u32 = 0x0e1425; // Primary sidebar background
pub const HEADER: u32 = 0x0e1425; // Top navigation bar
pub const SURFACE: u32 = 0x141b30; // Input, panel, and card background
pub const SURFACE_ELEVATED: u32 = 0x1a2340; // Dropdowns, popovers, active tabs
pub const HOVER: u32 = 0x222c4e; // Hover state for interactive items
pub const ACTIVE: u32 = 0x2a3763; // Selected/active row or tab

// --- Borders & Dividers ---
pub const BORDER: u32 = 0x273259; // Luminous dividing borders
pub const BORDER_SUBTLE: u32 = 0x1a2342; // Very subtle interior borders
pub const BORDER_FOCUS: u32 = 0x8b5cf6; // Active input focus ring (Violet)
pub const BORDER_GLOW: u32 = 0x8b5cf655; // Violet glow accents (RGBA)

// --- Typography & Content Colors ---
pub const TEXT: u32 = 0xf2f5fd; // Primary text (near-white)
pub const TEXT_SECONDARY: u32 = 0xa9b4cf; // Secondary text (cool slate)
pub const MUTED: u32 = 0x67769a; // Muted labels, placeholders, breadcrumbs
pub const MUTED_DARK: u32 = 0x46536f; // Very subtle helper text

// --- Brand & Semantic Accents ---
pub const ACCENT: u32 = 0x8b5cf6; // Primary vivid violet action
pub const ACCENT_DEEP: u32 = 0x6d28d9; // Gradient/pressed violet depth
pub const ACCENT_HOVER: u32 = 0x7c3aed; // Violet hover
pub const ACCENT_LIGHT: u32 = 0xa78bfa; // Soft violet highlight
pub const ACCENT_BG: u32 = 0x1d1445; // Violet tint background
pub const SPOTLIGHT: u32 = 0x22d3ee; // Cyan spotlight for live/secondary accents
pub const SPOTLIGHT_BG: u32 = 0x07333d; // Cyan tint background
pub const INK: u32 = 0xffffff; // Text on primary buttons
pub const DANGER: u32 = 0xfb7185; // Danger / Delete rose
pub const DANGER_BG: u32 = 0x471422; // Danger tint background
pub const WARNING: u32 = 0xfbbf24; // Warning amber
pub const WARNING_BG: u32 = 0x45300a; // Warning tint background
pub const SUCCESS: u32 = 0x34d399; // Success emerald
pub const SUCCESS_BG: u32 = 0x06382a; // Success tint background

/// Violet-to-blue brand gradient for primary actions (Send, logo, key CTAs).
/// Angle 90deg sweeps left-to-right.
pub fn brand_gradient() -> Background {
    linear_gradient(
        90.,
        linear_color_stop(rgba(0xa855f7ff), 0.),
        linear_color_stop(rgba(0x3b82f6ff), 1.),
    )
}

/// Brighter hover variant of the brand gradient.
pub fn brand_gradient_hover() -> Background {
    linear_gradient(
        90.,
        linear_color_stop(rgba(0xb46bffff), 0.),
        linear_color_stop(rgba(0x4f8dffff), 1.),
    )
}

/// Pressed/active variant of the brand gradient.
pub fn brand_gradient_active() -> Background {
    linear_gradient(
        90.,
        linear_color_stop(rgba(0x9333eaff), 0.),
        linear_color_stop(rgba(0x2563ebff), 1.),
    )
}

/// Subtle top-lit card sheen for elevated panels.
pub fn card_sheen() -> Background {
    linear_gradient(
        180.,
        linear_color_stop(rgba(0xffffff10), 0.),
        linear_color_stop(rgba(0xffffff00), 1.),
    )
}

// --- HTTP Method Colors (bright modern set) ---
pub const METHOD_GET: u32 = 0x34d399; // Emerald
pub const METHOD_GET_BG: u32 = 0x06382a;
pub const METHOD_POST: u32 = 0x38bdf8; // Sky Blue
pub const METHOD_POST_BG: u32 = 0x0b2c4e;
pub const METHOD_PUT: u32 = 0xfbbf24; // Amber
pub const METHOD_PUT_BG: u32 = 0x45300a;
pub const METHOD_PATCH: u32 = 0xa78bfa; // Violet
pub const METHOD_PATCH_BG: u32 = 0x2e1a5e;
pub const METHOD_DELETE: u32 = 0xfb7185; // Rose Red
pub const METHOD_DELETE_BG: u32 = 0x471422;
pub const METHOD_HEAD: u32 = 0x22d3ee; // Cyan
pub const METHOD_HEAD_BG: u32 = 0x07333d;
pub const METHOD_OPTIONS: u32 = 0xf472b6; // Pink
pub const METHOD_OPTIONS_BG: u32 = 0x43122b;

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
