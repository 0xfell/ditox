//! Collection model for ditox
//!
//! Collections allow users to organize entries into named groups.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A collection for organizing clipboard entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Unique identifier
    pub id: String,
    /// Collection name (unique, user-visible)
    pub name: String,
    /// Optional hex color for visual distinction (e.g., "#ff5500")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Optional quick access key (1-9)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keybind: Option<char>,
    /// Display position for ordering
    pub position: i32,
    /// When the collection was created
    pub created_at: DateTime<Utc>,
}

impl Collection {
    /// Create a new collection with a given name
    #[allow(dead_code)]
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color: None,
            keybind: None,
            position: 0,
            created_at: Utc::now(),
        }
    }

    /// Create a collection with all fields specified
    pub fn with_options(
        name: String,
        color: Option<String>,
        keybind: Option<char>,
        position: i32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            color,
            keybind,
            position,
            created_at: Utc::now(),
        }
    }
}
