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
gpui::actions!(
    packetsmith_main,
    [
        Quit,
        About,
        Hide,
        HideOthers,
        ShowAll,
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
    ]
);

#[cfg(feature = "gpui-ui")]
fn launch_gpui(state: AppState) -> Result<()> {
    use gpui::{prelude::*, *};
    use gpui_platform::application;

    let win_state = state.window_manager.state();
    let min_width = 1020.0f32;
    let min_height = 640.0f32;
    let width = win_state.width.max(min_width);
    let height = win_state.height.max(min_height);

    application().run(move |cx: &mut App| {
        cx.activate(true);
        crate::shell::workbench_view::WorkbenchView::bind_keys(cx);
        crate::shell::environment_view::EnvironmentView::bind_keys(cx);

        // macOS system action handlers
        #[cfg(target_os = "macos")]
        {
            cx.on_action(|_: &Hide, cx| cx.hide());
            cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
            cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        }
        cx.on_action(|_: &Quit, cx| cx.quit());

        // Native application menu bar
        let menus = vec![
            Menu {
                name: "PacketSmith".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("About PacketSmith", About),
                    MenuItem::separator(),
                    #[cfg(target_os = "macos")]
                    MenuItem::os_submenu("Services", gpui::SystemMenuType::Services),
                    #[cfg(target_os = "macos")]
                    MenuItem::separator(),
                    #[cfg(target_os = "macos")]
                    MenuItem::action("Hide PacketSmith", Hide),
                    #[cfg(target_os = "macos")]
                    MenuItem::action("Hide Others", HideOthers),
                    #[cfg(target_os = "macos")]
                    MenuItem::action("Show All", ShowAll),
                    #[cfg(target_os = "macos")]
                    MenuItem::separator(),
                    MenuItem::action("Quit PacketSmith", Quit),
                ],
            },
            Menu {
                name: "File".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("New Request Tab", crate::shell::workbench_view::NewRequest),
                    MenuItem::action("Close Tab", crate::shell::workbench_view::CloseTab),
                    MenuItem::separator(),
                    MenuItem::action("Send Request", crate::shell::workbench_view::SendRequest),
                    MenuItem::action("Beautify JSON Body", crate::shell::workbench_view::BeautifyJson),
                ],
            },
            Menu {
                name: "Edit".into(),
                disabled: false,
                items: vec![
                    MenuItem::os_action("Undo", Undo, OsAction::Undo),
                    MenuItem::os_action("Redo", Redo, OsAction::Redo),
                    MenuItem::separator(),
                    MenuItem::os_action("Cut", Cut, OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, OsAction::Paste),
                    MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                ],
            },
            Menu {
                name: "View".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("Collections", crate::shell::workbench_view::SelectCollections),
                    MenuItem::action("Environments", crate::shell::workbench_view::SelectEnvironments),
                    MenuItem::action("History", crate::shell::workbench_view::SelectHistory),
                    MenuItem::separator(),
                    MenuItem::action("Command Palette...", crate::shell::workbench_view::CommandPalette),
                ],
            },
            Menu {
                name: "Help".into(),
                disabled: false,
                items: vec![
                    MenuItem::action("Documentation", crate::shell::workbench_view::OpenDocs),
                    MenuItem::action("GitHub Repository", crate::shell::workbench_view::OpenGitHub),
                ],
            },
        ];
        cx.set_menus(menus);

        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(min_width), px(min_height))),
            titlebar: Some(TitlebarOptions {
                title: Some("PacketSmith".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.), px(14.))),
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
