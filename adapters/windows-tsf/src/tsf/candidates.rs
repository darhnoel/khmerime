//! Candidate popup window and inline-preview helpers.

use std::sync::OnceLock;

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, DeleteObject, EndPaint, FillRect, GetMonitorInfoW, GetSysColor, GetSysColorBrush,
    InvalidateRect, MonitorFromPoint, SelectObject, SetBkMode, SetTextColor, TextOutW, UpdateWindow, COLOR_HIGHLIGHT,
    COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
    TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, GetWindowLongPtrW, RegisterClassExW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HMENU, HWND_TOPMOST,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WM_NCHITTEST, WM_PAINT, WNDCLASSEXW, WS_BORDER,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use khmerime_session::{CandidateDisplayEntry, CursorLocation, SegmentPreviewEntry};

use crate::com::dll_module::module_instance;
use crate::diagnostics::log;
use crate::WindowsRenderState;

pub const CANDIDATE_SOURCE_FIELDS: &[&str] = &[
    "SessionSnapshot.candidates",
    "SessionSnapshot.candidate_display",
    "SessionSnapshot.selected_index",
];

// Window class name — registered once per process.
const WCLASS: PCWSTR = w!("KhmerIMECandidates");

const WIN_W: i32 = 300;
const ROW_H: i32 = 32;
const PAD_X: i32 = 10;
const PAD_Y: i32 = 6;
const FONT_HEIGHT: i32 = 22;
const MAX_ROWS: usize = 9;
const ANCHOR_GAP: i32 = 2;
const RECOMMENDED_MARK: &str = "\u{2713}";
const DERIVED_MARK: &str = "~";
const SEGMENT_SEPARATOR: &str = " | ";

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

    /// Refresh the candidate list and show the window next to `location`.
    /// Hides the window if `candidates` is empty.
    pub fn update(
        &self,
        candidates: &[String],
        candidate_display: &[CandidateDisplayEntry],
        segment_preview: &[SegmentPreviewEntry],
        selected: usize,
        location: &CursorLocation,
    ) {
        if candidates.is_empty() {
            self.hide();
            return;
        }

        let display = display_candidate_rows(candidates, candidate_display);
        let mut rows: Vec<(String, bool)> = Vec::new();
        if let Some(preview) = segment_preview_text(segment_preview) {
            rows.push((preview, false));
        }
        rows.extend(
            candidates
                .iter()
                .zip(display.iter())
                .take(MAX_ROWS)
                .enumerate()
                .map(|(i, (_candidate, label))| (format!("{}  {}", i + 1, label), i == selected)),
        );

        let height = candidate_window_height(rows.len());
        let placement = candidate_window_placement(location, WIN_W, height, monitor_work_area_for_location(location));

        unsafe {
            // Replace paint data atomically (still on the STA thread).
            let old = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut CandidateData;
            if !old.is_null() {
                drop(Box::from_raw(old));
            }
            let data = Box::new(CandidateData { rows });
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, Box::into_raw(data) as isize);

            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                placement.x,
                placement.y,
                placement.width,
                placement.height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = InvalidateRect(self.hwnd, None, true);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CandidateWindowPlacement {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

fn candidate_window_height(row_count: usize) -> i32 {
    PAD_Y * 2 + row_count as i32 * ROW_H
}

fn candidate_window_placement(
    anchor: &CursorLocation,
    width: i32,
    height: i32,
    work_area: RECT,
) -> CandidateWindowPlacement {
    let anchor_left = anchor.x;
    let anchor_top = anchor.y;
    let anchor_bottom = anchor.y + anchor.height.max(0);
    let below_y = anchor_bottom + ANCHOR_GAP;
    let above_y = anchor_top - height - ANCHOR_GAP;

    let x = clamp(anchor_left, work_area.left, work_area.right - width);
    let y = if below_y + height <= work_area.bottom {
        below_y
    } else if above_y >= work_area.top {
        above_y
    } else {
        clamp(below_y, work_area.top, work_area.bottom - height)
    };

    CandidateWindowPlacement { x, y, width, height }
}

fn monitor_work_area_for_location(location: &CursorLocation) -> RECT {
    unsafe {
        let point = POINT {
            x: location.x,
            y: location.y + location.height.max(0),
        };
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..MONITORINFO::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            return info.rcWork;
        }

        RECT {
            left: 0,
            top: 0,
            right: GetSystemMetrics(SM_CXSCREEN),
            bottom: GetSystemMetrics(SM_CYSCREEN),
        }
    }
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if min > max {
        min
    } else {
        value.max(min).min(max)
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

unsafe extern "system" fn candidate_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const CandidateData;
            let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
            let hdc = BeginPaint(hwnd, &mut ps);

            if !data_ptr.is_null() {
                let data = &*data_ptr;
                let hfont = CreateFontW(-FONT_HEIGHT, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 34, w!("Khmer UI"));
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
                    let _ = TextOutW(hdc, PAD_X, row_top + (ROW_H - FONT_HEIGHT) / 2, &utf16);
                }

                SelectObject(hdc, old_font);
                let _ = DeleteObject(hfont);
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
    if render_state.segmented_active {
        if let Some(segment_preview) = segment_preview_text(&render_state.segment_preview) {
            preview.push_str("  {");
            preview.push_str(&segment_preview);
            preview.push('}');
        }
    }
    if !render_state.candidates.is_empty() {
        let display = display_candidate_rows(&render_state.candidates, &render_state.candidate_display);
        let candidates = display
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

pub fn segment_preview_text(entries: &[SegmentPreviewEntry]) -> Option<String> {
    let parts = entries
        .iter()
        .filter_map(|entry| {
            let output = entry.output.trim();
            if output.is_empty() {
                return None;
            }

            let input = entry.input.trim();
            let segment = if input.is_empty() {
                output.to_owned()
            } else {
                format!("{output}({input})")
            };

            if entry.focused {
                Some(format!("[{segment}]"))
            } else {
                Some(segment)
            }
        })
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(SEGMENT_SEPARATOR))
    }
}

fn display_candidate_rows(candidates: &[String], candidate_display: &[CandidateDisplayEntry]) -> Vec<String> {
    let use_display = candidate_display.len() == candidates.len();
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            if !use_display {
                return candidate.clone();
            }

            let entry = &candidate_display[index];
            let output = if entry.output.trim().is_empty() {
                candidate.as_str()
            } else {
                entry.output.trim()
            };
            let hints = entry
                .roman_hints
                .iter()
                .map(|hint| hint.trim())
                .filter(|hint| !hint.is_empty())
                .take(3)
                .collect::<Vec<_>>();

            let mut label = if entry.recommended {
                format!("{RECOMMENDED_MARK} {output}")
            } else if hints.is_empty() {
                format!("{DERIVED_MARK} {output}")
            } else {
                output.to_owned()
            };

            if !hints.is_empty() {
                label.push_str(" (");
                label.push_str(&hints.join(" / "));
                label.push(')');
            }
            label
        })
        .collect()
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
    fn inline_preview_includes_segment_preview_when_active() {
        let preview = inline_preview_text(&WindowsRenderState {
            preedit: "firstsecond".to_owned(),
            candidates: vec!["first".to_owned()],
            selected_index: Some(0),
            segmented_active: true,
            segment_preview: vec![
                SegmentPreviewEntry {
                    output: "first".to_owned(),
                    input: "foo".to_owned(),
                    focused: true,
                },
                SegmentPreviewEntry {
                    output: "second".to_owned(),
                    input: "bar".to_owned(),
                    focused: false,
                },
            ],
            ..WindowsRenderState::default()
        });

        assert_eq!(
            preview.as_deref(),
            Some("firstsecond  {[first(foo)] | second(bar)}  [1. *first]")
        );
    }

    #[test]
    fn empty_preedit_hides_preview() {
        assert!(inline_preview_text(&WindowsRenderState::default()).is_none());
    }

    #[test]
    fn segment_preview_marks_focused_chunk() {
        let preview = segment_preview_text(&[
            SegmentPreviewEntry {
                output: "first".to_owned(),
                input: "foo".to_owned(),
                focused: true,
            },
            SegmentPreviewEntry {
                output: "second".to_owned(),
                input: "bar".to_owned(),
                focused: false,
            },
        ]);

        assert_eq!(preview.as_deref(), Some("[first(foo)] | second(bar)"));
    }

    #[test]
    fn candidate_rows_match_ibus_metadata_labels() {
        let rows = display_candidate_rows(
            &["ជា".to_owned(), "ជៀ".to_owned(), "jea".to_owned()],
            &[
                CandidateDisplayEntry {
                    output: "ជា".to_owned(),
                    recommended: true,
                    roman_hints: vec!["jea".to_owned(), "chea".to_owned()],
                },
                CandidateDisplayEntry {
                    output: "ជៀ".to_owned(),
                    recommended: false,
                    roman_hints: vec!["jia".to_owned()],
                },
                CandidateDisplayEntry {
                    output: "jea".to_owned(),
                    recommended: false,
                    roman_hints: vec![],
                },
            ],
        );

        assert_eq!(rows, vec!["\u{2713} ជា (jea / chea)", "ជៀ (jia)", "~ jea"]);
    }

    #[test]
    fn candidate_rows_fall_back_to_plain_candidates_without_metadata() {
        let rows = display_candidate_rows(&["ជា".to_owned(), "jea".to_owned()], &[]);

        assert_eq!(rows, vec!["ជា", "jea"]);
    }

    #[test]
    fn placement_sits_just_below_normal_anchor() {
        let placement = candidate_window_placement(
            &CursorLocation {
                x: 100,
                y: 200,
                width: 2,
                height: 18,
            },
            220,
            74,
            work_area(),
        );

        assert_eq!(placement.x, 100);
        assert_eq!(placement.y, 220);
    }

    #[test]
    fn placement_flips_above_near_bottom_edge() {
        let placement = candidate_window_placement(
            &CursorLocation {
                x: 100,
                y: 570,
                width: 2,
                height: 18,
            },
            220,
            74,
            work_area(),
        );

        assert_eq!(placement.y, 494);
    }

    #[test]
    fn placement_clamps_to_work_area_edges() {
        let left = candidate_window_placement(
            &CursorLocation {
                x: -30,
                y: 100,
                width: 2,
                height: 18,
            },
            220,
            74,
            work_area(),
        );
        let right = candidate_window_placement(
            &CursorLocation {
                x: 760,
                y: 100,
                width: 2,
                height: 18,
            },
            220,
            74,
            work_area(),
        );

        assert_eq!(left.x, 0);
        assert_eq!(right.x, 580);
    }

    fn work_area() -> RECT {
        RECT {
            left: 0,
            top: 0,
            right: 800,
            bottom: 600,
        }
    }
}
