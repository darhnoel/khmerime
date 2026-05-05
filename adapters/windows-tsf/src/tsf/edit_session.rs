//! TSF edit-session wrappers.

use std::sync::{Arc, Mutex};

use windows::core::{implement, Interface, Result};
use windows::Win32::Foundation::{BOOL, FALSE, POINT, RECT};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
use windows::Win32::UI::TextServices::{
    ITfCompositionSink, ITfContext, ITfContextComposition, ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection,
    ITfRange, TF_AE_END, TF_ANCHOR_END, TF_CONTEXT_EDIT_CONTEXT_FLAGS, TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_NOQUERY,
    TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};

use khmerime_session::CursorLocation;

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

        let composition_range = self.update_composition(ec, &self.render_state.preedit)?;
        let candidate_anchor = self
            .measure_range_location(ec, &composition_range)
            .or_else(gui_caret_location)
            .or_else(uia_focused_element_location)
            .or_else(|| usable_cursor_location(self.render_state.cursor_location));
        self.update_candidates(candidate_anchor.as_ref());
        Ok(())
    }
}

impl KhmerImeEditSession_Impl {
    fn update_composition(&self, ec: u32, preedit: &str) -> Result<ITfRange> {
        let text = preedit.encode_utf16().collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .map_err(|_| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;

        if let Some(composition) = &state.composition {
            let range = unsafe { composition.GetRange()? };
            unsafe {
                range.SetText(ec, 0, &text)?;
            }
            let measure_range = unsafe { range.Clone()? };
            let selection_range = unsafe { range.Clone()? };
            unsafe {
                selection_range.Collapse(ec, TF_ANCHOR_END)?;
            }
            set_selection_to_range(&self.context, ec, selection_range)?;
            return Ok(measure_range);
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
        }
        let measure_range = unsafe { comp_range.Clone()? };
        let selection_range = unsafe { comp_range.Clone()? };
        unsafe {
            selection_range.Collapse(ec, TF_ANCHOR_END)?;
        }
        set_selection_to_range(&self.context, ec, selection_range)?;
        state.composition = Some(composition);
        Ok(measure_range)
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
    fn update_candidates(&self, anchor: Option<&CursorLocation>) {
        if let Ok(mut state) = self.state.lock() {
            if state.candidate_window.is_none() {
                state.candidate_window = CandidateWindow::create()
                    .map_err(|e| log(format!("CandidateWindow::create failed: {e:?}")))
                    .ok();
            }
            if let Some(window) = &state.candidate_window {
                if let Some(anchor) = anchor {
                    window.update(
                        &self.render_state.candidates,
                        &self.render_state.candidate_display,
                        &self.render_state.segment_preview,
                        self.render_state.selected_index.unwrap_or(0),
                        anchor,
                    );
                } else {
                    log("edit_session::candidate anchor unavailable; hiding candidates");
                    window.hide();
                }
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

    fn measure_range_location(&self, ec: u32, range: &ITfRange) -> Option<CursorLocation> {
        match self.try_measure_range_location(ec, range) {
            Ok(Some(location)) => Some(location),
            Ok(None) => {
                log("edit_session::GetTextExt returned unusable candidate anchor");
                None
            }
            Err(err) => {
                log(format!("edit_session::GetTextExt failed: {err:?}"));
                None
            }
        }
    }

    fn try_measure_range_location(&self, ec: u32, range: &ITfRange) -> Result<Option<CursorLocation>> {
        let view = unsafe { self.context.GetActiveView()? };
        let mut rect = RECT::default();
        let mut clipped = BOOL(0);
        unsafe {
            view.GetTextExt(ec, range, &mut rect, &mut clipped)?;
        }
        let location = cursor_location_from_text_ext_rect(rect);
        log(format!(
            "edit_session::GetTextExt rect=({}, {}, {}, {}) clipped={} usable={}",
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            clipped.as_bool(),
            location.is_some()
        ));
        Ok(location)
    }
}

fn cursor_location_from_rect(rect: RECT) -> Option<CursorLocation> {
    if !is_usable_anchor_rect(&rect) {
        return None;
    }

    Some(CursorLocation {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}

fn cursor_location_from_text_ext_rect(rect: RECT) -> Option<CursorLocation> {
    if !is_usable_anchor_rect(&rect) {
        return None;
    }

    // TSF returns the full composition range. Candidate UI should track the
    // active typing/caret edge, not the start of the roman/composed span.
    Some(CursorLocation {
        x: rect.right.saturating_sub(2),
        y: rect.top,
        width: 2,
        height: rect.bottom - rect.top,
    })
}

fn usable_cursor_location(location: CursorLocation) -> Option<CursorLocation> {
    if !is_usable_anchor_rect(&RECT {
        left: location.x,
        top: location.y,
        right: location.x + location.width,
        bottom: location.y + location.height,
    }) {
        None
    } else {
        Some(location)
    }
}

fn is_usable_anchor_rect(rect: &RECT) -> bool {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 || (rect.left == 0 && rect.top == 0) {
        return false;
    }

    // Focused browser documents can report a whole viewport/window. That is too
    // coarse to feel like an IME candidate anchor, so keep only field-like rects.
    width <= 1400 && height <= 220
}

fn gui_caret_location() -> Option<CursorLocation> {
    unsafe {
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..GUITHREADINFO::default()
        };
        if GetGUIThreadInfo(0, &mut info).is_err() || info.hwndCaret.0.is_null() {
            return None;
        }

        let mut top_left = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.top,
        };
        let mut bottom_right = POINT {
            x: info.rcCaret.right,
            y: info.rcCaret.bottom,
        };
        if !ClientToScreen(info.hwndCaret, &mut top_left).as_bool()
            || !ClientToScreen(info.hwndCaret, &mut bottom_right).as_bool()
        {
            return None;
        }

        let location = cursor_location_from_rect(RECT {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        });
        log(format!(
            "edit_session::GetGUIThreadInfo caret=({}, {}, {}, {}) hwnd=0x{:X} usable={}",
            top_left.x,
            top_left.y,
            bottom_right.x,
            bottom_right.y,
            info.hwndCaret.0 as usize,
            location.is_some()
        ));
        location
    }
}

fn uia_focused_element_location() -> Option<CursorLocation> {
    match try_uia_focused_element_location() {
        Ok(Some(location)) => Some(location),
        Ok(None) => {
            log("edit_session::UIA focused element returned unusable anchor");
            None
        }
        Err(err) => {
            log(format!("edit_session::UIA focused element failed: {err:?}"));
            None
        }
    }
}

fn try_uia_focused_element_location() -> Result<Option<CursorLocation>> {
    unsafe {
        let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let element = automation.GetFocusedElement()?;
        let rect = element.CurrentBoundingRectangle()?;
        let location = cursor_location_from_rect(rect);
        log(format!(
            "edit_session::UIA focused rect=({}, {}, {}, {}) usable={}",
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            location.is_some()
        ));
        Ok(location)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_default_and_empty_anchor_rects() {
        assert!(!is_usable_anchor_rect(&RECT {
            left: 0,
            top: 0,
            right: 20,
            bottom: 20,
        }));
        assert!(!is_usable_anchor_rect(&RECT {
            left: 100,
            top: 100,
            right: 100,
            bottom: 120,
        }));
    }

    #[test]
    fn rejects_full_window_style_anchor_rects() {
        assert!(!is_usable_anchor_rect(&RECT {
            left: 100,
            top: 100,
            right: 1700,
            bottom: 1000,
        }));
    }

    #[test]
    fn accepts_browser_input_sized_anchor_rects() {
        assert_eq!(
            cursor_location_from_rect(RECT {
                left: 140,
                top: 84,
                right: 860,
                bottom: 132,
            }),
            Some(CursorLocation {
                x: 140,
                y: 84,
                width: 720,
                height: 48,
            })
        );
    }

    #[test]
    fn text_ext_anchor_tracks_trailing_caret_edge() {
        assert_eq!(
            cursor_location_from_text_ext_rect(RECT {
                left: 178,
                top: 235,
                right: 255,
                bottom: 271,
            }),
            Some(CursorLocation {
                x: 253,
                y: 235,
                width: 2,
                height: 36,
            })
        );
    }
}
