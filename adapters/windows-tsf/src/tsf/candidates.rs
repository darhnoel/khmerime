//! Candidate popup window and inline-preview helpers.
//!
//! The popup is a Windows 11 style vertical list. What it shows
//! (page, numbering, marks, sizes at a DPI) is decided in `render::candidate_popup`; this file
//! only measures text and paints.

use std::ffi::c_void;
use std::sync::OnceLock;

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{COLORREF, ERROR_SUCCESS, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWM_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateRoundRectRgn,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetMonitorInfoW, GetSysColor,
    GetTextExtentPoint32W, InvalidateRect, MonitorFromPoint, ReleaseDC, RoundRect, SelectObject, SetBkMode,
    SetTextColor, SetWindowRgn, UpdateWindow, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT,
    DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, HDC,
    HFONT, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, GetWindowLongPtrW, RegisterClassExW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, CS_DROPSHADOW, GWLP_USERDATA, HMENU,
    HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SPI_GETHIGHCONTRAST, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SW_HIDE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WM_APP, WM_ERASEBKGND, WM_NCHITTEST, WM_PAINT,
    WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use khmerime_session::{CursorLocation, SegmentPreviewEntry};

use crate::com::dll_module::module_instance;
use crate::diagnostics::log;
use crate::render::candidate_popup::{page_size, popup_model, row_parts, Header, PopupMetrics, PopupModel, RowMark};
use crate::render::candidate_surface::{CandidateSurface, CandidateSurfaceMode};
use crate::WindowsRenderState;

pub const CANDIDATE_SOURCE_FIELDS: &[&str] = &[
    "SessionSnapshot.candidates",
    "SessionSnapshot.candidate_display",
    "SessionSnapshot.selected_index",
];

// Window class name — registered once per process.
const WCLASS: PCWSTR = w!("KhmerIMECandidates");

const ANCHOR_GAP: i32 = 2;
const SEGMENT_SEPARATOR: &str = " | ";

const KHMER_FACE: PCWSTR = w!("Khmer UI");
const UI_FACE: PCWSTR = w!("Segoe UI");
const SYMBOL_FACE: PCWSTR = w!("Segoe UI Symbol");

static CLASS_REGISTERED: OnceLock<()> = OnceLock::new();

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

/// The popup's colours for the user's current Windows theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Palette {
    bg: COLORREF,
    border: COLORREF,
    text: COLORREF,
    muted: COLORREF,
    faint: COLORREF,
    accent: COLORREF,
    selected_fill: COLORREF,
    /// High contrast fills the selected row with the highlight colour, so every glyph on it
    /// must switch to the highlight-text colour instead of keeping its own colour.
    selected_text: Option<COLORREF>,
    /// Draw the accent bar at the selected row's left edge.
    selection_bar: bool,
    recommended: COLORREF,
    model: COLORREF,
    derived: COLORREF,
}

// Windows 11 light/dark flyout colours; the selected fill is the accent at 10% / 14% over the
// background, precomputed because GDI has no alpha.
const LIGHT: Palette = Palette {
    bg: rgb(0xfc, 0xfc, 0xfc),
    border: rgb(0xe5, 0xe5, 0xe5),
    text: rgb(0x1b, 0x1b, 0x1b),
    muted: rgb(0x61, 0x61, 0x61),
    faint: rgb(0x8a, 0x8a, 0x8a),
    accent: rgb(0x00, 0x5f, 0xb8),
    selected_fill: rgb(0xe3, 0xec, 0xf5),
    selected_text: None,
    selection_bar: true,
    recommended: rgb(0x0f, 0x7b, 0x0f),
    model: rgb(0x8a, 0x3f, 0xfc),
    derived: rgb(0x9d, 0x5d, 0x00),
};

const DARK: Palette = Palette {
    bg: rgb(0x2c, 0x2c, 0x2c),
    border: rgb(0x3e, 0x3e, 0x3e),
    text: rgb(0xff, 0xff, 0xff),
    muted: rgb(0xc5, 0xc5, 0xc5),
    faint: rgb(0x9a, 0x9a, 0x9a),
    accent: rgb(0x60, 0xcd, 0xff),
    selected_fill: rgb(0x33, 0x43, 0x4a),
    selected_text: None,
    selection_bar: true,
    recommended: rgb(0x6c, 0xcb, 0x5f),
    model: rgb(0xc3, 0xa3, 0xff),
    derived: rgb(0xfc, 0xe1, 0x00),
};

impl Palette {
    fn current() -> Self {
        if high_contrast_on() {
            Self::high_contrast()
        } else if apps_use_light_theme() {
            LIGHT
        } else {
            DARK
        }
    }

    /// High contrast follows the user's chosen system colours exactly.
    fn high_contrast() -> Self {
        let sys = |index| COLORREF(unsafe { GetSysColor(index) });
        let text = sys(COLOR_WINDOWTEXT);
        Palette {
            bg: sys(COLOR_WINDOW),
            border: text,
            text,
            muted: text,
            faint: text,
            accent: sys(COLOR_HIGHLIGHT),
            selected_fill: sys(COLOR_HIGHLIGHT),
            selected_text: Some(sys(COLOR_HIGHLIGHTTEXT)),
            selection_bar: false,
            recommended: text,
            model: text,
            derived: text,
        }
    }

    fn mark(&self, mark: RowMark) -> COLORREF {
        match mark {
            RowMark::Model => self.model,
            RowMark::Recommended => self.recommended,
            RowMark::Derived => self.derived,
        }
    }
}

fn high_contrast_on() -> bool {
    let mut hc = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..HIGHCONTRASTW::default()
    };
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            hc.cbSize,
            Some(&mut hc as *mut HIGHCONTRASTW as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && (hc.dwFlags.0 & HCF_HIGHCONTRASTON.0) != 0
}

/// Windows' app light/dark setting. Missing or unreadable means light (the Windows default).
fn apps_use_light_theme() -> bool {
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut c_void),
            Some(&mut size),
        )
    };
    status != ERROR_SUCCESS || value != 0
}

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// Every font the popup uses at one DPI. Deleted on drop.
struct Fonts {
    word: HFONT,
    small: HFONT,
    small_bold: HFONT,
    mark: HFONT,
    footer: HFONT,
    header_khmer: HFONT,
    header_roman: HFONT,
}

impl Fonts {
    fn new(m: &PopupMetrics) -> Self {
        Self {
            word: font(m.word_font, 400, KHMER_FACE),
            small: font(m.small_font, 400, UI_FACE),
            small_bold: font(m.small_font, 600, UI_FACE),
            mark: font(m.small_font, 400, SYMBOL_FACE),
            footer: font(m.footer_font, 400, UI_FACE),
            header_khmer: font(m.header_khmer_font, 400, KHMER_FACE),
            header_roman: font(m.header_roman_font, 400, UI_FACE),
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        for f in [
            self.word,
            self.small,
            self.small_bold,
            self.mark,
            self.footer,
            self.header_khmer,
            self.header_roman,
        ] {
            unsafe {
                let _ = DeleteObject(f);
            }
        }
    }
}

fn font(px: i32, weight: i32, face: PCWSTR) -> HFONT {
    // DEFAULT_CHARSET (1), CLEARTYPE_QUALITY (5).
    unsafe { CreateFontW(-px, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face) }
}

// ---------------------------------------------------------------------------
// Per-window paint data — stored in GWLP_USERDATA as a raw Box pointer.
// ---------------------------------------------------------------------------

struct CandidateData {
    model: PopupModel,
    metrics: PopupMetrics,
    palette: Palette,
    width: i32,
    height: i32,
    /// DWM rounds the corners and draws the border (Windows 11); otherwise the window region
    /// rounds them and the paint routine draws the border.
    dwm_frame: bool,
}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// Native Win32 popup showing numbered candidates near the composition cursor.
/// Never activates (WS_EX_NOACTIVATE) so focus stays in the host application.
pub struct CandidateWindow {
    hwnd: HWND,
    dwm_frame: bool,
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
                WS_POPUP,
                0,
                0,
                1,
                1,
                HWND::default(),
                HMENU::default(),
                module_instance(),
                None,
            )?
        };

        // Windows 11 rounds the popup and draws a hairline border itself; earlier Windows
        // rejects the attribute and we fall back to a window region.
        let corner: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
        let dwm_frame = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner as *const DWM_WINDOW_CORNER_PREFERENCE as *const c_void,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            )
        }
        .is_ok();

        Ok(Self { hwnd, dwm_frame })
    }

    /// Raw window handle, for posting [`WM_KHMERIME_REFINE_READY`] from a worker thread.
    ///
    /// `HWND` is not `Send`, so callers pass the raw value across the thread boundary and
    /// reconstruct it; posting a message is one of the few window operations documented as safe
    /// from any thread.
    pub fn hwnd_raw(&self) -> isize {
        self.hwnd.0 as isize
    }

    /// Refresh the candidate list and show the window next to `location`.
    /// Hides the window if there are no candidates.
    pub fn update(&self, surface: &CandidateSurface, location: &CursorLocation) {
        if surface.rows().is_empty() {
            self.hide();
            return;
        }

        let model = popup_model(surface, page_size());
        let work_area = monitor_work_area_for_location(location);

        unsafe {
            // Move onto the caret's monitor before measuring, so the DPI we size for is the DPI
            // the popup will be shown at (per-monitor-aware hosts report it per monitor).
            let _ = SetWindowPos(
                self.hwnd,
                HWND::default(),
                location.x,
                location.y + location.height.max(0),
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        let metrics = PopupMetrics::for_dpi(unsafe { GetDpiForWindow(self.hwnd) });
        let palette = Palette::current();

        let (widest_row, header_w) = self.measure(&model, &metrics);
        let width = metrics.width(widest_row, header_w, work_area_width(work_area));
        let height = metrics.height(&model);
        let placement = candidate_window_placement(location, width, height, work_area);

        unsafe {
            // Replace paint data atomically (still on the STA thread).
            let old = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut CandidateData;
            if !old.is_null() {
                drop(Box::from_raw(old));
            }
            let data = Box::new(CandidateData {
                model,
                metrics,
                palette,
                width,
                height,
                dwm_frame: self.dwm_frame,
            });
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, Box::into_raw(data) as isize);

            if self.dwm_frame {
                let border = palette.border;
                let _ = DwmSetWindowAttribute(
                    self.hwnd,
                    DWMWA_BORDER_COLOR,
                    &border as *const COLORREF as *const c_void,
                    std::mem::size_of::<COLORREF>() as u32,
                );
            } else {
                let r = metrics.corner_radius;
                let region = CreateRoundRectRgn(0, 0, placement.width + 1, placement.height + 1, r, r);
                let _ = SetWindowRgn(self.hwnd, region, true);
            }

            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                placement.x,
                placement.y,
                placement.width,
                placement.height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = InvalidateRect(self.hwnd, None, false);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    /// Measured content width of the widest row and of the segment header.
    fn measure(&self, model: &PopupModel, metrics: &PopupMetrics) -> (i32, i32) {
        let hdc = unsafe { GetDC(self.hwnd) };
        if hdc.is_invalid() {
            return (0, 0);
        }
        let fonts = Fonts::new(metrics);
        let widest = model
            .rows
            .iter()
            .map(|row| {
                let word = text_width(hdc, fonts.word, &row.word);
                let roman = text_width(hdc, fonts.small, &row.roman);
                metrics.row_content_width(word, roman)
            })
            .max()
            .unwrap_or(0);
        let header = model
            .header
            .as_ref()
            .map(|header| metrics.header_content_width(&header_column_widths(hdc, &fonts, metrics, header)))
            .unwrap_or(0);
        drop(fonts);
        unsafe {
            let _ = ReleaseDC(self.hwnd, hdc);
        }
        (widest, header)
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

fn work_area_width(work_area: RECT) -> i32 {
    (work_area.right - work_area.left).max(1)
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
    // The paint routine covers every pixel, so no background brush and no CS_*REDRAW
    // (update() invalidates explicitly). CS_DROPSHADOW lifts the popup off the document.
    wc.style = CS_DROPSHADOW;
    wc.lpfnWndProc = Some(candidate_wnd_proc);
    wc.hInstance = module_instance();
    wc.lpszClassName = WCLASS;

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

/// Posted by the refine worker once it has updated the session off the key path.
///
/// The repaint cannot happen on the worker: `update` swaps the window's paint data through a raw
/// pointer with no synchronisation, so it must run on the window's owning thread. Posting is the
/// handoff.
pub const WM_KHMERIME_REFINE_READY: u32 = WM_APP + 1;

unsafe extern "system" fn candidate_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_KHMERIME_REFINE_READY => {
            crate::tsf::refine::on_refine_ready();
            LRESULT(0)
        }
        WM_PAINT => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const CandidateData;
            let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !data_ptr.is_null() {
                paint_buffered(hdc, &*data_ptr);
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        // Painting is double-buffered and covers the whole client area; erasing first would
        // only flash the background.
        WM_ERASEBKGND => LRESULT(1),
        WM_NCHITTEST => {
            // Pass all mouse events through to the host application so focus
            // is never stolen.  Equivalent to HTTRANSPARENT (-1).
            LRESULT(-1)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Paint into an off-screen bitmap, then copy it to the window in one blit (no flicker).
unsafe fn paint_buffered(hdc: HDC, data: &CandidateData) {
    let mem = CreateCompatibleDC(hdc);
    let bitmap = CreateCompatibleBitmap(hdc, data.width, data.height);
    let old_bitmap = SelectObject(mem, bitmap);
    paint(mem, data);
    let _ = BitBlt(hdc, 0, 0, data.width, data.height, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old_bitmap);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(mem);
}

unsafe fn paint(hdc: HDC, data: &CandidateData) {
    let m = &data.metrics;
    let p = &data.palette;
    let fonts = Fonts::new(m);
    SetBkMode(hdc, TRANSPARENT);

    // Background (and the border, when DWM is not drawing one).
    fill_rect(hdc, rect(0, 0, data.width, data.height), p.bg);
    if !data.dwm_frame {
        round_rect(
            hdc,
            rect(0, 0, data.width, data.height),
            m.corner_radius,
            p.bg,
            p.border,
        );
    }

    let left = m.outer_pad;
    let right = data.width - m.outer_pad;
    let mut y = m.outer_pad;

    if let Some(header) = data.model.header.as_ref() {
        paint_header(
            hdc,
            &fonts,
            m,
            p,
            header,
            rect(left + m.row_left_pad, y, right - m.row_right_pad, y),
        );
        y += m.header_h();
        // Hairline between the segmentation and the rows.
        fill_rect(
            hdc,
            rect(left, y - 1, right, y - 1 + (m.dpi as i32 / 96).max(1)),
            p.border,
        );
    }

    for row in &data.model.rows {
        let r = rect(left, y, right, y + m.row_h);
        let override_text = if row.selected { p.selected_text } else { None };

        if row.selected {
            round_rect(hdc, r, m.select_radius, p.selected_fill, p.selected_fill);
            if p.selection_bar {
                let bar = rect(r.left, y + m.bar_inset_y, r.left + m.bar_w, y + m.row_h - m.bar_inset_y);
                round_rect(hdc, bar, m.bar_w, p.accent, p.accent);
            }
        }

        // Digit — the key that picks this row.
        let num_left = r.left + m.row_left_pad;
        let (num_font, num_color) = if row.selected {
            (fonts.small_bold, override_text.unwrap_or(p.accent))
        } else {
            (fonts.small, p.faint)
        };
        let num_rect = rect(num_left, y, num_left + m.num_w, y + m.row_h);
        draw_text(hdc, num_font, num_color, &row.number.to_string(), num_rect, DT_RIGHT);

        // Provenance mark.
        if let Some(mark) = row.mark {
            let mark_left = num_rect.right + m.col_gap;
            let mark_rect = rect(mark_left, y, mark_left + m.mark_w, y + m.row_h);
            let color = override_text.unwrap_or(p.mark(mark));
            draw_text(hdc, fonts.mark, color, mark.glyph(), mark_rect, DT_CENTER);
        }

        // Roman spelling, right-aligned. It only gives way when word + roman do not fit, and even
        // then the word keeps at least half the row (the word is what the user is choosing).
        let word_left = r.left + m.word_offset();
        let text_right = r.right - m.row_right_pad;
        let mut word_right = text_right;
        if !row.roman.is_empty() {
            let available = (text_right - word_left - m.roman_gap).max(0);
            let word_w = text_width(hdc, fonts.word, &row.word);
            let roman_room = (available - word_w).max(available / 2);
            let roman_w = text_width(hdc, fonts.small, &row.roman).min(roman_room);
            let roman_rect = rect(text_right - roman_w, y, text_right, y + m.row_h);
            let color = override_text.unwrap_or(p.muted);
            draw_text(
                hdc,
                fonts.small,
                color,
                &row.roman,
                roman_rect,
                DT_RIGHT | DT_END_ELLIPSIS,
            );
            word_right = roman_rect.left - m.roman_gap;
        }

        // The Khmer word.
        let word_rect = rect(word_left, y, word_right.max(word_left), y + m.row_h);
        let color = override_text.unwrap_or(p.text);
        draw_text(hdc, fonts.word, color, &row.word, word_rect, DT_LEFT | DT_END_ELLIPSIS);

        y += m.row_h;
    }

    // Footer: a hairline, then the mode on the left and the page / keys on the right.
    fill_rect(hdc, rect(left, y, right, y + (m.dpi as i32 / 96).max(1)), p.border);
    let footer = rect(left + m.footer_pad_x, y, right - m.footer_pad_x, y + m.footer_h);
    draw_text(hdc, fonts.footer, p.faint, data.model.mode_label, footer, DT_LEFT);
    draw_text(hdc, fonts.footer, p.faint, &data.model.footer_right(), footer, DT_RIGHT);
}

/// Width of each header column: the wider of the segment's Khmer and its roman, plus padding.
fn header_column_widths(hdc: HDC, fonts: &Fonts, m: &PopupMetrics, header: &Header) -> Vec<i32> {
    header
        .segments
        .iter()
        .map(|seg| {
            let khmer = text_width(hdc, fonts.header_khmer, &seg.khmer);
            let roman = text_width(hdc, fonts.header_roman, &seg.roman);
            m.header_column_width(khmer, roman)
        })
        .collect()
}

/// The segmentation as columns: each segment's Khmer with the roman typed for it beneath.
/// `area` gives the left/right bounds and the top; columns past the right edge are cut off and
/// the one crossing it ellipsizes.
unsafe fn paint_header(hdc: HDC, fonts: &Fonts, m: &PopupMetrics, p: &Palette, header: &Header, area: RECT) {
    let widths = header_column_widths(hdc, fonts, m, header);
    let khmer_bottom = area.top + m.header_khmer_h;
    let roman_bottom = khmer_bottom + m.header_roman_h;
    let mut x = area.left;
    for (seg, width) in header.segments.iter().zip(widths) {
        if x >= area.right {
            break;
        }
        let col_right = (x + width).min(area.right);
        // Word mode highlights the segment being edited; in phrase mode the header is context
        // and never looks selected.
        let focused = header.emphasize_focus && seg.focused;
        if focused {
            round_rect(
                hdc,
                rect(x, area.top, col_right, roman_bottom),
                m.select_radius,
                p.selected_fill,
                p.selected_fill,
            );
        }
        let (khmer_color, roman_color) = if focused {
            (p.selected_text.unwrap_or(p.accent), p.selected_text.unwrap_or(p.muted))
        } else if header.emphasize_focus {
            (p.text, p.faint)
        } else {
            (p.muted, p.faint)
        };
        let inner_left = x + m.header_chip_pad;
        let inner_right = (col_right - m.header_chip_pad).max(inner_left);
        draw_text(
            hdc,
            fonts.header_khmer,
            khmer_color,
            &seg.khmer,
            rect(inner_left, area.top, inner_right, khmer_bottom),
            DT_CENTER | DT_END_ELLIPSIS,
        );
        draw_text(
            hdc,
            fonts.header_roman,
            roman_color,
            &seg.roman,
            rect(inner_left, khmer_bottom, inner_right, roman_bottom),
            DT_CENTER | DT_END_ELLIPSIS,
        );
        x += width + m.header_gap;
    }
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

unsafe fn fill_rect(hdc: HDC, r: RECT, color: COLORREF) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, &r, brush);
    let _ = DeleteObject(brush);
}

unsafe fn round_rect(hdc: HDC, r: RECT, radius: i32, fill: COLORREF, outline: COLORREF) {
    let brush = CreateSolidBrush(fill);
    let pen = CreatePen(PS_SOLID, 1, outline);
    let old_brush = SelectObject(hdc, brush);
    let old_pen = SelectObject(hdc, pen);
    let _ = RoundRect(hdc, r.left, r.top, r.right, r.bottom, radius, radius);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_brush);
    let _ = DeleteObject(pen);
    let _ = DeleteObject(brush);
}

/// Single-line text, vertically centred in `r`, with `align` (left / centre / right, plus any
/// extra DT_ flags).
unsafe fn draw_text(hdc: HDC, font: HFONT, color: COLORREF, text: &str, r: RECT, align: DRAW_TEXT_FORMAT) {
    if text.is_empty() {
        return;
    }
    let old = SelectObject(hdc, font);
    SetTextColor(hdc, color);
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    let mut r = r;
    DrawTextW(
        hdc,
        &mut utf16,
        &mut r,
        align | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
    );
    SelectObject(hdc, old);
}

fn text_width(hdc: HDC, font: HFONT, text: &str) -> i32 {
    let utf16 = text.encode_utf16().collect::<Vec<_>>();
    if utf16.is_empty() {
        return 0;
    }
    let mut size = SIZE::default();
    unsafe {
        let old = SelectObject(hdc, font);
        let ok = GetTextExtentPoint32W(hdc, &utf16, &mut size).as_bool();
        SelectObject(hdc, old);
        if ok {
            size.cx
        } else {
            0
        }
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
    if render_state.candidate_surface.mode() != CandidateSurfaceMode::Flat {
        if let Some(segment_preview) = segment_preview_text(render_state.candidate_surface.context()) {
            preview.push_str("  {");
            preview.push_str(&segment_preview);
            preview.push('}');
        }
    }
    if !render_state.candidate_surface.rows().is_empty() {
        let display = display_candidate_rows(&render_state.candidate_surface);
        let candidates = display
            .iter()
            .take(5)
            .enumerate()
            .map(|(index, candidate)| {
                if Some(index) == render_state.candidate_surface.selected_index() {
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

/// One-line label per candidate ("✓ ខ្ញុំ (knhom / khnhom)"), for the inline preview. Built on the
/// same row parts the popup paints, so the marks cannot drift between the two.
fn display_candidate_rows(surface: &CandidateSurface) -> Vec<String> {
    row_parts(surface)
        .into_iter()
        .map(|part| {
            let mut label = match part.mark {
                Some(mark) => format!("{} {}", mark.glyph(), part.word),
                None => part.word,
            };
            if !part.hints.is_empty() {
                label.push_str(" (");
                label.push_str(&part.hints.join(" / "));
                label.push(')');
            }
            label
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use khmerime_session::{CandidateDisplayEntry, SessionSnapshot};

    const DERIVED_MARK: &str = "~";
    const MODEL_MARK: &str = "\u{2726}";

    /// Builds a flat-mode surface, the shape display_candidate_rows used to be handed directly.
    fn flat_surface(candidates: &[&str], display: Vec<CandidateDisplayEntry>) -> CandidateSurface {
        CandidateSurface::from_snapshot(&SessionSnapshot {
            candidates: candidates.iter().map(|c| (*c).to_owned()).collect(),
            candidate_display: display,
            ..SessionSnapshot::default()
        })
    }

    fn render_state(snapshot: SessionSnapshot, preedit: &str) -> WindowsRenderState {
        WindowsRenderState {
            preedit: preedit.to_owned(),
            candidate_surface: CandidateSurface::from_snapshot(&snapshot),
            ..WindowsRenderState::default()
        }
    }

    // Two things at once, because they interact: a model-derived row must be marked, and the
    // "~" derived mark must NOT appear on ordinary phrase rows. That mark keys off "no roman
    // hints", which was a fine proxy for the flat list but became meaningless once phrase rows
    // stopped carrying roman at all.
    // CONTEXT.md, Roman Hint: "If no exact Roman Hint exists for a candidate, the UI must not
    // invent one; it should show a derived marker instead." That rule is not flat-list-only -- a
    // phrase row with no canonical romanization must say so, or the reader cannot tell "no roman
    // exists" from "roman is not shown on this surface".
    #[test]
    fn a_phrase_row_without_canonical_roman_shows_the_derived_marker() {
        let snapshot = SessionSnapshot {
            segmented_active: true,
            phrase_candidates: vec![
                // Index 0 is the recommended row and takes ✓, so the derived rule is checked on
                // the row below it.
                khmerime_session::PhraseCandidate {
                    text: "ដុំរ៉ា".to_owned(),
                    ..Default::default()
                },
                khmerime_session::PhraseCandidate {
                    text: "ដមរ៉ា".to_owned(),
                    segments: vec![khmerime_session::PhraseSegment {
                        input: "domra".to_owned(),
                        output: "ដមរ៉ា".to_owned(),
                        roman_hints: vec![],
                    }],
                    ..Default::default()
                },
            ],
            ..SessionSnapshot::default()
        };
        let surface = CandidateSurface::from_snapshot(&snapshot);

        let rows = display_candidate_rows(&surface);

        assert!(
            rows[1].contains(DERIVED_MARK),
            "a phrase row with no exact Roman Hint must carry the derived marker: {:?}",
            rows[1]
        );
    }

    #[test]
    fn phrase_rows_mark_model_provenance_without_marking_everything_derived() {
        let snapshot = SessionSnapshot {
            segmented_active: true,
            phrase_candidates: vec![
                khmerime_session::PhraseCandidate {
                    text: "ដុំរ៉ា".to_owned(),
                    from_model: false,
                    ..Default::default()
                },
                khmerime_session::PhraseCandidate {
                    text: "ដំរី".to_owned(),
                    from_model: true,
                    ..Default::default()
                },
            ],
            ..SessionSnapshot::default()
        };
        let surface = CandidateSurface::from_snapshot(&snapshot);

        let rows = display_candidate_rows(&surface);

        assert!(rows[1].contains(MODEL_MARK), "model row unmarked: {:?}", rows[1]);
        assert!(
            !rows[1].contains(DERIVED_MARK),
            "model row must not also read as derived: {:?}",
            rows[1]
        );
        assert!(
            !rows[0].contains(DERIVED_MARK),
            "ordinary phrase row wrongly marked derived: {:?}",
            rows[0]
        );
    }

    #[test]
    fn inline_preview_marks_selected_candidate() {
        let preview = inline_preview_text(&render_state(
            SessionSnapshot {
                candidates: vec!["candidate".to_owned(), "other".to_owned()],
                selected_index: Some(1),
                ..SessionSnapshot::default()
            },
            "jea",
        ));
        assert_eq!(preview.as_deref(), Some("jea  [1. candidate  2. *other]"));
    }

    #[test]
    fn inline_preview_includes_segment_preview_when_active() {
        let preview = inline_preview_text(&render_state(
            SessionSnapshot {
                candidates: vec!["first".to_owned()],
                selected_index: Some(0),
                segmented_active: true,
                segment_edit_active: true,
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
                ..SessionSnapshot::default()
            },
            "firstsecond",
        ));

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
        let surface = flat_surface(
            &["ជា", "ជៀ", "jea"],
            vec![
                CandidateDisplayEntry {
                    output: "ជា".to_owned(),
                    recommended: true,
                    roman_hints: vec!["jea".to_owned(), "chea".to_owned()],
                    ..Default::default()
                },
                CandidateDisplayEntry {
                    output: "ជៀ".to_owned(),
                    recommended: false,
                    roman_hints: vec!["jia".to_owned()],
                    ..Default::default()
                },
                CandidateDisplayEntry {
                    output: "jea".to_owned(),
                    recommended: false,
                    roman_hints: vec![],
                    is_raw_fallback: true,
                    ..Default::default()
                },
            ],
        );

        let rows = display_candidate_rows(&surface);
        assert_eq!(rows, vec!["\u{2713} ជា (jea / chea)", "ជៀ (jia)", "~ jea"]);
    }

    #[test]
    fn candidate_rows_fall_back_to_plain_candidates_without_metadata() {
        let rows = display_candidate_rows(&flat_surface(&["ជា", "jea"], vec![]));

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
