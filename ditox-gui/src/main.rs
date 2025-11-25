//! Ditox Windows GUI - Clipboard manager with system tray and global hotkey

// Hide console window on Windows release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod startup;

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

    // Load config and database
    let config = Config::load()?;
    let db = Database::open()?;
    db.init_schema()?;

    tracing::info!("Ditox GUI starting...");

    // Run the iced application
    app::run(db, config)
}
