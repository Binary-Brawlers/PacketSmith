//! Settings store, validation, and schema definitions for PacketSmith.
//!
//! Enforces merge precedence: Workspace settings override user settings,
//! which override default configurations.

use ps_ui_components::ThemeMode;
use serde::{Deserialize, Serialize};

/// Master application-wide settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub appearance: AppearanceSettings,
    pub editor: EditorSettings,
    pub network: NetworkSettings,
    pub proxy: ProxySettings,
    pub privacy: PrivacySettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            appearance: AppearanceSettings::default(),
            editor: EditorSettings::default(),
            network: NetworkSettings::default(),
            proxy: ProxySettings::default(),
            privacy: PrivacySettings::default(),
        }
    }
}

/// Appearance and UI customization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: ThemeMode,
    pub ui_scale: u32,
    pub compact_mode: bool,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            ui_scale: 100,
            compact_mode: false,
        }
    }
}

/// Code and text editor settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_size: u32,
    pub font_family: String,
    pub tab_size: u32,
    pub word_wrap: bool,
    pub line_numbers: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: 13,
            font_family: "JetBrains Mono, Menlo, monospace".to_string(),
            tab_size: 2,
            word_wrap: true,
            line_numbers: true,
        }
    }
}

/// Global networking and transport defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub verify_ssl: bool,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            follow_redirects: true,
            max_redirects: 10,
            verify_ssl: true,
        }
    }
}

/// Proxy routing configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxySettings {
    pub enabled: bool,
    pub use_system_proxy: bool,
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub no_proxy: Vec<String>,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            use_system_proxy: true,
            http_proxy: None,
            https_proxy: None,
            no_proxy: vec!["localhost".to_string(), "127.0.0.1".to_string()],
        }
    }
}

/// Privacy and telemetry preferences (Local-First Guarantee).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Strictly false by default. Telemetry is opt-in only.
    pub telemetry_enabled: bool,
    /// Whether crash dumps may be generated locally.
    pub local_crash_reports: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            telemetry_enabled: false,
            local_crash_reports: true,
        }
    }
}

/// Workspace-level overrides.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WorkspaceSettingsOverride {
    pub default_timeout_ms: Option<u64>,
    pub verify_ssl: Option<bool>,
    pub follow_redirects: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_defaults() {
        let settings = AppSettings::default();
        // Crucial invariant: telemetry MUST be disabled by default
        assert!(!settings.privacy.telemetry_enabled);
        assert!(settings.network.verify_ssl);
    }
}
