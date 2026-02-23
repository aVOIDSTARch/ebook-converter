pub mod accessibility;
pub mod config;
pub mod convert;
pub mod cover;
pub mod dedup;
pub mod detect;
pub mod document;
pub mod encoding;
pub mod error;
pub mod library;
pub mod lookup;
pub mod merge;
pub mod meta;
pub mod optimize;
pub mod progress;
pub mod readers;
pub mod rename;
pub mod repair;
pub mod security;
pub mod split;
pub mod stats;
pub mod transform;
pub mod validate;
pub mod watch;
pub mod writers;

pub mod prelude {
    pub use crate::document::*;
    pub use crate::error::*;
}
