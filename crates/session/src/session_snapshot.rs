//! Render-facing snapshot projection for [`ImeSession`].
//!
//! Projects the live composition, segmented session, and candidate state into a
//! flat [`SessionSnapshot`] that adapters render. This is a read-only view; it
//! never mutates session state.

use std::collections::HashSet;

use crate::adapter_contract::{CandidateDisplayEntry, SegmentPreviewEntry, SessionSnapshot};
use crate::ime_session::ImeSession;
use crate::segment_model::{normalized_suggestion_key, SegmentedSession};

impl ImeSession {
    pub fn snapshot(&self) -> SessionSnapshot {
        let segmented_active = self.segmented_session.is_some();
        let preedit = self
            .segmented_session
            .as_ref()
            .map(SegmentedSession::composed_text)
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| self.composition_raw.clone());
        let candidates = self
            .segmented_session
            .as_ref()
            .map(SegmentedSession::focused_candidates)
            .unwrap_or_else(|| self.candidates.clone());
        let selected_index = if candidates.is_empty() {
            None
        } else {
            self.segmented_session
                .as_ref()
                .map(SegmentedSession::focused_selected)
                .or(Some(self.selected_index))
        };
        let candidate_input = self
            .segmented_session
            .as_ref()
            .and_then(|session| session.segments.get(session.focused))
            .map(|segment| segment.input.as_str())
            .unwrap_or(self.composition_raw.as_str());
        let recommended_keys = self
            .transliterator
            .exact_match_targets(candidate_input)
            .into_iter()
            .map(|item| normalized_suggestion_key(&item))
            .collect::<HashSet<_>>();
        let candidate_display = candidates
            .iter()
            .map(|item| {
                let mut roman_hints = self.transliterator.exact_match_roman_variants(candidate_input, item);
                roman_hints.truncate(3);
                CandidateDisplayEntry {
                    output: item.clone(),
                    recommended: recommended_keys.contains(&normalized_suggestion_key(item)),
                    roman_hints,
                    is_raw_fallback: item.as_str() == candidate_input,
                }
            })
            .collect::<Vec<_>>();
        let focused_segment_index = self.segmented_session.as_ref().map(|session| session.focused);
        let segment_edit_index = self
            .segment_edit_state
            .as_ref()
            .map(|state| state.index)
            .filter(|_| self.segmented_session.is_some());
        let segment_preview = self
            .segmented_session
            .as_ref()
            .map(|session| {
                session
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(index, segment)| SegmentPreviewEntry {
                        output: segment.selected_text(),
                        input: segment.input.clone(),
                        focused: index == session.focused,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        SessionSnapshot {
            enabled: self.enabled,
            focused: self.focused,
            input_mode: self.input_mode,
            preedit,
            raw_preedit: self.composition_raw.clone(),
            candidates,
            candidate_display,
            selected_index,
            segmented_active,
            focused_segment_index,
            segment_edit_active: segment_edit_index.is_some(),
            segment_edit_index,
            segment_preview,
            phrase_candidates: self.phrase_candidates.clone(),
            cursor_location: self.cursor_location,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use khmerime_core::{DecoderConfig, Transliterator};

    use crate::adapter_contract::CursorLocation;
    use crate::ime_session::ImeSession;
    use crate::test_support::{session, type_ascii};

    #[test]
    fn snapshot_surfaces_ranked_phrase_candidates_topping_the_best_preview() {
        // ADR-0014: the decoder already ranks whole-sentence hypotheses; the
        // snapshot must surface them (not just the single best) so the mobile
        // Phrase Wheel can scroll them. Top entry must equal today's best
        // segmented preview, so surfacing the list is not a regression.
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let snapshot = session.snapshot();

        assert!(
            snapshot.phrase_candidates.len() >= 2,
            "expected multiple whole-phrase candidates, got {:?}",
            snapshot
                .phrase_candidates
                .iter()
                .map(|entry| entry.text.clone())
                .collect::<Vec<_>>()
        );

        let best_preview: String = snapshot
            .segment_preview
            .iter()
            .map(|segment| segment.output.clone())
            .collect();
        assert!(!best_preview.is_empty(), "precondition: input should segment");
        assert_eq!(
            snapshot.phrase_candidates[0].text, best_preview,
            "top phrase candidate must equal the current best segmented preview"
        );
    }

    #[test]
    fn each_phrase_candidate_carries_its_own_segmentation() {
        // ADR-0014: a Phrase Candidate pairs Khmer with its segmentation so the wheel
        // card can show the roman row and Level-2 editing can target a word.
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let snapshot = session.snapshot();
        let top = snapshot
            .phrase_candidates
            .first()
            .expect("expected at least one phrase candidate");
        assert!(
            top.segments.len() >= 2,
            "khnhomtov should segment into >= 2 words, got {:?}",
            top.segments.iter().map(|seg| seg.output.clone()).collect::<Vec<_>>()
        );
        let rebuilt: String = top.segments.iter().map(|seg| seg.output.clone()).collect();
        assert_eq!(rebuilt, top.text, "a candidate's segment outputs must reconstruct its text");
        assert!(
            top.segments.iter().all(|seg| !seg.input.is_empty()),
            "each segment must carry its roman slice for the Roman Row"
        );
    }

    #[test]
    fn phrase_candidates_end_with_the_raw_roman_fallback() {
        // ADR-0014 + ADR-0013: the raw roman string is the Commit Rules floor and
        // must be the last, always-selectable card of the Phrase Wheel.
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let snapshot = session.snapshot();
        assert_eq!(
            snapshot.phrase_candidates.last().map(|entry| entry.text.as_str()),
            Some("khnhomtov"),
            "raw roman must be the last phrase candidate, got {:?}",
            snapshot
                .phrase_candidates
                .iter()
                .map(|entry| entry.text.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn snapshot_exposes_recommended_and_roman_hint_metadata() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let snapshot = session.snapshot();
        assert!(!snapshot.candidate_display.is_empty());
        assert_eq!(snapshot.raw_preedit, "jea");
        assert_eq!(snapshot.preedit, "jea");

        let recommended = snapshot
            .candidate_display
            .iter()
            .filter(|entry| entry.recommended)
            .collect::<Vec<_>>();
        assert!(!recommended.is_empty());
        assert!(recommended
            .iter()
            .any(|entry| entry.roman_hints.iter().any(|hint| hint == "jea")));
    }

    #[test]
    fn exact_match_candidates_stay_first_before_history_fuzzy_matches() {
        let transliterator = Transliterator::from_default_data_with_config(DecoderConfig::shadow_interactive())
            .expect("default data must load");
        let mut history = HashMap::new();
        history.insert("ដោយ".to_owned(), 99);
        let mut session = ImeSession::new(transliterator, history);
        session.focus_in();

        type_ascii(&mut session, "oy");

        let snapshot = session.snapshot();
        assert_eq!(snapshot.candidates.first().map(String::as_str), Some("ឲ្យ"));
        assert!(
            snapshot
                .candidate_display
                .first()
                .map(|entry| entry.recommended)
                .unwrap_or(false),
            "top IBus candidate should be an exact roman match"
        );
        assert!(snapshot
            .candidates
            .iter()
            .position(|candidate| candidate == "ដោយ")
            .is_some_and(|index| index > 0));
    }

    #[test]
    fn set_cursor_location_updates_snapshot() {
        let mut session = session();
        session.set_cursor_location(1, 2, 3, 4);
        assert_eq!(
            session.snapshot().cursor_location,
            CursorLocation {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            }
        );
    }
}
