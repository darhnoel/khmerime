mod candidate_pipeline;
mod commit_flow;
mod manual_flow;
mod segmented_flow;
mod state;
mod view_helpers;

pub(crate) use candidate_pipeline::{normalized_suggestion_key, update_candidates};
pub(crate) use commit_flow::{click_candidate, commit_active_selection, switch_input_mode};
pub(crate) use manual_flow::{
    dismiss_manual_save_request, remove_user_dictionary_mapping, save_manual_save_request, set_manual_kind_filter,
    skip_manual_roman_char, undo_manual_step,
};
pub(crate) use segmented_flow::{move_segment_focus, select_segment_candidate};
pub(crate) use state::{
    char_len, slice_chars, EditorSignals, InputMode, ManualSaveRequest, ManualTypingState, SegmentedChoice,
    SegmentedSession,
};
pub(crate) use view_helpers::{
    composition_preview_style, composition_style, is_space_key, popup_style, refresh_popup_position,
    segmented_composition_preview_style, segmented_preview_parts, shortcut_index, shortcut_label,
    should_exit_number_pick, visible_page_start,
};

#[cfg(test)]
use candidate_pipeline::{choose_visible_suggestions, connect_khmer_display, recommended_indices_and_roman_hints};
#[cfg(test)]
use segmented_flow::{build_segmented_session, reflow_segmented_session_from_selection};

#[cfg(test)]
mod tests {
    use roman_lookup::{DecodeSegment, DecoderMode, ShadowMismatch, ShadowObservation, Transliterator};

    use super::{
        build_segmented_session, char_len, choose_visible_suggestions, connect_khmer_display,
        recommended_indices_and_roman_hints, reflow_segmented_session_from_selection, slice_chars, SegmentedChoice,
        SegmentedSession,
    };

    fn sample_observation() -> ShadowObservation {
        ShadowObservation {
            mode: DecoderMode::Shadow,
            input: "khnhomtov".to_owned(),
            mismatch: ShadowMismatch::OutputMismatch,
            composer_chunks: vec!["khnhom".to_owned(), "t".to_owned(), "ov".to_owned()],
            composer_hint_chunks: vec!["tov".to_owned()],
            composer_pending_tail: String::new(),
            composer_fully_segmented: true,
            wfst_used_hint_chunks: true,
            wfst_top_segment_details: vec![
                DecodeSegment {
                    input: "khnhom".to_owned(),
                    output: "ខ្ញុំ".to_owned(),
                    weight_bps: 9_500,
                },
                DecodeSegment {
                    input: "tov".to_owned(),
                    output: "ទៅ".to_owned(),
                    weight_bps: 9_100,
                },
            ],
            wfst_top_segments: vec!["khnhom=>ខ្ញុំ".to_owned(), "tov=>ទៅ".to_owned()],
            legacy_latency_us: 10,
            wfst_latency_us: Some(8),
            legacy_failure: None,
            wfst_failure: None,
            legacy_top: Some("ខ្ញុំ ទៅ".to_owned()),
            wfst_top: Some("ខ្ញុំទៅ".to_owned()),
            legacy_top5: vec!["ខ្ញុំ ទៅ".to_owned()],
            wfst_top5: vec!["ខ្ញុំទៅ".to_owned()],
            legacy_top_in_wfst: false,
            wfst_top_in_legacy: false,
        }
    }

    fn assert_segment(segment: &SegmentedChoice, input: &str, start: usize, end: usize, selected_text: &str) {
        assert_eq!(segment.input, input);
        assert_eq!(segment.start, start);
        assert_eq!(segment.end, end);
        assert_eq!(segment.selected_text(), selected_text);
    }

    #[test]
    fn uses_segment_candidates_in_refine_mode() {
        let legacy = vec!["ខ្ញុំ ទៅ".to_owned()];
        let observation = sample_observation();
        assert_eq!(
            choose_visible_suggestions(
                &legacy,
                &observation,
                Some(&SegmentedSession {
                    raw_input: "khnhomtov".to_owned(),
                    segments: vec![
                        SegmentedChoice {
                            input: "khnhom".to_owned(),
                            start: 0,
                            end: 6,
                            candidates: vec!["ខ្ញុំ".to_owned()],
                            selected: 0,
                        },
                        SegmentedChoice {
                            input: "tov".to_owned(),
                            start: 6,
                            end: 9,
                            candidates: vec!["ទៅ".to_owned()],
                            selected: 0,
                        },
                    ],
                    focused: 0,
                }),
                true,
            ),
            vec!["ខ្ញុំ".to_owned()]
        );
    }

    #[test]
    fn builds_segmented_session_from_structured_wfst_segments() {
        let observation = sample_observation();
        let session = build_segmented_session(&observation, "khnhomtov", &std::collections::HashMap::new()).unwrap();

        assert_eq!(session.segments.len(), 2);
        assert_segment(&session.segments[0], "khnhom", 0, 6, "ខ្ញុំ");
        assert_segment(&session.segments[1], "tov", 6, 9, "ទៅ");
    }

    #[test]
    fn merges_wfst_and_legacy_suggestions_when_available() {
        let legacy = vec!["ខ្ញុំ ទៅ".to_owned(), "ខ្ញមទៅ".to_owned()];
        let observation = sample_observation();
        assert_eq!(
            choose_visible_suggestions(&legacy, &observation, None, false),
            vec!["ខ្ញុំទៅ".to_owned(), "ខ្ញមទៅ".to_owned()]
        );
    }

    #[test]
    fn reflows_suffix_when_selected_candidate_consumes_shorter_prefix() {
        let session = SegmentedSession {
            raw_input: "cheamnouslaor".to_owned(),
            segments: vec![
                SegmentedChoice {
                    input: "cheam".to_owned(),
                    start: 0,
                    end: 5,
                    candidates: vec!["ជា".to_owned()],
                    selected: 0,
                },
                SegmentedChoice {
                    input: "ous".to_owned(),
                    start: 5,
                    end: 8,
                    candidates: vec!["អូស".to_owned()],
                    selected: 0,
                },
                SegmentedChoice {
                    input: "laor".to_owned(),
                    start: 8,
                    end: 12,
                    candidates: vec!["ល្អ".to_owned()],
                    selected: 0,
                },
            ],
            focused: 0,
        };

        let reflowed = reflow_segmented_session_from_selection(&session, &std::collections::HashMap::new()).unwrap();

        assert_segment(&reflowed.segments[0], "chea", 0, 4, "ជា");
        assert_eq!(reflowed.segments[1].start, 4);
        assert_eq!(
            slice_chars(
                &reflowed.raw_input,
                reflowed.segments[1].start..char_len(&reflowed.raw_input)
            ),
            "mnouslaor"
        );
        assert_eq!(reflowed.focused, 0);
    }

    #[test]
    fn falls_back_to_legacy_suggestions_when_wfst_has_no_candidates() {
        let legacy = vec!["ខ្ញុំ ទៅ".to_owned()];
        let mut observation = sample_observation();
        observation.wfst_failure = Some("timeout".to_owned());
        observation.wfst_top5.clear();
        assert_eq!(
            choose_visible_suggestions(&legacy, &observation, None, false),
            vec!["ខ្ញុំទៅ".to_owned()]
        );
    }

    #[test]
    fn connects_multiword_khmer_display_strings() {
        assert_eq!(connect_khmer_display("ខ្ញុំ ទៅ"), "ខ្ញុំទៅ");
        assert_eq!(connect_khmer_display("foo bar"), "foo bar");
    }

    #[test]
    fn builds_recommended_indices_with_roman_hints() {
        let fixture = "jea\tជា\nchea\tជា\njeat\tជាត\n";
        let transliterator = Transliterator::from_tsv_str(fixture).unwrap();
        let items = vec!["ជា".to_owned(), "ជាត".to_owned()];
        let (indices, hints) = recommended_indices_and_roman_hints(&transliterator, "jea", &items);
        assert_eq!(indices, vec![0]);
        assert_eq!(hints.get(&0).cloned(), Some(vec!["jea".to_owned(), "chea".to_owned()]));
    }
}
