//! Centralised tracing-subscriber initialisation.
//!
//! Three modes:
//!
//! - [`Mode::Stderr`] — pretty `tracing-subscriber::fmt` to stderr.
//!   Default for interactive CLI / GUI use.
//! - [`Mode::Journald`] — gated behind the `journald` cargo feature
//!   (currently a stub; full integration arrives in task 016 along
//!   with the systemd unit).
//! - [`Mode::File`] — file logger for debug captures.
//!
//! Filtering: respects the `RUST_LOG` env var; falls back to
//! `ditox=info` for ditox crates and `warn` for everything else.
//!
//! Example:
//!
//! ```no_run
//! use ditox_core::logging::{init, Mode};
//! init(Mode::Stderr);
//! tracing::info!("hello from logging::init");
//! ```

use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Where to send logs.
#[derive(Debug, Clone)]
pub enum Mode {
    /// Pretty stderr formatter. Default.
    Stderr,
    /// Append to a file at the given path.
    File(PathBuf),
    /// systemd-journald structured output. **Not yet implemented**;
    /// falls back to `Stderr` until task 016 lands the dependency.
    Journald,
}

/// Initialise the global tracing subscriber. Safe to call multiple
/// times — subsequent calls are no-ops because the global subscriber
/// is set-once.
///
/// `RUST_LOG` is honoured; the default filter is
/// `ditox=info,ditox_core=info,ditox_tui=info,ditox_gui=info,warn`.
pub fn init(mode: Mode) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("ditox=info,ditox_core=info,ditox_tui=info,ditox_gui=info,warn")
    });

    match mode {
        Mode::Stderr => {
            let fmt_layer = fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_level(true);
            // `try_init` instead of `init` — second calls are no-ops
            // (helpful when both binary main and an embedded library
            // try to initialise).
            let _ = tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .try_init();
        }
        Mode::File(path) => {
            // Best-effort: if we can't open the file, fall back to
            // stderr so the user still sees errors.
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                Ok(file) => {
                    let fmt_layer = fmt::layer()
                        .with_writer(std::sync::Mutex::new(file))
                        .with_target(true)
                        .with_level(true)
                        .with_ansi(false);
                    let _ = tracing_subscriber::registry()
                        .with(env_filter)
                        .with(fmt_layer)
                        .try_init();
                }
                Err(e) => {
                    eprintln!(
                        "ditox: failed to open log file {}: {} — falling back to stderr",
                        path.display(),
                        e
                    );
                    init(Mode::Stderr);
                }
            }
        }
        Mode::Journald => {
            // Stub: full journald support arrives with the watcher
            // hardening task. For now route to stderr so systemd still
            // captures the lines.
            init(Mode::Stderr);
        }
    }
}
