//! What the candidate popup paints, decided without any Win32.
//!
//! The native window (`tsf::candidates`) only measures text and draws; every decision about
//! *which* rows are shown, how they are numbered, which mark a row carries and how big the
//! popup is at a given DPI lives here so it can be unit-tested.
//!
//! Layout is a Windows 11 style vertical list: a digit column, a mark column, the Khmer word,
//! and (flat mode only) the row's canonical roman right-aligned and muted, with a footer
//! carrying the page indicator.
//!
//! Phrase and word (segment) levels follow macOS (its ADR-0004, windows-tsf ADR-0003): the
//! roman lives once, in a segment header of Khmer chunks with each chunk's roman beneath, and
//! the rows are Khmer-only. Repeating the roman on every row only echoed the same spelling.

use khmerime_session::ImeSessionOptions;

use super::candidate_surface::{CandidateSurface, CandidateSurfaceMode};

/// Provenance/ranking mark on a candidate row. Precedence (highest first) is the order of the
/// variants: which engine produced a reading is more useful than whether ranking prefers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowMark {
    /// A span-proposal provider contributed this reading.
    Model,
    /// The ranking's preferred reading.
    Recommended,
    /// No canonical Lexicon roman exists for this reading.
    Derived,
}

impl RowMark {
    /// The glyph shared with the IBus and macOS adapters so the desktop surfaces read alike.
    pub fn glyph(self) -> &'static str {
        match self {
            RowMark::Model => "\u{2726}",
            RowMark::Recommended => "\u{2713}",
            RowMark::Derived => "~",
        }
    }
}

/// One candidate, split into the pieces the popup lays out separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowParts {
    pub mark: Option<RowMark>,
    pub word: String,
    /// Up to three canonical roman spellings, in display order.
    pub hints: Vec<String>,
}

/// Every candidate on the surface, split into mark / word / hints.
pub fn row_parts(surface: &CandidateSurface) -> Vec<RowParts> {
    let candidates = surface.rows();
    let display = surface.display();
    let use_display = display.len() == candidates.len();
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            if !use_display {
                return RowParts {
                    mark: None,
                    word: candidate.clone(),
                    hints: Vec::new(),
                };
            }
            let entry = &display[index];
            let word = if entry.output.trim().is_empty() {
                candidate.clone()
            } else {
                entry.output.trim().to_owned()
            };
            let hints: Vec<String> = entry
                .roman_hints
                .iter()
                .map(|hint| hint.trim())
                .filter(|hint| !hint.is_empty())
                .take(3)
                .map(str::to_owned)
                .collect();
            let mark = if entry.from_model {
                Some(RowMark::Model)
            } else if entry.recommended {
                Some(RowMark::Recommended)
            } else if hints.is_empty() {
                Some(RowMark::Derived)
            } else {
                None
            };
            RowParts { mark, word, hints }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupRow {
    /// The digit key that picks this row (1-based, page-relative).
    pub number: usize,
    pub mark: Option<RowMark>,
    pub word: String,
    /// Canonical roman spellings joined with " / ", or empty.
    pub roman: String,
    pub selected: bool,
}

/// One segment of the phrase, as a header column: its Khmer with its roman beneath.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderSegment {
    pub khmer: String,
    /// What the user typed for this segment; empty when unknown.
    pub roman: String,
    pub focused: bool,
}

/// The segmentation shown above the rows in phrase and word modes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    pub segments: Vec<HeaderSegment>,
    /// Word mode: highlight the segment being edited. Phrase mode: the header is dim context
    /// and must not read as a competing selection.
    pub emphasize_focus: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupModel {
    /// Segmentation header (phrase / word modes only).
    pub header: Option<Header>,
    pub rows: Vec<PopupRow>,
    /// Short label for the footer's left side ("Phrase", "Word", or empty).
    pub mode_label: &'static str,
    /// 1-based page of the selection.
    pub page: usize,
    pub page_count: usize,
}

/// Rows per page. Must equal the session's `page_size`: digit keys select page-relative
/// (ADR-0013), so painting a different page than the session's would make `1` pick a row the
/// user cannot see.
pub fn page_size() -> usize {
    ImeSessionOptions::default().page_size.max(1)
}

/// Build the popup for the page holding the selection.
pub fn popup_model(surface: &CandidateSurface, page_size: usize) -> PopupModel {
    let page_size = page_size.max(1);
    let flat = surface.mode() == CandidateSurfaceMode::Flat;
    let parts = row_parts(surface);
    let selected = surface.selected_index();
    let page_index = selected.unwrap_or(0) / page_size;
    let page_count = parts.len().div_ceil(page_size).max(1);
    let page_index = page_index.min(page_count - 1);
    let start = page_index * page_size;

    let rows = parts
        .into_iter()
        .enumerate()
        .skip(start)
        .take(page_size)
        .map(|(absolute, part)| PopupRow {
            number: absolute - start + 1,
            // Off the flat list the header carries the roman, so rows keep only provenance:
            // ✓ / ~ describe a row's roman, which these rows no longer show.
            mark: if flat {
                part.mark
            } else {
                part.mark.filter(|mark| *mark == RowMark::Model)
            },
            roman: if flat { part.hints.join(" / ") } else { String::new() },
            word: part.word,
            selected: selected == Some(absolute),
        })
        .collect();

    let segments: Vec<HeaderSegment> = surface
        .context()
        .iter()
        .filter(|entry| !entry.output.trim().is_empty())
        .map(|entry| HeaderSegment {
            khmer: entry.output.trim().to_owned(),
            roman: entry.input.trim().to_owned(),
            focused: entry.focused,
        })
        .collect();
    let header = (!flat && !segments.is_empty()).then(|| Header {
        segments,
        emphasize_focus: surface.mode() == CandidateSurfaceMode::Segment,
    });

    PopupModel {
        header,
        rows,
        mode_label: match surface.mode() {
            CandidateSurfaceMode::Flat => "",
            CandidateSurfaceMode::Phrase => "Phrase",
            CandidateSurfaceMode::Segment => "Word",
        },
        page: page_index + 1,
        page_count,
    }
}

impl PopupModel {
    /// Right-hand footer text: the page indicator when there is more than one page, then the
    /// keys that act on the popup.
    pub fn footer_right(&self) -> String {
        if self.page_count > 1 {
            format!("{} / {}  \u{00b7}  \u{2191}\u{2193}  Enter", self.page, self.page_count)
        } else {
            "\u{2191}\u{2193}  Enter".to_owned()
        }
    }
}

// ---------------------------------------------------------------------------
// Layout metrics. Designed in 96-DPI pixels and scaled to the window's DPI.
// ---------------------------------------------------------------------------

/// Popup geometry at one DPI. Every field is in device pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopupMetrics {
    pub dpi: u32,
    pub outer_pad: i32,
    pub row_h: i32,
    pub row_left_pad: i32,
    pub row_right_pad: i32,
    pub num_w: i32,
    pub mark_w: i32,
    pub col_gap: i32,
    pub roman_gap: i32,
    /// Header: Khmer row, roman row beneath it, space below before the separator.
    pub header_khmer_h: i32,
    pub header_roman_h: i32,
    pub header_bottom: i32,
    /// Gap between header columns, and padding inside each column.
    pub header_gap: i32,
    pub header_chip_pad: i32,
    pub footer_h: i32,
    pub footer_pad_x: i32,
    pub select_radius: i32,
    pub bar_w: i32,
    pub bar_inset_y: i32,
    pub corner_radius: i32,
    pub min_w: i32,
    pub max_w: i32,
    pub word_font: i32,
    pub small_font: i32,
    pub footer_font: i32,
    pub header_khmer_font: i32,
    pub header_roman_font: i32,
}

pub const BASE_DPI: u32 = 96;

impl PopupMetrics {
    pub fn for_dpi(dpi: u32) -> Self {
        let dpi = if dpi == 0 { BASE_DPI } else { dpi };
        let s = |v: i32| (v * dpi as i32 + BASE_DPI as i32 / 2) / BASE_DPI as i32;
        Self {
            dpi,
            outer_pad: s(4),
            row_h: s(36),
            row_left_pad: s(8),
            row_right_pad: s(10),
            num_w: s(22),
            mark_w: s(16),
            col_gap: s(8),
            roman_gap: s(20),
            header_khmer_h: s(28),
            header_roman_h: s(16),
            header_bottom: s(6),
            header_gap: s(6),
            header_chip_pad: s(6),
            footer_h: s(26),
            footer_pad_x: s(10),
            select_radius: s(4),
            bar_w: s(3).max(2),
            bar_inset_y: s(10),
            corner_radius: s(8),
            min_w: s(240),
            max_w: s(720),
            word_font: s(20),
            small_font: s(12),
            footer_font: s(11),
            header_khmer_font: s(16),
            header_roman_font: s(11),
        }
    }

    /// Total header height (Khmer row + roman row + space above the separator).
    pub fn header_h(&self) -> i32 {
        self.header_khmer_h + self.header_roman_h + self.header_bottom
    }

    /// Width of one header column: the wider of its Khmer and roman, plus padding.
    pub fn header_column_width(&self, khmer_w: i32, roman_w: i32) -> i32 {
        khmer_w.max(roman_w) + self.header_chip_pad * 2
    }

    /// Content width of the whole header, given each column's width.
    pub fn header_content_width(&self, column_widths: &[i32]) -> i32 {
        let gaps = column_widths.len().saturating_sub(1) as i32 * self.header_gap;
        column_widths.iter().sum::<i32>() + gaps
    }

    /// Left edge of the word, measured from the row's left edge.
    pub fn word_offset(&self) -> i32 {
        self.row_left_pad + self.num_w + self.col_gap + self.mark_w + self.col_gap
    }

    pub fn height(&self, model: &PopupModel) -> i32 {
        let header = if model.header.is_some() { self.header_h() } else { 0 };
        self.outer_pad * 2 + header + model.rows.len() as i32 * self.row_h + self.footer_h
    }

    /// Width for the widest row, clamped to `[min_w, max_w]` and to the monitor's work area.
    /// `widest_row` is the measured word + roman content of the widest row; `header_w` the
    /// header's content width (0 without a header). A header wider than the ceiling truncates.
    pub fn width(&self, widest_row: i32, header_w: i32, work_area_w: i32) -> i32 {
        let row = self.outer_pad * 2 + self.word_offset() + widest_row + self.row_right_pad;
        let header = self.outer_pad * 2 + self.row_left_pad * 2 + header_w;
        let work = work_area_w.max(1);
        let min = self.min_w.min(work);
        let max = self.max_w.min(work).max(min);
        row.max(header).clamp(min, max)
    }

    /// Content width of one row (word, plus gap and roman when there is roman).
    pub fn row_content_width(&self, word_w: i32, roman_w: i32) -> i32 {
        if roman_w > 0 {
            word_w + self.roman_gap + roman_w
        } else {
            word_w
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use khmerime_session::{CandidateDisplayEntry, SessionSnapshot};

    fn entry(output: &str, recommended: bool, from_model: bool, hints: &[&str]) -> CandidateDisplayEntry {
        CandidateDisplayEntry {
            output: output.into(),
            recommended,
            from_model,
            roman_hints: hints.iter().map(|h| h.to_string()).collect(),
            ..CandidateDisplayEntry::default()
        }
    }

    fn flat(n: usize, selected: usize) -> CandidateSurface {
        let candidates: Vec<String> = (0..n).map(|i| format!("w{i}")).collect();
        let display = (0..n).map(|i| entry(&format!("w{i}"), false, false, &["r"])).collect();
        CandidateSurface::from_snapshot(&SessionSnapshot {
            candidates,
            candidate_display: display,
            selected_index: Some(selected),
            ..SessionSnapshot::default()
        })
    }

    #[test]
    fn marks_follow_model_then_recommended_then_derived() {
        let surface = CandidateSurface::from_snapshot(&SessionSnapshot {
            candidates: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            candidate_display: vec![
                entry("a", true, true, &["x"]),
                entry("b", true, false, &[]),
                entry("c", false, false, &[]),
                entry("d", false, false, &["dd"]),
            ],
            ..SessionSnapshot::default()
        });
        let marks: Vec<_> = row_parts(&surface).into_iter().map(|p| p.mark).collect();
        assert_eq!(
            marks,
            vec![
                Some(RowMark::Model),
                Some(RowMark::Recommended),
                Some(RowMark::Derived),
                None
            ]
        );
    }

    #[test]
    fn roman_hints_are_capped_at_three_and_joined() {
        let surface = CandidateSurface::from_snapshot(&SessionSnapshot {
            candidates: vec!["a".into()],
            candidate_display: vec![entry("a", false, false, &["p", " q ", "", "r", "s"])],
            ..SessionSnapshot::default()
        });
        assert_eq!(popup_model(&surface, 9).rows[0].roman, "p / q / r");
    }

    #[test]
    fn first_page_is_numbered_from_one_and_marks_selection() {
        let model = popup_model(&flat(14, 2), 9);
        assert_eq!(model.rows.len(), 9);
        assert_eq!(model.rows[0].number, 1);
        assert_eq!(model.rows[8].number, 9);
        assert!(model.rows[2].selected);
        assert_eq!((model.page, model.page_count), (1, 2));
    }

    #[test]
    fn selection_past_the_first_page_shows_the_page_the_digits_act_on() {
        // Session digit math: absolute = (selected / page_size) * page_size + slot. With the
        // selection on row 11, `1` picks row 10, so row 10 must be painted as "1".
        let model = popup_model(&flat(14, 10), 9);
        assert_eq!((model.page, model.page_count), (2, 2));
        assert_eq!(model.rows.len(), 5);
        assert_eq!(model.rows[0].word, "w9");
        assert_eq!(model.rows[0].number, 1);
        assert!(model.rows[1].selected);
    }

    #[test]
    fn popup_page_size_matches_the_session() {
        assert_eq!(page_size(), ImeSessionOptions::default().page_size);
    }

    #[test]
    fn footer_shows_page_indicator_only_when_paged() {
        assert!(!popup_model(&flat(3, 0), 9).footer_right().contains('/'));
        assert!(popup_model(&flat(14, 0), 9).footer_right().starts_with("1 / 2"));
    }

    #[test]
    fn metrics_scale_with_dpi() {
        let base = PopupMetrics::for_dpi(96);
        let big = PopupMetrics::for_dpi(192);
        assert_eq!(big.row_h, base.row_h * 2);
        assert_eq!(big.word_font, base.word_font * 2);
        assert_eq!(PopupMetrics::for_dpi(144).row_h, 54);
        assert_eq!(PopupMetrics::for_dpi(0), base);
    }

    fn segmented(edit: bool) -> CandidateSurface {
        use khmerime_session::{PhraseCandidate, PhraseSegment, SegmentPreviewEntry};
        let seg = |input: &str, output: &str| PhraseSegment {
            input: input.into(),
            output: output.into(),
            roman_hints: vec![input.into()],
        };
        let preview = |output: &str, input: &str, focused: bool| SegmentPreviewEntry {
            output: output.into(),
            input: input.into(),
            focused,
        };
        CandidateSurface::from_snapshot(&SessionSnapshot {
            segmented_active: true,
            segment_edit_active: edit,
            segment_preview: vec![preview("ខ្ញុំ", "knhom", false), preview("ទៅ", "tov", true)],
            phrase_candidates: vec![
                PhraseCandidate {
                    text: "ខ្ញុំទៅ".into(),
                    segments: vec![seg("knhom", "ខ្ញុំ"), seg("tov", "ទៅ")],
                    ..PhraseCandidate::default()
                },
                PhraseCandidate {
                    text: "ខ្ញុំទុក".into(),
                    from_model: true,
                    ..PhraseCandidate::default()
                },
            ],
            // Segment edit lists the focused word's alternatives.
            candidates: vec!["ទៅ".into(), "ទូ".into()],
            candidate_display: vec![entry("ទៅ", true, false, &["tov"]), entry("ទូ", false, false, &["tou"])],
            selected_index: Some(0),
            ..SessionSnapshot::default()
        })
    }

    #[test]
    fn phrase_rows_are_khmer_only_and_keep_model_provenance() {
        // macOS ADR-0004: off the flat list the roman lives in the header, not on every row.
        let model = popup_model(&segmented(false), 9);
        assert_eq!(model.rows[0].word, "ខ្ញុំទៅ");
        assert!(model.rows.iter().all(|row| row.roman.is_empty()));
        assert_eq!(model.rows[0].mark, None, "✓/~ describe a roman the row no longer shows");
        assert_eq!(model.rows[1].mark, Some(RowMark::Model));
    }

    #[test]
    fn header_puts_each_segments_roman_under_its_khmer() {
        let header = popup_model(&segmented(false), 9)
            .header
            .expect("phrase mode has a header");
        let columns: Vec<_> = header
            .segments
            .iter()
            .map(|s| (s.khmer.as_str(), s.roman.as_str()))
            .collect();
        assert_eq!(columns, vec![("ខ្ញុំ", "knhom"), ("ទៅ", "tov")]);
        assert!(header.segments[1].focused);
    }

    #[test]
    fn header_is_dim_context_in_phrase_mode_and_highlights_focus_in_word_mode() {
        assert!(!popup_model(&segmented(false), 9).header.unwrap().emphasize_focus);
        let word = popup_model(&segmented(true), 9);
        assert!(word.header.unwrap().emphasize_focus);
        assert!(word.rows.iter().all(|row| row.roman.is_empty()));
        assert_eq!(word.mode_label, "Word");
    }

    #[test]
    fn flat_list_has_no_header_and_keeps_row_roman() {
        let model = popup_model(&flat(3, 0), 9);
        assert!(model.header.is_none());
        assert_eq!(model.rows[0].roman, "r");
    }

    #[test]
    fn height_counts_header_rows_and_footer() {
        let m = PopupMetrics::for_dpi(96);
        assert_eq!(m.height(&popup_model(&flat(3, 0), 9)), 8 + 3 * 36 + 26);
        assert_eq!(
            m.height(&popup_model(&segmented(false), 9)),
            8 + (28 + 16 + 6) + 2 * 36 + 26
        );
    }

    #[test]
    fn header_width_sums_columns_and_gaps() {
        let m = PopupMetrics::for_dpi(96);
        assert_eq!(m.header_column_width(30, 50), 50 + 12);
        assert_eq!(m.header_content_width(&[40, 60, 20]), 120 + 2 * 6);
        assert_eq!(m.header_content_width(&[]), 0);
    }

    #[test]
    fn width_grows_with_content_and_clamps() {
        let m = PopupMetrics::for_dpi(96);
        assert_eq!(m.width(0, 0, 1920), m.min_w);
        assert_eq!(m.width(10_000, 0, 1920), m.max_w);
        let mid = m.width(300, 0, 1920);
        assert_eq!(mid, 8 + m.word_offset() + 300 + 10);
        assert_eq!(m.width(10_000, 0, 200), 200);
    }
}
