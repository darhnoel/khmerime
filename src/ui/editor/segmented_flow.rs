use std::collections::HashMap;

use crate::engine;
use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation, Transliterator};

use super::candidate_pipeline::{connect_khmer_display, normalize_visible_suggestions};
use super::{char_len, slice_chars, EditorSignals, SegmentedChoice, SegmentedSession};

pub(crate) fn move_segment_focus(delta: isize, mut state: EditorSignals) -> bool {
    let Some(mut session) = state.segmented_session() else {
        return false;
    };
    if session.segments.len() <= 1 {
        return false;
    }
    let len = session.segments.len() as isize;
    let next = (session.focused as isize + delta).clamp(0, len - 1) as usize;
    if next == session.focused {
        if state.segmented_refine_mode() {
            return false;
        }
        state.segmented_refine_mode.set(true);
        state.suggestions.set(session.focused_candidates());
        state.recommended_indices.set(Vec::new());
        state.roman_variant_hints.set(HashMap::new());
        state.selected.set(session.focused_selected());
        state.selection_started.set(true);
        state.segmented_session.set(Some(session));
        return true;
    }
    session.focused = next;
    state.segmented_refine_mode.set(true);
    state.suggestions.set(session.focused_candidates());
    state.recommended_indices.set(Vec::new());
    state.roman_variant_hints.set(HashMap::new());
    state.selected.set(session.focused_selected());
    state.selection_started.set(true);
    state.segmented_session.set(Some(session));
    true
}

pub(crate) fn select_segment_candidate(candidate_index: usize, mut state: EditorSignals) -> bool {
    let Some(mut session) = state.segmented_session() else {
        return false;
    };
    let focused = session.focused;
    let Some(segment) = session.segments.get(focused) else {
        return false;
    };
    if candidate_index >= segment.candidates.len() {
        return false;
    }
    session.segments[focused].selected = candidate_index;
    if let Some(reflowed) = reflow_segmented_session_from_selection(&session, &state.history()) {
        let focused_candidates = reflowed.focused_candidates();
        let focused_selected = reflowed.focused_selected();
        state.segmented_refine_mode.set(true);
        state.suggestions.set(focused_candidates);
        state.recommended_indices.set(Vec::new());
        state.roman_variant_hints.set(HashMap::new());
        state.selected.set(focused_selected);
        state.selection_started.set(true);
        state.segmented_session.set(Some(reflowed));
        return true;
    }
    let focused_candidates = session
        .segments
        .get(focused)
        .map(|segment| segment.candidates.clone())
        .unwrap_or_default();
    state.segmented_refine_mode.set(true);
    state.suggestions.set(focused_candidates);
    state.recommended_indices.set(Vec::new());
    state.roman_variant_hints.set(HashMap::new());
    state.selected.set(candidate_index);
    state.selection_started.set(true);
    state.segmented_session.set(Some(session));
    true
}

pub(super) fn build_segmented_session(
    observation: &ShadowObservation,
    raw_input: &str,
    history: &HashMap<String, usize>,
) -> Option<SegmentedSession> {
    build_segmented_session_from_pairs(raw_input, observation_segment_pairs(observation), history, 0)
}

#[derive(Clone, Copy)]
struct SegmentChoiceContext<'a> {
    legacy: &'static Transliterator,
    history: &'a HashMap<String, usize>,
}

fn build_segment_choice(
    input: String,
    output: Option<String>,
    start: usize,
    ctx: SegmentChoiceContext<'_>,
) -> SegmentedChoice {
    let mut candidates = normalize_visible_suggestions(ctx.legacy.suggest(&input, ctx.history));
    if let Some(output) = output.map(|item| connect_khmer_display(&item)) {
        if let Some(position) = candidates.iter().position(|candidate| candidate == &output) {
            if position != 0 {
                let preferred = candidates.remove(position);
                candidates.insert(0, preferred);
            }
        } else {
            candidates.insert(0, output);
        }
    } else if candidates.is_empty() {
        candidates.push(input.clone());
    }
    candidates.truncate(10);

    SegmentedChoice {
        end: start + char_len(&input),
        input,
        start,
        candidates,
        selected: 0,
    }
}

fn observation_segment_pairs(observation: &ShadowObservation) -> Vec<(String, String)> {
    if !observation.wfst_top_segment_details.is_empty() {
        observation
            .wfst_top_segment_details
            .iter()
            .map(|segment| (segment.input.clone(), segment.output.clone()))
            .collect::<Vec<_>>()
    } else {
        observation
            .wfst_top_segments
            .iter()
            .filter_map(|segment| segment.split_once("=>"))
            .map(|(input, output)| (input.to_owned(), output.to_owned()))
            .collect::<Vec<_>>()
    }
}

fn build_segmented_session_from_pairs(
    raw_input: &str,
    pairs: Vec<(String, String)>,
    history: &HashMap<String, usize>,
    base_offset: usize,
) -> Option<SegmentedSession> {
    if pairs.len() < 2 {
        return None;
    }

    let ctx = SegmentChoiceContext {
        legacy: engine(DecoderMode::Legacy),
        history,
    };
    let mut cursor = base_offset;
    let segments = pairs
        .into_iter()
        .map(|(input, output)| {
            let start = cursor;
            cursor += char_len(&input);
            build_segment_choice(input, Some(output), start, ctx)
        })
        .collect::<Vec<_>>();

    Some(SegmentedSession {
        raw_input: raw_input.to_owned(),
        segments,
        focused: 0,
    })
}

pub(super) fn reflow_segmented_session_from_selection(
    session: &SegmentedSession,
    history: &HashMap<String, usize>,
) -> Option<SegmentedSession> {
    let focused = session.focused;
    let segment = session.segments.get(focused)?;
    let chosen = segment.selected_text();
    let tail = slice_chars(&session.raw_input, segment.start..char_len(&session.raw_input));
    let consumed = engine(DecoderMode::Legacy).best_prefix_consumption(&tail, &chosen)?;
    let consumed_len = char_len(&consumed);
    if consumed_len == 0 || consumed_len == segment.end.saturating_sub(segment.start) {
        return None;
    }

    let ctx = SegmentChoiceContext {
        legacy: engine(DecoderMode::Legacy),
        history,
    };
    let mut segments = session.segments[..focused].to_vec();
    segments.push(build_segment_choice(consumed.clone(), Some(chosen), segment.start, ctx));

    let tail_start = segment.start + consumed_len;
    let total_len = char_len(&session.raw_input);
    if tail_start < total_len {
        let remaining_tail = slice_chars(&session.raw_input, tail_start..total_len);
        let observation = engine(DecoderMode::Shadow).shadow_observation(&remaining_tail, history);
        if let Some(mut tail_session) = build_segmented_session_from_pairs(
            &session.raw_input,
            observation_segment_pairs(&observation),
            history,
            tail_start,
        ) {
            segments.append(&mut tail_session.segments);
        } else {
            segments.push(build_segment_choice(remaining_tail, None, tail_start, ctx));
        }
    }

    let focused = focused.min(segments.len().saturating_sub(1));
    Some(SegmentedSession {
        raw_input: session.raw_input.clone(),
        segments,
        focused,
    })
}
