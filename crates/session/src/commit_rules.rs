//! Commit Rules: the ordered precedence applied at commit time.
//!
//! When the user confirms a composition (Enter, Space in segment edit, single
//! keycap auto-commit), the Commit Text is chosen by the earliest rule that
//! yields useful Khmer, per `CONTEXT.md`:
//!
//! 1. Visible Segmented Commit — a visible Segmented Session is authoritative.
//! 2. Visible Candidate Commit — a selected visible candidate is authoritative.
//! 3. Hidden Commit Fallback — the Commit Refiner may recover Khmer only when
//!    visible state is not useful Commit Text.
//! 4. Raw roman floor — the literal roman string when nothing else applies.
//!
//! Visible state always outranks hidden refinement.

use khmerime_core::Transliterator;

use crate::adapter_contract::SessionResult;
use crate::ime_session::ImeSession;
use crate::segment_model::normalized_suggestion_key;

impl ImeSession {
    pub(crate) fn commit_selected_or_raw(&mut self) -> SessionResult {
        if self.composition_raw.is_empty() {
            return SessionResult::default();
        }

        let commit_segments = if let Some(outputs) = self.segmented_outputs() {
            outputs
        } else if let Some(outputs) = self.visible_candidate_outputs() {
            outputs
        } else {
            self.hidden_commit_fallback()
                .unwrap_or_else(|| vec![self.selected_or_raw_fallback()])
        };
        let commit_text = commit_segments.concat();
        let history_changed = !commit_text.is_empty() && commit_text != self.composition_raw;
        if history_changed {
            for segment in commit_segments.iter().filter(|segment| !segment.is_empty()) {
                Transliterator::learn(&mut self.history, segment);
            }
        }
        self.reset();
        SessionResult {
            consumed: true,
            commit_text: Some(commit_text),
            history_changed,
        }
    }

    pub(crate) fn selected_or_raw_fallback(&self) -> String {
        self.candidates
            .get(self.selected_index)
            .cloned()
            .unwrap_or_else(|| self.composition_raw.clone())
    }

    fn segmented_outputs(&self) -> Option<Vec<String>> {
        if let Some(session) = &self.segmented_session {
            let outputs = session
                .segments
                .iter()
                .map(|segment| segment.selected_text())
                .collect::<Vec<_>>();
            if outputs.iter().any(|output| !output.is_empty()) {
                return Some(outputs);
            }
        }
        None
    }

    fn visible_candidate_outputs(&self) -> Option<Vec<String>> {
        let visible = self.candidates.get(self.selected_index)?;
        if visible.is_empty() || visible == &self.composition_raw {
            return None;
        }
        if self.selected_index == 0 {
            if let Some(refined_segments) = &self.visible_refined_segments {
                if normalized_suggestion_key(&refined_segments.concat()) == normalized_suggestion_key(visible) {
                    return Some(refined_segments.clone());
                }
            }
        }
        Some(vec![visible.clone()])
    }

    fn hidden_commit_fallback(&self) -> Option<Vec<String>> {
        let visible_default = self.candidates.first()?;
        if !visible_default.is_empty() && visible_default != &self.composition_raw {
            return None;
        }
        let refiner = self.commit_refiner.as_ref()?;
        let refined = self.refined_phrase_segments_for(refiner, &self.composition_raw)?;
        (normalized_suggestion_key(&refined.concat()) != normalized_suggestion_key(&self.composition_raw))
            .then_some(refined)
    }

    pub(crate) fn visible_refined_phrase_segments_for(&self, raw_input: &str) -> Option<Vec<String>> {
        let refiner = self.visible_refiner.as_deref().or(self.commit_refiner.as_ref())?;
        self.refined_phrase_segments_for(refiner, raw_input)
    }

    fn refined_phrase_segments_for(&self, refiner: &Transliterator, raw_input: &str) -> Option<Vec<String>> {
        let observation = refiner.shadow_observation(raw_input, &self.history);
        if observation.wfst_failure.is_some() || observation.wfst_top_segment_details.len() < 2 {
            return None;
        }

        let recovered_input = observation
            .wfst_top_segment_details
            .iter()
            .map(|segment| segment.input.as_str())
            .collect::<String>();
        if recovered_input != raw_input {
            return None;
        }

        let refined = observation
            .wfst_top_segment_details
            .iter()
            .map(|segment| segment.output.clone())
            .collect::<Vec<_>>();
        refined.iter().any(|output| !output.is_empty()).then_some(refined)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{
        flat_default_session_with_commit_refiner, flat_default_session_with_split_refiners, session, type_ascii,
    };

    #[test]
    fn enter_commits_selected_candidate() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("ជា"));
        assert!(update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn enter_refines_long_flat_default_candidate_commit() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");
        let refined = session.apply_refined_candidate("nihjeasnadaiborkbrae");
        assert_eq!(refined.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));

        let update = session.process_key_event(0xFF0D, 0, 0);

        assert_eq!(update.commit_text.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));
        assert!(update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn bounded_refined_commit_learns_refined_segments_not_concatenated_phrase() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");
        let refined = session.apply_refined_candidate("nihjeasnadaiborkbrae");
        assert_eq!(refined.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));

        let update = session.process_key_event(0xFF0D, 0, 0);

        assert_eq!(update.commit_text.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));
        assert!(update.history_changed);
        for segment in ["នេះ", "ជា", "ស្នាដៃ", "បក", "ប្រែ"] {
            assert_eq!(session.history().get(segment), Some(&1));
        }
        assert_eq!(session.history().get("នេះជាស្នាដៃបកប្រែ"), None);
    }

    #[test]
    fn hidden_commit_refinement_does_not_override_visible_default_candidate() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "kasanmot");
        let snapshot = session.snapshot();
        assert_eq!(snapshot.candidates.first().map(String::as_str), Some("ការសន្មត"));

        let update = session.process_key_event(0xFF0D, 0, 0);

        assert_eq!(update.commit_text.as_deref(), Some("ការសន្មត"));
        assert_ne!(update.commit_text.as_deref(), Some("កសាងម៉ូត"));
    }

    #[test]
    fn short_exact_chunk_anchors_compound_phrase_refinement() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "gettengos");
        let refined = session.apply_refined_candidate("gettengos");
        assert_eq!(refined.as_deref(), Some("គេទាំងអស់"));

        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("គេទាំងអស់"));
    }

    #[test]
    fn short_exact_chunk_anchors_compound_phrase_with_long_prefix() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "jeanggettengos");
        let refined = session.apply_refined_candidate("jeanggettengos");
        assert_eq!(refined.as_deref(), Some("ជាងគេទាំងអស់"));

        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("ជាងគេទាំងអស់"));
    }

    #[test]
    fn visible_refinement_prepends_long_flat_default_candidate() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");
        assert_ne!(
            session.snapshot().candidates.first().map(String::as_str),
            Some("នេះជាស្នាដៃបកប្រែ")
        );

        let refined = session.apply_refined_candidate("nihjeasnadaiborkbrae");

        assert_eq!(refined.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));
        let snapshot = session.snapshot();
        assert_eq!(snapshot.candidates.first().map(String::as_str), Some("នេះជាស្នាដៃបកប្រែ"));
        assert_eq!(snapshot.raw_preedit, "nihjeasnadaiborkbrae");
        assert_eq!(snapshot.preedit, "nihjeasnadaiborkbrae");
        assert_eq!(snapshot.selected_index, Some(0));
    }

    #[test]
    fn visible_refinement_uses_bounded_visible_refiner() {
        let mut session = flat_default_session_with_split_refiners();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");

        let refined = session.apply_refined_candidate("nihjeasnadaiborkbrae");

        assert_eq!(refined.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));
        assert_eq!(
            session.snapshot().candidates.first().map(String::as_str),
            Some("នេះជាស្នាដៃបកប្រែ")
        );
    }

    #[test]
    fn visible_refinement_ignores_stale_raw_preedit() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");

        let refined = session.apply_refined_candidate("nihjeasnadai");

        assert!(refined.is_none());
        assert_ne!(
            session.snapshot().candidates.first().map(String::as_str),
            Some("នេះជាស្នាដៃបកប្រែ")
        );
    }

    #[test]
    fn visible_refinement_preserves_explicit_non_default_selection() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");
        let before = session.snapshot();
        assert!(
            before.candidates.len() >= 2,
            "test needs a non-default candidate to verify explicit selection"
        );
        let expected = before.candidates[1].clone();

        let down = session.process_key_event(0xFF54, 0, 0);
        assert!(down.consumed);
        assert_eq!(session.snapshot().selected_index, Some(1));
        let refined = session.apply_refined_candidate("nihjeasnadaiborkbrae");

        assert!(refined.is_none());
        let snapshot = session.snapshot();
        assert_eq!(snapshot.selected_index, Some(1));
        assert_eq!(snapshot.candidates.get(1).map(String::as_str), Some(expected.as_str()));
    }

    #[test]
    fn explicit_non_default_flat_selection_bypasses_commit_refinement() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");
        let before = session.snapshot();
        assert!(
            before.candidates.len() >= 2,
            "test needs a non-default candidate to verify explicit selection"
        );
        let expected = before.candidates[1].clone();

        let down = session.process_key_event(0xFF54, 0, 0);
        assert!(down.consumed);
        assert_eq!(session.snapshot().selected_index, Some(1));
        let update = session.process_key_event(0xFF0D, 0, 0);

        assert_eq!(update.commit_text.as_deref(), Some(expected.as_str()));
        assert_ne!(update.commit_text.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));
    }

    #[test]
    fn single_digit_keycap_commits_immediately() {
        let mut session = session();
        let update = session.process_key_event('1' as u32, 0, 0);
        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("១"));
        assert!(!update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn single_symbol_keycap_commits_immediately() {
        let mut session = session();
        let update = session.process_key_event('=' as u32, 0, 0);
        assert!(update.consumed);
        assert_eq!(update.commit_text.as_deref(), Some("៌"));
        assert!(!update.history_changed);
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn selecting_raw_fallback_candidate_commits_literal_without_learning() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let candidate_len = session.snapshot().candidates.len();
        assert!(candidate_len >= 2);

        for _ in 1..candidate_len {
            let down = session.process_key_event(0xFF54, 0, 0);
            assert!(down.consumed);
        }

        let snapshot = session.snapshot();
        assert_eq!(snapshot.selected_index, Some(candidate_len - 1));
        assert_eq!(snapshot.candidates.last().map(String::as_str), Some("jea"));

        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("jea"));
        assert!(!update.history_changed);
    }

    #[test]
    fn touched_raw_visible_fallback_still_allows_hidden_commit_refinement() {
        let mut session = flat_default_session_with_commit_refiner();
        type_ascii(&mut session, "nihjeasnadaiborkbrae");
        session.candidates = vec!["nihjeasnadaiborkbrae".to_owned()];
        session.selected_index = 0;
        session.selection_touched = true;
        session.segmented_session = None;

        let update = session.process_key_event(0xFF0D, 0, 0);

        assert_eq!(update.commit_text.as_deref(), Some("នេះជាស្នាដៃបកប្រែ"));
        assert!(update.history_changed);
    }

    #[test]
    fn enter_commits_full_segmented_phrase() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let update = session.process_key_event(0xFF0D, 0, 0);
        let commit_text = update.commit_text.expect("must commit text");
        assert!(!commit_text.is_empty());
        assert_ne!(commit_text, "khnhomtov");
    }

    #[test]
    fn segmented_commit_learns_each_segment_not_concatenated_phrase() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");

        let update = session.process_key_event(0xFF0D, 0, 0);

        assert_eq!(update.commit_text.as_deref(), Some("ខ្ញុំទៅ"));
        assert!(update.history_changed);
        assert_eq!(session.history().get("ខ្ញុំ"), Some(&1));
        assert_eq!(session.history().get("ទៅ"), Some(&1));
        assert_eq!(session.history().get("ខ្ញុំទៅ"), None);
    }

    #[test]
    fn enter_with_no_match_commits_raw_roman() {
        let mut session = session();
        for key in ['x', 'x', 'x'] {
            session.process_key_event(key as u32, 0, 0);
        }
        let update = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(update.commit_text.as_deref(), Some("xxx"));
        assert!(!update.history_changed);
    }
}
