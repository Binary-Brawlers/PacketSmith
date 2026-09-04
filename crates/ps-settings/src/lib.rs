//! Settings store, validation, and schema definitions for PacketSmith.
//!
//! Enforces merge precedence: Workspace settings override user settings,
//! which override default configurations.

use std::fs;
use std::path::PathBuf;
use ps_ui_components::ThemeMode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid setting: {0}")]
    Validation(String),
}

/// Master application-wide settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub appearance: AppearanceSettings,
    pub editor: EditorSettings,
    pub network: NetworkSettings,
    pub proxy: ProxySettings,
    pub privacy: PrivacySettings,
}

impl AppSettings {
    /// Validates all setting values to ensure safe runtime constraints.
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.network.timeout_ms == 0 {
            return Err(SettingsError::Validation("Network timeout must be greater than 0 ms".into()));
        }
        if self.network.max_redirects > 50 {
            return Err(SettingsError::Validation("Maximum redirects cannot exceed 50".into()));
        }
        if self.editor.font_size < 8 || self.editor.font_size > 72 {
            return Err(SettingsError::Validation("Editor font size must be between 8 and 72 pt".into()));
        }
        if self.editor.tab_size == 0 || self.editor.tab_size > 8 {
            return Err(SettingsError::Validation("Tab size must be between 1 and 8".into()));
        }
        Ok(())
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
    pub theme: Option<ThemeMode>,
}

/// Coordinator managing user-global settings, workspace overrides, and merge precedence.
#[derive(Debug, Clone)]
pub struct SettingsStore {
    user_settings: AppSettings,
    workspace_override: WorkspaceSettingsOverride,
    effective: AppSettings,
    user_settings_path: PathBuf,
}

impl SettingsStore {
    pub fn new(user_settings_path: PathBuf) -> Self {
        let mut store = Self {
            user_settings: AppSettings::default(),
            workspace_override: WorkspaceSettingsOverride::default(),
            effective: AppSettings::default(),
            user_settings_path,
        };
        store.recompute_effective();
        store
    }

    /// Loads user settings from file if present.
    pub fn load_user_settings(&mut self) -> Result<(), SettingsError> {
        if self.user_settings_path.exists() {
            let content = fs::read_to_string(&self.user_settings_path)?;
            let parsed: AppSettings = serde_json::from_str(&content)?;
            parsed.validate()?;
            self.user_settings = parsed;
            self.recompute_effective();
        }
        Ok(())
    }

    /// Saves the current user settings to disk.
    pub fn save_user_settings(&self) -> Result<(), SettingsError> {
        if let Some(parent) = self.user_settings_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.user_settings)?;
        fs::write(&self.user_settings_path, json)?;
        Ok(())
    }

    /// Applies workspace-level overrides.
    pub fn set_workspace_override(&mut self, overrides: WorkspaceSettingsOverride) {
        self.workspace_override = overrides;
        self.recompute_effective();
    }

    /// Merges default + user + workspace configurations into effective settings.
    fn recompute_effective(&mut self) {
        let mut eff = self.user_settings.clone();

        if let Some(to) = self.workspace_override.default_timeout_ms {
            eff.network.timeout_ms = to;
        }
        if let Some(vssl) = self.workspace_override.verify_ssl {
            eff.network.verify_ssl = vssl;
        }
        if let Some(fr) = self.workspace_override.follow_redirects {
            eff.network.follow_redirects = fr;
        }
        if let Some(th) = self.workspace_override.theme {
            eff.appearance.theme = th;
        }

        self.effective = eff;
    }

    pub fn effective(&self) -> &AppSettings {
        &self.effective
    }

    pub fn user_settings(&self) -> &AppSettings {
        &self.user_settings
    }

    pub fn user_settings_mut(&mut self) -> &mut AppSettings {
        &mut self.user_settings
    }

    /// Resets a specific category to defaults.
    pub fn reset_category(&mut self, category: &str) {
        match category {
            "appearance" => self.user_settings.appearance = AppearanceSettings::default(),
            "editor" => self.user_settings.editor = EditorSettings::default(),
            "network" => self.user_settings.network = NetworkSettings::default(),
            "proxy" => self.user_settings.proxy = ProxySettings::default(),
            "privacy" => self.user_settings.privacy = PrivacySettings::default(),
            _ => {}
        }
        self.recompute_effective();
    }

    /// Resets all settings to factory defaults.
    pub fn reset_all(&mut self) {
        self.user_settings = AppSettings::default();
        self.workspace_override = WorkspaceSettingsOverride::default();
        self.recompute_effective();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_defaults() {
        let settings = AppSettings::default();
        assert!(!settings.privacy.telemetry_enabled);
        assert!(settings.network.verify_ssl);
    }

    #[test]
    fn test_merge_precedence() {
        let mut store = SettingsStore::new(PathBuf::from("/tmp/settings.json"));
        assert_eq!(store.effective().network.timeout_ms, 30_000);

        // User sets timeout to 15,000
        store.user_settings_mut().network.timeout_ms = 15_000;
        store.recompute_effective();
        assert_eq!(store.effective().network.timeout_ms, 15_000);

        // Workspace overrides timeout to 5,000
        store.set_workspace_override(WorkspaceSettingsOverride {
            default_timeout_ms: Some(5_000),
            ..Default::default()
        });
        assert_eq!(store.effective().network.timeout_ms, 5_000);

        // Workspace override takes precedence
        assert_eq!(store.user_settings().network.timeout_ms, 15_000);
    }

    #[test]
    fn test_validation() {
        let mut settings = AppSettings::default();
        settings.network.timeout_ms = 0;
        assert!(settings.validate().is_err());
    }
}
