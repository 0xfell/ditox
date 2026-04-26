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
mod hyprland_config;
mod ipc;
mod startup;

use clap::Parser;
use ditox_core::foreground::build_default_tracker;
use ditox_core::logging;
use ditox_core::paste::cursor::PersistentSelectionCursor;
use ditox_core::paste::synthesize::pick_chain;
use ditox_core::platform::detect as detect_platform;
use ditox_core::{Config, Database, Result};
use ipc::SendOutcome;

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

    // Phase 4 sub-task 4.11: Hyprland config helper. Pure
    // file-generation; no daemon / IPC involvement.
    if matches!(action, cli::Action::InstallHyprlandConfig) {
        let path = hyprland_config::install()?;
        println!("Wrote: {}", path.display());
        println!();
        println!("To activate, add this line to your hyprland.conf:");
        println!();
        println!("    source = ~/.config/hypr/conf.d/ditox.conf");
        println!();
        println!("Then reload:  hyprctl reload");
        return Ok(());
    }
    if matches!(action, cli::Action::UninstallHyprlandConfig) {
        let removed = hyprland_config::uninstall()?;
        if removed {
            println!("Removed ditox-managed snippet from ~/.config/hypr/conf.d/ditox.conf");
            println!();
            println!("If you no longer want the file at all, delete it manually:");
            println!("    rm -i ~/.config/hypr/conf.d/ditox.conf");
            println!();
            println!("Reload Hyprland:  hyprctl reload");
        } else {
            println!("No ditox-managed snippet found; nothing to remove.");
        }
        return Ok(());
    }

    // -----------------------------------------------------------------
    // Phase 4 sub-tasks 4.1 + 4.2 — single-instance + IPC.
    //
    // For every CLI action including bare `Launch`, we first try to
    // talk to a running daemon. If the daemon answers we forward the
    // command and exit. Only when no daemon is reachable do we try
    // to acquire the lock and become one ourselves. This lets the
    // user re-bind the same `ditox-gui` keybind for both "first
    // launch" (start the daemon) and "summon" (forward to the
    // running one).
    //
    // The daemon's full long-running UX (window stays open, hide on
    // blur, modifier-held cycling, layer-shell) lands incrementally
    // across sub-tasks 4.3-4.12. This commit ships the plumbing only
    // — the daemon mode currently still exits on copy (one-shot
    // semantics retained) but accepts IPC commands while alive.
    // -----------------------------------------------------------------

    // Step 1: forward action to the daemon if one is running.
    match ipc::try_send_to_daemon(action) {
        SendOutcome::Sent { reply } => {
            tracing::info!(action = ?action, %reply, "forwarded to running daemon");
            // A non-OK reply is surfaced to the user so they can react.
            if !reply.starts_with("OK") {
                eprintln!("ditox-gui: daemon replied: {reply}");
                std::process::exit(1);
            }
            return Ok(());
        }
        SendOutcome::Rejected { message } => {
            eprintln!("ditox-gui: daemon rejected command: {message}");
            std::process::exit(1);
        }
        SendOutcome::NoDaemon => {
            // Fall through. For Launch / Toggle we'll start a daemon;
            // for Quit we just exit cleanly because there's nothing to
            // quit.
        }
    }

    if matches!(action, cli::Action::Quit) {
        tracing::info!("--quit requested but no daemon is running; nothing to do");
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

    // Step 2: try to acquire the daemon lock. If another process
    // grabbed it between our `try_send_to_daemon` call and now (race
    // condition during simultaneous starts), retry the IPC send once
    // before giving up.
    let _lock_guard = match ipc::acquire_lock() {
        Some(file) => file,
        None => {
            tracing::debug!("daemon lock contended; retrying IPC send");
            match ipc::try_send_to_daemon(action) {
                SendOutcome::Sent { reply } => {
                    tracing::info!(%reply, "race resolved; forwarded to other daemon");
                    return Ok(());
                }
                _ => {
                    eprintln!("ditox-gui: another instance is starting up; try again in a moment.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Step 3: bind the IPC socket and become the daemon.
    let (ipc_rx, _socket_guard) = ipc::spawn_listener().map_err(|e| {
        ditox_core::DitoxError::Other(format!("could not bind ditox-gui IPC socket: {e}"))
    })?;
    tracing::info!(
        socket = %ipc::socket_path().display(),
        "ditox-gui daemon listening on IPC socket"
    );

    tracing::info!("Ditox GUI starting (daemon, action={:?})", action);

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

    // Run the iced application. The IPC receiver is threaded
    // through so the GUI's update loop can drain DaemonCommands
    // and reply to clients.
    app::run_with(
        db,
        config,
        start_hidden,
        previous_foreground,
        foreground_tracker,
        synthesizer_chain,
        initial_selection,
        Some(ipc_rx),
    )
}
