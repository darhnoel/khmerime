//! TSF edit-session wrappers.

use std::sync::{Arc, Mutex};

use windows::core::{implement, Interface, Result};
use windows::Win32::Foundation::FALSE;
use windows::Win32::UI::TextServices::{
    ITfCompositionSink, ITfContext, ITfContextComposition, ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection,
    ITfRange, TF_AE_END, TF_ANCHOR_END, TF_CONTEXT_EDIT_CONTEXT_FLAGS, TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_NOQUERY,
    TF_IAS_QUERYONLY,
    TF_SELECTION, TF_SELECTIONSTYLE,
};

use crate::diagnostics::log;

use crate::com::text_service::TextServiceState;
use crate::tsf::candidates::CandidateWindow;
use crate::tsf::composition::KhmerImeCompositionSink;
use crate::WindowsRenderState;

/// Planned mutation responsibilities for edit sessions.
pub const EDIT_SESSION_RESPONSIBILITIES: &[&str] = &[
    "update composition text",
    "commit selected text once",
    "end composition after commit or reset",
];

pub fn request_render_edit_session(
    context: &ITfContext,
    client_id: u32,
    render_state: WindowsRenderState,
    state: Arc<Mutex<TextServiceState>>,
) -> Result<()> {
    if render_state.commit_text.is_none() && render_state.preedit.is_empty() {
        if state.lock().map(|state| state.composition.is_none()).unwrap_or(true) {
            return Ok(());
        }
    }

    let edit_session: ITfEditSession = KhmerImeEditSession::new(context.clone(), render_state, state).into();
    let flags = TF_CONTEXT_EDIT_CONTEXT_FLAGS(TF_ES_SYNC.0 | TF_ES_READWRITE.0);
    unsafe {
        match context.RequestEditSession(client_id, &edit_session, flags) {
            Err(e) => return Err(e),
            Ok(hr) if hr.is_err() => log(format!("edit_session::DoEditSession failed hr=0x{:08X}", hr.0)),
            Ok(_) => {}
        }
    }
    Ok(())
}

#[implement(ITfEditSession)]
struct KhmerImeEditSession {
    context: ITfContext,
    render_state: WindowsRenderState,
    state: Arc<Mutex<TextServiceState>>,
}

impl KhmerImeEditSession {
    fn new(context: ITfContext, render_state: WindowsRenderState, state: Arc<Mutex<TextServiceState>>) -> Self {
        Self {
            context,
            render_state,
            state,
        }
    }
}

impl ITfEditSession_Impl for KhmerImeEditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        if let Some(commit_text) = &self.render_state.commit_text {
            self.commit_text(ec, commit_text)?;
            self.hide_candidates();
            return Ok(());
        }

        if self.render_state.preedit.is_empty() {
            self.clear_composition(ec)?;
            self.hide_candidates();
            return Ok(());
        }

        self.update_composition(ec, &self.render_state.preedit)?;
        self.update_candidates();
        Ok(())
    }
}

impl KhmerImeEditSession_Impl {
    fn update_composition(&self, ec: u32, preedit: &str) -> Result<()> {
        let text = preedit.encode_utf16().collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;

        if let Some(composition) = &state.composition {
            let range = unsafe { composition.GetRange()? };
            unsafe {
                range.SetText(ec, 0, &text)?;
                range.Collapse(ec, TF_ANCHOR_END)?;
            }
            set_selection_to_range(&self.context, ec, range)?;
            return Ok(());
        }

        // Standard TSF composition start:
        // 1. Query the insertion range without modifying the document (TF_IAS_QUERYONLY).
        // 2. Start composition on that range.
        // 3. Set text on the composition's own range.
        // Inserting text before StartComposition (TF_IAS_NOQUERY) commits plain text to the
        // document first; if StartComposition then fails, the text is stranded with no
        // composition tracking and every subsequent keystroke inserts more raw text.
        let insert_at_selection = self.context.cast::<ITfInsertAtSelection>()?;
        let range = unsafe { insert_at_selection.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])? };
        let composition_context = self.context.cast::<ITfContextComposition>()?;
        let sink: ITfCompositionSink = KhmerImeCompositionSink::new(Arc::clone(&self.state)).into();
        let composition = unsafe { composition_context.StartComposition(ec, &range, &sink)? };
        let comp_range = unsafe { composition.GetRange()? };
        unsafe {
            comp_range.SetText(ec, 0, &text)?;
            comp_range.Collapse(ec, TF_ANCHOR_END)?;
        }
        set_selection_to_range(&self.context, ec, comp_range)?;
        state.composition = Some(composition);
        Ok(())
    }

    fn commit_text(&self, ec: u32, commit_text: &str) -> Result<()> {
        let text = commit_text.encode_utf16().collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;

        if let Some(composition) = state.composition.take() {
            let range = unsafe { composition.GetRange()? };
            unsafe {
                range.SetText(ec, 0, &text)?;
                composition.EndComposition(ec)?;
                range.Collapse(ec, TF_ANCHOR_END)?;
            }
            set_selection_to_range(&self.context, ec, range)?;
        } else {
            let insert_at_selection = self.context.cast::<ITfInsertAtSelection>()?;
            let range = unsafe { insert_at_selection.InsertTextAtSelection(ec, TF_IAS_NOQUERY, &text)? };
            unsafe {
                range.Collapse(ec, TF_ANCHOR_END)?;
            }
            set_selection_to_range(&self.context, ec, range)?;
        }
        Ok(())
    }

    fn clear_composition(&self, ec: u32) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
        if let Some(composition) = state.composition.take() {
            let range = unsafe { composition.GetRange()? };
            unsafe {
                range.SetText(ec, 0, &[])?;
                composition.EndComposition(ec)?;
            }
        }
        Ok(())
    }

    /// Show or update the candidate popup after a successful composition update.
    fn update_candidates(&self) {
        if let Ok(mut state) = self.state.lock() {
            if state.candidate_window.is_none() {
                state.candidate_window = CandidateWindow::create()
                    .map_err(|e| log(format!("CandidateWindow::create failed: {e:?}")))
                    .ok();
            }
            if let Some(window) = &state.candidate_window {
                window.update(
                    &self.render_state.candidates,
                    self.render_state.selected_index.unwrap_or(0),
                    &self.render_state.cursor_location,
                );
            }
        }
    }

    /// Hide the candidate popup after commit or composition clear.
    fn hide_candidates(&self) {
        if let Ok(state) = self.state.lock() {
            if let Some(window) = &state.candidate_window {
                window.hide();
            }
        }
    }
}

fn set_selection_to_range(context: &ITfContext, ec: u32, range: ITfRange) -> Result<()> {
    let mut selection = TF_SELECTION {
        range: std::mem::ManuallyDrop::new(Some(range)),
        style: TF_SELECTIONSTYLE {
            ase: TF_AE_END,
            fInterimChar: FALSE,
        },
    };
    unsafe {
        context.SetSelection(ec, std::slice::from_ref(&selection))?;
        std::mem::ManuallyDrop::drop(&mut selection.range);
    }
    Ok(())
}
