//! macOS candidate policy projected from the platform-neutral session snapshot.
//!
//! The Swift `CandidatePanel` consumes this single surface instead of deciding whether the
//! session's candidates mean complete phrases or alternatives for one focused segment. Ports the
//! Windows TSF `render::candidate_surface` shape (windows-tsf ADR-0002); see macos-imk ADR-0004.

use khmerime_session::{CandidateDisplayEntry, PhraseCandidate, SessionCommand, SessionSnapshot};

// X11 keysyms as delivered by the macOS KeyvalMapping — same values TSF uses.
const KEY_UP: u32 = 0xFF52;
const KEY_DOWN: u32 = 0xFF54;
const KEY_SPACE: u32 = 0x20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CandidateSurfaceMode {
    #[default]
    Flat,
    Phrase,
    Segment,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateSurface {
    mode: CandidateSurfaceMode,
    rows: Vec<String>,
    selected_index: Option<usize>,
    display: Vec<CandidateDisplayEntry>,
    /// For Phrase mode only: the session `phrase_candidates` index for each visible row, since the
    /// visible rows are a filtered subset (flat single-word fallbacks are hidden). `SelectPhrase`
    /// must target the session index, not the visible row position.
    phrase_indices: Vec<usize>,
}

impl CandidateSurface {
    pub fn from_snapshot(snapshot: &SessionSnapshot) -> Self {
        // Phrase level: a live Segmented Session, not editing a single segment. Rows are the whole
        // Phrase Candidates; the segmentation becomes context (a dim header, not selectable rows).
        if snapshot.segmented_active && !snapshot.segment_edit_active {
            // Keep only whole-composition readings: multi-segment phrases OR single-word AI rescues
            // (`from_model`). Drop single-word flat fallbacks (from_model=false) — they are first-word
            // guesses, not alternatives for the whole phrase, and made the panel look wrong. Track
            // each kept row's original session index so `SelectPhrase` targets the right candidate.
            let kept: Vec<(usize, &PhraseCandidate)> = snapshot
                .phrase_candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| c.segments.len() >= 2 || c.from_model)
                .collect();
            let phrase_indices: Vec<usize> = kept.iter().map(|(i, _)| *i).collect();
            let selected_index = phrase_indices.iter().position(|&i| i == snapshot.selected_phrase_index);
            return Self {
                mode: CandidateSurfaceMode::Phrase,
                rows: kept.iter().map(|(_, c)| c.text.clone()).collect(),
                selected_index,
                display: kept
                    .iter()
                    .map(|(orig, candidate)| CandidateDisplayEntry {
                        output: candidate.text.clone(),
                        recommended: *orig == 0,
                        roman_hints: vec![candidate
                            .segments
                            .iter()
                            .map(|segment| segment.input.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")],
                        ..CandidateDisplayEntry::default()
                    })
                    .collect(),
                phrase_indices,
            };
        }

        // Segment level (editing one word) or Flat (single-segment composition): rows are the
        // ordinary Candidate List. Segment mode keeps the segmentation as context; Flat has none.
        Self {
            mode: if snapshot.segment_edit_active {
                CandidateSurfaceMode::Segment
            } else {
                CandidateSurfaceMode::Flat
            },
            rows: snapshot.candidates.clone(),
            selected_index: snapshot.selected_index,
            display: snapshot.candidate_display.clone(),
            phrase_indices: Vec::new(),
        }
    }

    pub fn mode(&self) -> CandidateSurfaceMode {
        self.mode
    }

    pub fn rows(&self) -> &[String] {
        &self.rows
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn display(&self) -> &[CandidateDisplayEntry] {
        &self.display
    }

    /// Maps a visible phrase-row selection to the shared session command. Only Phrase mode
    /// overrides selection; Flat and Segment delegate to the session's existing key behavior. The
    /// visible rows are a filtered subset, so map the row back to its session phrase index.
    pub fn select_phrase_row(&self, row: usize) -> Option<SessionCommand> {
        (self.mode == CandidateSurfaceMode::Phrase)
            .then(|| self.phrase_indices.get(row).copied())
            .flatten()
            .map(SessionCommand::SelectPhrase)
    }

    pub fn cycle_phrase(&self, delta: isize) -> Option<SessionCommand> {
        if self.mode != CandidateSurfaceMode::Phrase || self.rows.is_empty() {
            return None;
        }
        let current_row = self.selected_index.unwrap_or(0) % self.rows.len();
        let next_row = (current_row as isize + delta).rem_euclid(self.rows.len() as isize) as usize;
        self.phrase_indices
            .get(next_row)
            .copied()
            .map(SessionCommand::SelectPhrase)
    }

    /// The whole-phrase command for a candidate key at the Phrase level. `None` delegates the key
    /// to the shared session unchanged (Flat/Segment levels, and non-cycle keys).
    pub fn command_for_key(&self, keyval: u32) -> Option<SessionCommand> {
        match keyval {
            KEY_UP => self.cycle_phrase(-1),
            KEY_DOWN | KEY_SPACE => self.cycle_phrase(1),
            value @ 0x31..=0x39 => self.select_phrase_row((value - 0x31) as usize),
            0x30 => self.select_phrase_row(9),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use khmerime_session::{PhraseCandidate, PhraseSegment, SegmentPreviewEntry, SessionSnapshot};

    fn segmented_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            segmented_active: true,
            segment_edit_active: false,
            selected_phrase_index: 0,
            phrase_candidates: vec![
                PhraseCandidate {
                    text: "ខ្ញុំទៅ".to_owned(),
                    segments: vec![
                        PhraseSegment {
                            input: "khnhom".to_owned(),
                            output: "ខ្ញុំ".to_owned(),
                            roman_hints: vec![],
                        },
                        PhraseSegment {
                            input: "tov".to_owned(),
                            output: "ទៅ".to_owned(),
                            roman_hints: vec![],
                        },
                    ],
                    ..PhraseCandidate::default()
                },
                PhraseCandidate {
                    text: "ខ្ចុំទៅ".to_owned(),
                    segments: vec![
                        PhraseSegment {
                            input: "khnhom".to_owned(),
                            output: "ខ្ចុំ".to_owned(),
                            roman_hints: vec![],
                        },
                        PhraseSegment {
                            input: "tov".to_owned(),
                            output: "ទៅ".to_owned(),
                            roman_hints: vec![],
                        },
                    ],
                    ..PhraseCandidate::default()
                },
            ],
            segment_preview: vec![
                SegmentPreviewEntry {
                    output: "ខ្ញុំ".to_owned(),
                    ..SegmentPreviewEntry::default()
                },
                SegmentPreviewEntry {
                    output: "ទៅ".to_owned(),
                    ..SegmentPreviewEntry::default()
                },
            ],
            candidates: vec!["ខ្ញុំ".to_owned(), "ខ្ចុំ".to_owned()],
            ..SessionSnapshot::default()
        }
    }

    // A snapshot whose phrase_candidates mix ONE real multi-word phrase with single-word fallbacks
    // (the decoder bundles flat suggestions into phrase_candidates). Mirrors real input like
    // "kompongtrovkarsaklbong": one 4-segment phrase + ~19 one-word guesses.
    fn mixed_phrase_snapshot() -> SessionSnapshot {
        let seg = |i: &str, o: &str| PhraseSegment {
            input: i.to_owned(),
            output: o.to_owned(),
            roman_hints: vec![],
        };
        SessionSnapshot {
            segmented_active: true,
            segment_edit_active: false,
            selected_phrase_index: 0,
            phrase_candidates: vec![
                PhraseCandidate {
                    text: "ខ្ញុំទៅ".to_owned(),
                    segments: vec![seg("khnhom", "ខ្ញុំ"), seg("tov", "ទៅ")],
                    ..PhraseCandidate::default()
                },
                // single-word FLAT fallback (from_model=false) — NOT a whole-phrase alternative, drop
                PhraseCandidate {
                    text: "ខ្ចុំ".to_owned(),
                    segments: vec![seg("khnhomtov", "ខ្ចុំ")],
                    from_model: false,
                    ..PhraseCandidate::default()
                },
                // a second real phrase (2 segments) — keep
                PhraseCandidate {
                    text: "ខ្ចុំទៅ".to_owned(),
                    segments: vec![seg("khnhom", "ខ្ចុំ"), seg("tov", "ទៅ")],
                    ..PhraseCandidate::default()
                },
                // a single-word AI RESCUE (from_model=true): the model reads the whole roman span as
                // one Khmer word. A whole-composition reading, so KEEP it even though n_segs==1.
                PhraseCandidate {
                    text: "ខ្ញុំទៅ៏".to_owned(),
                    segments: vec![seg("khnhomtov", "ខ្ញុំទៅ៏")],
                    from_model: true,
                    ..PhraseCandidate::default()
                },
            ],
            segment_preview: vec![
                SegmentPreviewEntry {
                    output: "ខ្ញុំ".to_owned(),
                    ..SegmentPreviewEntry::default()
                },
                SegmentPreviewEntry {
                    output: "ទៅ".to_owned(),
                    ..SegmentPreviewEntry::default()
                },
            ],
            ..SessionSnapshot::default()
        }
    }

    #[test]
    fn phrase_mode_keeps_real_phrases_and_ai_rescues_drops_flat_fallbacks() {
        // Keep: multi-segment phrases (whole composition) AND single-word AI rescues (from_model).
        // Drop: single-word flat fallbacks (from_model=false), which are first-word guesses, not
        // whole-phrase alternatives.
        let surface = CandidateSurface::from_snapshot(&mixed_phrase_snapshot());
        assert_eq!(surface.mode(), CandidateSurfaceMode::Phrase);
        assert_eq!(
            surface.rows(),
            &["ខ្ញុំទៅ".to_owned(), "ខ្ចុំទៅ".to_owned(), "ខ្ញុំទៅ៏".to_owned()],
            "phrases + the AI single-word rescue are kept; the flat single-word fallback is dropped"
        );
    }

    #[test]
    fn phrase_selection_maps_back_to_the_unfiltered_session_index() {
        // Visible row 1 is the 2nd real phrase = session index 2 (index 1 was the dropped flat word);
        // visible row 2 is the AI rescue = session index 3. SelectPhrase must target the session index.
        let surface = CandidateSurface::from_snapshot(&mixed_phrase_snapshot());
        assert_eq!(surface.select_phrase_row(1), Some(SessionCommand::SelectPhrase(2)));
        assert_eq!(surface.select_phrase_row(2), Some(SessionCommand::SelectPhrase(3)));
        // Space from selected 0 cycles to the next VISIBLE phrase, i.e. session index 2.
        assert_eq!(
            surface.command_for_key(KEY_SPACE),
            Some(SessionCommand::SelectPhrase(2))
        );
    }

    #[test]
    fn segmented_composition_projects_phrase_mode_with_phrase_rows() {
        let surface = CandidateSurface::from_snapshot(&segmented_snapshot());
        assert_eq!(surface.mode(), CandidateSurfaceMode::Phrase);
        assert_eq!(surface.rows(), &["ខ្ញុំទៅ".to_owned(), "ខ្ចុំទៅ".to_owned()]);
        assert_eq!(surface.selected_index(), Some(0));
    }

    #[test]
    fn segment_edit_projects_segment_mode_with_word_rows() {
        let mut snap = segmented_snapshot();
        snap.segment_edit_active = true;
        snap.selected_index = Some(1);
        let surface = CandidateSurface::from_snapshot(&snap);
        assert_eq!(surface.mode(), CandidateSurfaceMode::Segment);
        // rows are the focused segment's words, not phrases
        assert_eq!(surface.rows(), &["ខ្ញុំ".to_owned(), "ខ្ចុំ".to_owned()]);
        assert_eq!(surface.selected_index(), Some(1));
    }

    #[test]
    fn flat_composition_projects_flat_mode_with_no_context() {
        let snap = SessionSnapshot {
            segmented_active: false,
            candidates: vec!["ជា".to_owned(), "ចា".to_owned()],
            selected_index: Some(0),
            ..SessionSnapshot::default()
        };
        let surface = CandidateSurface::from_snapshot(&snap);
        assert_eq!(surface.mode(), CandidateSurfaceMode::Flat);
        assert_eq!(surface.rows(), &["ជា".to_owned(), "ចា".to_owned()]);
    }

    #[test]
    fn phrase_mode_keys_cycle_and_select_phrases() {
        let surface = CandidateSurface::from_snapshot(&segmented_snapshot());
        // Space / Down cycle to the next phrase; Up wraps to the last.
        assert_eq!(
            surface.command_for_key(KEY_SPACE),
            Some(SessionCommand::SelectPhrase(1))
        );
        assert_eq!(surface.command_for_key(KEY_DOWN), Some(SessionCommand::SelectPhrase(1)));
        assert_eq!(surface.command_for_key(KEY_UP), Some(SessionCommand::SelectPhrase(1))); // wrap from 0
                                                                                            // Digit picks a phrase directly (1 -> index 0).
        assert_eq!(surface.command_for_key(0x31), Some(SessionCommand::SelectPhrase(0)));
        assert_eq!(surface.command_for_key(0x32), Some(SessionCommand::SelectPhrase(1)));
        // A digit past the row count is ignored (no phantom command).
        assert_eq!(surface.command_for_key(0x39), None);
        // A non-cycle key delegates to the session.
        assert_eq!(surface.command_for_key(0xFF09 /* Tab */), None);
    }

    #[test]
    fn flat_and_segment_modes_delegate_keys_to_the_session() {
        // Flat: no phrase override at all.
        let flat = SessionSnapshot {
            segmented_active: false,
            candidates: vec!["ជា".to_owned()],
            ..SessionSnapshot::default()
        };
        assert_eq!(CandidateSurface::from_snapshot(&flat).command_for_key(KEY_SPACE), None);
        // Segment: Space cycles words via the session, not phrases here.
        let mut seg = segmented_snapshot();
        seg.segment_edit_active = true;
        assert_eq!(CandidateSurface::from_snapshot(&seg).command_for_key(KEY_SPACE), None);
    }
}
