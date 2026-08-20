//! Windows candidate policy projected from the platform-neutral session snapshot.
//!
//! Native TSF code consumes this single surface instead of deciding whether
//! session candidates mean complete phrases or alternatives for one segment.

use khmerime_session::{CandidateDisplayEntry, SegmentPreviewEntry, SessionCommand, SessionSnapshot};

use crate::input::key_convert::{SESSION_KEY_DOWN, SESSION_KEY_SPACE, SESSION_KEY_UP};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateSurfaceMode {
    Flat,
    Phrase,
    Segment,
}

impl Default for CandidateSurfaceMode {
    fn default() -> Self {
        Self::Flat
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateSurface {
    mode: CandidateSurfaceMode,
    rows: Vec<String>,
    selected_index: Option<usize>,
    display: Vec<CandidateDisplayEntry>,
    context: Vec<SegmentPreviewEntry>,
}

impl CandidateSurface {
    pub fn from_snapshot(snapshot: &SessionSnapshot) -> Self {
        if snapshot.segmented_active && !snapshot.segment_edit_active {
            return Self {
                mode: CandidateSurfaceMode::Phrase,
                rows: snapshot
                    .phrase_candidates
                    .iter()
                    .map(|candidate| candidate.text.clone())
                    .collect(),
                selected_index: Some(snapshot.selected_phrase_index),
                display: snapshot
                    .phrase_candidates
                    .iter()
                    .enumerate()
                    .map(|(index, candidate)| CandidateDisplayEntry {
                        output: candidate.text.clone(),
                        recommended: index == 0,
                        // Deliberately empty: this hint was the INPUT's segmentation, identical
                        // for every row, so unrelated readings all claimed the same romanization.
                        // The header already shows roman per segment.
                        roman_hints: Vec::new(),
                        from_model: candidate.from_model,
                        ..CandidateDisplayEntry::default()
                    })
                    .collect(),
                context: snapshot.segment_preview.clone(),
            };
        }

        Self {
            mode: if snapshot.segment_edit_active {
                CandidateSurfaceMode::Segment
            } else {
                CandidateSurfaceMode::Flat
            },
            rows: snapshot.candidates.clone(),
            selected_index: snapshot.selected_index,
            display: snapshot.candidate_display.clone(),
            context: if snapshot.segmented_active {
                snapshot.segment_preview.clone()
            } else {
                Vec::new()
            },
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

    pub fn context(&self) -> &[SegmentPreviewEntry] {
        &self.context
    }

    /// Maps a visible phrase-row selection to the shared session command.
    /// Flat and Segment surfaces retain the session's existing key behavior.
    pub fn select_phrase_row(&self, index: usize) -> Option<SessionCommand> {
        (self.mode() == CandidateSurfaceMode::Phrase && index < self.rows.len())
            .then_some(SessionCommand::SelectPhrase(index))
    }

    pub fn cycle_phrase(&self, delta: isize) -> Option<SessionCommand> {
        if self.mode() != CandidateSurfaceMode::Phrase || self.rows.is_empty() {
            return None;
        }
        let current = self.selected_index.unwrap_or(0) % self.rows.len();
        let next = (current as isize + delta).rem_euclid(self.rows.len() as isize) as usize;
        Some(SessionCommand::SelectPhrase(next))
    }

    /// Returns the Windows-specific whole-phrase command for a candidate key.
    /// `None` delegates the key to the shared session unchanged.
    pub fn command_for_key(&self, keyval: u32) -> Option<SessionCommand> {
        match keyval {
            SESSION_KEY_UP => self.cycle_phrase(-1),
            SESSION_KEY_DOWN | SESSION_KEY_SPACE => self.cycle_phrase(1),
            value @ 0x31..=0x39 => self.select_phrase_row((value - 0x31) as usize),
            0x30 => self.select_phrase_row(9),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use khmerime_session::{
        CandidateDisplayEntry, PhraseCandidate, PhraseSegment, SegmentPreviewEntry, SessionCommand, SessionSnapshot,
    };

    use super::{CandidateSurface, CandidateSurfaceMode};

    // Phrase rows must be Khmer-only. The hint here was the INPUT's segmentation, which is the
    // same string for every row, so a candidate like កាម្រា rendered as "កាម្រា (domra)" even
    // though it does not romanize to domra at all. Roman belongs in the header, which already
    // shows it per segment. Matches the linux-ibus two-level candidate surface decision.
    #[test]
    fn phrase_rows_carry_no_per_row_roman_hint() {
        let snapshot = SessionSnapshot {
            segmented_active: true,
            phrase_candidates: vec![PhraseCandidate {
                text: "ដំរី".to_owned(),
                segments: vec![PhraseSegment {
                    input: "domra".to_owned(),
                    ..PhraseSegment::default()
                }],
                ..PhraseCandidate::default()
            }],
            ..SessionSnapshot::default()
        };

        let surface = CandidateSurface::from_snapshot(&snapshot);

        assert!(
            surface.display()[0].roman_hints.is_empty(),
            "phrase rows must not repeat roman; got {:?}",
            surface.display()[0].roman_hints
        );
    }

    // Model provenance has to survive into the display entry or the popup cannot draw ✦. The
    // Lexicon gate is a marker, not a filter: an out-of-Lexicon word may be a name or a loanword,
    // so the row stays visible and selectable and is merely labelled.
    #[test]
    fn a_model_derived_phrase_row_carries_its_provenance() {
        let snapshot = SessionSnapshot {
            segmented_active: true,
            phrase_candidates: vec![
                PhraseCandidate {
                    text: "ដុំរ៉ា".to_owned(),
                    from_model: false,
                    ..PhraseCandidate::default()
                },
                PhraseCandidate {
                    text: "ដំរី".to_owned(),
                    from_model: true,
                    ..PhraseCandidate::default()
                },
            ],
            ..SessionSnapshot::default()
        };

        let surface = CandidateSurface::from_snapshot(&snapshot);

        assert!(!surface.display()[0].from_model);
        assert!(surface.display()[1].from_model);
    }

    #[test]
    fn segmented_composition_projects_whole_phrase_candidates_by_default() {
        let snapshot = SessionSnapshot {
            candidates: vec!["word one".to_owned(), "word two".to_owned()],
            segmented_active: true,
            phrase_candidates: vec![
                PhraseCandidate {
                    text: "whole phrase one".to_owned(),
                    ..PhraseCandidate::default()
                },
                PhraseCandidate {
                    text: "whole phrase two".to_owned(),
                    ..PhraseCandidate::default()
                },
            ],
            selected_phrase_index: 1,
            ..SessionSnapshot::default()
        };

        let surface = CandidateSurface::from_snapshot(&snapshot);

        assert_eq!(surface.mode(), CandidateSurfaceMode::Phrase);
        assert_eq!(surface.rows(), ["whole phrase one", "whole phrase two"]);
        assert_eq!(surface.selected_index(), Some(1));
    }

    #[test]
    fn segment_edit_projects_only_the_focused_words_candidates() {
        let snapshot = SessionSnapshot {
            candidates: vec!["word one".to_owned(), "word two".to_owned()],
            candidate_display: vec![CandidateDisplayEntry {
                output: "word one".to_owned(),
                recommended: true,
                ..CandidateDisplayEntry::default()
            }],
            selected_index: Some(0),
            segmented_active: true,
            segment_edit_active: true,
            segment_preview: vec![SegmentPreviewEntry {
                output: "word one".to_owned(),
                input: "roman".to_owned(),
                focused: true,
            }],
            phrase_candidates: vec![PhraseCandidate {
                text: "whole phrase".to_owned(),
                ..PhraseCandidate::default()
            }],
            ..SessionSnapshot::default()
        };

        let surface = CandidateSurface::from_snapshot(&snapshot);

        assert_eq!(surface.mode(), CandidateSurfaceMode::Segment);
        assert_eq!(surface.rows(), ["word one", "word two"]);
        assert_eq!(surface.context(), snapshot.segment_preview);
        assert!(surface.display()[0].recommended);
    }

    #[test]
    fn flat_composition_preserves_the_existing_candidate_list() {
        let snapshot = SessionSnapshot {
            candidates: vec!["flat one".to_owned(), "flat two".to_owned()],
            selected_index: Some(1),
            ..SessionSnapshot::default()
        };

        let surface = CandidateSurface::from_snapshot(&snapshot);

        assert_eq!(surface.mode(), CandidateSurfaceMode::Flat);
        assert_eq!(surface.rows(), snapshot.candidates);
        assert!(surface.context().is_empty());
    }

    #[test]
    fn phrase_surface_owns_phrase_navigation_commands() {
        let snapshot = SessionSnapshot {
            segmented_active: true,
            phrase_candidates: vec![
                PhraseCandidate {
                    text: "one".to_owned(),
                    segments: vec![PhraseSegment {
                        input: "muoy".to_owned(),
                        output: "one".to_owned(),
                        roman_hints: vec![],
                    }],
                    ..PhraseCandidate::default()
                },
                PhraseCandidate {
                    text: "two".to_owned(),
                    ..PhraseCandidate::default()
                },
            ],
            ..SessionSnapshot::default()
        };
        let surface = CandidateSurface::from_snapshot(&snapshot);

        assert_eq!(surface.cycle_phrase(1), Some(SessionCommand::SelectPhrase(1)));
        assert_eq!(surface.cycle_phrase(-1), Some(SessionCommand::SelectPhrase(1)));
        assert_eq!(surface.select_phrase_row(0), Some(SessionCommand::SelectPhrase(0)));
        assert_eq!(surface.select_phrase_row(2), None);
        // Phrase rows are Khmer-only now; see phrase_rows_carry_no_per_row_roman_hint.
        assert!(surface.display()[0].roman_hints.is_empty());
        assert_eq!(
            surface.command_for_key(super::SESSION_KEY_DOWN),
            Some(SessionCommand::SelectPhrase(1))
        );
        assert_eq!(
            surface.command_for_key('2' as u32),
            Some(SessionCommand::SelectPhrase(1))
        );
    }
}
