//! Shell subsystems: Windowing, notifications, shutdown coordinator, and state hierarchies.

pub mod notifications;
pub mod shutdown;
pub mod state;
pub mod variables;
pub mod window;
pub mod workbench;

pub use notifications::{NotificationManager, Toast, ToastLevel};
pub use shutdown::ShutdownCoordinator;
pub use state::{AppState, RequestTabState, WorkspaceState};
pub use variables::{
    VariableDefinitionTarget, VariableEditorAnalysis, VariableHighlight,
    VariableHighlightState, VariableHoverInfo, VariableTextResource, VariableUiController,
    VariableUiError, VariableUsage,
};
pub use window::{WindowManager, WindowState};
pub use workbench::{CloseTabError, ClosedTabInfo, SplitTree, WorkbenchPane, WorkbenchState};

pub mod environments;
#[cfg(feature = "gpui-ui")]
mod environment_input;
#[cfg(feature = "gpui-ui")]
pub mod environment_view;

#[cfg(feature = "gpui-ui")]
pub mod workbench_view;
#[cfg(feature = "gpui-ui")]
mod typography;
