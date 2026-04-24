//! Static bridge between the IPC server thread and the iced subscription.
//!
//! Iced 0.14 requires the subscription boot closure to be `Fn` (called on
//! every run). We can't move an `mpsc::Receiver` into it, so we stash both
//! the sender and receiver in statics: the IPC server clones the sender, and
//! the subscription polls the receiver via [`try_recv`].

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use crate::ipc::IpcCommand;

static SENDER: OnceLock<Sender<IpcCommand>> = OnceLock::new();
static RECEIVER: OnceLock<Mutex<Receiver<IpcCommand>>> = OnceLock::new();

/// Initialise the bridge. Must be called exactly once, before the IPC server
/// starts pushing commands. Returns a cloneable sender for the server.
pub fn init() -> Sender<IpcCommand> {
    let (tx, rx) = channel();
    let _ = SENDER.set(tx.clone());
    let _ = RECEIVER.set(Mutex::new(rx));
    tx
}

/// Non-blocking receive — used from the iced subscription poll loop.
pub fn try_recv() -> Option<IpcCommand> {
    let rx_lock = RECEIVER.get()?;
    let rx = rx_lock.lock().ok()?;
    rx.try_recv().ok()
}
