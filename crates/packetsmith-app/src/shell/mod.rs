//! Shell subsystems: Windowing, notifications, shutdown coordinator, and state hierarchies.

pub mod notifications;
pub mod shutdown;
pub mod state;
pub mod window;
pub mod workbench;

pub use notifications::{NotificationManager, Toast, ToastLevel};
pub use shutdown::ShutdownCoordinator;
pub use state::{AppState, RequestTabState, WorkspaceState};
pub use window::{WindowManager, WindowState};
pub use workbench::{CloseTabError, ClosedTabInfo, SplitTree, WorkbenchPane, WorkbenchState};
