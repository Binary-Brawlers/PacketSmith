//! Main desktop application shell for PacketSmith.
//!
//! Orchestrates the application lifecycle, settings loading, local SQLite cache initialization,
//! window persistence, notification routing, and native window bootstrapping via GPUI.

pub mod shell;

use std::path::PathBuf;
use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::{FmtSubscriber, util::SubscriberInitExt};
use crate::shell::{AppState, WindowState};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    // Include GPUI's log-facade font/renderer diagnostics in terminal output.
    subscriber.try_init().ok();

    info!("Starting PacketSmith desktop platform v{}", env!("CARGO_PKG_VERSION"));

    // 2. Load persistent window bounds
    let window_state_path = PathBuf::from(".packetsmith/window_state.json");
    let window_state = WindowState::load_from_file(&window_state_path);

    // 3. Initialize application state coordinator
    let state = AppState::new(window_state);
    info!(
        "Application state initialized: theme={:?}, verify_ssl={}, window_bounds={}x{}",
        state.settings.appearance.theme,
        state.settings.network.verify_ssl,
        state.window_manager.state().width,
        state.window_manager.state().height,
    );

    // 4. Register graceful shutdown hook for window persistence
    let window_save_path = window_state_path.clone();
    let current_window_state = state.window_manager.state().clone();
    state.shutdown_coordinator.register_hook(move || {
        let _ = current_window_state.save_to_file(&window_save_path);
    });

    // 5. Send initial welcome toast notification
    state.notification_manager.info(
        "Welcome to PacketSmith",
        Some("Open a workspace or press Cmd+Shift+P for commands".into()),
    );

    // 6. Launch native window shell
    #[cfg(feature = "gpui-ui")]
    {
        info!("Launching GPUI window system");
        launch_gpui(state)?;
    }

    #[cfg(not(feature = "gpui-ui"))]
    {
        info!("PacketSmith core shell initialized (GPUI UI feature flag disabled)");
    }

    Ok(())
}

#[cfg(feature = "gpui-ui")]
fn launch_gpui(state: AppState) -> Result<()> {
    use gpui::{prelude::*, *};
    use gpui_platform::application;

    let win_state = state.window_manager.state();
    let width = win_state.width;
    let height = win_state.height;

    application().run(move |cx: &mut App| {
        cx.activate(true);
        crate::shell::workbench_view::WorkbenchView::bind_keys(cx);
        crate::shell::environment_view::EnvironmentView::bind_keys(cx);
        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("PacketSmith".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| crate::shell::workbench_view::WorkbenchView::new(state, cx));
            window.focus(&view.focus_handle(cx), cx);
            view
        })
        .expect("Failed to open main window");
    });

    Ok(())
}
