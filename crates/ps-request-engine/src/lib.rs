//! Protocol-independent request execution engine and lifecycle architecture for PacketSmith.
//!
//! Provides the core abstraction `ProtocolExecutor`, execution context, cancellation support,
//! and streaming execution event pipeline.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ps_domain::{RequestDocument, ResourceId};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{mpsc, watch};

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Execution was cancelled")]
    Cancelled,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Timeout after {0} ms")]
    Timeout(u64),
    #[error("Internal execution error: {0}")]
    Internal(String),
}

/// Structured events emitted during the request execution lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    Preparing { timestamp: DateTime<Utc> },
    ResolvingVariables { timestamp: DateTime<Utc> },
    Connecting { url: String, timestamp: DateTime<Utc> },
    UploadProgress { bytes_sent: usize, total_bytes: Option<usize> },
    HeadersReceived { status_code: u16, headers: HashMap<String, String>, timestamp: DateTime<Utc> },
    DownloadProgress { bytes_received: usize },
    Completed { summary: ExecutionSummary },
    Failed { error: String, timestamp: DateTime<Utc> },
    Cancelled { timestamp: DateTime<Utc> },
}

/// Execution summary after a request finishes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub run_id: ResourceId,
    pub request_id: ResourceId,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub bytes_sent: usize,
    pub bytes_received: usize,
    pub completed_at: DateTime<Utc>,
}

/// Stream event sink for streaming execution updates to the UI or CLI reporter.
#[derive(Debug, Clone)]
pub struct EventSink {
    sender: mpsc::Sender<ExecutionEvent>,
}

impl EventSink {
    pub fn new(sender: mpsc::Sender<ExecutionEvent>) -> Self {
        Self { sender }
    }

    pub async fn emit(&self, event: ExecutionEvent) {
        let _ = self.sender.send(event).await;
    }
}

/// Cancellation token wrapper using Tokio watch channels.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    receiver: watch::Receiver<bool>,
}

impl CancellationToken {
    pub fn new() -> (Self, watch::Sender<bool>) {
        let (sender, receiver) = watch::channel(false);
        (Self { receiver }, sender)
    }

    pub fn is_cancelled(&self) -> bool {
        *self.receiver.borrow()
    }

    pub async fn wait_for_cancellation(&mut self) {
        let _ = self.receiver.wait_for(|&is_cancelled| is_cancelled).await;
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        let (token, _) = Self::new();
        token
    }
}

/// Context provided to protocol executors during execution.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub run_id: ResourceId,
    pub variables: Arc<HashMap<String, String>>,
    pub secrets: Arc<HashMap<String, String>>,
    pub cancellation_token: CancellationToken,
}

impl ExecutionContext {
    pub fn new(variables: HashMap<String, String>, secrets: HashMap<String, String>) -> Self {
        Self {
            run_id: ResourceId::new(),
            variables: Arc::new(variables),
            secrets: Arc::new(secrets),
            cancellation_token: CancellationToken::default(),
        }
    }

    pub fn with_cancellation(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = token;
        self
    }
}

/// The core protocol executor trait that all protocol drivers implement.
#[async_trait]
pub trait ProtocolExecutor: Send + Sync {
    /// Executes the specified request within the given execution context and emits lifecycle events.
    async fn execute(
        &self,
        request: &RequestDocument,
        ctx: &ExecutionContext,
        event_sink: &EventSink,
    ) -> Result<ExecutionSummary, ExecutionError>;
}
