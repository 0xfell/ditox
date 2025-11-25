//! Ditox Core Library
//!
//! This crate contains the shared business logic for Ditox clipboard manager,
//! used by both the TUI and GUI frontends.

pub mod actions;
pub mod app;
pub mod clipboard;
pub mod collection;
pub mod config;
pub mod content_type;
pub mod db;
pub mod entry;
pub mod error;
pub mod stats;
pub mod watcher;

// Re-export commonly used types
pub use actions::Action;
pub use app::App;
pub use clipboard::Clipboard;
pub use collection::Collection;
pub use config::Config;
pub use db::Database;
pub use entry::{Entry, EntryType};
pub use error::{DitoxError, Result};
pub use stats::Stats;
pub use watcher::Watcher;
