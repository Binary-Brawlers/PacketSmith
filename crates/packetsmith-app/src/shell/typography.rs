//! Explicit platform fonts shared by all native surfaces.

/// GPUI's built-in `.SystemUIFont` virtual font identifier dynamically resolves
/// to the platform's native system UI font (`.AppleSystemUIFont` / San Francisco on macOS,
/// `Segoe UI` on Windows, and desktop font fallbacks on Linux).
pub(super) const UI_FONT: &str = ".SystemUIFont";
