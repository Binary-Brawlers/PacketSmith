//! Main desktop application shell for PacketSmith.
//!
//! Orchestrates the application lifecycle, settings loading, local SQLite cache initialization,
//! and native window bootstrapping via GPUI.

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use ps_settings::AppSettings;
use ps_storage::CacheStorage;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Global application state root.
#[derive(Debug)]
pub struct AppState {
    pub settings: AppSettings,
    pub storage: Option<Arc<CacheStorage>>,
    pub active_workspace_path: Option<PathBuf>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            settings: AppSettings::default(),
            storage: None,
            active_workspace_path: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();

    info!("Starting PacketSmith desktop platform v{}", env!("CARGO_PKG_VERSION"));

    // 2. Initialize application state
    let state = AppState::new();
    info!("Default settings initialized: theme={:?}, verify_ssl={}",
        state.settings.appearance.theme,
        state.settings.network.verify_ssl
    );

    // 3. Launch native window shell
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
fn launch_gpui(_state: AppState) -> Result<()> {
    use gpui::*;

    struct PacketSmithAppView;

    impl Render for PacketSmithAppView {
        fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
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

    App::new().run(|cx: &mut AppContext| {
        let options = WindowOptions {
            bounds: WindowBounds::Fixed(Bounds {
                origin: Point::default(),
                size: size(px(1280.0), px(800.0)),
            }),
            titlebar: Some(TitlebarOptions {
                title: Some("PacketSmith".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            ..Default::default()
        };

        cx.open_window(options, |cx| {
            cx.new_view(|_cx| PacketSmithAppView)
        })
        .expect("Failed to open main window");
    });

    Ok(())
}
