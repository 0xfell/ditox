mod confirm;
mod help;
mod layout;
mod list;
mod note_editor;
pub mod preview;
mod search;
mod snippets;
mod tabs;
mod theme;

use ditox_core::actions::Action;
use ditox_core::app::{App, InputMode, PreviewMode};
use ditox_core::config::Config;
use ditox_core::db::Database;
use ditox_core::error::Result;
use crate::keybindings::{KeybindingResolver, KeybindingsConfigExt};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use preview::{ImageCache, ImageLoader};
use ratatui::prelude::*;
use ratatui_image::picker::{Picker, ProtocolType};
use std::io;
use std::time::{Duration, Instant};
use theme::Theme;

use ditox_core::config::GraphicsProtocol;

/// Create image picker with terminal detection
fn create_picker(
    override_protocol: Option<GraphicsProtocol>,
    override_font_size: Option<(u16, u16)>,
) -> Option<Picker> {
    let is_ghostty = std::env::var("GHOSTTY_RESOURCES_DIR").is_ok();
    let is_kitty = std::env::var("KITTY_WINDOW_ID").is_ok();
    let is_wezterm = std::env::var("WEZTERM_PANE").is_ok();
    let needs_kitty = is_ghostty || is_kitty || is_wezterm;

    let mut picker = if let Some((w, h)) = override_font_size {
        tracing::info!("Using configured font size: {}x{}", w, h);
        Picker::from_fontsize((w, h))
    } else {
        match Picker::from_query_stdio() {
            Ok(p) => p,
            Err(_) => Picker::from_fontsize((9, 18)),
        }
    };

    if let Some(protocol) = override_protocol {
        let proto_type = match protocol {
            GraphicsProtocol::Kitty => ProtocolType::Kitty,
            GraphicsProtocol::Sixel => ProtocolType::Sixel,
            GraphicsProtocol::Iterm2 => ProtocolType::Iterm2,
            GraphicsProtocol::Halfblocks => ProtocolType::Halfblocks,
        };
        picker.set_protocol_type(proto_type);
    } else if needs_kitty {
        picker.set_protocol_type(ProtocolType::Kitty);
    }

    Some(picker)
}

pub fn run(db: Database, config: Config) -> Result<()> {
    // Initialize image picker BEFORE entering alternate screen
    let mut picker = create_picker(config.ui.graphics_protocol, config.ui.font_size);

    // Create keybinding resolver from config
    let keybindings = config.keybindings.create_resolver();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(db, config)?;
    let theme = Theme::from_config(&app.config().ui.theme);

    // Initialize quick snippets from most-used entries
    let _ = app.refresh_snippets();

    // Image cache and background loader
    let mut image_cache = ImageCache::new(10); // Cache up to 10 images
    let image_loader = ImageLoader::new();

    // Auto-refresh timer
    let refresh_interval = Duration::from_secs(2);
    let mut last_refresh = Instant::now();

    // Main loop
    let result = run_loop(
        &mut terminal,
        &mut app,
        &theme,
        &mut image_cache,
        &mut picker,
        &image_loader,
        refresh_interval,
        &mut last_refresh,
        &keybindings,
    );

    // Cleanup
    image_loader.shutdown();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    result
}

/// Track mouse state for double-click detection
struct MouseState {
    last_click: Option<Instant>,
    last_click_row: Option<u16>,
}

impl MouseState {
    fn new() -> Self {
        Self {
            last_click: None,
            last_click_row: None,
        }
    }

    /// Returns true if this is a double-click (same row, within 500ms)
    fn is_double_click(&mut self, row: u16) -> bool {
        let now = Instant::now();
        let is_double = if let (Some(last), Some(last_row)) = (self.last_click, self.last_click_row)
        {
            last_row == row && now.duration_since(last) < Duration::from_millis(500)
        } else {
            false
        };

        self.last_click = Some(now);
        self.last_click_row = Some(row);

        is_double
    }
}

fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    theme: &Theme,
    cache: &mut ImageCache,
    picker: &mut Option<Picker>,
    loader: &ImageLoader,
    refresh_interval: Duration,
    last_refresh: &mut Instant,
    keybindings: &KeybindingResolver,
) -> Result<()> {
    let mut mouse_state = MouseState::new();

    loop {
        // Auto-refresh entries periodically
        if last_refresh.elapsed() >= refresh_interval {
            let old_count = app.entries.len();
            if app.reload_entries().is_ok() {
                let new_count = app.entries.len();
                if new_count > old_count {
                    app.set_message(format!("{} new entries", new_count - old_count));
                }
            }
            *last_refresh = Instant::now();
        }

        terminal.draw(|f| layout::draw(f, app, theme, cache, picker, loader, keybindings))?;
        // Note: layout::draw updates app.terminal_height for page navigation

        // Poll with short timeout to allow for responsive UI and auto-refresh
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                // Only handle key press events (crossterm 0.28+ sends Press, Release, and Repeat)
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key(app, key, keybindings)?
                }
                Event::Mouse(mouse) => handle_mouse(app, mouse, &mut mouse_state)?,
                _ => {}
            }
        }

        // Clear message after timeout (2 seconds)
        if app.is_message_expired() {
            app.clear_message();
        }

        if app.should_copy_and_quit {
            app.copy_selected()?;
            break;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent, keybindings: &KeybindingResolver) -> Result<()> {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key, keybindings),
        InputMode::Search => handle_search_mode(app, key, keybindings),
        InputMode::EditNote => handle_edit_note_mode(app, key),
        InputMode::Confirm => handle_confirm_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent, keybindings: &KeybindingResolver) -> Result<()> {
    // Resolve key to action
    let action = keybindings.resolve(key);

    match action {
        Some(Action::Quit) => app.should_quit = true,
        Some(Action::ForceQuit) => app.should_quit = true,

        // Navigation
        Some(Action::MoveDown) => app.move_down(),
        Some(Action::MoveUp) => app.move_up(),
        Some(Action::GoTop) => app.go_top(),
        Some(Action::GoBottom) => app.go_bottom(),
        Some(Action::PageUp) => app.page_up(app.terminal_height as usize),
        Some(Action::PageDown) => app.page_down(app.terminal_height as usize),
        Some(Action::PrevPage) => {
            // In scroll preview mode, left/right scroll the preview
            if app.show_preview && app.preview_mode == PreviewMode::Scroll {
                app.preview_scroll_left();
            } else {
                app.prev_page();
            }
        }
        Some(Action::NextPage) => {
            // In scroll preview mode, left/right scroll the preview
            if app.show_preview && app.preview_mode == PreviewMode::Scroll {
                app.preview_scroll_right();
            } else {
                app.next_page();
            }
        }

        // Operations
        Some(Action::CopyAndQuit) => {
            if app.multi_select_mode && !app.multi_selected.is_empty() {
                // In multi-select mode with selections, copy all and exit
                app.copy_selected_multi()?;
                app.should_quit = true;
            } else {
                app.should_copy_and_quit = true;
            }
        }
        Some(Action::Copy) => {
            if app.multi_select_mode && !app.multi_selected.is_empty() {
                app.copy_selected_multi()?;
            } else {
                app.copy_selected()?;
            }
        }
        Some(Action::Delete) => {
            if app.multi_select_mode && !app.multi_selected.is_empty() {
                app.request_delete_multi();
            } else {
                app.request_delete_selected();
            }
        }
        Some(Action::ClearAll) => app.request_clear_all(),
        Some(Action::ToggleFavorite) => app.toggle_favorite()?,
        Some(Action::Refresh) => {
            app.reload_entries()?;
            app.set_message("Refreshed");
        }

        // Modes
        Some(Action::EnterSearch) => app.start_search(),
        Some(Action::TogglePreview) => app.show_preview = !app.show_preview,
        Some(Action::ToggleExpanded) => app.show_expanded = !app.show_expanded,
        Some(Action::ToggleHelp) => app.show_help = !app.show_help,

        // Multi-select
        Some(Action::ToggleMultiSelect) => app.toggle_multi_select(),
        Some(Action::SelectCurrent) => {
            if app.multi_select_mode {
                app.toggle_current_selection();
            }
        }
        Some(Action::SelectAll) => {
            if app.multi_select_mode {
                app.toggle_select_all();
            }
        }

        // Handle Esc specially for closing modes
        Some(Action::ExitSearch) => {
            if app.multi_select_mode {
                // Exit multi-select mode first
                app.toggle_multi_select();
            } else if app.show_expanded {
                app.show_expanded = false;
            } else if app.show_help {
                app.show_help = false;
            } else if !app.search_query.is_empty() {
                app.clear_search();
            }
        }

        // Search mode features
        Some(Action::EnterRegexSearch) => {
            app.enter_regex_search();
        }
        Some(Action::ToggleSearchMode) => {
            app.toggle_search_mode();
        }
        Some(Action::ShowActions) => {
            // TODO: Implement command palette
        }
        Some(Action::CyclePreviewMode) => {
            app.cycle_preview_mode();
        }
        Some(Action::ToggleLineNumbers) => {
            app.toggle_line_numbers();
        }
        Some(Action::NextTab) => {
            app.next_tab();
        }
        Some(Action::PrevTab) => {
            app.prev_tab();
        }
        Some(Action::QuickSlot1) => app.copy_snippet(1)?,
        Some(Action::QuickSlot2) => app.copy_snippet(2)?,
        Some(Action::QuickSlot3) => app.copy_snippet(3)?,
        Some(Action::QuickSlot4) => app.copy_snippet(4)?,
        Some(Action::QuickSlot5) => app.copy_snippet(5)?,
        Some(Action::QuickSlot6) => app.copy_snippet(6)?,
        Some(Action::QuickSlot7) => app.copy_snippet(7)?,
        Some(Action::QuickSlot8) => app.copy_snippet(8)?,
        Some(Action::QuickSlot9) => app.copy_snippet(9)?,
        Some(Action::EditAnnotation) => {
            app.start_edit_note();
        }
        Some(Action::ShowStats) => {
            // TODO: Implement in Phase 1
        }

        None => {}
    }
    Ok(())
}

fn handle_search_mode(app: &mut App, key: KeyEvent, keybindings: &KeybindingResolver) -> Result<()> {
    // In search mode, we handle text input directly, but some keys trigger actions
    match key.code {
        KeyCode::Esc => app.end_search(),
        KeyCode::Enter => app.end_search(),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Char(c) => {
            // Check if this is a control combo that should trigger an action
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT)
            {
                // Allow certain actions while in search mode
                if let Some(action) = keybindings.resolve(key) {
                    match action {
                        Action::TogglePreview => app.show_preview = !app.show_preview,
                        Action::ForceQuit => app.should_quit = true,
                        _ => {}
                    }
                }
            } else {
                app.push_search_char(c);
            }
        }
        // Handle Tab in search mode - toggle preview without exiting search
        KeyCode::Tab => {
            if let Some(action) = keybindings.resolve(key) {
                if action == Action::TogglePreview {
                    app.show_preview = !app.show_preview;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_edit_note_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    // In note editing mode, handle text input for the note
    match key.code {
        KeyCode::Esc => app.cancel_edit_note(),
        KeyCode::Enter => app.save_note()?,
        KeyCode::Backspace => app.pop_note_char(),
        KeyCode::Char(c) => {
            // Check for ctrl+c to force quit
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                app.should_quit = true;
            } else {
                app.push_note_char(c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    // In confirmation mode, handle y/n/Enter/Esc
    match key.code {
        // Confirm with y or Enter
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.confirm_action()?;
        }
        // Cancel with n, N, or Esc
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_confirm();
        }
        // Ctrl+C force quits
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, state: &mut MouseState) -> Result<()> {
    // Layout: search bar (3 lines), content area, status bar (1 line)
    // Content area starts at row 3 and ends at (terminal_height - 1)
    // Within content area, we have a 1-line border at top and bottom
    let list_start_row = 4u16; // Row 3 (0-indexed) + 1 for border = row 4
    let list_end_row = app.terminal_height.saturating_sub(2); // -1 for status, -1 for border

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check if click is in the list area
            if mouse.row >= list_start_row && mouse.row < list_end_row {
                // Calculate which entry was clicked (accounting for scroll offset)
                // Currently we don't track scroll offset separately - Ratatui handles it
                // So we map the click to the visible position
                let clicked_visible_row = (mouse.row - list_start_row) as usize;

                // The visible entries start from some offset based on selection
                // For simplicity, we calculate the entry index relative to the scroll
                let visible_height = (list_end_row - list_start_row) as usize;

                // Calculate scroll offset (how many items are scrolled off-screen)
                let scroll_offset = if app.selected >= visible_height {
                    app.selected.saturating_sub(visible_height / 2)
                } else {
                    0
                };

                let clicked_index = scroll_offset + clicked_visible_row;

                if clicked_index < app.filtered.len() {
                    // Check for double-click
                    if state.is_double_click(mouse.row) {
                        // Double-click: copy and exit
                        app.selected = clicked_index;
                        app.should_copy_and_quit = true;
                    } else {
                        // Single click: just select
                        app.selected = clicked_index;
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            app.move_up();
        }
        MouseEventKind::ScrollDown => {
            app.move_down();
        }
        _ => {}
    }

    Ok(())
}
