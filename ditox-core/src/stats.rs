//! Usage statistics for ditox clipboard manager
//!
//! This module provides statistics computation and display for clipboard usage patterns.

use serde::Serialize;

/// Usage statistics for the clipboard manager
#[derive(Debug, Serialize)]
pub struct Stats {
    /// Total number of entries in the database
    pub total_entries: usize,
    /// Number of text entries
    pub text_count: usize,
    /// Number of image entries
    pub image_count: usize,
    /// Number of favorite entries
    pub favorites_count: usize,
    /// Database file size in bytes
    pub db_size_bytes: u64,
    /// Total size of images directory in bytes
    pub images_size_bytes: u64,
    /// Top entries by usage count (entry, usage_count)
    pub top_entries: Vec<TopEntry>,
    /// Number of copies today
    pub copies_today: usize,
    /// Number of copies this week
    pub copies_week: usize,
    /// Number of copies this month
    pub copies_month: usize,
    /// Total usage count across all entries
    pub total_usage: u64,
}

/// A top entry with its usage count
#[derive(Debug, Serialize)]
pub struct TopEntry {
    /// The entry ID
    pub id: String,
    /// Preview of the content (truncated)
    pub preview: String,
    /// Entry type (text/image)
    pub entry_type: String,
    /// Number of times this entry has been copied
    pub usage_count: u32,
}

impl Stats {
    /// Format stats for human-readable display
    pub fn display(&self) -> String {
        let mut output = String::new();

        output.push_str("Ditox Usage Statistics\n");
        output.push_str("══════════════════════\n\n");

        // Entry counts
        output.push_str(&format!("Total entries:     {}\n", self.total_entries));
        if self.total_entries > 0 {
            let text_pct = (self.text_count as f64 / self.total_entries as f64 * 100.0) as u32;
            let image_pct = (self.image_count as f64 / self.total_entries as f64 * 100.0) as u32;
            output.push_str(&format!(
                "  Text:            {} ({}%)\n",
                self.text_count, text_pct
            ));
            output.push_str(&format!(
                "  Images:          {} ({}%)\n",
                self.image_count, image_pct
            ));
            output.push_str(&format!("  Favorites:       {}\n", self.favorites_count));
        }

        output.push('\n');

        // Storage
        output.push_str("Storage:\n");
        output.push_str(&format!(
            "  Database:        {}\n",
            format_bytes(self.db_size_bytes)
        ));
        output.push_str(&format!(
            "  Images:          {}\n",
            format_bytes(self.images_size_bytes)
        ));

        output.push('\n');

        // Most copied
        if !self.top_entries.is_empty() {
            output.push_str("Most copied (top 5):\n");
            for (i, entry) in self.top_entries.iter().enumerate() {
                output.push_str(&format!(
                    "  {}. {} (copied {} times)\n",
                    i + 1,
                    truncate_preview(&entry.preview, 35),
                    entry.usage_count
                ));
            }
            output.push('\n');
        }

        // Activity
        output.push_str("Activity:\n");
        output.push_str(&format!(
            "  Today:           {} copies\n",
            self.copies_today
        ));
        output.push_str(&format!("  This week:       {} copies\n", self.copies_week));
        output.push_str(&format!(
            "  This month:      {} copies\n",
            self.copies_month
        ));
        output.push_str(&format!("  Total:           {} copies\n", self.total_usage));

        output
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate preview string for display
fn truncate_preview(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ").replace('\r', "");
    if s.chars().count() <= max_len {
        s
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}
