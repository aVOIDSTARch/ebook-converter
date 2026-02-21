//! Progress reporting trait and events for all operations.

/// Event emitted during long-running operations.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub operation: String,
    pub current: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

/// Trait for receiving progress updates. Implement this to integrate with
/// progress bars, GUI status displays, or FFI callbacks.
pub trait ProgressHandler: Send {
    fn on_progress(&self, event: ProgressEvent);
}

/// A no-op progress handler for when progress reporting is not needed.
pub struct NoopProgress;

impl ProgressHandler for NoopProgress {
    fn on_progress(&self, _event: ProgressEvent) {}
}

/// Helper to emit a progress event if a handler is provided.
pub fn emit_progress(
    handler: Option<&dyn ProgressHandler>,
    operation: &str,
    current: u64,
    total: Option<u64>,
    message: Option<&str>,
) {
    if let Some(h) = handler {
        h.on_progress(ProgressEvent {
            operation: operation.to_string(),
            current,
            total,
            message: message.map(|s| s.to_string()),
        });
    }
}
