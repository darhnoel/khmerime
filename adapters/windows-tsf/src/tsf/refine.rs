//! Debounced **Visible Refiner** delivery for TSF.
//!
//! The model runs on the debounced refiner, never on the keystroke path (ADR-0016). On Linux the
//! debounce lives in the Python adapter and the result is applied on the GLib main loop; TSF has
//! no such loop, and its candidate window is thread-affine, so the same split is expressed with a
//! worker thread plus one posted window message:
//!
//! ```text
//! worker:   pause detected -> lock -> refine_now() -> PostMessage(hwnd, WM_KHMERIME_REFINE_READY)
//! wnd_proc: message -> derive render state -> refresh_candidates()   // window's owning thread
//! ```

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::session_driver::WindowsSessionDriver;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::com::text_service::TextServiceState;
use crate::diagnostics::log;
use crate::session_driver::refine_is_due;
use crate::tsf::candidates::WM_KHMERIME_REFINE_READY;
use crate::tsf::edit_session::refresh_candidates;

/// How often the worker checks whether the user has paused.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// The half of the text service that is **not** thread-affine.
///
/// `TextServiceState` holds COM interfaces and an `HWND`, so it is neither `Send` nor `Sync` and
/// can never cross a thread boundary. The session, the debounce timestamps and the popup handle
/// can, so they live here and the COM state merely references them. Without this split the refine
/// worker could not exist at all.
#[derive(Default)]
pub struct SessionShared {
    pub driver: Mutex<Option<WindowsSessionDriver>>,
    /// When the last key arrived, so the worker can tell typing from a pause.
    pub last_keystroke: Mutex<Option<Instant>>,
    /// Composition the worker has already answered, so a refine that produced nothing is not
    /// retried every poll for the same input.
    pub refine_done_for: Mutex<Option<String>>,
    /// Candidate popup handle as a raw value, published once the window exists.
    pub candidate_hwnd: AtomicIsize,
}

impl SessionShared {
    /// Records a keystroke, invalidating whatever the last refine answered.
    pub fn note_keystroke(&self) {
        if let Ok(mut at) = self.last_keystroke.lock() {
            *at = Some(Instant::now());
        }
        if let Ok(mut done) = self.refine_done_for.lock() {
            *done = None;
        }
    }

    /// Publishes the popup handle so the worker can post to it.
    pub fn publish_candidate_hwnd(&self, hwnd: isize) {
        self.candidate_hwnd.store(hwnd, Ordering::Relaxed);
    }
}

thread_local! {
    /// The TSF state owned by *this* thread.
    ///
    /// TSF activates the text service per thread and the candidate window belongs to that same
    /// thread, so the message handler can only ever need this thread's state. A thread-local keeps
    /// that invariant structural instead of threading a handle through the window struct.
    static ACTIVE_STATE: RefCell<Option<Arc<Mutex<TextServiceState>>>> = const { RefCell::new(None) };
}

/// Binds this thread's state so [`on_refine_ready`] can find it. Called from `Activate`.
pub fn bind_thread_state(state: Arc<Mutex<TextServiceState>>) {
    ACTIVE_STATE.with(|slot| *slot.borrow_mut() = Some(state));
}

/// Releases the binding. Called from `Deactivate`.
pub fn clear_thread_state() {
    ACTIVE_STATE.with(|slot| *slot.borrow_mut() = None);
}

/// Repaints the candidate popup after a refine. Runs on the window's owning thread.
pub fn on_refine_ready() {
    let Some(state) = ACTIVE_STATE.with(|slot| slot.borrow().clone()) else {
        return;
    };
    let render_state = {
        let Ok(guard) = state.lock() else { return };
        let Ok(driver) = guard.shared.driver.lock() else { return };
        let Some(driver) = driver.as_ref() else { return };
        driver.snapshot_render_state()
    };
    refresh_candidates(&state, &render_state);
}

/// Starts the debounce worker for this text service.
///
/// Returns the flag that stops it; `Deactivate` clears it so the thread does not outlive the
/// profile it belongs to.
pub fn spawn_refine_worker(shared: Arc<SessionShared>) -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let stop = Arc::clone(&running);

    thread::spawn(move || {
        while stop.load(Ordering::Relaxed) {
            thread::sleep(POLL_INTERVAL);

            let idle = shared
                .last_keystroke
                .lock()
                .ok()
                .and_then(|at| *at)
                .map(|at| at.elapsed())
                .unwrap_or_default();

            // The lock is held across inference. That is acceptable *because* the refine only
            // starts after the user has paused: a keystroke racing it is a bounded, rare stall,
            // not a per-keystroke cost. Refining without it would need the session to hand out a
            // read-only refine, which it does not.
            let Ok(mut driver_slot) = shared.driver.lock() else {
                break;
            };
            let Some(driver) = driver_slot.as_mut() else { continue };

            let raw = driver.session().composition_raw().to_owned();
            let already_done = shared
                .refine_done_for
                .lock()
                .ok()
                .map(|done| done.as_deref() == Some(raw.as_str()))
                .unwrap_or(false);
            if already_done || !refine_is_due(idle, &raw) {
                continue;
            }

            let changed = driver.refine_now();
            drop(driver_slot);

            // Record the attempt either way: a refine that produced nothing must not be retried
            // every poll for the same composition.
            if let Ok(mut done) = shared.refine_done_for.lock() {
                *done = Some(raw);
            }

            let hwnd = shared.candidate_hwnd.load(Ordering::Relaxed);
            if changed && hwnd != 0 {
                log("[refine] applied; posting repaint");
                // SAFETY: PostMessageW is documented as callable from any thread; it queues the
                // message for the window's owning thread rather than touching window state here.
                unsafe {
                    let _ = PostMessageW(HWND(hwnd as *mut _), WM_KHMERIME_REFINE_READY, WPARAM(0), LPARAM(0));
                }
            }
        }
    });

    running
}
