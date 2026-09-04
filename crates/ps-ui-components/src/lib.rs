//! UI design tokens, theme palettes, component descriptors, and accessibility primitives for PacketSmith.
//!
//! Designed to remain independent from GPUI rendering details so that themes, layouts,
//! and accessibility hierarchies can be modeled, tested, and shared cleanly.

use serde::{Deserialize, Serialize};

pub mod editor;
pub use editor::*;

// ---------------------------------------------------------------------------
// 1. Scales & Tokens
// ---------------------------------------------------------------------------

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

/// Border width tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderTokens;

impl BorderTokens {
    pub const THIN: f32 = 1.0;
    pub const DEFAULT: f32 = 1.0;
    pub const THICK: f32 = 2.0;
    pub const FOCUS_RING: f32 = 2.0;
}

/// Elevation and Z-Index scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elevation;

impl Elevation {
    pub const BASE: i32 = 0;
    pub const DROPDOWN: i32 = 1000;
    pub const POPOVER: i32 = 1500;
    pub const MODAL_BACKDROP: i32 = 1999;
    pub const MODAL: i32 = 2000;
    pub const TOOLTIP: i32 = 3000;
    pub const TOAST: i32 = 4000;
}

/// Focus ring tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRing {
    pub width: f32,
    pub offset: f32,
}

impl Default for FocusRing {
    fn default() -> Self {
        Self {
            width: 2.0,
            offset: 2.0,
        }
    }
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

    pub const LINE_HEIGHT_TIGHT: f32 = 1.25;
    pub const LINE_HEIGHT_NORMAL: f32 = 1.5;
    pub const LINE_HEIGHT_RELAXED: f32 = 1.75;
}

/// Font weight definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontWeight {
    #[default]
    Regular,
    Medium,
    Semibold,
    Bold,
}

// ---------------------------------------------------------------------------
// 2. Colors & Themes
// ---------------------------------------------------------------------------

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

    /// Calculates relative luminance for WCAG contrast checks.
    pub fn luminance(&self) -> f32 {
        let f = |val: u8| -> f32 {
            let v = val as f32 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(self.r) + 0.7152 * f(self.g) + 0.0722 * f(self.b)
    }

    /// Returns the WCAG contrast ratio against another color.
    pub fn contrast_ratio(&self, other: &Color) -> f32 {
        let l1 = self.luminance();
        let l2 = other.luminance();
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
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
    pub primary_hover: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub focus_ring: Color,
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
            primary_hover: Color::hex(0x4f46e5),
            success: Color::hex(0x22c55e),
            warning: Color::hex(0xeab308),
            error: Color::hex(0xef4444),
            focus_ring: Color::hex(0x818cf8),
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
            primary_hover: Color::hex(0x4338ca),
            success: Color::hex(0x16a34a),
            warning: Color::hex(0xca8a04),
            error: Color::hex(0xdc2626),
            focus_ring: Color::hex(0x6366f1),
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

/// Theme registry providing theme switching and active palette resolution.
#[derive(Debug, Clone)]
pub struct ThemeRegistry {
    mode: ThemeMode,
    is_system_dark: bool,
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self {
            mode: ThemeMode::System,
            is_system_dark: true,
        }
    }
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.mode = mode;
    }

    pub fn set_system_dark(&mut self, is_dark: bool) {
        self.is_system_dark = is_dark;
    }

    pub fn active_palette(&self) -> ThemePalette {
        match self.mode {
            ThemeMode::Dark => ThemePalette::dark(),
            ThemeMode::Light => ThemePalette::light(),
            ThemeMode::System => {
                if self.is_system_dark {
                    ThemePalette::dark()
                } else {
                    ThemePalette::light()
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Core Component Descriptors & Primitives
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonProps {
    pub label: String,
    pub icon: Option<String>,
    pub variant: ButtonVariant,
    pub size: ButtonSize,
    pub disabled: bool,
    pub loading: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputProps {
    pub value: String,
    pub placeholder: String,
    pub is_password: bool,
    pub is_search: bool,
    pub clearable: bool,
    pub disabled: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleProps {
    pub checked: bool,
    pub label: Option<String>,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxProps {
    pub checked: bool,
    pub label: String,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: String,
    pub title: String,
    pub is_active: bool,
    pub is_dirty: bool,
    pub is_pinned: bool,
    pub method_badge: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SplitPaneProps {
    pub orientation: SplitOrientation,
    pub ratio: f32, // 0.0 to 1.0
    pub min_ratio: f32,
    pub max_ratio: f32,
}

impl Default for SplitPaneProps {
    fn default() -> Self {
        Self {
            orientation: SplitOrientation::Horizontal,
            ratio: 0.5,
            min_ratio: 0.15,
            max_ratio: 0.85,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeNodeType {
    Collection,
    Folder,
    Request { method: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    pub id: String,
    pub name: String,
    pub node_type: TreeNodeType,
    pub children: Vec<TreeNode>,
    pub is_expanded: bool,
    pub is_selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmptyState {
    pub title: String,
    pub description: String,
    pub action_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorState {
    pub title: String,
    pub message: String,
    pub retry_label: Option<String>,
}

// ---------------------------------------------------------------------------
// 4. Accessibility
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A11yRole {
    Button,
    Tab,
    TabPanel,
    TextField,
    Checkbox,
    Switch,
    TreeItem,
    Dialog,
    Alert,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_registry() {
        let mut registry = ThemeRegistry::new();
        registry.set_mode(ThemeMode::Dark);
        assert_eq!(registry.active_palette().background, Color::hex(0x18181b));

        registry.set_mode(ThemeMode::Light);
        assert_eq!(registry.active_palette().background, Color::hex(0xffffff));
    }

    #[test]
    fn test_color_contrast() {
        let white = Color::rgb(255, 255, 255);
        let black = Color::rgb(0, 0, 0);
        let ratio = white.contrast_ratio(&black);
        assert!(ratio >= 21.0); // Maximum possible contrast is 21:1
    }
}
