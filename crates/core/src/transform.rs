//! Plugin / transform hook system for composable IR transformations.

use crate::document::Document;
use crate::error::TransformError;

/// A transform receives a mutable Document and can modify it in any way.
pub trait Transform: Send + Sync {
    fn name(&self) -> &str;
    fn apply(&self, doc: &mut Document) -> Result<(), TransformError>;
}

impl Document {
    pub fn apply_transform(&mut self, transform: &dyn Transform) -> Result<(), TransformError> {
        transform.apply(self)
    }
}
