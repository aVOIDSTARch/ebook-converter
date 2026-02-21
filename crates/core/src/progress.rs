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
