//! Main desktop application shell for PacketSmith.
//!
//! Orchestrates the application lifecycle, settings loading, local SQLite cache initialization,
//! window persistence, notification routing, and native window bootstrapping via GPUI.

pub mod shell;

use std::path::PathBuf;
use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use crate::shell::{AppState, WindowState};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

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

    struct PacketSmithAppView;

    impl Render for PacketSmithAppView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .size_full()
                .bg(rgb(0x18181b))
                .text_color(rgb(0xf4f4f5))
                .child(
                    div()
                        .h_8()
                        .w_full()
                        .px_4()
                        .flex()
                        .items_center()
                        .border_b_1()
                        .border_color(rgb(0x3f3f46))
                        .child("PacketSmith — API Platform"),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("Ready to craft requests"),
                )
        }
    }

    let win_state = state.window_manager.state();
    let width = win_state.width;
    let height = win_state.height;

    application().run(move |cx: &mut App| {
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

        cx.open_window(options, |_, cx| {
            cx.new(|_| PacketSmithAppView)
        })
        .expect("Failed to open main window");
    });

    Ok(())
}
