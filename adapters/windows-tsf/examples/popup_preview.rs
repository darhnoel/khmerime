//! Opens the real native candidate popup with sample candidates, without installing the IME.
//!
//!   cargo run -p khmerime_windows_tsf --example popup_preview [seconds]
//!
//! Shows five popups side by side (flat word, long word, a paged list with the selection on
//! page 2, and one segmented phrase at the phrase level and at the word level after Tab) in
//! the current Windows theme, then exits. Prints
//! the screen rectangle the popups cover, so a screenshot can be cropped to it.

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    use std::time::{Duration, Instant};

    use khmerime_session::{
        CandidateDisplayEntry, CursorLocation, PhraseCandidate, PhraseSegment, SegmentPreviewEntry, SessionSnapshot,
    };
    use khmerime_windows_tsf::render::candidate_surface::CandidateSurface;
    use khmerime_windows_tsf::tsf::candidates::CandidateWindow;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetSystemMetrics, GetWindowRect, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, SM_CXSCREEN,
    };

    let seconds: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(8);
    // Notepad and most modern hosts are per-monitor DPI aware; preview the popup the same way.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let entry = |out: &str, rec: bool, model: bool, hints: &[&str]| CandidateDisplayEntry {
        output: out.into(),
        recommended: rec,
        from_model: model,
        roman_hints: hints.iter().map(|h| h.to_string()).collect(),
        ..CandidateDisplayEntry::default()
    };
    let flat = |rows: Vec<CandidateDisplayEntry>, selected: usize| {
        CandidateSurface::from_snapshot(&SessionSnapshot {
            candidates: rows.iter().map(|r| r.output.clone()).collect(),
            candidate_display: rows,
            selected_index: Some(selected),
            ..SessionSnapshot::default()
        })
    };

    let knhom = flat(
        vec![
            entry("ខ្ញុំ", true, false, &["knhom", "khnhom"]),
            entry("ខ្ញម", false, false, &[]),
            entry("ក្ញុំ", false, true, &[]),
            entry("ញុំ", false, false, &["nhom"]),
            entry("ខ្ញំ", false, false, &["khnhom"]),
        ],
        0,
    );
    let minister = flat(
        vec![
            entry("រដ្ឋមន្ត្រី", true, false, &["rothmontrei", "rathmontrei"]),
            entry("រដ្ឋមន្រ្តី", false, false, &["rothmontrei"]),
            entry("រដ្ឋមន្ទ្រី", false, true, &[]),
            entry("រដ្ឋមន្ត្រីស្ដីទី", false, false, &["rothmontrei sdeiti"]),
        ],
        1,
    );
    let ka_words = [
        "ក|ka",
        "កា|ka",
        "ការ|kar",
        "ក៏|kor",
        "កាំ|kam",
        "កៈ|",
        "កក|kak",
        "កាក|kak",
        "កាត់|kat",
        "កាន់|kan",
        "កាប|kab",
        "កាយ|kay",
        "កាល|kal",
        "កាស|kas",
    ];
    let ka = flat(
        ka_words
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (out, hint) = s.split_once('|').unwrap();
                let hints: Vec<&str> = if hint.is_empty() { vec![] } else { vec![hint] };
                entry(out, i == 2, i == 5, &hints)
            })
            .collect(),
        10,
    );
    let seg = |input: &str, output: &str, hint: &str| PhraseSegment {
        input: input.into(),
        output: output.into(),
        roman_hints: vec![hint.into()],
        ..PhraseSegment::default()
    };
    // Same segmented composition at both levels: phrase mode, and word mode after Tab (editing ទៅ).
    let segmented = |edit: bool| SessionSnapshot {
        segmented_active: true,
        segment_edit_active: edit,
        candidates: if edit {
            vec!["ទៅ".into(), "ទូ".into(), "តូ".into()]
        } else {
            vec![]
        },
        candidate_display: if edit {
            vec![
                entry("ទៅ", true, false, &["tov"]),
                entry("ទូ", false, false, &["tou"]),
                entry("តូ", false, false, &["tou"]),
            ]
        } else {
            vec![]
        },
        selected_index: if edit { Some(0) } else { None },
        segment_preview: vec![
            SegmentPreviewEntry {
                output: "ខ្ញុំ".into(),
                input: "knhom".into(),
                focused: false,
            },
            SegmentPreviewEntry {
                output: "ទៅ".into(),
                input: "tov".into(),
                focused: true,
            },
            SegmentPreviewEntry {
                output: "ផ្សារ".into(),
                input: "phsar".into(),
                focused: false,
            },
        ],
        phrase_candidates: vec![
            PhraseCandidate {
                text: "ខ្ញុំទៅផ្សារ".into(),
                segments: vec![
                    seg("knhom", "ខ្ញុំ", "knhom"),
                    seg("tov", "ទៅ", "tov"),
                    seg("phsar", "ផ្សារ", "phsar"),
                ],
                from_model: false,
                ..PhraseCandidate::default()
            },
            PhraseCandidate {
                text: "ខ្ញុំទៅផ្សា".into(),
                segments: vec![
                    seg("knhom", "ខ្ញុំ", "knhom"),
                    seg("tov", "ទៅ", "tov"),
                    seg("phsa", "ផ្សា", "phsa"),
                ],
                from_model: false,
                ..PhraseCandidate::default()
            },
            PhraseCandidate {
                text: "ខ្ញុំទុកផ្សារ".into(),
                segments: vec![],
                from_model: true,
                ..PhraseCandidate::default()
            },
        ],
        selected_phrase_index: 0,
        ..SessionSnapshot::default()
    };
    let phrase = CandidateSurface::from_snapshot(&segmented(false));
    let word = CandidateSurface::from_snapshot(&segmented(true));

    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let column = screen_w / 5;
    let mut windows = Vec::new();
    let mut bounds = RECT {
        left: i32::MAX,
        top: i32::MAX,
        right: i32::MIN,
        bottom: i32::MIN,
    };
    for (i, surface) in [knhom, minister, ka, phrase, word].iter().enumerate() {
        let window = CandidateWindow::create().expect("create popup");
        let caret = CursorLocation {
            x: 20 + column * i as i32,
            y: 120,
            width: 2,
            height: 24,
        };
        window.update(surface, &caret);
        let mut r = RECT::default();
        unsafe {
            let _ = GetWindowRect(HWND(window.hwnd_raw() as *mut _), &mut r);
        }
        bounds.left = bounds.left.min(r.left);
        bounds.top = bounds.top.min(r.top);
        bounds.right = bounds.right.max(r.right);
        bounds.bottom = bounds.bottom.max(r.bottom);
        windows.push(window);
    }
    println!(
        "BOUNDS {} {} {} {}",
        bounds.left,
        bounds.top,
        bounds.right - bounds.left,
        bounds.bottom - bounds.top
    );

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let mut msg = MSG::default();
    while Instant::now() < deadline {
        unsafe {
            while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    drop(windows);
}
