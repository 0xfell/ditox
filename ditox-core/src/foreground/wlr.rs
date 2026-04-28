//! wlroots `wlr-foreign-toplevel-management` foreground tracker.

use super::{ForegroundId, ForegroundSnapshot, ForegroundTracker};
use crate::error::{DitoxError, Result};
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use wayland_client::globals::{registry_queue_init, BindError, GlobalListContents};
use wayland_client::protocol::{wl_output, wl_registry, wl_seat};
use wayland_client::{
    delegate_noop, event_created_child, Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    self, ZwlrForeignToplevelHandleV1,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{
    self, ZwlrForeignToplevelManagerV1,
};

#[derive(Debug, Clone, Default)]
struct ToplevelInfo {
    app_id: String,
    title: String,
    activated: bool,
}

#[derive(Default)]
struct SharedState {
    current: Mutex<Option<ForegroundSnapshot>>,
    handles: Mutex<HashMap<String, ZwlrForeignToplevelHandleV1>>,
    infos: Mutex<HashMap<String, ToplevelInfo>>,
    seat: Mutex<Option<wl_seat::WlSeat>>,
    subscriber: Mutex<Option<mpsc::Sender<ForegroundSnapshot>>>,
}

struct WlrState {
    shared: Arc<SharedState>,
    _manager: ZwlrForeignToplevelManagerV1,
}

/// Foreground tracker for Sway/river/generic wlroots compositors.
pub struct WlrForegroundTracker {
    connection: Connection,
    shared: Arc<SharedState>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WlrForegroundTracker {
    pub fn new() -> Result<Self> {
        let connection = Connection::connect_to_env()
            .map_err(|e| DitoxError::Other(format!("connect to Wayland compositor: {e}")))?;
        let (globals, mut queue) = registry_queue_init::<WlrState>(&connection)
            .map_err(|e| DitoxError::Other(format!("initialise Wayland registry: {e}")))?;
        let qh = queue.handle();
        let manager = globals
            .bind::<ZwlrForeignToplevelManagerV1, _, _>(&qh, 1..=3, ())
            .map_err(bind_error("wlr-foreign-toplevel-management"))?;
        let shared = Arc::new(SharedState::default());
        if let Ok(seat) = globals.bind::<wl_seat::WlSeat, _, _>(&qh, 1..=9, ()) {
            *shared.seat.lock().unwrap() = Some(seat);
        }

        let mut state = WlrState {
            shared: Arc::clone(&shared),
            _manager: manager,
        };
        let _ = queue.roundtrip(&mut state);

        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("ditox-wlr-foreground".into())
            .spawn(move || run_event_loop(queue, state, thread_stop))
            .map_err(|e| DitoxError::Other(format!("spawn wlr foreground thread: {e}")))?;

        Ok(Self {
            connection,
            shared,
            stop,
            thread: Some(thread),
        })
    }
}

impl ForegroundTracker for WlrForegroundTracker {
    fn name(&self) -> &str {
        "wlr-foreign-toplevel"
    }

    fn snapshot(&self) -> Result<Option<ForegroundSnapshot>> {
        Ok(self.shared.current.lock().unwrap().clone())
    }

    fn restore(&self, snapshot: &ForegroundSnapshot) -> Result<()> {
        let ForegroundId::Wlr { handle_id, .. } = &snapshot.identifier else {
            return Err(DitoxError::Other("non-wlr foreground snapshot".into()));
        };
        let handle = self.shared.handles.lock().unwrap().get(handle_id).cloned();
        let seat = self.shared.seat.lock().unwrap().clone();
        if let (Some(handle), Some(seat)) = (handle, seat) {
            handle.activate(&seat);
            self.connection
                .flush()
                .map_err(|e| DitoxError::Other(format!("flush Wayland restore request: {e}")))?;
        }
        Ok(())
    }

    fn subscribe(&mut self) -> Result<mpsc::Receiver<ForegroundSnapshot>> {
        let (tx, rx) = mpsc::channel();
        *self.shared.subscriber.lock().unwrap() = Some(tx);
        Ok(rx)
    }

    fn shutdown(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }
}

impl Drop for WlrForegroundTracker {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn run_event_loop(mut queue: EventQueue<WlrState>, mut state: WlrState, stop: Arc<AtomicBool>) {
    const POLL_TIMEOUT: Duration = Duration::from_millis(100);

    while !stop.load(Ordering::Relaxed) {
        if let Err(error) = queue.flush() {
            tracing::debug!(%error, "wlr foreground flush failed");
            break;
        }
        match queue.dispatch_pending(&mut state) {
            Ok(dispatched) if dispatched > 0 => continue,
            Ok(_) => {}
            Err(error) => {
                tracing::debug!(%error, "wlr foreground dispatch failed");
                break;
            }
        }

        let Some(read_guard) = queue.prepare_read() else {
            continue;
        };
        let mut pollfd = libc::pollfd {
            fd: read_guard.connection_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pollfd, 1, POLL_TIMEOUT.as_millis() as i32) };
        if ready > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            if let Err(error) = read_guard.read() {
                tracing::debug!(%error, "wlr foreground read failed");
                break;
            }
            continue;
        }
        drop(read_guard);
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                tracing::debug!(%error, "wlr foreground poll failed");
                break;
            }
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WlrState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for WlrState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        _event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(WlrState, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for WlrState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let handle_id = handle_id(proxy);
        state
            .shared
            .handles
            .lock()
            .unwrap()
            .insert(handle_id.clone(), proxy.clone());
        let mut infos = state.shared.infos.lock().unwrap();
        let info = infos.entry(handle_id.clone()).or_default();
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => info.title = title,
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => info.app_id = app_id,
            zwlr_foreign_toplevel_handle_v1::Event::State { state: states } => {
                info.activated = state_array_has_activated(&states);
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                infos.remove(&handle_id);
                state.shared.handles.lock().unwrap().remove(&handle_id);
                return;
            }
            _ => {}
        }
        if info.activated {
            publish_snapshot(&state.shared, handle_id, info.clone());
        }
    }
}

delegate_noop!(WlrState: ignore wl_seat::WlSeat);
delegate_noop!(WlrState: ignore wl_output::WlOutput);

fn bind_error(interface: &'static str) -> impl FnOnce(BindError) -> DitoxError {
    move |error| DitoxError::Other(format!("Wayland global {interface} unavailable: {error}"))
}

fn handle_id(handle: &ZwlrForeignToplevelHandleV1) -> String {
    format!("{:?}", handle.id())
}

fn publish_snapshot(shared: &Arc<SharedState>, handle_id: String, info: ToplevelInfo) {
    let snapshot = ForegroundSnapshot {
        identifier: ForegroundId::Wlr {
            handle_id,
            app_id: info.app_id.clone(),
            title: info.title.clone(),
        },
        process_basename: app_id_basename(&info.app_id),
        title: info.title,
        captured_at: SystemTime::now(),
    };
    *shared.current.lock().unwrap() = Some(snapshot.clone());
    if let Some(tx) = shared.subscriber.lock().unwrap().as_ref() {
        let _ = tx.send(snapshot);
    }
}

fn app_id_basename(app_id: &str) -> String {
    app_id
        .rsplit(['/', '\\'])
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(app_id)
        .to_string()
}

fn state_array_has_activated(states: &[u8]) -> bool {
    states
        .chunks_exact(4)
        .any(|chunk| u32::from_ne_bytes(chunk.try_into().unwrap()) == 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_basename_strips_paths() {
        assert_eq!(
            app_id_basename("org.wezfurlong.wezterm"),
            "org.wezfurlong.wezterm"
        );
        assert_eq!(app_id_basename("/usr/bin/foot"), "foot");
    }

    #[test]
    fn state_array_detects_activated_flag() {
        let mut states = Vec::new();
        states.extend_from_slice(&0_u32.to_ne_bytes());
        states.extend_from_slice(&2_u32.to_ne_bytes());
        assert!(state_array_has_activated(&states));
        assert!(!state_array_has_activated(&0_u32.to_ne_bytes()));
    }
}
