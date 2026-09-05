//! Modern Obsidian/Slate theme palette for PacketSmith.
//! High contrast, quiet surfaces, and vibrant semantic accents inspired by Postman, Linear, and Zed.

// --- Surfaces & Canvas ---
pub const CANVAS: u32 = 0x0c0e14;          // Deep obsidian dark canvas
pub const ACTIVITY_BAR: u32 = 0x090b10;    // Narrow leftmost navigation rail
pub const SIDEBAR: u32 = 0x12151e;         // Primary sidebar background
pub const HEADER: u32 = 0x12151e;          // Top navigation bar
pub const SURFACE: u32 = 0x181c26;         // Input, panel, and card background
pub const SURFACE_ELEVATED: u32 = 0x1f2432;// Dropdowns, popovers, active tabs
pub const HOVER: u32 = 0x252b3b;           // Hover state for interactive items
pub const ACTIVE: u32 = 0x2c3347;          // Selected/active row or tab

// --- Borders & Dividers ---
pub const BORDER: u32 = 0x232838;          // Clean dividing borders
pub const BORDER_SUBTLE: u32 = 0x1a1e2b;   // Very subtle interior borders
pub const BORDER_FOCUS: u32 = 0x6366f1;    // Active input focus ring (Indigo)

// --- Typography & Content Colors ---
pub const TEXT: u32 = 0xf8fafc;            // Primary text (slate-50)
pub const TEXT_SECONDARY: u32 = 0xcad4e0;  // Secondary text (slate-300)
pub const MUTED: u32 = 0x7e8b9f;           // Muted labels, placeholders, breadcrumbs
pub const MUTED_DARK: u32 = 0x4a5568;      // Very subtle helper text

// --- Brand & Semantic Accents ---
pub const ACCENT: u32 = 0x6366f1;          // Primary Indigo action
pub const ACCENT_HOVER: u32 = 0x4f46e5;    // Indigo hover
pub const ACCENT_LIGHT: u32 = 0x818cf8;    // Subtle indigo glow
pub const ACCENT_BG: u32 = 0x1e1b4b;       // Indigo tint background
pub const INK: u32 = 0xffffff;             // Text on primary buttons
pub const DANGER: u32 = 0xef4444;          // Danger / Delete red
pub const DANGER_BG: u32 = 0x3b1414;       // Danger tint background
pub const WARNING: u32 = 0xf59e0b;         // Warning amber
pub const WARNING_BG: u32 = 0x382205;      // Warning tint background
pub const SUCCESS: u32 = 0x10b981;         // Success emerald
pub const SUCCESS_BG: u32 = 0x052e1f;      // Success tint background

// --- HTTP Method Colors (Postman/Insomnia Standard) ---
pub const METHOD_GET: u32 = 0x10b981;      // Emerald Green
pub const METHOD_GET_BG: u32 = 0x063725;
pub const METHOD_POST: u32 = 0x3b82f6;     // Vibrant Blue
pub const METHOD_POST_BG: u32 = 0x0f2a5c;
pub const METHOD_PUT: u32 = 0xf59e0b;      // Amber Orange
pub const METHOD_PUT_BG: u32 = 0x452805;
pub const METHOD_PATCH: u32 = 0xa855f7;    // Purple
pub const METHOD_PATCH_BG: u32 = 0x35124f;
pub const METHOD_DELETE: u32 = 0xef4444;   // Crimson Red
pub const METHOD_DELETE_BG: u32 = 0x421010;
pub const METHOD_HEAD: u32 = 0x06b6d4;     // Cyan
pub const METHOD_HEAD_BG: u32 = 0x082c33;
pub const METHOD_OPTIONS: u32 = 0xec4899;  // Pink
pub const METHOD_OPTIONS_BG: u32 = 0x3b0e25;

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
