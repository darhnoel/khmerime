//! Segmented Session navigation and reflow for [`ImeSession`].
//!
//! A Segmented Session is the multi-chunk view of a long Composition where the
//! decoder identifies internal word boundaries. This module owns focus movement
//! (Left/Right), per-segment candidate cycling/selection, segment input
//! rewriting and reflow, and the (re)build of the session from a decoder
//! observation. Segment Edit Mode itself lives in
//! [`crate::segment_edit_mode`].

use std::collections::HashMap;
use std::sync::Arc;

use khmerime_core::Transliterator;

use crate::adapter_contract::{PhraseCandidate, PhraseSegment, SegmentedPreviewMode, SessionResult};
use crate::ime_session::{exact_matches_first, offset_index, recompute_segment_ranges_and_raw, ImeSession};
use crate::segment_model::{
    build_segmented_session, build_segmented_session_from_pairs, move_session_focus, normalize_visible_suggestions,
    reflow_segmented_session_from_selection, SegmentedChoice, SegmentedSession,
};

/// A segmented refinement computed *off* the session lock (the model runs while producing
/// this), then applied back under a brief lock. Carries exactly what `apply_segmented_refinement`
/// assigns, so the slow model compute never holds the session mutex.
pub struct SegmentedRefinement {
    pub(crate) segmented_session: Option<SegmentedSession>,
    pub(crate) candidates: Vec<String>,
    pub(crate) phrase_candidates: Vec<PhraseCandidate>,
}

fn has_khmer(text: &str) -> bool {
    text.chars().any(|ch| ('\u{1780}'..='\u{17FF}').contains(&ch))
}

fn would_degrade_to_roman(current: &[String], refinement: &SegmentedRefinement) -> bool {
    current.first().is_some_and(|candidate| has_khmer(candidate))
        && !refinement
            .candidates
            .first()
            .is_some_and(|candidate| has_khmer(candidate))
}

/// True when applying `refinement` would drop a live multi-word segmentation. The debounced model
/// refiner runs on a tight latency budget (250 ms); when the model overshoots it, the weighted-span
/// decode times out and collapses to a single-word top, whose `segmented_session` is `None` or has
/// fewer segments. Overwriting the good live segmentation with that makes the phrase strip vanish
/// (e.g. `kalpimun` → `កាល|ពី|មុន` replaced by a lone `កើន`). Keep the richer segmentation instead.
fn would_collapse_segmentation(current: Option<&SegmentedSession>, refinement: &SegmentedRefinement) -> bool {
    let Some(current) = current else {
        return false; // nothing segmented to lose
    };
    let current_segments = current.segments.len();
    if current_segments < 2 {
        return false; // a single-segment "session" carries no multi-word structure to protect
    }
    let refined_segments = refinement
        .segmented_session
        .as_ref()
        .map(|s| s.segments.len())
        .unwrap_or(0);
    refined_segments < current_segments
}

/// The model compute for a segmented refinement — **pure**: reads the refiner + input + history,
/// returns the result, mutates nothing. This is where the model time is spent, so it is the
/// part that must run OFF the session lock. The adapter calls it between
/// [`ImeSession::refine_inputs`] (snapshot under a brief lock) and
/// [`ImeSession::apply_segmented_refinement`] (apply under a brief lock).
pub fn compute_segmented_refinement(
    refiner: &Transliterator,
    raw: &str,
    history: &HashMap<String, usize>,
) -> SegmentedRefinement {
    // `phrase_candidates` runs Weighted Span once and preserves model provenance. Reuse that
    // single result for both the strip and Phrase Wheel; calling `shadow_observation` as well
    // would run the model twice on every pause.
    let decoded_phrases = refiner.phrase_candidates(raw, history);
    let top_segments = decoded_phrases
        .first()
        .map(|candidate| {
            candidate
                .segments
                .iter()
                .map(|segment| (segment.input.clone(), segment.output.clone()))
                .collect()
        })
        .unwrap_or_default();
    let segmented_session = build_segmented_session_from_pairs(raw, top_segments, history, 0, &|input, hist| {
        exact_matches_first(
            refiner,
            input,
            normalize_visible_suggestions(refiner.suggest(input, hist)),
        )
    });
    let candidates = exact_matches_first(
        refiner,
        raw,
        normalize_visible_suggestions(refiner.suggest(raw, history)),
    );
    let phrase_candidates = decoded_phrases
        .into_iter()
        .map(|candidate| PhraseCandidate {
            text: candidate.text,
            from_model: candidate.from_model,
            lexicon_verified: candidate.lexicon_verified,
            segments: candidate
                .segments
                .into_iter()
                .map(|segment| PhraseSegment {
                    input: segment.input,
                    output: segment.output,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    // A whole-word model rescue intentionally has one segment, while a Segmented Session is
    // only created for two or more. Keep that winner visible instead of falling back to Shadow
    // mode's Standard-only `suggest()` list.
    let candidates = if segmented_session.is_none() {
        merge_phrase_outputs_first(&phrase_candidates, candidates)
    } else {
        candidates
    };
    SegmentedRefinement {
        segmented_session,
        candidates,
        phrase_candidates,
    }
}

fn merge_phrase_outputs_first(phrases: &[PhraseCandidate], fallback: Vec<String>) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for candidate in phrases.iter().map(|candidate| candidate.text.clone()).chain(fallback) {
        let key = crate::segment_model::normalized_suggestion_key(&candidate);
        if seen.insert(key) {
            merged.push(candidate);
        }
    }
    merged
}

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

    /// Make Phrase Candidate `index` the active **Segmented Session** so the next
    /// commit takes that whole-phrase hypothesis (ADR-0014, Visible Segmented Commit).
    /// `selection_touched` pins it against the next preview rebuild.
    pub(crate) fn select_phrase(&mut self, index: usize) -> SessionResult {
        let Some(phrase) = self.phrase_candidates.get(index).cloned() else {
            return SessionResult::default();
        };
        let segments = if phrase.segments.is_empty() {
            vec![SegmentedChoice {
                input: self.composition_raw.clone(),
                start: 0,
                end: self.composition_raw.chars().count(),
                candidates: vec![phrase.text.clone()],
                selected: 0,
            }]
        } else {
            let mut start = 0usize;
            phrase
                .segments
                .iter()
                .map(|segment| {
                    let len = segment.input.chars().count();
                    let choice = SegmentedChoice {
                        input: segment.input.clone(),
                        start,
                        end: start + len,
                        candidates: vec![segment.output.clone()],
                        selected: 0,
                    };
                    start += len;
                    choice
                })
                .collect()
        };
        self.selected_phrase_index = index;
        self.segmented_session = Some(SegmentedSession {
            raw_input: self.composition_raw.clone(),
            segments,
            focused: 0,
        });
        self.selection_touched = true;
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub(crate) fn recompute_composition_state(&mut self) {
        if self.composition_raw.is_empty() {
            self.candidates.clear();
            self.phrase_candidates.clear();
            self.selected_phrase_index = 0;
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
        self.phrase_candidates = self
            .transliterator
            .phrase_candidates(&self.composition_raw, &self.history)
            .into_iter()
            .map(|candidate| PhraseCandidate {
                text: candidate.text,
                from_model: candidate.from_model,
                lexicon_verified: candidate.lexicon_verified,
                segments: candidate
                    .segments
                    .into_iter()
                    .map(|segment| PhraseSegment {
                        input: segment.input,
                        output: segment.output,
                    })
                    .collect(),
            })
            .collect();
        self.selected_phrase_index = 0;
        self.selected_index = 0;
        self.selection_touched = false;
        self.visible_refined_segments = None;

        if self.options.segmented_preview != SegmentedPreviewMode::Enabled {
            self.segmented_session = None;
            self.segment_edit_state = None;
            return;
        }

        self.rebuild_segmented_session_from_phrase_candidates();
    }

    /// Build the Segmented Session from the top Phrase Candidate's segments, which
    /// `recompute_composition_state` already decoded into `self.phrase_candidates`. This reuses the
    /// single Weighted Span pass instead of running `shadow_observation` a second time on the same
    /// composition — the deterministic recompute happens on every keystroke, so the extra decode
    /// dominated the per-key latency. Mirrors the debounced refine path
    /// ([`compute_segmented_refinement`]), which was already single-decode. When no phrase candidate
    /// is available (empty/failed decode), there are no segment pairs and no Segmented Session is
    /// built — the same outcome the observation path gave for that case.
    fn rebuild_segmented_session_from_phrase_candidates(&mut self) {
        let top_segments: Vec<(String, String)> = self
            .phrase_candidates
            .first()
            .map(|candidate| {
                candidate
                    .segments
                    .iter()
                    .map(|segment| (segment.input.clone(), segment.output.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let transliterator = &self.transliterator;
        self.segmented_session = build_segmented_session_from_pairs(
            &self.composition_raw,
            top_segments,
            &self.history,
            0,
            &|input, history| {
                exact_matches_first(
                    transliterator,
                    input,
                    normalize_visible_suggestions(transliterator.suggest(input, history)),
                )
            },
        );
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
        if self.selection_touched {
            return true;
        }
        self.rebuild_segmented_session_from_observation();
        self.segmented_session.is_some()
    }

    /// Deferred/debounced refine: rebuild the segmented preview using the VISIBLE refiner
    /// (e.g. the model span-proposal provider) instead of the cheap live engine. Called off
    /// the keystroke hot path. Falls back to the live rebuild if no visible refiner is set.
    pub fn refine_segmented_with_visible_refiner(&mut self, raw_preedit: &str) -> bool {
        if self.options.segmented_preview != SegmentedPreviewMode::Enabled {
            return false;
        }
        if self.segment_edit_state.is_some() {
            return self.segmented_session.is_some();
        }
        if self.composition_raw.is_empty() || self.composition_raw != raw_preedit {
            return false;
        }
        if self.selection_touched {
            return true;
        }
        let Some(refiner) = self.visible_refiner.clone() else {
            self.rebuild_segmented_session_from_observation();
            return self.segmented_session.is_some();
        };
        // Same compute as the lock-free path; synchronous here so the apply always matches.
        let refinement = compute_segmented_refinement(&refiner, &self.composition_raw, &self.history);
        if would_degrade_to_roman(&self.candidates, &refinement) {
            return self.segmented_session.is_some();
        }
        if would_collapse_segmentation(self.segmented_session.as_ref(), &refinement) {
            return self.segmented_session.is_some();
        }
        self.segmented_session = refinement.segmented_session;
        self.candidates = refinement.candidates;
        self.phrase_candidates = refinement.phrase_candidates;
        self.selected_phrase_index = 0;
        self.selected_index = 0;
        self.segmented_session.is_some()
    }

    /// Snapshot the inputs a lock-free refine needs (all cheap clones — `Arc` bump, two small
    /// copies), or `None` if a refine shouldn't run. The caller runs [`compute_segmented_refinement`]
    /// OFF the session lock, then re-locks for [`Self::apply_segmented_refinement`].
    pub fn refine_inputs(&self, raw: &str) -> Option<(Arc<Transliterator>, String, HashMap<String, usize>)> {
        if self.options.segmented_preview != SegmentedPreviewMode::Enabled {
            return None;
        }
        if self.segment_edit_state.is_some() {
            return None;
        }
        if self.composition_raw.is_empty() || self.composition_raw != raw {
            return None;
        }
        if self.selection_touched {
            return None;
        }
        let refiner = self.visible_refiner.clone()?;
        Some((refiner, self.composition_raw.clone(), self.history.clone()))
    }

    /// Apply a refinement computed off-lock (see [`SegmentedRefinement`]). Re-validates the same
    /// guards as the inline path against the *current* session state: if the composition changed
    /// while the model ran (`composition_raw != raw`), the stale refinement is discarded rather
    /// than clobbering the candidates the user is now looking at.
    pub fn apply_segmented_refinement(&mut self, raw: &str, refinement: SegmentedRefinement) -> bool {
        if self.options.segmented_preview != SegmentedPreviewMode::Enabled {
            return false;
        }
        if self.segment_edit_state.is_some() {
            return self.segmented_session.is_some();
        }
        if self.composition_raw.is_empty() || self.composition_raw != raw {
            return false; // staleness guard
        }
        if self.selection_touched {
            return true;
        }
        if would_degrade_to_roman(&self.candidates, &refinement) {
            return self.segmented_session.is_some();
        }
        if would_collapse_segmentation(self.segmented_session.as_ref(), &refinement) {
            return true; // keep the richer live segmentation; the timed-out refine is worse
        }
        self.segmented_session = refinement.segmented_session;
        self.candidates = refinement.candidates;
        self.phrase_candidates = refinement.phrase_candidates;
        self.selected_phrase_index = 0;
        self.selected_index = 0;
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
    use std::collections::HashMap;

    use khmerime_core::{DecoderConfig, Transliterator};

    use crate::ime_session::ImeSession;
    use crate::test_support::{
        phase_a_session_without_segmented_preview, segmented_default_session_like_ibus_bridge, session, type_ascii,
    };

    // Lock-free refine: the model runs OFF the session lock, then the result is applied. If the
    // user typed more while it computed, the composition no longer matches and the stale
    // refinement must be discarded rather than clobber the current candidates.
    #[test]
    fn apply_discards_refinement_when_composition_changed() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let before = session.snapshot().candidates.clone();

        // A refinement computed against an older composition ("oldraw" != current "khnhomtov").
        let stale = super::SegmentedRefinement {
            segmented_session: None,
            candidates: vec!["STALE".to_string()],
            phrase_candidates: Vec::new(),
        };
        let applied = session.apply_segmented_refinement("oldraw", stale);

        assert!(!applied, "stale refinement must be discarded");
        assert_eq!(
            session.snapshot().candidates,
            before,
            "candidates must not be clobbered by a stale apply"
        );
    }

    #[test]
    fn delayed_refinement_cannot_collapse_a_multiword_segmentation() {
        // Regression ("kalpimun showed កាលពីមុន then the phrase disappeared"): the live path
        // built a 3-word segmented session (កាល|ពី|មុន), but the debounced model refiner timed
        // out on its 250 ms budget and returned a collapsed single-word top (កើន). Applying that
        // wiped segmented_session → the phrase strip vanished. A refinement that drops a live
        // multi-word segmentation must be rejected, keeping the good phrase visible.
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        assert!(
            session.snapshot().segmented_active,
            "khnhomtov builds a segmented session"
        );

        let collapsed = super::SegmentedRefinement {
            segmented_session: None, // model timeout → single-word top → no segmentation
            candidates: vec!["កើន".to_string()],
            phrase_candidates: Vec::new(),
        };
        let applied = session.apply_segmented_refinement("khnhomtov", collapsed);

        assert!(!applied || session.snapshot().segmented_active);
        assert!(
            session.snapshot().segmented_active,
            "a refinement that collapses the multi-word segmentation must not wipe the phrase strip"
        );
    }

    #[test]
    fn delayed_refinement_cannot_replace_a_khmer_winner_with_raw_roman() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        assert_eq!(session.snapshot().candidates.first().map(String::as_str), Some("ជា"));

        let degraded = super::SegmentedRefinement {
            segmented_session: None,
            candidates: vec!["jea".to_string()],
            phrase_candidates: Vec::new(),
        };
        let applied = session.apply_segmented_refinement("jea", degraded);

        assert!(
            !applied,
            "a delayed refinement that only recovered Roman must be rejected"
        );
        assert_eq!(
            session.snapshot().candidates.first().map(String::as_str),
            Some("ជា"),
            "the already-visible Khmer winner must remain stable"
        );
    }

    #[test]
    fn inline_visible_refine_keeps_existing_khmer_when_refiner_only_has_roman() {
        let live = Transliterator::from_tsv_str_with_config("jea\tជា\n", DecoderConfig::shadow_interactive())
            .expect("live fixture");
        let roman_only_refiner =
            Transliterator::from_tsv_str_with_config("zzz\tហ្ស៊ី\n", DecoderConfig::shadow_interactive())
                .expect("refiner fixture");
        let mut session = ImeSession::builder(live, HashMap::new())
            .visible_refiner(roman_only_refiner)
            .build();
        session.focus_in();
        type_ascii(&mut session, "jea");

        session.refine_segmented_with_visible_refiner("jea");

        assert_eq!(
            session.snapshot().candidates.first().map(String::as_str),
            Some("ជា"),
            "the actual synchronous iOS refine path must not degrade Khmer to Roman"
        );
    }

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

    #[test]
    fn visible_refine_does_not_clobber_a_touched_selection() {
        // Regression (macOS debounced pause-refine): a visible refine must NOT reset the
        // user's in-progress candidate selection. The selection_touched guard protects it.
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        assert!(session.snapshot().segmented_active);
        let raw = session.snapshot().raw_preedit.clone();

        let down = session.process_key_event(0xFF54, 0, 0); // Down -> select candidate 1
        assert!(down.consumed);
        assert_eq!(session.snapshot().selected_index, Some(1));

        // Simulate the debounced pause-refine firing after the selection.
        session.refine_segmented_with_visible_refiner(&raw);

        assert_eq!(
            session.snapshot().selected_index,
            Some(1),
            "visible refine must not reset a selection the user already made"
        );
    }

    #[test]
    fn visible_refine_does_not_clobber_a_touched_selection_when_single_word() {
        // Bug A (macOS Space-cycle reset): the touched-selection guard also has to hold for a
        // SINGLE-word composition, where no segmented session exists. Space cycles the flat
        // candidate list and sets selection_touched; the debounced pause-refine then reran and
        // reset selected_index to 0 because the old guard also required a segmented session.
        let live = Transliterator::from_tsv_str_with_config("jea\tជា\nchea\tជា\n", DecoderConfig::shadow_interactive())
            .expect("live fixture");
        let refiner = Transliterator::from_tsv_str_with_config("jea\tជា\n", DecoderConfig::shadow_interactive())
            .expect("refiner fixture");
        let mut session = ImeSession::builder(live, HashMap::new())
            .visible_refiner(refiner)
            .build();
        session.focus_in();
        type_ascii(&mut session, "jea");
        assert!(
            !session.snapshot().segmented_active,
            "single word: no segmented session"
        );
        assert!(
            session.snapshot().candidates.len() > 1,
            "need at least two candidates to cycle"
        );

        let down = session.process_key_event(0xFF54, 0, 0); // Down -> select candidate 1
        assert!(down.consumed);
        assert_eq!(session.snapshot().selected_index, Some(1));

        session.refine_segmented_with_visible_refiner("jea");

        assert_eq!(
            session.snapshot().selected_index,
            Some(1),
            "single-word visible refine must not reset the user's Space/Down selection"
        );
    }

    #[test]
    fn visible_refine_rebuilds_segmented_preview_when_untouched() {
        // Happy path: with no manual selection yet, the pause-refine (re)builds the
        // segmented preview through the visible refiner and stays segmented.
        let mut session = segmented_default_session_like_ibus_bridge();
        type_ascii(&mut session, "khnhomtov");
        let raw = session.snapshot().raw_preedit.clone();
        let changed = session.refine_segmented_with_visible_refiner(&raw);
        assert!(changed, "refine should build a segmented preview when untouched");
        assert!(session.snapshot().segmented_active);
    }

    // Perf characterization (Android per-keystroke latency): one recompute of a segmenting
    // composition must run the Weighted Span decoder exactly once. It currently runs twice —
    // once in `phrase_candidates` and again in `shadow_observation` while building the segmented
    // session — even though the top phrase candidate already carries the segments the session
    // needs. This is the duplicate decode that roughly doubles the deterministic recompute cost.
    #[test]
    fn recompute_of_a_segmented_composition_runs_weighted_span_once() {
        let mut session = segmented_default_session_like_ibus_bridge();
        type_ascii(&mut session, "khnhomtov");
        assert!(
            session.snapshot().segmented_active,
            "khnhomtov must build a segmented session"
        );

        khmerime_core::reset_weighted_span_decode_calls();
        session.recompute_now();
        let calls = khmerime_core::weighted_span_decode_calls();

        assert_eq!(
            calls, 1,
            "one recompute must run Weighted Span once, not {calls} times (duplicate decode)"
        );
    }
}
