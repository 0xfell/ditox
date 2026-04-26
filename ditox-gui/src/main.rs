//! Ditox GUI — cross-platform clipboard manager frontend.
//!
//! On Windows the binary is hidden from the console (`windows_subsystem =
//! "windows"`) in release builds.
//!
//! On Linux the binary doubles as its own "summon" tool: a second launch
//! (typically from a compositor keybind) will find the first instance through
//! a Unix socket and forward its `--toggle` / `--show` / `--hide` / `--quit`
//! intent, then exit. When launched without flags and no other instance is
//! running, it starts the iced GUI.

// Hide console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod startup;

use clap::Parser;
use ditox_core::foreground::build_default_tracker;
use ditox_core::logging;
use ditox_core::paste::cursor::PersistentSelectionCursor;
use ditox_core::paste::synthesize::pick_chain;
use ditox_core::platform::detect as detect_platform;
use ditox_core::{Config, Database, Result};

fn main() {
    if let Err(e) = run() {
        // Last-resort: tracing may not be initialised on early errors.
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialise logging via the shared helper. RUST_LOG honoured.
    logging::init(logging::Mode::Stderr);

    let cli = cli::Cli::parse();
    let action = cli.action();

    // -----------------------------------------------------------------
    // One-shot launcher model: each invocation is an independent process
    // that runs until the user copies, cancels, or the window loses
    // focus. Because there's no long-lived daemon, the IPC actions
    // (`--toggle`, `--show`, `--hide`, `--quit`) are no longer
    // meaningful: a fresh launch IS a "show", and exit handles
    // "hide"/"quit" as soon as the user is done. We keep the flags for
    // backward-compatibility (so existing keybinds don't break), but
    // they're all rolled into "just launch" except `--quit` which
    // exits immediately.
    // -----------------------------------------------------------------
    if matches!(action, cli::Action::Quit) {
        tracing::info!("--quit requested; one-shot mode has no daemon to signal, exiting");
        return Ok(());
    }

    // Load config and database
    let config = Config::load()?;
    if let Some(override_dir) = config.apply_storage_override()? {
        tracing::info!("data_dir override active: {}", override_dir.display());
        if Config::legacy_db_exists_outside(&override_dir) {
            tracing::warn!(
                "data_dir override points at {} (no ditox.db there yet) but \
                 a legacy default ditox.db exists. The override starts a \
                 new history; copy or move the legacy DB if you want it.",
                override_dir.display()
            );
        }
    }
    let db = Database::open()?;
    db.init_schema()?;

    tracing::info!("Ditox GUI starting (one-shot, action={:?})", action);

    // `--hide` is preserved as a no-op compatibility flag; in one-shot
    // mode there's nothing to hide.
    let start_hidden = matches!(action, cli::Action::Hide);

    // ---------------------------------------------------------------
    // Phase 2 paste-back: capture the foreground window BEFORE iced
    // creates its own window. By the time `app::run_with` returns,
    // ditox-gui IS the focused window — so a snapshot from inside
    // `boot_app` would return ourselves (and `ForegroundFilter` would
    // correctly drop it, leaving us with no restore target).
    // ---------------------------------------------------------------
    let foreground_tracker = build_default_tracker();
    let previous_foreground = match foreground_tracker.snapshot() {
        Ok(snap) => {
            if let Some(s) = &snap {
                tracing::info!(
                    process = %s.process_basename,
                    title = %s.title,
                    kind = %s.identifier.kind(),
                    "captured previous-foreground snapshot for paste-back"
                );
            } else {
                tracing::info!(
                    "no previous-foreground snapshot (no foreground or platform unsupported); \
                     paste-back will write clipboard only"
                );
            }
            snap
        }
        Err(e) => {
            tracing::warn!(error = %e, "foreground snapshot failed; paste-back will write clipboard only");
            None
        }
    };
    let synthesizer_chain = pick_chain(detect_platform());
    tracing::debug!(
        chain = ?synthesizer_chain.iter().map(|s| s.name()).collect::<Vec<_>>(),
        "constructed paste-back synthesizer chain"
    );

    // ---------------------------------------------------------------
    // Phase 2 paste-back sub-task 2.9: groundwork for modifier-held
    // cycling. Persist a SelectionCursor across launcher invocations.
    // Each launch fires the cursor: re-fires within
    // `paste.cursor_refire_window_ms` (default 800 ms) advance the
    // index by one, otherwise the index resets to 0. The launcher
    // pre-selects the entry at that index, so rapid-firing
    // Ctrl+Shift+V cycles through the most-recent clips even in the
    // current one-shot model.
    //
    // Phase 4's daemon-mode revert will replace the filesystem
    // round-trip with in-memory state, but the SelectionCursor
    // primitive itself is unchanged.
    // ---------------------------------------------------------------
    let initial_selection = match PersistentSelectionCursor::at_default_path() {
        Ok(persistent) => {
            let cursor = persistent.fire_and_persist(
                std::time::SystemTime::now(),
                config.paste.cursor_refire_window(),
            );
            tracing::debug!(
                index = cursor.index(),
                window_ms = config.paste.cursor_refire_window().as_millis(),
                "fired selection cursor"
            );
            cursor.index()
        }
        Err(e) => {
            tracing::warn!(error = %e, "selection cursor unavailable; defaulting to top of list");
            0
        }
    };

    // Run the iced application
    app::run_with(
        db,
        config,
        start_hidden,
        previous_foreground,
        foreground_tracker,
        synthesizer_chain,
        initial_selection,
    )
}
