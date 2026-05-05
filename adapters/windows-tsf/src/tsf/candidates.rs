//! Candidate popup window and inline-preview helpers.

use std::sync::OnceLock;

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, FillRect, GetStockObject, GetSysColor, GetSysColorBrush, InvalidateRect,
    SelectObject, SetBkMode, SetTextColor, TextOutW, UpdateWindow, COLOR_HIGHLIGHT,
    COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DEFAULT_GUI_FONT, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, GetWindowLongPtrW,
    RegisterClassExW, SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW,
    GWLP_USERDATA, HMENU, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, WM_NCHITTEST, WM_PAINT, WNDCLASSEXW, WS_BORDER, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use khmerime_session::CursorLocation;

use crate::com::dll_module::module_instance;
use crate::diagnostics::log;
use crate::WindowsRenderState;

pub const CANDIDATE_SOURCE_FIELDS: &[&str] =
    &["SessionSnapshot.candidates", "SessionSnapshot.selected_index"];

// Window class name — registered once per process.
const WCLASS: PCWSTR = w!("KhmerIMECandidates");

const WIN_W: i32 = 220;
const ROW_H: i32 = 22;
const PAD_X: i32 = 8;
const PAD_Y: i32 = 4;
const MAX_ROWS: usize = 9;

static CLASS_REGISTERED: OnceLock<()> = OnceLock::new();

// ---------------------------------------------------------------------------
// Per-window paint data — stored in GWLP_USERDATA as a raw Box pointer.
// ---------------------------------------------------------------------------

struct CandidateData {
    // Each entry is the display string and whether that row is selected.
    rows: Vec<(String, bool)>,
}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// Native Win32 popup showing numbered candidates near the composition cursor.
/// Never activates (WS_EX_NOACTIVATE) so focus stays in the host application.
pub struct CandidateWindow {
    hwnd: HWND,
}

impl CandidateWindow {
    /// Create the popup window.  Returns an error if Win32 setup fails.
    pub fn create() -> Result<Self> {
        CLASS_REGISTERED.get_or_init(|| {
            if let Err(e) = register_class() {
                log(format!("CandidateWindow: RegisterClassExW failed: {e:?}"));
            }
        });

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                WCLASS,
                PCWSTR::null(),
                WS_POPUP | WS_BORDER,
                0,
                0,
                WIN_W,
                ROW_H,
                HWND::default(),
                HMENU::default(),
                module_instance(),
                None,
            )?
        };

        Ok(Self { hwnd })
    }

    /// Refresh the candidate list and show the window below `location`.
    /// Hides the window if `candidates` is empty.
    pub fn update(&self, candidates: &[String], selected: usize, location: &CursorLocation) {
        if candidates.is_empty() {
            self.hide();
            return;
        }

        let rows: Vec<(String, bool)> = candidates
            .iter()
            .take(MAX_ROWS)
            .enumerate()
            .map(|(i, c)| (format!("{}  {}", i + 1, c), i == selected))
            .collect();

        let count = rows.len() as i32;
        let height = PAD_Y * 2 + count * ROW_H;

        unsafe {
            // Replace paint data atomically (still on the STA thread).
            let old = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut CandidateData;
            if !old.is_null() {
                drop(Box::from_raw(old));
            }
            let data = Box::new(CandidateData { rows });
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, Box::into_raw(data) as isize);

            // Position below the preedit cursor; flip above when near screen bottom.
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let cx = location.x as i32;
            let cy = (location.y + location.height) as i32;
            let x = cx.min(screen_w - WIN_W).max(0);
            let y = if cy + height > screen_h && location.y as i32 >= height {
                location.y as i32 - height
            } else {
                cy
            };

            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                WIN_W,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = InvalidateRect(self.hwnd, None, true);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    pub fn hide(&self) {
        unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        unsafe {
            // Free paint data before the window is destroyed.
            let old = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut CandidateData;
            if !old.is_null() {
                drop(Box::from_raw(old));
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            }
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

// ---------------------------------------------------------------------------
// Win32 internals
// ---------------------------------------------------------------------------

fn register_class() -> Result<()> {
    let mut wc = unsafe { std::mem::zeroed::<WNDCLASSEXW>() };
    wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
    wc.style = CS_HREDRAW | CS_VREDRAW;
    wc.lpfnWndProc = Some(candidate_wnd_proc);
    wc.hInstance = module_instance();
    wc.lpszClassName = WCLASS;
    wc.hbrBackground = unsafe { GetSysColorBrush(COLOR_WINDOW) };

    let atom = unsafe { RegisterClassExW(&wc) };
    if atom == 0 {
        let e = windows::core::Error::from_win32();
        // ERROR_CLASS_ALREADY_EXISTS (0x582) is benign — another instance
        // of the DLL already registered the class.
        if e.code().0 as u32 != 0x80070582 {
            return Err(e);
        }
    }
    Ok(())
}

unsafe extern "system" fn candidate_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const CandidateData;
            let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
            let hdc = BeginPaint(hwnd, &mut ps);

            if !data_ptr.is_null() {
                let data = &*data_ptr;
                let hfont = GetStockObject(DEFAULT_GUI_FONT);
                let old_font = SelectObject(hdc, hfont);
                SetBkMode(hdc, TRANSPARENT);

                for (i, (text, is_selected)) in data.rows.iter().enumerate() {
                    let row_top = PAD_Y + i as i32 * ROW_H;
                    let row_rect = RECT {
                        left: 0,
                        top: row_top,
                        right: WIN_W,
                        bottom: row_top + ROW_H,
                    };

                    if *is_selected {
                        FillRect(hdc, &row_rect, GetSysColorBrush(COLOR_HIGHLIGHT));
                        SetTextColor(hdc, COLORREF(GetSysColor(COLOR_HIGHLIGHTTEXT)));
                    } else {
                        SetTextColor(hdc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
                    }

                    let utf16: Vec<u16> = text.encode_utf16().collect();
                    // Vertically center text in the row (font is ~14 px tall).
                    let _ = TextOutW(hdc, PAD_X, row_top + (ROW_H - 14) / 2, &utf16);
                }

                SelectObject(hdc, old_font);
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCHITTEST => {
            // Pass all mouse events through to the host application so focus
            // is never stolen.  Equivalent to HTTRANSPARENT (-1).
            LRESULT(-1)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---------------------------------------------------------------------------
// Inline-preview helper (kept for tests and non-popup fallback).
// ---------------------------------------------------------------------------

pub fn inline_preview_text(render_state: &WindowsRenderState) -> Option<String> {
    if render_state.preedit.is_empty() {
        return None;
    }
    let mut preview = render_state.preedit.clone();
    if !render_state.candidates.is_empty() {
        let candidates = render_state
            .candidates
            .iter()
            .take(5)
            .enumerate()
            .map(|(index, candidate)| {
                if Some(index) == render_state.selected_index {
                    format!("{}. *{}", index + 1, candidate)
                } else {
                    format!("{}. {}", index + 1, candidate)
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
        preview.push_str("  [");
        preview.push_str(&candidates);
        preview.push(']');
    }
    Some(preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_preview_marks_selected_candidate() {
        let preview = inline_preview_text(&WindowsRenderState {
            preedit: "jea".to_owned(),
            candidates: vec!["candidate".to_owned(), "other".to_owned()],
            selected_index: Some(1),
            ..WindowsRenderState::default()
        });
        assert_eq!(preview.as_deref(), Some("jea  [1. candidate  2. *other]"));
    }

    #[test]
    fn empty_preedit_hides_preview() {
        assert!(inline_preview_text(&WindowsRenderState::default()).is_none());
    }
}
