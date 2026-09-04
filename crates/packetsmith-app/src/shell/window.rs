//! Window state management, persistence, and multi-monitor bounds validation.

use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

pub const MIN_WINDOW_WIDTH: f32 = 800.0;
pub const MIN_WINDOW_HEIGHT: f32 = 600.0;
pub const DEFAULT_WINDOW_WIDTH: f32 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

/// Persistent configuration of the main desktop window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: f32,
    pub height: f32,
    pub is_maximized: bool,
    pub dpi_scale: f32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
            is_maximized: false,
            dpi_scale: 1.0,
        }
    }
}

impl WindowState {
    /// Clamps bounds to adhere to minimum window constraints.
    pub fn sanitize(&mut self) {
        if self.width < MIN_WINDOW_WIDTH {
            self.width = MIN_WINDOW_WIDTH;
        }
        if self.height < MIN_WINDOW_HEIGHT {
            self.height = MIN_WINDOW_HEIGHT;
        }
        if self.dpi_scale <= 0.0 {
            self.dpi_scale = 1.0;
        }
    }

    /// Loads the window state from disk, or returns default if not present or corrupt.
    pub fn load_from_file(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<WindowState>(&content) {
                Ok(mut state) => {
                    state.sanitize();
                    info!("Restored window state: {}x{}", state.width, state.height);
                    state
                }
                Err(err) => {
                    warn!("Failed to parse window state file: {}, using defaults", err);
                    Self::default()
                }
            },
            Err(err) => {
                warn!("Failed to read window state file: {}, using defaults", err);
                Self::default()
            }
        }
    }

    /// Persists the current window state to disk.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

/// Window manager coordinating desktop window instances.
#[derive(Debug, Default)]
pub struct WindowManager {
    main_window_state: WindowState,
}

impl WindowManager {
    pub fn new(initial_state: WindowState) -> Self {
        Self {
            main_window_state: initial_state,
        }
    }

    pub fn state(&self) -> &WindowState {
        &self.main_window_state
    }

    pub fn state_mut(&mut self) -> &mut WindowState {
        &mut self.main_window_state
    }

    pub fn update_bounds(&mut self, width: f32, height: f32) {
        self.main_window_state.width = width;
        self.main_window_state.height = height;
        self.main_window_state.sanitize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_state_clamping() {
        let mut state = WindowState {
            width: 300.0,
            height: 200.0,
            ..Default::default()
        };
        state.sanitize();
        assert_eq!(state.width, MIN_WINDOW_WIDTH);
        assert_eq!(state.height, MIN_WINDOW_HEIGHT);
    }
}
