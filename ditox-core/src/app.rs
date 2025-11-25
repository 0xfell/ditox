use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::db::Database;
use crate::entry::Entry;
use crate::error::Result;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher};
use regex::RegexBuilder;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Number of entries per page
const PAGE_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    EditNote,
    Confirm,
}

/// Action pending confirmation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    /// Delete the currently selected entry
    DeleteSelected,
    /// Delete all entries (clear history)
    ClearAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Fuzzy matching (default) - forgiving, typo-tolerant search
    Fuzzy,
    /// Regex matching - precise pattern matching
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewMode {
    /// Text wraps at pane width (default)
    #[default]
    Wrap,
    /// Horizontal scroll for long lines
    Scroll,
    /// First N lines, "... X more lines" indicator
    Truncate,
    /// Hex dump view for binary/special content
    Hex,
    /// Show escape sequences, control chars visible
    Raw,
}

impl PreviewMode {
    /// Get the display label for the mode
    pub fn label(&self) -> &'static str {
        match self {
            PreviewMode::Wrap => "wrap",
            PreviewMode::Scroll => "scroll",
            PreviewMode::Truncate => "truncate",
            PreviewMode::Hex => "hex",
            PreviewMode::Raw => "raw",
        }
    }

    /// Cycle to the next preview mode
    pub fn next(&self) -> Self {
        match self {
            PreviewMode::Wrap => PreviewMode::Scroll,
            PreviewMode::Scroll => PreviewMode::Truncate,
            PreviewMode::Truncate => PreviewMode::Hex,
            PreviewMode::Hex => PreviewMode::Raw,
            PreviewMode::Raw => PreviewMode::Wrap,
        }
    }
}

/// Tab filter for filtering entries in the TUI
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabFilter {
    /// Show all entries (no filter)
    All,
    /// Show only text entries
    Text,
    /// Show only image entries
    Images,
    /// Show only favorite entries
    Favorites,
    /// Show entries created today
    Today,
    /// Show entries in a specific collection
    #[allow(dead_code)]
    Collection(String), // collection_id
}

impl TabFilter {
    /// Get the display label for the tab
    pub fn label(&self) -> String {
        match self {
            TabFilter::All => "All".to_string(),
            TabFilter::Text => "Text".to_string(),
            TabFilter::Images => "Images".to_string(),
            TabFilter::Favorites => "Favorites".to_string(),
            TabFilter::Today => "Today".to_string(),
            TabFilter::Collection(name) => name.clone(),
        }
    }

    /// Get the DB filter string and optional collection_id
    pub fn db_filter(&self) -> (&'static str, Option<&str>) {
        match self {
            TabFilter::All => ("all", None),
            TabFilter::Text => ("text", None),
            TabFilter::Images => ("image", None),
            TabFilter::Favorites => ("favorite", None),
            TabFilter::Today => ("today", None),
            TabFilter::Collection(id) => ("collection", Some(id.as_str())),
        }
    }
}

pub struct App {
    /// Currently loaded entries for the current page
    pub entries: Vec<Entry>,
    /// Indices into entries after filtering/sorting (for search results)
    pub filtered: Vec<usize>,
    /// Currently selected index in filtered list (within current page)
    pub selected: usize,
    pub input_mode: InputMode,
    pub search_query: String,
    /// Current search mode (fuzzy or regex)
    pub search_mode: SearchMode,
    /// Regex error message (if regex is invalid)
    pub regex_error: Option<String>,
    pub show_preview: bool,
    pub show_expanded: bool,
    pub show_help: bool,
    pub should_quit: bool,
    pub should_copy_and_quit: bool,
    pub message: Option<String>,
    /// Timestamp when message was set (for timeout)
    pub message_time: Option<Instant>,
    /// Current terminal height for page navigation calculations
    pub terminal_height: u16,
    /// Match indices for each filtered entry (entry_idx -> char indices that matched)
    pub match_indices: HashMap<usize, Vec<u32>>,
    /// Multi-select mode enabled
    pub multi_select_mode: bool,
    /// Set of selected entry indices (indices into filtered, not entries)
    pub multi_selected: HashSet<usize>,
    /// Note input buffer (for EditNote mode)
    pub note_input: String,
    /// Entry ID being edited (for EditNote mode)
    pub editing_entry_id: Option<String>,
    /// Current preview pane mode
    pub preview_mode: PreviewMode,
    /// Horizontal scroll offset for Scroll preview mode
    pub preview_scroll_offset: usize,
    /// Quick snippet slots (1-9) - entry IDs for quick access
    pub snippet_slots: [Option<String>; 9],
    /// Show snippets bar (can be toggled off for narrow terminals)
    pub show_snippets: bool,
    /// Tab filters available
    pub tabs: Vec<TabFilter>,
    /// Active tab index
    pub active_tab: usize,
    /// Show tabs bar
    pub show_tabs: bool,
    /// Show line numbers in preview
    pub show_line_numbers: bool,
    /// Last refresh timestamp for status bar display
    pub last_refresh: Instant,
    /// Action pending confirmation (for delete confirmations)
    pub pending_confirm: Option<ConfirmAction>,

    // Pagination state
    /// Total count of entries in database (or matching search query)
    pub total_count: usize,
    /// Current page number (0-indexed)
    pub current_page: usize,

    db: Database,
    config: Config,
    matcher: Matcher,
}

impl App {
    pub fn new(db: Database, config: Config) -> Result<Self> {
        // Load first page
        let total_count = db.count()?;
        let entries = db.get_page(0, PAGE_SIZE)?;
        let filtered: Vec<usize> = (0..entries.len()).collect();

        Ok(Self {
            entries,
            filtered,
            selected: 0,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_mode: SearchMode::Fuzzy,
            regex_error: None,
            show_preview: config.ui.show_preview,
            show_expanded: false,
            show_help: false,
            should_quit: false,
            should_copy_and_quit: false,
            message: None,
            message_time: None,
            terminal_height: 24, // Default, will be updated on first draw
            match_indices: HashMap::new(),
            multi_select_mode: false,
            multi_selected: HashSet::new(),
            note_input: String::new(),
            editing_entry_id: None,
            preview_mode: PreviewMode::default(),
            preview_scroll_offset: 0,
            snippet_slots: [None, None, None, None, None, None, None, None, None],
            show_snippets: true,
            tabs: vec![
                TabFilter::All,
                TabFilter::Text,
                TabFilter::Images,
                TabFilter::Favorites,
                TabFilter::Today,
            ],
            active_tab: 0,
            show_tabs: true,
            show_line_numbers: false,
            last_refresh: Instant::now(),
            pending_confirm: None,
            total_count,
            current_page: 0,
            db,
            config,
            matcher: Matcher::new(MatcherConfig::DEFAULT),
        })
    }

    /// Reload entries for current page (used for refresh and after modifications)
    pub fn reload_entries(&mut self) -> Result<()> {
        self.last_refresh = Instant::now();

        // Get filter for current tab
        let filter = self.active_tab_filter().clone();
        let (filter_str, collection_id) = filter.db_filter();

        if self.search_query.is_empty() {
            // Get filtered count for correct pagination
            self.total_count = self.db.count_filtered(filter_str, collection_id)?;

            // Ensure current page is still valid
            let total_pages = self.total_pages();
            if self.current_page >= total_pages && total_pages > 0 {
                self.current_page = total_pages - 1;
            }

            // Load current page with filter
            let offset = self.current_page * PAGE_SIZE;
            self.entries = self.db.get_page_filtered(offset, PAGE_SIZE, filter_str, collection_id)?;
            self.filtered = (0..self.entries.len()).collect();
            self.match_indices.clear();
        } else {
            // With search: reload search results (filtering handled in search)
            self.load_search_results()?;
            // Apply tab filter to search results (in-memory since search is already in-memory)
            self.filter_by_tab(&filter);
        }

        // Adjust selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }

        Ok(())
    }

    /// Load a specific page
    fn load_page(&mut self, page: usize) -> Result<()> {
        let total_pages = self.total_pages();
        if page >= total_pages && total_pages > 0 {
            return Ok(()); // Invalid page
        }

        // Get filter for current tab
        let filter = self.active_tab_filter().clone();
        let (filter_str, collection_id) = filter.db_filter();

        self.current_page = page;
        let offset = page * PAGE_SIZE;
        self.entries = self.db.get_page_filtered(offset, PAGE_SIZE, filter_str, collection_id)?;
        self.filtered = (0..self.entries.len()).collect();
        self.match_indices.clear();
        self.selected = 0; // Reset selection to top of new page
        self.multi_selected.clear(); // Clear multi-selection on page change

        Ok(())
    }

    /// Get total number of pages
    pub fn total_pages(&self) -> usize {
        if self.total_count == 0 {
            1
        } else {
            (self.total_count + PAGE_SIZE - 1) / PAGE_SIZE
        }
    }

    /// Get current page number (1-indexed for display)
    pub fn display_page(&self) -> usize {
        self.current_page + 1
    }

    /// Load search results using DB pre-filtering + in-memory matching
    fn load_search_results(&mut self) -> Result<()> {
        // Use DB LIKE to pre-filter, then apply search mode specific matching
        let max_search_results = self.config.general.max_entries;
        self.entries = self.db.search_entries(&self.search_query, max_search_results)?;
        self.total_count = self.entries.len();
        self.current_page = 0; // Reset to first page for search results

        // Apply search mode specific filtering
        match self.search_mode {
            SearchMode::Fuzzy => self.apply_fuzzy_filter(),
            SearchMode::Regex => self.apply_regex_filter(),
        }

        Ok(())
    }

    /// Filter entries - for search, loads from DB; for no search, shows loaded entries
    pub fn filter_entries(&mut self) {
        self.match_indices.clear();

        if self.search_query.is_empty() {
            // No search: show all currently loaded entries
            self.filtered = (0..self.entries.len()).collect();
            // Reset to first page when clearing search
            if let Ok(count) = self.db.count() {
                self.total_count = count;
            }
        } else {
            // With search: load from DB with LIKE pre-filter, then fuzzy match
            if let Err(e) = self.load_search_results() {
                tracing::error!("Failed to load search results: {}", e);
                self.filtered = Vec::new();
            }
        }

        // Adjust selection
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    /// Apply fuzzy matching on already-loaded entries (for scoring and highlighting)
    fn apply_fuzzy_filter(&mut self) {
        self.match_indices.clear();

        if self.search_query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
            return;
        }

        let pattern = Pattern::parse(
            &self.search_query,
            CaseMatching::Ignore,
            Normalization::Smart,
        );

        let mut matches: Vec<(usize, u32, Vec<u32>)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let haystack = match entry.entry_type {
                    crate::entry::EntryType::Text => &entry.content,
                    crate::entry::EntryType::Image => &entry.content,
                };
                let mut buf = Vec::new();
                let haystack_str = nucleo_matcher::Utf32Str::new(haystack, &mut buf);

                // Get score and indices
                let mut indices = Vec::new();
                let score =
                    pattern.indices(haystack_str, &mut self.matcher, &mut indices)?;

                Some((idx, score, indices))
            })
            .collect();

        // Sort by score descending
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        // Store filtered indices and match positions
        self.filtered = matches.iter().map(|(idx, _, _)| *idx).collect();

        for (idx, _, indices) in matches {
            if !indices.is_empty() {
                self.match_indices.insert(idx, indices);
            }
        }

        // Adjust selection
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    /// Apply regex matching on already-loaded entries
    fn apply_regex_filter(&mut self) {
        self.match_indices.clear();
        self.regex_error = None;

        if self.search_query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
            return;
        }

        // Build case-insensitive regex
        let regex = match RegexBuilder::new(&self.search_query)
            .case_insensitive(true)
            .build()
        {
            Ok(re) => re,
            Err(e) => {
                // Invalid regex - show error and return empty results
                self.regex_error = Some(format!("Invalid regex: {}", e));
                self.filtered = Vec::new();
                return;
            }
        };

        // Filter entries that match the regex
        let matches: Vec<(usize, Vec<u32>)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let content = match entry.entry_type {
                    crate::entry::EntryType::Text => &entry.content,
                    crate::entry::EntryType::Image => &entry.content,
                };

                // Find all matches and their positions
                let mut indices = Vec::new();
                for mat in regex.find_iter(content) {
                    // Convert byte positions to char indices
                    let start_char = content[..mat.start()].chars().count() as u32;
                    let end_char = start_char + content[mat.start()..mat.end()].chars().count() as u32;
                    for i in start_char..end_char {
                        indices.push(i);
                    }
                }

                if indices.is_empty() && !regex.is_match(content) {
                    return None;
                }

                Some((idx, indices))
            })
            .collect();

        // Store filtered indices and match positions
        self.filtered = matches.iter().map(|(idx, _)| *idx).collect();

        for (idx, indices) in matches {
            if !indices.is_empty() {
                self.match_indices.insert(idx, indices);
            }
        }

        // Adjust selection
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    /// Toggle between fuzzy and regex search modes
    pub fn toggle_search_mode(&mut self) {
        self.search_mode = match self.search_mode {
            SearchMode::Fuzzy => SearchMode::Regex,
            SearchMode::Regex => SearchMode::Fuzzy,
        };
        self.regex_error = None;

        // Re-filter with new mode
        self.filter_entries();

        let mode_name = match self.search_mode {
            SearchMode::Fuzzy => "Fuzzy",
            SearchMode::Regex => "Regex",
        };
        self.set_message(format!("Search mode: {}", mode_name));
    }

    /// Enter regex search mode directly
    pub fn enter_regex_search(&mut self) {
        self.search_mode = SearchMode::Regex;
        self.regex_error = None;
        self.input_mode = InputMode::Search;
        self.set_message("Regex search mode");
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.entries.get(idx))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        // Don't go to previous page - stay on current page
    }

    pub fn move_down(&mut self) {
        if self.selected < self.filtered.len().saturating_sub(1) {
            self.selected += 1;
        }
        // Don't go to next page - stay on current page
    }

    pub fn go_top(&mut self) {
        self.selected = 0;
    }

    pub fn go_bottom(&mut self) {
        self.selected = self.filtered.len().saturating_sub(1);
    }

    /// Go to previous page (Left arrow)
    pub fn prev_page(&mut self) {
        if self.search_query.is_empty() && self.current_page > 0 {
            let _ = self.load_page(self.current_page - 1);
        }
    }

    /// Go to next page (Right arrow)
    pub fn next_page(&mut self) {
        if self.search_query.is_empty() && self.current_page + 1 < self.total_pages() {
            let _ = self.load_page(self.current_page + 1);
        }
    }

    /// Move up by half a page (Ctrl+U style) - within current page
    pub fn page_up(&mut self, page_size: usize) {
        let half_page = page_size / 2;
        self.selected = self.selected.saturating_sub(half_page);
    }

    /// Move down by half a page (Ctrl+D style) - within current page
    pub fn page_down(&mut self, page_size: usize) {
        let half_page = page_size / 2;
        self.selected = (self.selected + half_page).min(self.filtered.len().saturating_sub(1));
    }

    pub fn copy_selected(&mut self) -> Result<()> {
        if let Some(entry) = self.selected_entry() {
            let id = entry.id.clone();
            let preview = entry.preview(30);
            match entry.entry_type {
                crate::entry::EntryType::Text => {
                    Clipboard::set_text(&entry.content)?;
                    self.set_message(format!("Copied: {}", preview));
                }
                crate::entry::EntryType::Image => {
                    Clipboard::set_image(&entry.content)?;
                    self.set_message(format!("Copied image: {}", preview));
                }
            }
            // Update last_used timestamp
            self.db.touch(&id)?;
        }
        Ok(())
    }

    pub fn delete_selected(&mut self) -> Result<()> {
        if let Some(entry) = self.selected_entry() {
            let id = entry.id.clone();
            self.db.delete(&id)?;
            self.reload_entries()?;
            self.set_message("Entry deleted");
        }
        Ok(())
    }

    pub fn toggle_favorite(&mut self) -> Result<()> {
        if let Some(entry) = self.selected_entry() {
            let id = entry.id.clone();
            self.db.toggle_favorite(&id)?;
            self.reload_entries()?;
            self.set_message("Favorite toggled");
        }
        Ok(())
    }

    pub fn clear_all(&mut self) -> Result<()> {
        self.db.clear_all()?;
        self.reload_entries()?;
        self.set_message("All entries cleared");
        Ok(())
    }

    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
    }

    pub fn end_search(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        // Reload entries to reset to paginated view
        let _ = self.reload_entries();
        self.end_search();
    }

    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.filter_entries();
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.filter_entries();
    }

    pub fn clear_message(&mut self) {
        self.message = None;
        self.message_time = None;
    }

    /// Set a message with timestamp for timeout
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
        self.message_time = Some(Instant::now());
    }

    /// Check if message has timed out (2 seconds)
    pub fn is_message_expired(&self) -> bool {
        if let Some(time) = self.message_time {
            time.elapsed().as_secs() >= 2
        } else {
            false
        }
    }

    /// Get human-readable time since last refresh
    pub fn time_since_refresh(&self) -> String {
        let elapsed = self.last_refresh.elapsed();
        let secs = elapsed.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h", secs / 3600)
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    // Multi-select operations

    /// Toggle multi-select mode
    pub fn toggle_multi_select(&mut self) {
        self.multi_select_mode = !self.multi_select_mode;
        if !self.multi_select_mode {
            self.multi_selected.clear();
        }
        let msg = if self.multi_select_mode {
            "Multi-select ON (Space to select, v to toggle all)"
        } else {
            "Multi-select OFF"
        };
        self.set_message(msg);
    }

    /// Toggle selection of current entry in multi-select mode
    pub fn toggle_current_selection(&mut self) {
        if self.multi_select_mode && !self.filtered.is_empty() {
            if self.multi_selected.contains(&self.selected) {
                self.multi_selected.remove(&self.selected);
            } else {
                self.multi_selected.insert(self.selected);
            }
        }
    }

    /// Select all entries in current filter
    pub fn select_all(&mut self) {
        if self.multi_select_mode {
            self.multi_selected = (0..self.filtered.len()).collect();
            self.set_message(format!("Selected all {} entries", self.filtered.len()));
        }
    }

    /// Deselect all entries
    pub fn deselect_all(&mut self) {
        if self.multi_select_mode {
            self.multi_selected.clear();
            self.set_message("Deselected all");
        }
    }

    /// Toggle select all / deselect all
    pub fn toggle_select_all(&mut self) {
        if self.multi_select_mode {
            if self.multi_selected.len() == self.filtered.len() {
                self.deselect_all();
            } else {
                self.select_all();
            }
        }
    }

    /// Check if an entry is selected in multi-select mode
    pub fn is_multi_selected(&self, filtered_idx: usize) -> bool {
        self.multi_select_mode && self.multi_selected.contains(&filtered_idx)
    }

    /// Delete all selected entries in multi-select mode
    pub fn delete_selected_multi(&mut self) -> Result<()> {
        if !self.multi_select_mode || self.multi_selected.is_empty() {
            return Ok(());
        }

        let count = self.multi_selected.len();

        // Get actual entry IDs to delete
        let ids_to_delete: Vec<String> = self
            .multi_selected
            .iter()
            .filter_map(|&filtered_idx| {
                self.filtered
                    .get(filtered_idx)
                    .and_then(|&entry_idx| self.entries.get(entry_idx))
                    .map(|e| e.id.clone())
            })
            .collect();

        // Delete each entry
        for id in &ids_to_delete {
            self.db.delete(id)?;
        }

        // Clear selection and reload
        self.multi_selected.clear();
        self.reload_entries()?;
        self.set_message(format!("Deleted {} entries", count));

        Ok(())
    }

    /// Copy all selected entries in multi-select mode (concatenate text)
    pub fn copy_selected_multi(&mut self) -> Result<()> {
        if !self.multi_select_mode || self.multi_selected.is_empty() {
            return Ok(());
        }

        // Collect all selected text entries
        let mut texts: Vec<String> = Vec::new();
        let mut ids: Vec<String> = Vec::new();

        for &filtered_idx in &self.multi_selected {
            if let Some(&entry_idx) = self.filtered.get(filtered_idx) {
                if let Some(entry) = self.entries.get(entry_idx) {
                    match entry.entry_type {
                        crate::entry::EntryType::Text => {
                            texts.push(entry.content.clone());
                            ids.push(entry.id.clone());
                        }
                        crate::entry::EntryType::Image => {
                            // Skip images in multi-copy (can't concatenate images)
                        }
                    }
                }
            }
        }

        if texts.is_empty() {
            self.set_message("No text entries to copy");
            return Ok(());
        }

        // Join with newlines and copy
        let combined = texts.join("\n");
        Clipboard::set_text(&combined)?;

        // Touch all copied entries
        for id in &ids {
            let _ = self.db.touch(id);
        }

        self.set_message(format!("Copied {} text entries", texts.len()));
        Ok(())
    }

    // Note editing operations

    /// Start editing note for currently selected entry
    pub fn start_edit_note(&mut self) {
        // Clone values to avoid borrow issues
        let (id, notes) = if let Some(entry) = self.selected_entry() {
            (entry.id.clone(), entry.notes.clone())
        } else {
            return;
        };
        self.editing_entry_id = Some(id);
        self.note_input = notes.unwrap_or_default();
        self.input_mode = InputMode::EditNote;
    }

    /// Save the note and return to normal mode
    pub fn save_note(&mut self) -> Result<()> {
        if let Some(id) = self.editing_entry_id.take() {
            let notes = if self.note_input.trim().is_empty() {
                None
            } else {
                Some(self.note_input.trim())
            };
            self.db.update_notes(&id, notes)?;
            self.reload_entries()?;
            self.set_message("Note saved");
        }
        self.note_input.clear();
        self.input_mode = InputMode::Normal;
        Ok(())
    }

    /// Cancel note editing and return to normal mode
    pub fn cancel_edit_note(&mut self) {
        self.editing_entry_id = None;
        self.note_input.clear();
        self.input_mode = InputMode::Normal;
        self.set_message("Edit cancelled");
    }

    /// Push a character to the note input
    pub fn push_note_char(&mut self, c: char) {
        self.note_input.push(c);
    }

    /// Remove the last character from note input
    pub fn pop_note_char(&mut self) {
        self.note_input.pop();
    }

    // Preview mode operations

    /// Cycle to the next preview mode
    pub fn cycle_preview_mode(&mut self) {
        self.preview_mode = self.preview_mode.next();
        self.preview_scroll_offset = 0; // Reset scroll when changing modes
        self.set_message(format!("Preview mode: {}", self.preview_mode.label()));
    }

    /// Scroll preview left (for Scroll mode)
    pub fn preview_scroll_left(&mut self) {
        if self.preview_mode == PreviewMode::Scroll {
            self.preview_scroll_offset = self.preview_scroll_offset.saturating_sub(10);
        }
    }

    /// Scroll preview right (for Scroll mode)
    pub fn preview_scroll_right(&mut self) {
        if self.preview_mode == PreviewMode::Scroll {
            self.preview_scroll_offset += 10;
        }
    }

    // Quick snippet operations

    /// Refresh snippet slots from most-used entries
    pub fn refresh_snippets(&mut self) -> Result<()> {
        let top_entries = self.db.get_top_by_usage(9)?;
        for (i, slot) in self.snippet_slots.iter_mut().enumerate() {
            *slot = top_entries.get(i).map(|e| e.id.clone());
        }
        Ok(())
    }

    /// Get entry for a snippet slot (1-9)
    #[allow(dead_code)]
    pub fn get_snippet(&self, slot: usize) -> Option<&Entry> {
        if slot < 1 || slot > 9 {
            return None;
        }
        let entry_id = self.snippet_slots[slot - 1].as_ref()?;
        self.entries.iter().find(|e| &e.id == entry_id)
    }

    /// Get snippet entry info for display (preview text and entry ID)
    #[allow(dead_code)]
    pub fn get_snippet_info(&self, slot: usize) -> Option<(String, String)> {
        if slot < 1 || slot > 9 {
            return None;
        }
        let entry_id = self.snippet_slots[slot - 1].as_ref()?;

        // First try to find in current entries
        if let Some(entry) = self.entries.iter().find(|e| &e.id == entry_id) {
            return Some((entry.preview(8), entry.id.clone()));
        }

        // If not in current page, we still have the ID
        Some(("...".to_string(), entry_id.clone()))
    }

    /// Copy snippet slot (1-9) to clipboard
    pub fn copy_snippet(&mut self, slot: usize) -> Result<()> {
        if slot < 1 || slot > 9 {
            self.set_message("Invalid slot number");
            return Ok(());
        }

        let entry_id = match self.snippet_slots[slot - 1].clone() {
            Some(id) => id,
            None => {
                self.set_message(format!("Slot {} is empty", slot));
                return Ok(());
            }
        };

        // Try to get entry from database directly
        if let Some(entry) = self.db.get_by_id(&entry_id)? {
            let preview = entry.preview(20);
            match entry.entry_type {
                crate::entry::EntryType::Text => {
                    Clipboard::set_text(&entry.content)?;
                    self.set_message(format!("Slot {}: {}", slot, preview));
                }
                crate::entry::EntryType::Image => {
                    Clipboard::set_image(&entry.content)?;
                    self.set_message(format!("Slot {} (image): {}", slot, preview));
                }
            }
            // Update usage count
            self.db.touch(&entry_id)?;
        } else {
            self.set_message(format!("Slot {} entry not found", slot));
        }

        Ok(())
    }

    /// Toggle snippets bar visibility
    #[allow(dead_code)]
    pub fn toggle_snippets(&mut self) {
        self.show_snippets = !self.show_snippets;
        let msg = if self.show_snippets {
            "Snippets bar shown"
        } else {
            "Snippets bar hidden"
        };
        self.set_message(msg);
    }

    // Tab operations

    /// Get the current active tab filter
    pub fn active_tab_filter(&self) -> &TabFilter {
        self.tabs.get(self.active_tab).unwrap_or(&TabFilter::All)
    }

    /// Move to the next tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.apply_tab_filter();
        }
    }

    /// Move to the previous tab
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
            self.apply_tab_filter();
        }
    }

    /// Apply the current tab filter to entries
    fn apply_tab_filter(&mut self) {
        let filter = self.active_tab_filter().clone();
        self.match_indices.clear();

        // Reset to first page when switching tabs
        self.current_page = 0;

        // If there's a search query, combine with tab filter (in-memory)
        if !self.search_query.is_empty() {
            // Re-apply search filter first
            self.filter_entries();
            // Then further filter by tab
            self.filter_by_tab(&filter);
        } else {
            // Reload from DB with the new filter for correct pagination
            let (filter_str, collection_id) = filter.db_filter();
            if let Ok(count) = self.db.count_filtered(filter_str, collection_id) {
                self.total_count = count;
            }
            if let Ok(entries) = self.db.get_page_filtered(0, PAGE_SIZE, filter_str, collection_id) {
                self.entries = entries;
                self.filtered = (0..self.entries.len()).collect();
            }
        }

        // Adjust selection
        self.selected = 0;

        // Show message
        self.set_message(format!("Tab: {}", filter.label()));
    }

    /// Filter entries by tab filter
    fn filter_by_tab(&mut self, filter: &TabFilter) {
        use chrono::{Duration, Utc};

        self.filtered.retain(|&idx| {
            if let Some(entry) = self.entries.get(idx) {
                match filter {
                    TabFilter::All => true,
                    TabFilter::Text => entry.entry_type == crate::entry::EntryType::Text,
                    TabFilter::Images => entry.entry_type == crate::entry::EntryType::Image,
                    TabFilter::Favorites => entry.favorite,
                    TabFilter::Today => {
                        let today = Utc::now() - Duration::hours(24);
                        entry.created_at > today
                    }
                    TabFilter::Collection(collection_id) => {
                        entry.collection_id.as_ref() == Some(collection_id)
                    }
                }
            } else {
                false
            }
        });
    }

    /// Toggle tabs bar visibility
    #[allow(dead_code)]
    pub fn toggle_tabs(&mut self) {
        self.show_tabs = !self.show_tabs;
        let msg = if self.show_tabs {
            "Tabs bar shown"
        } else {
            "Tabs bar hidden"
        };
        self.set_message(msg);
    }

    /// Toggle line numbers in preview
    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
        let msg = if self.show_line_numbers {
            "Line numbers ON"
        } else {
            "Line numbers OFF"
        };
        self.set_message(msg);
    }

    // Confirmation operations

    /// Request confirmation for deleting the selected entry
    pub fn request_delete_selected(&mut self) {
        if self.selected_entry().is_some() {
            self.pending_confirm = Some(ConfirmAction::DeleteSelected);
            self.input_mode = InputMode::Confirm;
        }
    }

    /// Request confirmation for deleting multiple selected entries
    pub fn request_delete_multi(&mut self) {
        if self.multi_select_mode && !self.multi_selected.is_empty() {
            self.pending_confirm = Some(ConfirmAction::DeleteSelected);
            self.input_mode = InputMode::Confirm;
        }
    }

    /// Request confirmation for clearing all entries
    pub fn request_clear_all(&mut self) {
        if self.total_count > 0 {
            self.pending_confirm = Some(ConfirmAction::ClearAll);
            self.input_mode = InputMode::Confirm;
        }
    }

    /// Execute the pending confirmation action
    pub fn confirm_action(&mut self) -> Result<()> {
        if let Some(action) = self.pending_confirm.take() {
            self.input_mode = InputMode::Normal;
            match action {
                ConfirmAction::DeleteSelected => {
                    if self.multi_select_mode && !self.multi_selected.is_empty() {
                        self.delete_selected_multi()?;
                    } else {
                        self.delete_selected()?;
                    }
                }
                ConfirmAction::ClearAll => {
                    self.clear_all()?;
                }
            }
        }
        Ok(())
    }

    /// Cancel the pending confirmation
    pub fn cancel_confirm(&mut self) {
        self.pending_confirm = None;
        self.input_mode = InputMode::Normal;
        self.set_message("Cancelled");
    }

    /// Get a description of the pending confirmation action
    pub fn confirm_message(&self) -> Option<String> {
        self.pending_confirm.map(|action| match action {
            ConfirmAction::DeleteSelected => {
                if self.multi_select_mode && !self.multi_selected.is_empty() {
                    format!("Delete {} selected entries?", self.multi_selected.len())
                } else if let Some(entry) = self.selected_entry() {
                    format!("Delete \"{}\"?", entry.preview(30))
                } else {
                    "Delete selected entry?".to_string()
                }
            }
            ConfirmAction::ClearAll => {
                format!("Delete ALL {} entries? This cannot be undone!", self.total_count)
            }
        })
    }
}
