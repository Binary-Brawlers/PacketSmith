//! UI design tokens, theme palettes, and UI component definitions for PacketSmith.
//!
//! Provides design system scales (spacing, radius, typography), semantic color tokens,
//! and protocol-specific visual cues (e.g. HTTP method badge styling).

use serde::{Deserialize, Serialize};

/// Spacing scale in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spacing;

impl Spacing {
    pub const NONE: f32 = 0.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// Border radius scale in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Radius;

impl Radius {
    pub const NONE: f32 = 0.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const FULL: f32 = 9999.0;
}

/// Typography scale font sizes in points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Typography;

impl Typography {
    pub const CAPTION: f32 = 11.0;
    pub const BODY_SM: f32 = 12.0;
    pub const BODY: f32 = 13.0;
    pub const HEADING_SM: f32 = 14.0;
    pub const HEADING_MD: f32 = 16.0;
    pub const HEADING_LG: f32 = 20.0;
}

/// RGBA 32-bit color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }
}

/// Semantic color palette definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePalette {
    pub background: Color,
    pub surface: Color,
    pub surface_elevated: Color,
    pub border: Color,
    pub border_subtle: Color,
    pub text: Color,
    pub text_muted: Color,
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

impl ThemePalette {
    pub fn dark() -> Self {
        Self {
            background: Color::hex(0x18181b),
            surface: Color::hex(0x27272a),
            surface_elevated: Color::hex(0x3f3f46),
            border: Color::hex(0x3f3f46),
            border_subtle: Color::hex(0x27272a),
            text: Color::hex(0xf4f4f5),
            text_muted: Color::hex(0xa1a1aa),
            primary: Color::hex(0x6366f1),
            success: Color::hex(0x22c55e),
            warning: Color::hex(0xeab308),
            error: Color::hex(0xef4444),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::hex(0xffffff),
            surface: Color::hex(0xf4f4f5),
            surface_elevated: Color::hex(0xe4e4e7),
            border: Color::hex(0xd4d4d8),
            border_subtle: Color::hex(0xe4e4e7),
            text: Color::hex(0x09090b),
            text_muted: Color::hex(0x71717a),
            primary: Color::hex(0x4f46e5),
            success: Color::hex(0x16a34a),
            warning: Color::hex(0xca8a04),
            error: Color::hex(0xdc2626),
        }
    }
}

/// Visual color indicators for HTTP methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpMethodColors;

impl HttpMethodColors {
    pub const GET: Color = Color::hex(0x10b981);       // Emerald Green
    pub const POST: Color = Color::hex(0x3b82f6);      // Bright Blue
    pub const PUT: Color = Color::hex(0xf59e0b);       // Amber Orange
    pub const PATCH: Color = Color::hex(0x8b5cf6);     // Purple
    pub const DELETE: Color = Color::hex(0xef4444);    // Crimson Red
    pub const HEAD: Color = Color::hex(0x64748b);      // Slate Gray
    pub const OPTIONS: Color = Color::hex(0x14b8a6);   // Teal
    pub const CUSTOM: Color = Color::hex(0x94a3b8);    // Muted Gray

    pub fn color_for_method(method: &str) -> Color {
        match method.to_uppercase().as_str() {
            "GET" => Self::GET,
            "POST" => Self::POST,
            "PUT" => Self::PUT,
            "PATCH" => Self::PATCH,
            "DELETE" => Self::DELETE,
            "HEAD" => Self::HEAD,
            "OPTIONS" => Self::OPTIONS,
            _ => Self::CUSTOM,
        }
    }
}

/// Application theme mode setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_colors() {
        assert_eq!(HttpMethodColors::color_for_method("GET"), HttpMethodColors::GET);
        assert_eq!(HttpMethodColors::color_for_method("post"), HttpMethodColors::POST);
        assert_eq!(HttpMethodColors::color_for_method("DELETE"), HttpMethodColors::DELETE);
        assert_eq!(HttpMethodColors::color_for_method("UNKNOWN"), HttpMethodColors::CUSTOM);
    }
}
