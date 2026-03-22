//! Event-driven Wayland clipboard watcher using the zwlr_data_control protocol.
//!
//! Instead of polling the clipboard (which races with XWayland bridge propagation),
//! this maintains a persistent Wayland connection that receives `selection` events
//! as they happen. `handle_inspect` waits for the *next* clipboard change rather
//! than hoping the data has already arrived.

use std::io::Read as _;
use std::os::fd::AsFd;
use std::sync::{mpsc, Arc, Mutex};

use tauri::Manager;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{delegate_noop, event_created_child, Connection, Dispatch, QueueHandle};

use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
};

/// Shared coordination between the watcher thread and `handle_inspect`.
struct ClipboardRequest {
    pending: Mutex<Option<mpsc::SyncSender<String>>>,
}

/// Persistent clipboard watcher using the zwlr_data_control Wayland protocol.
///
/// Managed as Tauri state. Call `request_next()` to get a receiver that delivers
/// the text content of the next clipboard selection change.
pub struct ClipboardWatcher {
    request: Arc<ClipboardRequest>,
}

impl ClipboardWatcher {
    /// Start the watcher on a background thread.
    ///
    /// Returns `None` if the compositor doesn't support `zwlr_data_control`
    /// or the Wayland connection fails.
    pub fn start() -> Option<Self> {
        let request = Arc::new(ClipboardRequest {
            pending: Mutex::new(None),
        });
        let req_clone = Arc::clone(&request);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<bool>(1);

        std::thread::Builder::new()
            .name("clipboard-watcher".into())
            .spawn(move || {
                run_watcher(req_clone, ready_tx);
            })
            .ok()?;

        match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(true) => Some(ClipboardWatcher { request }),
            _ => None,
        }
    }

    /// Request the next clipboard change. Returns a receiver that will
    /// deliver the text content when a `selection` event fires.
    ///
    /// Replaces any previous pending request (the old sender is dropped,
    /// causing the old receiver to get `RecvError`).
    fn request_next(&self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::sync_channel(1);
        *self.request.pending.lock().unwrap() = Some(tx);
        rx
    }
}

/// Linux clipboard acquisition: single keystroke, race Wayland watcher vs X11 poll.
///
/// 1. Snapshot X11 clipboard (for change detection).
/// 2. Arm Wayland watcher if available.
/// 3. Send one keystroke.
/// 4. Poll both paths: Wayland watcher (non-blocking) and X11 clipboard change.
pub fn acquire_clipboard(app: &tauri::AppHandle) -> Option<String> {
    use super::wayland;

    // Snapshot X11 clipboard BEFORE keystroke for change detection
    let old = wayland::read_x11_clipboard(100).unwrap_or_default();

    // Arm Wayland watcher if available
    let watcher_rx = app
        .try_state::<ClipboardWatcher>()
        .map(|w| w.request_next());

    // Send one keystroke
    if let Err(e) = crate::clipboard_acquire::send_copy_keystroke() {
        eprintln!("[inspect] Keystroke FAILED: {e}");
        return None;
    }

    // Race both paths concurrently
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        // Check Wayland watcher (non-blocking)
        if let Some(rx) = &watcher_rx {
            if let Ok(text) = rx.try_recv() {
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Check X11 clipboard for change
        if let Ok(text) = wayland::read_x11_clipboard(100) {
            if !text.is_empty() && text != old {
                return Some(text);
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Wayland dispatch state
// ---------------------------------------------------------------------------

/// State for the background Wayland event loop.
struct WatcherState {
    request: Arc<ClipboardRequest>,
    /// Offer from the most recent `data_offer` event (not yet promoted).
    incoming_offer: Option<ZwlrDataControlOfferV1>,
    /// MIME types collected for the incoming offer.
    incoming_mime_types: Vec<String>,
    /// The current clipboard selection offer.
    current_offer: Option<ZwlrDataControlOfferV1>,
    /// MIME types for the current selection.
    current_mime_types: Vec<String>,
}

impl WatcherState {
    /// If a pending request exists and the current offer has a text MIME type,
    /// read the clipboard content via a pipe and send it to the requester.
    fn try_fulfill_request(&mut self, conn: &Connection) {
        let offer = match &self.current_offer {
            Some(o) => o,
            None => return,
        };

        let sender = match self.request.pending.lock().unwrap().take() {
            Some(s) => s,
            None => return,
        };

        // Find best text MIME type
        let mime = self
            .current_mime_types
            .iter()
            .find(|m| m.as_str() == "text/plain;charset=utf-8")
            .or_else(|| {
                self.current_mime_types
                    .iter()
                    .find(|m| m.as_str() == "UTF8_STRING")
            })
            .or_else(|| {
                self.current_mime_types
                    .iter()
                    .find(|m| m.as_str() == "text/plain")
            });

        let mime = match mime {
            Some(m) => m.clone(),
            None => {
                eprintln!("[clipboard-watcher] No text MIME type in offer");
                return;
            }
        };

        let (read_fd, write_fd) = match os_pipe::pipe() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[clipboard-watcher] pipe() failed: {e}");
                return;
            }
        };

        // Request data from the selection source via the compositor
        offer.receive(mime, write_fd.as_fd());
        drop(write_fd);
        let _ = conn.flush();

        // Read the pipe on a separate thread to avoid blocking the Wayland event loop
        std::thread::spawn(move || {
            let mut buf = String::new();
            let mut reader = read_fd;
            match reader.read_to_string(&mut buf) {
                Ok(_) => {
                    let _ = sender.send(buf);
                }
                Err(e) => {
                    eprintln!("[clipboard-watcher] pipe read error: {e}");
                }
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Dispatch implementations
// ---------------------------------------------------------------------------

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WatcherState {
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

delegate_noop!(WatcherState: ignore wl_seat::WlSeat);
delegate_noop!(WatcherState: ignore ZwlrDataControlManagerV1);

impl Dispatch<ZwlrDataControlDeviceV1, ()> for WatcherState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _data: &(),
        conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                state.incoming_offer = Some(id);
                state.incoming_mime_types.clear();
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                // Destroy old current offer
                if let Some(old) = state.current_offer.take() {
                    old.destroy();
                }
                if id.is_some() {
                    // Promote incoming offer (same underlying object) to current
                    state.current_offer = state.incoming_offer.take();
                    state.current_mime_types = std::mem::take(&mut state.incoming_mime_types);
                    state.try_fulfill_request(conn);
                }
            }
            zwlr_data_control_device_v1::Event::Finished => {}
            _ => {}
        }
    }

    // Register the child object type created by data_offer (opcode 0)
    event_created_child!(WatcherState, ZwlrDataControlDeviceV1, [
        0 => (ZwlrDataControlOfferV1, ())
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for WatcherState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.incoming_mime_types.push(mime_type);
        }
    }
}

// ---------------------------------------------------------------------------
// Background event loop
// ---------------------------------------------------------------------------

/// Run the Wayland event loop. Blocks forever on success.
fn run_watcher(request: Arc<ClipboardRequest>, ready_tx: mpsc::SyncSender<bool>) {
    let result: Result<(), String> = (|| {
        let conn = Connection::connect_to_env().map_err(|e| format!("Wayland connect: {e}"))?;

        let (globals, mut queue) = registry_queue_init::<WatcherState>(&conn)
            .map_err(|e| format!("Registry init: {e}"))?;
        let qh = queue.handle();

        let manager: ZwlrDataControlManagerV1 = globals
            .bind(&qh, 1..=2, ())
            .map_err(|e| format!("Bind data_control_manager: {e}"))?;
        let seat: wl_seat::WlSeat = globals
            .bind(&qh, 1..=9, ())
            .map_err(|e| format!("Bind wl_seat: {e}"))?;

        let _device = manager.get_data_device(&seat, &qh, ());

        let mut state = WatcherState {
            request,
            incoming_offer: None,
            incoming_mime_types: Vec::new(),
            current_offer: None,
            current_mime_types: Vec::new(),
        };

        // Initial roundtrip processes the first selection event (current state)
        queue
            .roundtrip(&mut state)
            .map_err(|e| format!("Roundtrip: {e}"))?;

        let _ = ready_tx.send(true);

        loop {
            queue
                .blocking_dispatch(&mut state)
                .map_err(|e| format!("Dispatch: {e}"))?;
        }
    })();

    if let Err(e) = result {
        eprintln!("[clipboard-watcher] {e}");
        let _ = ready_tx.send(false);
    }
}
