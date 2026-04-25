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
mod ipc;
mod ipc_bridge;
mod startup;

use clap::Parser;
use ditox_core::{Config, Database, Result};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("ditox_gui=info")),
        )
        .init();

    let cli = cli::Cli::parse();
    let action = cli.action();

    // ---------------------------------------------------------------------
    // Single-instance coordination (Linux/Unix). If another GUI is already
    // running, forward our action over IPC and exit.
    // ---------------------------------------------------------------------
    let instance_lock = match ipc::try_acquire_instance_lock() {
        Ok(Some(lock)) => lock,
        Ok(None) => {
            // Another instance owns the lock. Forward and exit.
            match action {
                cli::Action::Hide => {
                    // Autostart `--hide` while already running is a no-op.
                    tracing::info!("another ditox-gui is already running; nothing to do");
                    return Ok(());
                }
                _ => {
                    match ipc::send_to_existing(action) {
                        Ok(()) => {
                            tracing::info!("forwarded {:?} to running ditox-gui, exiting", action);
                            return Ok(());
                        }
                        Err(e) => {
                            // Socket exists but not responding — report and bail.
                            return Err(ditox_core::DitoxError::Other(format!(
                                "another instance holds the lock but isn't responding: {}",
                                e
                            )));
                        }
                    }
                }
            }
        }
        Err(e) => {
            // Proceed without lock — losing single-instance semantics is
            // preferable to refusing to start at all.
            tracing::warn!("instance lock error, continuing without it: {e}");
            return Err(ditox_core::DitoxError::Other(format!(
                "could not acquire single-instance lock: {e}"
            )));
        }
    };

    // `--quit` / `--hide` as the FIRST instance: nothing to do beyond
    // acknowledging (we have no other process to message).
    if matches!(action, cli::Action::Quit) {
        tracing::info!("no running ditox-gui to quit");
        return Ok(());
    }

    // Initialise IPC bridge (producer/consumer channel for the subscription).
    let tx = ipc_bridge::init();

    // Spawn the IPC server so later `--toggle` et al. can reach us.
    if let Err(e) = ipc::spawn_server(&instance_lock, tx) {
        tracing::warn!("Failed to start IPC server: {e}");
    }

    // Load config and database
    let config = Config::load()?;
    let db = Database::open()?;
    db.init_schema()?;

    tracing::info!("Ditox GUI starting... (action={:?})", action);

    let start_hidden = matches!(action, cli::Action::Hide);

    // Keep `instance_lock` alive for the duration of the iced loop by holding
    // it in a local that outlives the call.
    let _lock_guard = instance_lock;

    // Run the iced application
    app::run_with(db, config, start_hidden)
}
