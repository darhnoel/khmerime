//! Segmented Session navigation and reflow for [`ImeSession`].
//!
//! A Segmented Session is the multi-chunk view of a long Composition where the
//! decoder identifies internal word boundaries. This module owns focus movement
//! (Left/Right), per-segment candidate cycling/selection, segment input
//! rewriting and reflow, and the (re)build of the session from a decoder
//! observation. Segment Edit Mode itself lives in
//! [`crate::segment_edit_mode`].

use std::collections::HashMap;

use khmerime_core::{
    build_segmented_session, move_session_focus, normalize_visible_suggestions,
    reflow_segmented_session_from_selection, SegmentedSession,
};

use crate::adapter_contract::{SegmentedPreviewMode, SessionResult};
use crate::ime_session::{exact_matches_first, offset_index, recompute_segment_ranges_and_raw, ImeSession};

impl ImeSession {
    pub(crate) fn handle_left(&mut self) -> SessionResult {
        let Some(mut session) = self.segmented_session.clone() else {
            return SessionResult::default();
        };
        move_session_focus(&mut session, -1);
        self.segmented_session = Some(session);
        self.segment_edit_state = None;
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub(crate) fn handle_right(&mut self) -> SessionResult {
        let Some(mut session) = self.segmented_session.clone() else {
            return SessionResult::default();
        };
        move_session_focus(&mut session, 1);
        self.segmented_session = Some(session);
        self.segment_edit_state = None;
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub(crate) fn cycle_candidates(&mut self, delta: isize) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }

        if let Some(session) = self.segmented_session.clone() {
            let focused = session.focused;
            let Some(segment) = session.segments.get(focused) else {
                return SessionResult::default();
            };
            if segment.candidates.is_empty() {
                return SessionResult {
                    consumed: true,
                    ..SessionResult::default()
                };
            }
            let next_index = offset_index(segment.selected, segment.candidates.len(), delta);
            self.select_focused_segment_candidate(next_index);
            self.selection_touched = true;
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        if self.candidates.is_empty() {
            return SessionResult::default();
        }

        self.selected_index = offset_index(self.selected_index, self.candidates.len(), delta);
        self.selection_touched = true;
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub(crate) fn select_focused_segment_candidate(&mut self, index: usize) {
        let Some(mut session) = self.segmented_session.clone() else {
            return;
        };
        let focused = session.focused;
        let Some(segment) = session.segments.get(focused) else {
            return;
        };
        if index >= segment.candidates.len() {
            return;
        }
        session.segments[focused].selected = index;
        self.segmented_session = Some(self.maybe_reflow_segmented_session(session));
    }

    pub(crate) fn select_segment_candidate_without_reflow(&mut self, segment_index: usize, candidate_index: usize) {
        let Some(session) = &mut self.segmented_session else {
            return;
        };
        let Some(segment) = session.segments.get_mut(segment_index) else {
            return;
        };
        if candidate_index < segment.candidates.len() {
            segment.selected = candidate_index;
        }
    }

    pub(crate) fn replace_segment_input(&mut self, index: usize, input: String) {
        let candidates = self.candidates_for_segment_input(&input);
        let Some(session) = &mut self.segmented_session else {
            return;
        };
        let Some(segment) = session.segments.get_mut(index) else {
            return;
        };
        segment.input = input;
        segment.candidates = candidates;
        segment.selected = 0;
        self.composition_raw = recompute_segment_ranges_and_raw(session);
        session.raw_input = self.composition_raw.clone();
    }

    fn candidates_for_segment_input(&self, input: &str) -> Vec<String> {
        let mut candidates = exact_matches_first(
            &self.transliterator,
            input,
            normalize_visible_suggestions(self.transliterator.suggest(input, &self.history)),
        );
        if candidates.is_empty() {
            candidates.push(input.to_owned());
        }
        candidates.truncate(10);
        candidates
    }

    fn maybe_reflow_segmented_session(&self, session: SegmentedSession) -> SegmentedSession {
        let transliterator = &self.transliterator;
        let suggest = |input: &str, history: &HashMap<String, usize>| -> Vec<String> {
            exact_matches_first(
                transliterator,
                input,
                normalize_visible_suggestions(transliterator.suggest(input, history)),
            )
        };
        reflow_segmented_session_from_selection(
            &session,
            &self.history,
            &suggest,
            &|input, target| transliterator.best_prefix_consumption(input, target),
            &|input, history| transliterator.shadow_observation(input, history),
        )
        .unwrap_or(session)
    }

    pub(crate) fn recompute_composition_state(&mut self) {
        if self.composition_raw.is_empty() {
            self.candidates.clear();
            self.selected_index = 0;
            self.selection_touched = false;
            self.segmented_session = None;
            self.segment_edit_state = None;
            self.visible_refined_segments = None;
            return;
        }

        self.candidates = exact_matches_first(
            &self.transliterator,
            &self.composition_raw,
            normalize_visible_suggestions(self.transliterator.suggest(&self.composition_raw, &self.history)),
        );
        self.selected_index = 0;
        self.selection_touched = false;
        self.visible_refined_segments = None;

        if self.options.segmented_preview != SegmentedPreviewMode::Enabled {
            self.segmented_session = None;
            self.segment_edit_state = None;
            return;
        }

        self.rebuild_segmented_session_from_observation();
    }

    pub fn refresh_segmented_preview(&mut self, raw_preedit: &str) -> bool {
        if self.options.segmented_preview == SegmentedPreviewMode::Disabled {
            self.segmented_session = None;
            return false;
        }
        if self.segment_edit_state.is_some() {
            return self.segmented_session.is_some();
        }
        if self.composition_raw.is_empty() || self.composition_raw != raw_preedit {
            return false;
        }
        if self.segmented_session.is_some() && self.selection_touched {
            return true;
        }
        self.rebuild_segmented_session_from_observation();
        self.segmented_session.is_some()
    }

    fn rebuild_segmented_session_from_observation(&mut self) {
        let observation = self
            .transliterator
            .shadow_observation(&self.composition_raw, &self.history);
        let transliterator = &self.transliterator;
        self.segmented_session =
            build_segmented_session(&observation, &self.composition_raw, &self.history, &|input, history| {
                exact_matches_first(
                    transliterator,
                    input,
                    normalize_visible_suggestions(transliterator.suggest(input, history)),
                )
            });
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        phase_a_session_without_segmented_preview, segmented_default_session_like_ibus_bridge, session, type_ascii,
    };

    #[test]
    fn segmented_preview_can_be_disabled_for_phase_a_sessions() {
        let mut session = phase_a_session_without_segmented_preview();

        type_ascii(&mut session, "nihjeasnadaiborkbrae");

        let snapshot = session.snapshot();
        assert_eq!(snapshot.raw_preedit, "nihjeasnadaiborkbrae");
        assert!(!snapshot.candidates.is_empty());
        assert!(!snapshot.segmented_active);
        assert!(snapshot.segment_preview.is_empty());
    }

    #[test]
    fn segmenter_does_not_collapse_steurthleay_into_rare_pali_compound() {
        let mut session = segmented_default_session_like_ibus_bridge();
        type_ascii(&mut session, "teungttrungsteurthleay");
        let snapshot = session.snapshot();
        assert!(snapshot.segmented_active, "expected segmented session");
        let outputs: Vec<&str> = snapshot
            .segment_preview
            .iter()
            .map(|entry| entry.output.as_str())
            .collect();
        assert_eq!(
            outputs,
            vec!["តឹង", "ទ្រូង", "ស្ទើរ", "ធ្លាយ"],
            "segmenter should split steurthleay rather than fall through to a frequency-1 Pali compound"
        );
        for segment in &snapshot.segment_preview {
            assert_ne!(segment.output, "អច្ឆិទ្ទវុត្តី");
        }
    }

    #[test]
    fn segment_focus_moves_with_left_right() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let snapshot = session.snapshot();
        assert!(snapshot.segmented_active);
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert!(!snapshot.segment_preview.is_empty());

        let right = session.process_key_event(0xFF53, 0, 0);
        assert!(right.consumed);
        assert_eq!(session.snapshot().focused_segment_index, Some(1));

        let left = session.process_key_event(0xFF51, 0, 0);
        assert!(left.consumed);
        assert_eq!(session.snapshot().focused_segment_index, Some(0));
    }

    #[test]
    fn up_down_cycle_segment_candidates_without_moving_focus() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");

        let snapshot = session.snapshot();
        assert!(snapshot.segmented_active);
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert_eq!(snapshot.selected_index, Some(0));
        assert!(snapshot.candidates.len() >= 2);

        let down = session.process_key_event(0xFF54, 0, 0);
        assert!(down.consumed);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert_eq!(snapshot.selected_index, Some(1));

        let up = session.process_key_event(0xFF52, 0, 0);
        assert!(up.consumed);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.focused_segment_index, Some(0));
        assert_eq!(snapshot.selected_index, Some(0));
    }

    #[test]
    fn left_right_pass_through_without_segmented_session() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let left = session.process_key_event(0xFF51, 0, 0);
        let right = session.process_key_event(0xFF53, 0, 0);
        assert!(!left.consumed);
        assert!(!right.consumed);
    }
}
