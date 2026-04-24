//! Command-line argument parsing for `ditox-gui`.
//!
//! The GUI is normally launched without arguments. A handful of flags let
//! the user drive a running instance from a compositor keybind or from the
//! shell:
//!
//! - `--toggle` — show the window if hidden, hide it if shown.
//! - `--show`   — always show the window.
//! - `--hide`   — always hide the window (used by autostart).
//! - `--quit`   — ask the running instance to exit.
//!
//! When one of these action flags is given and another instance is already
//! running, the flag is forwarded over the IPC socket and this process exits.
//! When no action flag is given the GUI is launched as usual; if another
//! instance is already running we send a `toggle` and exit (same-binary
//! "summon" behaviour).

use clap::Parser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No explicit action — launch the GUI (or toggle if already running).
    Launch,
    Toggle,
    Show,
    Hide,
    Quit,
}

impl Action {
    /// Serialised form sent over the IPC socket.
    pub fn wire(&self) -> Option<&'static str> {
        match self {
            Action::Launch | Action::Toggle => Some("TOGGLE"),
            Action::Show => Some("SHOW"),
            Action::Hide => Some("HIDE"),
            Action::Quit => Some("QUIT"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "ditox-gui",
    about = "Ditox clipboard manager (GUI)",
    version
)]
pub struct Cli {
    /// Toggle the window (show if hidden, hide if shown).
    #[arg(long, conflicts_with_all = ["show", "hide", "quit"])]
    pub toggle: bool,

    /// Force the window to show.
    #[arg(long, conflicts_with_all = ["toggle", "hide", "quit"])]
    pub show: bool,

    /// Force the window to hide.
    #[arg(long, conflicts_with_all = ["toggle", "show", "quit"])]
    pub hide: bool,

    /// Ask the running GUI instance to quit.
    #[arg(long, conflicts_with_all = ["toggle", "show", "hide"])]
    pub quit: bool,
}

impl Cli {
    pub fn action(&self) -> Action {
        if self.toggle {
            Action::Toggle
        } else if self.show {
            Action::Show
        } else if self.hide {
            Action::Hide
        } else if self.quit {
            Action::Quit
        } else {
            Action::Launch
        }
    }
}
