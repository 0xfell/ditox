//! Single-instance lock + IPC socket.
//!
//! # Linux
//!
//! Each running GUI holds an exclusive `flock` on
//! `$XDG_RUNTIME_DIR/ditox-gui-$UID.lock` (fallback `/tmp/...`) and listens
//! on a Unix socket `$XDG_RUNTIME_DIR/ditox-gui-$UID.sock`.
//!
//! Second launches connect to that socket, send one line (`TOGGLE`, `SHOW`,
//! `HIDE`, `QUIT`), and exit. Messages are pushed into an `mpsc::channel` so
//! the iced event loop can translate them into [`Message`](crate::app::Message)
//! through an `iced::Subscription`.
//!
//! # Windows
//!
//! On Windows we currently leave single-instance handling to the existing
//! global hotkey path and simply no-op. A future port could back this by a
//! named pipe.

use std::io;
#[cfg(unix)]
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::cli::Action;

/// Command carried over the IPC socket after parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcCommand {
    Toggle,
    Show,
    Hide,
    Quit,
}

impl IpcCommand {
    fn parse(line: &str) -> Option<Self> {
        match line.trim().to_ascii_uppercase().as_str() {
            "TOGGLE" => Some(Self::Toggle),
            "SHOW" => Some(Self::Show),
            "HIDE" => Some(Self::Hide),
            "QUIT" => Some(Self::Quit),
            _ => None,
        }
    }
}

/// A handle to the single-instance lock; released when dropped.
pub struct InstanceLock {
    #[cfg(unix)]
    _file: std::fs::File,
    #[cfg(unix)]
    path: PathBuf,
    #[cfg(unix)]
    socket_path: PathBuf,
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            // Best-effort cleanup; the flock is released when `_file` drops.
            let _ = std::fs::remove_file(&self.socket_path);
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// ---------------------------------------------------------------------------
// Unix implementation
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod unix_impl {
    use super::{InstanceLock, IpcCommand};
    use crate::cli::Action;
    use std::io::{self, BufRead, BufReader, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;
    use std::thread;

    fn runtime_base() -> PathBuf {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(dir);
            if p.is_dir() {
                return p;
            }
        }
        PathBuf::from("/tmp")
    }

    fn suffix() -> String {
        // Keep per-user files so multi-user systems don't collide.
        unsafe { format!("-{}", libc::getuid()) }
    }

    pub fn paths() -> (PathBuf, PathBuf) {
        let base = runtime_base();
        let suf = suffix();
        (
            base.join(format!("ditox-gui{}.lock", suf)),
            base.join(format!("ditox-gui{}.sock", suf)),
        )
    }

    /// Try to take the single-instance lock. Returns `Ok(Some(lock))` if we
    /// got it, `Ok(None)` if another instance holds it. Errors bubble up for
    /// unusual filesystem problems.
    pub fn try_acquire() -> io::Result<Option<InstanceLock>> {
        let (lock_path, socket_path) = paths();

        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)?;

        // Non-blocking exclusive flock.
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc == -1 {
            let err = io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
                return Ok(None);
            }
            return Err(err);
        }

        // Clear any stale socket left over from a crashed previous instance
        // (flock is gone, so we know it's safe).
        let _ = std::fs::remove_file(&socket_path);

        Ok(Some(InstanceLock {
            _file: file,
            path: lock_path,
            socket_path,
        }))
    }

    /// Spawn the IPC server in a background thread. Forwards commands through
    /// `tx`. The listener is owned by the thread and shuts down when the
    /// process exits.
    pub fn spawn_server(lock: &InstanceLock, tx: Sender<IpcCommand>) -> io::Result<()> {
        let listener = UnixListener::bind(&lock.socket_path)?;
        // Tighten permissions (0600) — runtime dir is already per-user but belt+braces.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(&lock.socket_path, std::fs::Permissions::from_mode(0o600));
        }

        thread::Builder::new()
            .name("ditox-ipc-server".into())
            .spawn(move || loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let tx = tx.clone();
                        thread::spawn(move || handle_client(stream, tx));
                    }
                    Err(e) => {
                        tracing::warn!("IPC accept error: {e}");
                        // Tiny backoff so we don't spin on fatal errors.
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            })?;

        Ok(())
    }

    fn handle_client(stream: UnixStream, tx: Sender<IpcCommand>) {
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let mut writer = match stream.try_clone() {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("IPC client clone failed: {e}");
                return;
            }
        };
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.is_empty() {
                continue;
            }
            match IpcCommand::parse(&line) {
                Some(cmd) => {
                    let _ = tx.send(cmd);
                    let _ = writeln!(writer, "OK");
                }
                None => {
                    let _ = writeln!(writer, "ERR unknown command");
                }
            }
        }
    }

    pub fn send_command(action: Action) -> io::Result<()> {
        let (_, socket_path) = paths();
        let Some(wire) = action.wire() else {
            return Ok(());
        };
        let mut stream = UnixStream::connect(&socket_path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("no running ditox-gui on {}: {}", socket_path.display(), e),
            )
        })?;
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(2)));
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        stream.write_all(wire.as_bytes())?;
        stream.write_all(b"\n")?;
        // Best-effort read of the OK/ERR reply for diagnostics.
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        let _ = reader.read_line(&mut response);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Non-Unix stub
// ---------------------------------------------------------------------------

#[cfg(not(unix))]
mod stub_impl {
    use super::{InstanceLock, IpcCommand};
    use crate::cli::Action;
    use std::io;
    use std::sync::mpsc::Sender;

    pub fn try_acquire() -> io::Result<Option<InstanceLock>> {
        // Always act as the "first" instance; Windows relies on the existing
        // single-instance behaviour (or simply allows multiple windows).
        Ok(Some(InstanceLock::new()))
    }

    pub fn spawn_server(_lock: &InstanceLock, _tx: Sender<IpcCommand>) -> io::Result<()> {
        Ok(())
    }

    pub fn send_command(_action: Action) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "IPC is not implemented on this platform",
        ))
    }
}

// ---------------------------------------------------------------------------
// Public platform-agnostic API
// ---------------------------------------------------------------------------

/// Try to claim the single-instance lock. Returns `Ok(Some(lock))` if we own
/// it, `Ok(None)` if another instance is already running.
pub fn try_acquire_instance_lock() -> io::Result<Option<InstanceLock>> {
    #[cfg(unix)]
    {
        unix_impl::try_acquire()
    }
    #[cfg(not(unix))]
    {
        stub_impl::try_acquire()
    }
}

/// Start the IPC server. Must be called once we own the lock. Commands land
/// on `tx`.
pub fn spawn_server(lock: &InstanceLock, tx: Sender<IpcCommand>) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix_impl::spawn_server(lock, tx)
    }
    #[cfg(not(unix))]
    {
        stub_impl::spawn_server(lock, tx)
    }
}

/// Send an action to the running instance (used from the "second launch"
/// path). Returns `Err` if nothing is listening.
pub fn send_to_existing(action: Action) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix_impl::send_command(action)
    }
    #[cfg(not(unix))]
    {
        stub_impl::send_command(action)
    }
}

// On non-unix builds the InstanceLock is a unit-ish struct.
#[cfg(not(unix))]
impl InstanceLock {
    // Private constructor so the stub impl can build one.
    pub(crate) fn new() -> Self {
        InstanceLock {}
    }
}
