//! Platform-appropriate application data and workspace locations.
//!
//! Installed builds must never persist state relative to the process working
//! directory: on macOS a Finder launch starts in `/`, and on Windows a Start
//! Menu shortcut can start anywhere. Application state therefore resolves to
//! an OS-specific data directory, overridable with [`DATA_DIR_ENV`] for
//! development runs and tests.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Overrides the application data directory when set to a non-empty value.
pub const DATA_DIR_ENV: &str = "PACKETSMITH_DATA_DIR";

/// Resolves the directory that stores persistent PacketSmith application state.
pub fn data_dir() -> PathBuf {
    let override_dir = std::env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty());
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    data_dir_from(
        std::env::consts::OS,
        override_dir.as_deref(),
        home.as_deref(),
        appdata.as_deref(),
        xdg_data_home.as_deref(),
    )
}

fn data_dir_from(
    platform: &str,
    override_dir: Option<&OsStr>,
    home: Option<&Path>,
    appdata: Option<&Path>,
    xdg_data_home: Option<&Path>,
) -> PathBuf {
    if let Some(dir) = override_dir {
        return PathBuf::from(dir);
    }

    match platform {
        "macos" => home
            .map(|dir| dir.join("Library/Application Support/PacketSmith"))
            .unwrap_or_else(fallback_dir),
        "windows" => appdata
            .map(|dir| dir.join("PacketSmith"))
            .or_else(|| home.map(|dir| dir.join("AppData/Roaming/PacketSmith")))
            .unwrap_or_else(fallback_dir),
        _ => xdg_data_home
            .map(|dir| dir.join("packetsmith"))
            .or_else(|| home.map(|dir| dir.join(".local/share/packetsmith")))
            .unwrap_or_else(fallback_dir),
    }
}

fn fallback_dir() -> PathBuf {
    std::env::temp_dir().join("packetsmith")
}

/// Directory used to initialize workspace selection dialogs.
pub fn default_workspace_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// Persistent main-window bounds for installed builds.
pub fn window_state_path() -> PathBuf {
    data_dir().join("window_state.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(value: &str) -> std::ffi::OsString {
        std::ffi::OsString::from(value)
    }

    #[test]
    fn override_takes_precedence_on_every_platform() {
        let override_dir = os("/custom/data");
        for platform in ["macos", "windows", "linux"] {
            let resolved = data_dir_from(
                platform,
                Some(override_dir.as_os_str()),
                Some(Path::new("/home/dev")),
                Some(Path::new("C:/Users/dev/AppData/Roaming")),
                Some(Path::new("/home/dev/.local/share")),
            );
            assert_eq!(resolved, PathBuf::from("/custom/data"));
        }
    }

    #[test]
    fn macos_uses_application_support() {
        let resolved = data_dir_from("macos", None, Some(Path::new("/Users/dev")), None, None);
        assert_eq!(
            resolved,
            PathBuf::from("/Users/dev/Library/Application Support/PacketSmith")
        );
    }

    #[test]
    fn windows_prefers_appdata_and_falls_back_to_home() {
        let with_appdata = data_dir_from(
            "windows",
            None,
            Some(Path::new("C:/Users/dev")),
            Some(Path::new("C:/Users/dev/AppData/Roaming")),
            None,
        );
        assert_eq!(
            with_appdata,
            PathBuf::from("C:/Users/dev/AppData/Roaming/PacketSmith")
        );

        let without_appdata =
            data_dir_from("windows", None, Some(Path::new("C:/Users/dev")), None, None);
        assert_eq!(
            without_appdata,
            PathBuf::from("C:/Users/dev/AppData/Roaming/PacketSmith")
        );
    }

    #[test]
    fn linux_prefers_xdg_data_home_and_falls_back_to_home() {
        let with_xdg = data_dir_from(
            "linux",
            None,
            Some(Path::new("/home/dev")),
            None,
            Some(Path::new("/home/dev/.local/share")),
        );
        assert_eq!(
            with_xdg,
            PathBuf::from("/home/dev/.local/share/packetsmith")
        );

        let without_xdg = data_dir_from("linux", None, Some(Path::new("/home/dev")), None, None);
        assert_eq!(
            without_xdg,
            PathBuf::from("/home/dev/.local/share/packetsmith")
        );
    }

    #[test]
    fn missing_home_uses_temporary_fallback() {
        let resolved = data_dir_from("linux", None, None, None, None);
        assert_eq!(resolved, std::env::temp_dir().join("packetsmith"));
    }

    #[test]
    fn window_state_lives_in_the_data_directory() {
        let path = window_state_path();
        assert_eq!(path.file_name(), Some(OsStr::new("window_state.json")));
        assert!(path.starts_with(data_dir()));
    }
}
