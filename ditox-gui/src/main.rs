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
    let db = Database::open()?;
    db.init_schema()?;

    tracing::info!("Ditox GUI starting (one-shot, action={:?})", action);

    // `--hide` is preserved as a no-op compatibility flag; in one-shot
    // mode there's nothing to hide.
    let start_hidden = matches!(action, cli::Action::Hide);

    // Run the iced application
    app::run_with(db, config, start_hidden)
}
