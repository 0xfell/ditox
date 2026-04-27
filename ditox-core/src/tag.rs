//! Tag model for ditox.
//!
//! Tags are many-to-many labels for entries. They are orthogonal to
//! collections: an entry can be in at most one collection, but it can have
//! any number of tags.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier.
    pub id: String,
    /// Unique user-visible name.
    pub name: String,
    /// Optional hex color for chip rendering, e.g. `#ff5500`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// When the tag was created.
    pub created_at: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: impl Into<String>, color: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            color,
            created_at: Utc::now(),
        }
    }
}
