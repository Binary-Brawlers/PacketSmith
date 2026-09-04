//! Toast notifications and system alert manager.

use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use ps_domain::ResourceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toast {
    pub id: ResourceId,
    pub level: ToastLevel,
    pub title: String,
    pub message: Option<String>,
    pub duration_ms: u64,
    pub created_at: DateTime<Utc>,
}

impl Toast {
    pub fn new(level: ToastLevel, title: impl Into<String>) -> Self {
        Self {
            id: ResourceId::new(),
            level,
            title: title.into(),
            message: None,
            duration_ms: 4000,
            created_at: Utc::now(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }
}

/// Thread-safe toast and notification queue coordinator.
#[derive(Debug, Clone, Default)]
pub struct NotificationManager {
    active_toasts: Arc<Mutex<Vec<Toast>>>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            active_toasts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push(&self, toast: Toast) {
        if let Ok(mut list) = self.active_toasts.lock() {
            list.push(toast);
        }
    }

    pub fn info(&self, title: impl Into<String>, message: Option<String>) {
        let mut t = Toast::new(ToastLevel::Info, title);
        t.message = message;
        self.push(t);
    }

    pub fn success(&self, title: impl Into<String>, message: Option<String>) {
        let mut t = Toast::new(ToastLevel::Success, title);
        t.message = message;
        self.push(t);
    }

    pub fn error(&self, title: impl Into<String>, message: Option<String>) {
        let mut t = Toast::new(ToastLevel::Error, title);
        t.message = message;
        self.push(t);
    }

    pub fn dismiss(&self, id: &ResourceId) {
        if let Ok(mut list) = self.active_toasts.lock() {
            list.retain(|t| &t.id != id);
        }
    }

    pub fn list(&self) -> Vec<Toast> {
        self.active_toasts
            .lock()
            .map(|list| list.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_manager() {
        let mgr = NotificationManager::new();
        mgr.info("Request Sent", Some("HTTP 200 OK".into()));
        assert_eq!(mgr.list().len(), 1);
        let id = mgr.list()[0].id;
        mgr.dismiss(&id);
        assert_eq!(mgr.list().len(), 0);
    }
}
