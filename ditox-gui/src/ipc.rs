//! Single-instance IPC for ditox-gui (Phase 4 sub-tasks 4.1 + 4.2).
//!
//! When the user (or a compositor keybind) launches `ditox-gui`, we
//! either become the daemon (acquire the runtime lock + bind the
//! socket + run iced) or detect a running daemon and forward the
//! requested [`crate::cli::Action`] over the socket.
//!
//! ## Protocol
//!
//! Newline-terminated text. Each request is a single line:
//!
//! ```text
//! TOGGLE
//! SHOW
//! HIDE
//! QUIT
//! STATUS
//! ```
//!
//! Reply lines:
//!
//! ```text
//! OK
//! OK <payload>          # for STATUS
//! ERR <message>
//! ```
//!
//! ## Lock + socket paths (Linux)
//!
//! - Lock: `$XDG_RUNTIME_DIR/ditox-gui-<uid>.lock` (locked exclusively
//!   via `flock(LOCK_EX | LOCK_NB)`).
//! - Socket: `$XDG_RUNTIME_DIR/ditox-gui-<uid>.sock` (mode `0600`).
//!
//! Both paths are stable for a UID across reboots — the OS clears
//! `XDG_RUNTIME_DIR` on logout. If `XDG_RUNTIME_DIR` is unset (very
//! minimal session managers) we fall back to `/tmp/ditox-gui-<uid>`.
//!
//! ## Windows
//!
//! Out of scope for the first Phase 4 cut (cfg-gated below). Will
//! land alongside the layer-shell work in 4.3 since both are
//! Linux-first; Windows IPC uses a named pipe and lands in a
//! follow-up commit.

use crate::cli::Action;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

/// File-system suffix to reach the lock + sock files inside the
/// runtime dir.
const LOCK_BASENAME: &str = "ditox-gui-";
const LOCK_EXT: &str = ".lock";
const SOCK_EXT: &str = ".sock";

/// Resolve the lock path for the current user. Used on Unix only.
#[cfg(unix)]
pub fn lock_path() -> PathBuf {
    runtime_path(LOCK_EXT)
}

/// Resolve the socket path for the current user. Unix only.
#[cfg(unix)]
pub fn socket_path() -> PathBuf {
    runtime_path(SOCK_EXT)
}

#[cfg(unix)]
fn runtime_path(ext: &str) -> PathBuf {
    // SAFETY: getuid() is always safe to call.
    let uid = unsafe { libc::getuid() };
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/ditox-gui-{uid}")));
    let _ = std::fs::create_dir_all(&dir);
    let mut name = String::from(LOCK_BASENAME);
    name.push_str(&uid.to_string());
    name.push_str(ext);
    dir.join(name)
}

/// Convert a CLI [`Action`] to its on-the-wire command string.
/// Returns `None` for [`Action::Launch`] when there's no active
/// daemon to send to (the caller starts a fresh daemon instead),
/// and for the local-only Hyprland-config actions
/// ([`Action::InstallHyprlandConfig`] / [`Action::UninstallHyprlandConfig`])
/// which are handled in `main.rs::run` before any IPC attempt.
pub fn action_to_wire(action: Action) -> Option<String> {
    action.wire()
}

/// Result of a [`try_send_to_daemon`] call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    /// Daemon present, command sent, reply received.
    Sent { reply: String },
    /// No daemon listening on the socket. Caller should bind it
    /// themselves and become the daemon.
    NoDaemon,
    /// Daemon was reachable but rejected the command (`ERR ...`).
    Rejected { message: String },
}

/// Attempt to send `action` over the daemon socket. Returns
/// [`SendOutcome::NoDaemon`] when the socket file is absent or
/// connection is refused — the typical "first launch" case.
///
/// Linux only; on other platforms returns
/// [`SendOutcome::NoDaemon`] without attempting anything.
pub fn try_send_to_daemon(action: Action) -> SendOutcome {
    #[cfg(unix)]
    {
        let path = socket_path();
        let wire = match action_to_wire(action) {
            Some(w) => w,
            None => return SendOutcome::NoDaemon, // Launch + no daemon → caller starts one
        };
        match send_unix(&path, &wire) {
            Ok(reply) => {
                if let Some(rest) = reply.strip_prefix("ERR ") {
                    SendOutcome::Rejected {
                        message: rest.trim().to_string(),
                    }
                } else {
                    SendOutcome::Sent {
                        reply: reply.trim().to_string(),
                    }
                }
            }
            Err(e) => {
                tracing::debug!(error = %e, path = %path.display(), "no daemon on socket");
                SendOutcome::NoDaemon
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = action;
        SendOutcome::NoDaemon
    }
}

#[cfg(unix)]
fn send_unix(path: &std::path::Path, wire: &str) -> std::io::Result<String> {
    use std::os::unix::net::UnixStream;
    let stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream.set_write_timeout(Some(Duration::from_millis(500)))?;
    let mut writer = &stream;
    writer.write_all(wire.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line)
}

/// Acquire the daemon lock. Returns the locked file (caller MUST
/// keep it alive for the daemon's lifetime — drop releases the
/// lock) or `None` if another process holds it.
///
/// Linux only. The lock file is created `0600`; the rest of the
/// runtime dir is the OS's responsibility.
#[cfg(unix)]
pub fn acquire_lock() -> Option<std::fs::File> {
    use fs2::FileExt;
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;
    let path = lock_path();
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
        .ok()?;
    if f.try_lock_exclusive().is_ok() {
        Some(f)
    } else {
        None
    }
}

/// Stub for non-Unix targets so `main.rs` compiles cleanly.
#[cfg(not(unix))]
pub fn acquire_lock() -> Option<std::fs::File> {
    None
}

/// Spawn the daemon's IPC accept loop. Returns the receiver end of
/// a `mpsc::Receiver<DaemonCommand>` that the iced subscription
/// will poll. The accept loop runs on a dedicated background
/// thread; the `JoinHandle` is dropped (the thread exits when the
/// listener is dropped or the channel receiver is gone).
///
/// Linux only. Returns an error if the socket can't be bound.
#[cfg(unix)]
pub fn spawn_listener() -> std::io::Result<(std::sync::mpsc::Receiver<DaemonCommand>, SocketGuard)>
{
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    let path = socket_path();
    // Best-effort cleanup of any stale socket file. If a previous
    // daemon crashed, the kernel garbage-collects the inode but
    // the path remains — `bind` would fail with "address in use".
    let _ = std::fs::remove_file(&path);

    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;

    let (tx, rx) = std::sync::mpsc::channel::<DaemonCommand>();

    let listener_path = path.clone();
    std::thread::Builder::new()
        .name("ditox-gui-ipc".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let stream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::debug!(error = %e, "ipc accept failed");
                        continue;
                    }
                };
                let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
                let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

                let tx = tx.clone();
                std::thread::Builder::new()
                    .name("ditox-gui-ipc-client".into())
                    .spawn(move || handle_client(stream, tx))
                    .ok(); // best-effort spawn; lost worker = lost reply, but daemon survives
            }
        })?;

    Ok((
        rx,
        SocketGuard {
            path: listener_path,
        },
    ))
}

#[cfg(not(unix))]
pub fn spawn_listener() -> std::io::Result<(std::sync::mpsc::Receiver<DaemonCommand>, SocketGuard)>
{
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "ditox-gui IPC not implemented for this platform",
    ))
}

#[cfg(unix)]
fn handle_client(
    stream: std::os::unix::net::UnixStream,
    tx: std::sync::mpsc::Sender<DaemonCommand>,
) {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let cmd = line.trim();

    let parsed = parse_command(cmd);

    let response = match parsed {
        Some(parsed) => {
            // Forward to the daemon's GUI thread via the channel.
            // The reply has to be synchronous-ish so the client's
            // CLI exit code reflects success/failure; we use a
            // one-shot reply channel.
            let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel::<String>(1);
            let _ = tx.send(DaemonCommand {
                command: parsed,
                reply: Some(reply_tx),
            });
            // Wait briefly for the GUI thread to reply.
            match reply_rx.recv_timeout(Duration::from_millis(2000)) {
                Ok(reply) => reply,
                Err(_) => "ERR daemon-busy".to_string(),
            }
        }
        None => format!("ERR unknown-command {cmd}"),
    };

    let mut writer = &stream;
    let _ = writer.write_all(response.as_bytes());
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();
}

/// Parsed command sent to the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Show,
    Hide,
    Quit,
    Status,
    PasteClip(String),
    Emit(String),
    CollectionAdd {
        name: String,
        color: Option<String>,
        keybind: Option<char>,
    },
    TagEntry {
        entry_id: String,
        tag_name: String,
    },
    ScriptRun {
        script_id: String,
        entry_id: String,
    },
    ScriptReload,
    ReloadConfig,
    GetEntry(String),
    ListEntries {
        limit: usize,
        json: bool,
    },
}

fn parse_command(line: &str) -> Option<Command> {
    let mut parts = line.split_whitespace();
    let verb = parts.next()?.to_ascii_uppercase();
    Some(match verb.as_str() {
        "TOGGLE" => Command::Toggle,
        "SHOW" => Command::Show,
        "HIDE" => Command::Hide,
        "QUIT" => Command::Quit,
        "STATUS" => Command::Status,
        "PASTE-CLIP" => Command::PasteClip(parts.next()?.to_string()),
        "EMIT" => Command::Emit(parts.next()?.to_string()),
        "COLLECTION-ADD" => Command::CollectionAdd {
            name: parts.next()?.to_string(),
            color: parts.next().map(str::to_string),
            keybind: parts.next().and_then(|s| s.chars().next()),
        },
        "TAG-ENTRY" => Command::TagEntry {
            entry_id: parts.next()?.to_string(),
            tag_name: parts.next()?.to_string(),
        },
        "SCRIPT-RUN" => Command::ScriptRun {
            script_id: parts.next()?.to_string(),
            entry_id: parts.next()?.to_string(),
        },
        "SCRIPT-RELOAD" => Command::ScriptReload,
        "RELOAD-CONFIG" => Command::ReloadConfig,
        "GET-ENTRY" => Command::GetEntry(parts.next()?.to_string()),
        "LIST-ENTRIES" => {
            let limit = parts
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10);
            let json = parts.any(|s| s.eq_ignore_ascii_case("json") || s == "--json");
            Command::ListEntries { limit, json }
        }
        _ => return None,
    })
}

/// Wraps a daemon-side received command together with a reply
/// channel so the GUI's update loop can both act on the command
/// and send a status string back to the IPC client.
pub struct DaemonCommand {
    pub command: Command,
    pub reply: Option<std::sync::mpsc::SyncSender<String>>,
}

impl DaemonCommand {
    /// Reply with `OK` (or `OK <payload>` for STATUS-like commands).
    /// Best-effort — if the client disconnected before we replied,
    /// the send silently fails. Idempotent: only the first reply
    /// is sent.
    pub fn reply_ok(&mut self) {
        if let Some(tx) = self.reply.take() {
            let _ = tx.send("OK".to_string());
        }
    }

    /// Reply with `OK <payload>`.
    pub fn reply_ok_with(&mut self, payload: &str) {
        if let Some(tx) = self.reply.take() {
            let _ = tx.send(format!("OK {payload}"));
        }
    }

    /// Reply with `ERR <message>`. Currently unused — every command
    /// path replies `OK` — but kept on the API surface for future
    /// commands that can fail (e.g. a `paste-clip <id>` Phase 4
    /// command would reject unknown ids with `ERR not-found`).
    #[allow(dead_code)]
    pub fn reply_err(&mut self, message: &str) {
        if let Some(tx) = self.reply.take() {
            let _ = tx.send(format!("ERR {message}"));
        }
    }
}

impl Drop for DaemonCommand {
    fn drop(&mut self) {
        // Send an ERR if the GUI dropped the command without
        // replying — keeps the client from hanging on read.
        if let Some(tx) = self.reply.take() {
            let _ = tx.send("ERR no-reply".to_string());
        }
    }
}

/// RAII wrapper around the listener's socket file. On drop the
/// socket file is unlinked from the filesystem.
pub struct SocketGuard {
    path: PathBuf,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_accepts_canonical_names() {
        assert_eq!(parse_command("TOGGLE"), Some(Command::Toggle));
        assert_eq!(parse_command("SHOW"), Some(Command::Show));
        assert_eq!(parse_command("HIDE"), Some(Command::Hide));
        assert_eq!(parse_command("QUIT"), Some(Command::Quit));
        assert_eq!(parse_command("STATUS"), Some(Command::Status));
    }

    #[test]
    fn parse_command_is_case_insensitive() {
        assert_eq!(parse_command("toggle"), Some(Command::Toggle));
        assert_eq!(parse_command("Show"), Some(Command::Show));
    }

    #[test]
    fn parse_phase5_commands() {
        assert_eq!(
            parse_command("PASTE-CLIP abc"),
            Some(Command::PasteClip("abc".into()))
        );
        assert_eq!(
            parse_command("COLLECTION-ADD Work #ff00aa w"),
            Some(Command::CollectionAdd {
                name: "Work".into(),
                color: Some("#ff00aa".into()),
                keybind: Some('w'),
            })
        );
        assert_eq!(
            parse_command("TAG-ENTRY entry-1 rust"),
            Some(Command::TagEntry {
                entry_id: "entry-1".into(),
                tag_name: "rust".into(),
            })
        );
        assert_eq!(
            parse_command("SCRIPT-RUN strip entry-1"),
            Some(Command::ScriptRun {
                script_id: "strip".into(),
                entry_id: "entry-1".into(),
            })
        );
        assert_eq!(parse_command("SCRIPT-RELOAD"), Some(Command::ScriptReload));
        assert_eq!(
            parse_command("LIST-ENTRIES 25 --json"),
            Some(Command::ListEntries {
                limit: 25,
                json: true,
            })
        );
    }

    #[test]
    fn parse_command_rejects_unknown() {
        assert_eq!(parse_command(""), None);
        assert_eq!(parse_command("FROBNICATE"), None);
        assert_eq!(parse_command("TOGGLE "), Some(Command::Toggle));
    }

    #[test]
    fn action_to_wire_round_trip() {
        assert_eq!(action_to_wire(Action::Toggle), Some("TOGGLE".into()));
        assert_eq!(action_to_wire(Action::Show), Some("SHOW".into()));
        assert_eq!(action_to_wire(Action::Hide), Some("HIDE".into()));
        assert_eq!(action_to_wire(Action::Quit), Some("QUIT".into()));
        // Launch maps to TOGGLE for the "summon a running daemon"
        // case; when no daemon, the caller falls back to becoming
        // one.
        assert_eq!(action_to_wire(Action::Launch), Some("TOGGLE".into()));
    }

    #[cfg(unix)]
    #[test]
    fn lock_path_lives_in_runtime_dir_or_tmp() {
        let p = lock_path();
        let s = p.to_string_lossy();
        assert!(s.contains("ditox-gui-"));
        assert!(s.ends_with(".lock"));
    }

    #[cfg(unix)]
    #[test]
    fn daemon_command_reply_ok_sends_ok() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<String>(1);
        let mut cmd = DaemonCommand {
            command: Command::Toggle,
            reply: Some(tx),
        };
        cmd.reply_ok();
        let got = rx.recv().unwrap();
        assert_eq!(got, "OK");
    }

    #[cfg(unix)]
    #[test]
    fn daemon_command_reply_err_sends_err() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<String>(1);
        let mut cmd = DaemonCommand {
            command: Command::Toggle,
            reply: Some(tx),
        };
        cmd.reply_err("oops");
        let got = rx.recv().unwrap();
        assert_eq!(got, "ERR oops");
    }

    #[cfg(unix)]
    #[test]
    fn daemon_command_drop_without_reply_sends_err() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<String>(1);
        let cmd = DaemonCommand {
            command: Command::Toggle,
            reply: Some(tx),
        };
        drop(cmd);
        let got = rx.recv().unwrap();
        assert_eq!(got, "ERR no-reply");
    }
}
