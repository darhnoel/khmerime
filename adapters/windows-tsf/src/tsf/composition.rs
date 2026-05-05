//! TSF composition/preedit rendering.

use std::sync::{Arc, Mutex};

use windows::core::{implement, Result};
use windows::Win32::UI::TextServices::{ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl};

use crate::com::text_service::TextServiceState;

/// Source field for future TSF preedit rendering.
pub const PREEDIT_SOURCE_FIELD: &str = "SessionSnapshot.preedit";

#[implement(ITfCompositionSink)]
pub struct KhmerImeCompositionSink {
    state: Arc<Mutex<TextServiceState>>,
}

impl KhmerImeCompositionSink {
    pub fn new(state: Arc<Mutex<TextServiceState>>) -> Self {
        Self { state }
    }
}

impl ITfCompositionSink_Impl for KhmerImeCompositionSink_Impl {
    fn OnCompositionTerminated(&self, _ecwrite: u32, _pcomposition: Option<&ITfComposition>) -> Result<()> {
        // TSF externally ended the composition (focus change, click, paste, etc.).
        // Clear our handle so the next key press starts a fresh composition rather
        // than calling GetRange() on the now-invalid ITfComposition.
        //
        // try_lock: EndComposition in commit_text/clear_composition calls this
        // re-entrantly on the same thread while holding the state lock. Those
        // paths already called .take(), so skipping here is safe.
        if let Ok(mut state) = self.state.try_lock() {
            state.composition = None;
        }
        Ok(())
    }
}
