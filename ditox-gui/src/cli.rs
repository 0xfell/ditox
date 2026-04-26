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
    /// Phase 4 sub-task 4.11: write Hyprland config snippet.
    InstallHyprlandConfig,
    /// Phase 4 sub-task 4.11: remove the previously-written snippet.
    UninstallHyprlandConfig,
}

impl Action {
    /// Serialised form for the deprecated IPC path. Retained so the
    /// shape of `Action` keeps round-tripping if we ever need to
    /// reintroduce a daemon.
    #[allow(dead_code)]
    pub fn wire(&self) -> Option<&'static str> {
        match self {
            Action::Launch | Action::Toggle => Some("TOGGLE"),
            Action::Show => Some("SHOW"),
            Action::Hide => Some("HIDE"),
            Action::Quit => Some("QUIT"),
            // The Hyprland-config actions are local to this
            // process — they never go over IPC.
            Action::InstallHyprlandConfig | Action::UninstallHyprlandConfig => None,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "ditox-gui", about = "Ditox clipboard manager (GUI)", version)]
pub struct Cli {
    /// Toggle the window (show if hidden, hide if shown).
    #[arg(long, conflicts_with_all = ["show", "hide", "quit", "install_hyprland_config", "uninstall_hyprland_config"])]
    pub toggle: bool,

    /// Force the window to show.
    #[arg(long, conflicts_with_all = ["toggle", "hide", "quit", "install_hyprland_config", "uninstall_hyprland_config"])]
    pub show: bool,

    /// Force the window to hide.
    #[arg(long, conflicts_with_all = ["toggle", "show", "quit", "install_hyprland_config", "uninstall_hyprland_config"])]
    pub hide: bool,

    /// Ask the running GUI instance to quit.
    #[arg(long, conflicts_with_all = ["toggle", "show", "hide", "install_hyprland_config", "uninstall_hyprland_config"])]
    pub quit: bool,

    /// Write a Hyprland config snippet to
    /// `~/.config/hypr/conf.d/ditox.conf` and print the one-line
    /// addition needed in `hyprland.conf`. Idempotent: re-running
    /// overwrites the snippet between its `# >>> ditox-managed >>>`
    /// markers without touching anything else.
    #[arg(long, conflicts_with_all = ["toggle", "show", "hide", "quit", "uninstall_hyprland_config"])]
    pub install_hyprland_config: bool,

    /// Remove the snippet written by `--install-hyprland-config`.
    #[arg(long, conflicts_with_all = ["toggle", "show", "hide", "quit", "install_hyprland_config"])]
    pub uninstall_hyprland_config: bool,
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
        } else if self.install_hyprland_config {
            Action::InstallHyprlandConfig
        } else if self.uninstall_hyprland_config {
            Action::UninstallHyprlandConfig
        } else {
            Action::Launch
        }
    }
}
