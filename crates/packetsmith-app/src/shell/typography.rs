//! Explicit platform fonts shared by all native surfaces.

/// GPUI's built-in `.SystemUIFont` virtual font identifier dynamically resolves
/// to the platform's native system UI font (`.AppleSystemUIFont` / San Francisco on macOS,
/// `Segoe UI` on Windows, and desktop font fallbacks on Linux).
pub(super) const UI_FONT: &str = ".SystemUIFont";

#[cfg(target_os = "macos")]
pub(super) const MONO_FONT: &str = "Menlo";
#[cfg(target_os = "windows")]
pub(super) const MONO_FONT: &str = "Consolas";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) const MONO_FONT: &str = "monospace";
