//! Graceful application shutdown coordinator.

use std::sync::{Arc, Mutex};
use tracing::info;

type ShutdownHook = Box<dyn Fn() + Send + Sync>;

/// Coordinates clean teardown of background runtimes, SQLite flushes, and window state persistence.
#[derive(Default)]
pub struct ShutdownCoordinator {
    hooks: Arc<Mutex<Vec<ShutdownHook>>>,
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registers a cleanup hook to execute upon application exit.
    pub fn register_hook<F>(&self, hook: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        if let Ok(mut list) = self.hooks.lock() {
            list.push(Box::new(hook));
        }
    }

    /// Executes all registered shutdown hooks in reverse order of registration.
    pub fn shutdown(&self) {
        info!("Initiating graceful application shutdown");
        if let Ok(mut list) = self.hooks.lock() {
            while let Some(hook) = list.pop() {
                hook();
            }
        }
        info!("All shutdown hooks completed cleanly");
    }
}
