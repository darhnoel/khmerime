//! Segment Edit Mode for [`ImeSession`].
//!
//! Segment Edit Mode is a sub-state of a Segmented Session in which one focused
//! segment is rewritten in isolation while its siblings stay pinned. This module
//! owns the edit state and the asymmetric keystroke semantics described in
//! `CONTEXT.md`: Tab enters/exits, the first printable replaces the whole roman
//! slice, Backspace deletes one char at a time, and Backspace on an empty
//! in-edit segment transfers the mode to the previous segment (or dissolves the
//! Segmented Session). Escape (cancel/restore) is dispatched from
//! [`crate::ime_session`] since it also handles the non-segmented case.

use crate::adapter_contract::SessionResult;
use crate::ime_session::{recompute_segment_ranges_and_raw, ImeSession};
use crate::segment_model::SegmentedChoice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentEditState {
    pub(crate) index: usize,
    pub(crate) original_segment: SegmentedChoice,
    pub(crate) replace_next_printable: bool,
}

impl ImeSession {
    pub(crate) fn handle_tab(&mut self) -> SessionResult {
        let Some(session) = &self.segmented_session else {
            return SessionResult::default();
        };
        if self.segment_edit_state.is_some() {
            self.segment_edit_state = None;
        } else {
            let Some(original_segment) = session.segments.get(session.focused).cloned() else {
                return SessionResult::default();
            };
            self.segment_edit_state = Some(SegmentEditState {
                index: session.focused,
                original_segment,
                replace_next_printable: true,
            });
        }
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub(crate) fn handle_segment_edit_printable(&mut self, ch: char) -> SessionResult {
        let Some(edit_state) = self.segment_edit_state.as_ref() else {
            return SessionResult::default();
        };
        let Some(session) = &self.segmented_session else {
            self.segment_edit_state = None;
            return SessionResult::default();
        };
        let Some(segment) = session.segments.get(edit_state.index) else {
            self.segment_edit_state = None;
            return SessionResult::default();
        };

        let mut input = if edit_state.replace_next_printable {
            String::new()
        } else {
            segment.input.clone()
        };
        input.push(ch);
        self.replace_segment_input(edit_state.index, input);
        self.selection_touched = true;
        if let Some(edit_state) = &mut self.segment_edit_state {
            edit_state.replace_next_printable = false;
        }
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    pub(crate) fn handle_segment_edit_backspace(&mut self) -> SessionResult {
        let Some(edit_state) = self.segment_edit_state.as_ref() else {
            return SessionResult::default();
        };
        let Some(session) = &self.segmented_session else {
            self.segment_edit_state = None;
            return SessionResult::default();
        };
        let Some(segment) = session.segments.get(edit_state.index) else {
            self.segment_edit_state = None;
            return SessionResult::default();
        };
        if segment.input.is_empty() {
            return self.handle_empty_segment_edit_backspace(edit_state.index);
        }

        let mut input = segment.input.clone();
        input.pop();
        self.replace_segment_input(edit_state.index, input);
        self.selection_touched = true;
        if let Some(edit_state) = &mut self.segment_edit_state {
            edit_state.replace_next_printable = false;
        }
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }

    fn handle_empty_segment_edit_backspace(&mut self, index: usize) -> SessionResult {
        if index == 0 {
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        let Some(session) = &mut self.segmented_session else {
            self.segment_edit_state = None;
            return SessionResult::default();
        };
        if index >= session.segments.len() {
            self.segment_edit_state = None;
            return SessionResult::default();
        }

        session.segments.remove(index);
        self.composition_raw = recompute_segment_ranges_and_raw(session);
        session.raw_input = self.composition_raw.clone();

        if session.segments.len() <= 1 {
            self.segmented_session = None;
            self.segment_edit_state = None;
            self.recompute_composition_state();
            return SessionResult {
                consumed: true,
                ..SessionResult::default()
            };
        }

        let next_index = index - 1;
        session.focused = next_index;
        let original_segment = session.segments[next_index].clone();
        self.segment_edit_state = Some(SegmentEditState {
            index: next_index,
            original_segment,
            replace_next_printable: true,
        });
        SessionResult {
            consumed: true,
            ..SessionResult::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{session, type_ascii};

    #[test]
    fn tab_in_segmented_session_enters_segment_edit_mode_on_focused_segment() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let before = session.snapshot();
        assert!(before.segmented_active);
        assert_eq!(before.focused_segment_index, Some(0));
        assert!(!before.segment_edit_active);
        assert_eq!(before.segment_edit_index, None);

        let tab = session.process_key_event(0xFF09, 0, 0);

        assert!(tab.consumed);
        let snapshot = session.snapshot();
        assert!(snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_edit_index, before.focused_segment_index);
    }

    #[test]
    fn tab_in_segment_edit_mode_exits_and_re_pins_segment() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let down = session.process_key_event(0xFF54, 0, 0);
        assert!(down.consumed);
        let selected_output = session.snapshot().segment_preview[0].output.clone();

        let enter_edit = session.process_key_event(0xFF09, 0, 0);
        assert!(enter_edit.consumed);
        assert!(session.snapshot().segment_edit_active);

        let exit_edit = session.process_key_event(0xFF09, 0, 0);

        assert!(exit_edit.consumed);
        let snapshot = session.snapshot();
        assert!(!snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_edit_index, None);
        assert_eq!(snapshot.segment_preview[0].output, selected_output);
    }

    #[test]
    fn escape_in_segment_edit_mode_cancels_and_restores_original_segment() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let original = session.snapshot().segment_preview[0].clone();

        let enter_edit = session.process_key_event(0xFF09, 0, 0);
        assert!(enter_edit.consumed);
        let s = session.process_key_event('s' as u32, 0, 0);
        assert!(s.consumed);
        assert_ne!(session.snapshot().segment_preview[0], original);

        let escape = session.process_key_event(0xFF1B, 0, 0);

        assert!(escape.consumed);
        let snapshot = session.snapshot();
        assert!(snapshot.segmented_active);
        assert!(!snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_edit_index, None);
        assert_eq!(snapshot.segment_preview[0], original);
        assert_eq!(snapshot.raw_preedit, "khnhomtov");
    }

    #[test]
    fn tab_is_inert_outside_segmented_session() {
        let mut session = session();
        type_ascii(&mut session, "jea");
        let before = session.snapshot();
        assert!(!before.segmented_active);
        assert!(!before.segment_edit_active);

        let tab = session.process_key_event(0xFF09, 0, 0);

        assert!(!tab.consumed);
        assert_eq!(session.snapshot(), before);
    }

    #[test]
    fn first_printable_in_segment_edit_mode_replaces_segment_roman() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let before = session.snapshot();
        let sibling = before.segment_preview[1].clone();

        let tab = session.process_key_event(0xFF09, 0, 0);
        assert!(tab.consumed);
        let s = session.process_key_event('s' as u32, 0, 0);

        assert!(s.consumed);
        let snapshot = session.snapshot();
        assert!(snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_preview.len(), before.segment_preview.len());
        assert_eq!(snapshot.segment_preview[0].input, "s");
        assert_eq!(snapshot.segment_preview[1], sibling);

        let o = session.process_key_event('o' as u32, 0, 0);

        assert!(o.consumed);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.segment_preview[0].input, "so");
        assert_eq!(snapshot.segment_preview[1], sibling);
    }

    #[test]
    fn backspace_in_segment_edit_mode_deletes_one_char_at_a_time() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let tab = session.process_key_event(0xFF09, 0, 0);
        assert!(tab.consumed);
        type_ascii(&mut session, "tver");
        assert_eq!(session.snapshot().segment_preview[0].input, "tver");

        let backspace = session.process_key_event(0xFF08, 0, 0);

        assert!(backspace.consumed);
        assert_eq!(session.snapshot().segment_preview[0].input, "tve");

        let backspace = session.process_key_event(0xFF08, 0, 0);

        assert!(backspace.consumed);
        assert_eq!(session.snapshot().segment_preview[0].input, "tv");
    }

    #[test]
    fn backspace_on_empty_in_edit_segment_transfers_mode_to_previous_segment() {
        let mut transfer = session();
        type_ascii(&mut transfer, "khnhomtovkhnhom");
        assert_eq!(transfer.snapshot().segment_preview.len(), 3);
        transfer.process_key_event(0xFF53, 0, 0);
        transfer.process_key_event(0xFF53, 0, 0);
        assert_eq!(transfer.snapshot().focused_segment_index, Some(2));
        transfer.process_key_event(0xFF09, 0, 0);

        for _ in 0.."khnhom".len() {
            let backspace = transfer.process_key_event(0xFF08, 0, 0);
            assert!(backspace.consumed);
        }
        assert_eq!(transfer.snapshot().segment_preview[2].input, "");
        let remove_empty = transfer.process_key_event(0xFF08, 0, 0);

        assert!(remove_empty.consumed);
        let snapshot = transfer.snapshot();
        assert!(snapshot.segmented_active);
        assert!(snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_edit_index, Some(1));
        assert_eq!(snapshot.focused_segment_index, Some(1));
        assert_eq!(snapshot.segment_preview.len(), 2);

        let mut first_segment = session();
        type_ascii(&mut first_segment, "khnhomtov");
        first_segment.process_key_event(0xFF09, 0, 0);
        for _ in 0.."khnhom".len() {
            first_segment.process_key_event(0xFF08, 0, 0);
        }
        let before = first_segment.snapshot();
        assert_eq!(before.segment_edit_index, Some(0));
        assert_eq!(before.segment_preview[0].input, "");
        let no_op = first_segment.process_key_event(0xFF08, 0, 0);
        assert!(no_op.consumed);
        assert_eq!(first_segment.snapshot(), before);

        let mut dissolving = session();
        type_ascii(&mut dissolving, "khnhomtov");
        dissolving.process_key_event(0xFF53, 0, 0);
        dissolving.process_key_event(0xFF09, 0, 0);
        for _ in 0.."tov".len() {
            dissolving.process_key_event(0xFF08, 0, 0);
        }
        let remove_empty = dissolving.process_key_event(0xFF08, 0, 0);
        assert!(remove_empty.consumed);
        let snapshot = dissolving.snapshot();
        assert!(!snapshot.segmented_active);
        assert!(!snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_edit_index, None);
    }

    #[test]
    fn left_right_auto_exit_segment_edit_mode_and_navigate() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        session.process_key_event(0xFF09, 0, 0);
        assert!(session.snapshot().segment_edit_active);

        let right = session.process_key_event(0xFF53, 0, 0);

        assert!(right.consumed);
        let snapshot = session.snapshot();
        assert!(!snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_edit_index, None);
        assert_eq!(snapshot.focused_segment_index, Some(1));
    }

    #[test]
    fn digit_in_segment_edit_mode_selects_candidate_without_committing_whole_composition() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let snapshot = session.snapshot();
        assert!(snapshot.candidates.len() >= 2);
        assert_eq!(snapshot.candidates[1], "ខ្ញំ");

        session.process_key_event(0xFF09, 0, 0);
        let digit = session.process_key_event('2' as u32, 0, 0);

        assert!(digit.consumed);
        assert_eq!(digit.commit_text, None);
        let snapshot = session.snapshot();
        assert!(snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_preview[0].output, "ខ្ញំ");

        let enter = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(enter.commit_text.as_deref(), Some("ខ្ញំទៅ"));
    }

    #[test]
    fn space_in_segment_edit_mode_commits_whole_composition() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        session.process_key_event(0xFF09, 0, 0);

        let space = session.process_key_event(0x20, 0, 0);

        assert!(space.consumed);
        assert_eq!(space.commit_text.as_deref(), Some("ខ្ញុំទៅ"));
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn enter_in_segment_edit_mode_commits_whole_composition() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        session.process_key_event(0xFF09, 0, 0);

        let enter = session.process_key_event(0xFF0D, 0, 0);

        assert!(enter.consumed);
        assert_eq!(enter.commit_text.as_deref(), Some("ខ្ញុំទៅ"));
        assert!(session.snapshot().preedit.is_empty());
    }

    #[test]
    fn zero_candidate_in_edit_roman_commits_literal_roman_on_exit() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        session.process_key_event(0xFF09, 0, 0);
        type_ascii(&mut session, "xqz");

        let tab = session.process_key_event(0xFF09, 0, 0);

        assert!(tab.consumed);
        let snapshot = session.snapshot();
        assert!(!snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_preview[0].input, "xqz");
        assert_eq!(snapshot.segment_preview[0].output, "xqz");

        let enter = session.process_key_event(0xFF0D, 0, 0);
        assert_eq!(enter.commit_text.as_deref(), Some("xqzទៅ"));
    }

    #[test]
    fn decoder_runs_flat_inside_segment_edit_mode_no_internal_resegmentation() {
        let mut session = session();
        type_ascii(&mut session, "khnhomtov");
        let before_len = session.snapshot().segment_preview.len();
        assert!(before_len >= 2);
        session.process_key_event(0xFF09, 0, 0);

        type_ascii(&mut session, "khnhomtov");

        let snapshot = session.snapshot();
        assert!(snapshot.segment_edit_active);
        assert_eq!(snapshot.segment_preview.len(), before_len);
        assert_eq!(snapshot.segment_preview[0].input, "khnhomtov");
    }
}
