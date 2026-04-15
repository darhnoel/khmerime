use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation, Transliterator};

use crate::ui::platform::{current_editor_caret, editor_composition_mark, editor_popup_position};
use crate::ui::storage::{save_editor_text, save_history};
use crate::{engine, CompositionMark, SuggestionPopup, FALLBACK_POPUP_LEFT, FALLBACK_POPUP_TOP};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentedChoice {
    pub input: String,
    pub start: usize,
    pub end: usize,
    pub candidates: Vec<String>,
    pub selected: usize,
}

impl SegmentedChoice {
    pub(crate) fn selected_text(&self) -> String {
        self.candidates
            .get(self.selected)
            .cloned()
            .or_else(|| self.candidates.first().cloned())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentedSession {
    pub raw_input: String,
    pub segments: Vec<SegmentedChoice>,
    pub focused: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct EditorSignals {
    pub text: Signal<String>,
    pub roman_enabled: Signal<bool>,
    pub decoder_mode: Signal<DecoderMode>,
    pub engine_ready: Signal<bool>,
    pub suggestions: Signal<Vec<String>>,
    pub popup: Signal<Option<SuggestionPopup>>,
    pub composition: Signal<Option<CompositionMark>>,
    pub shadow_debug: Signal<Option<ShadowObservation>>,
    pub segmented_session: Signal<Option<SegmentedSession>>,
    pub segmented_refine_mode: Signal<bool>,
    pub active_token: Signal<String>,
    pub recommended_indices: Signal<Vec<usize>>,
    pub number_pick_mode: Signal<bool>,
    pub selection_started: Signal<bool>,
    pub selected: Signal<usize>,
    pub pending_caret: Signal<Option<usize>>,
    pub history: Signal<HashMap<String, usize>>,
}

impl EditorSignals {
    pub(crate) fn text(self) -> String {
        (self.text)()
    }

    pub(crate) fn roman_enabled(self) -> bool {
        (self.roman_enabled)()
    }

    pub(crate) fn decoder_mode(self) -> DecoderMode {
        (self.decoder_mode)()
    }

    pub(crate) fn engine_ready(self) -> bool {
        (self.engine_ready)()
    }

    pub(crate) fn suggestions(self) -> Vec<String> {
        (self.suggestions)()
    }

    pub(crate) fn popup(self) -> Option<SuggestionPopup> {
        (self.popup)()
    }

    pub(crate) fn composition(self) -> Option<CompositionMark> {
        (self.composition)()
    }

    pub(crate) fn shadow_debug(self) -> Option<ShadowObservation> {
        (self.shadow_debug)()
    }

    pub(crate) fn segmented_session(self) -> Option<SegmentedSession> {
        (self.segmented_session)()
    }

    pub(crate) fn segmented_refine_mode(self) -> bool {
        (self.segmented_refine_mode)()
    }

    pub(crate) fn active_token(self) -> String {
        (self.active_token)()
    }

    pub(crate) fn number_pick_mode(self) -> bool {
        (self.number_pick_mode)()
    }

    pub(crate) fn recommended_indices(self) -> Vec<usize> {
        (self.recommended_indices)()
    }

    pub(crate) fn selection_started(self) -> bool {
        (self.selection_started)()
    }

    pub(crate) fn selected(self) -> usize {
        (self.selected)()
    }

    pub(crate) fn history(self) -> HashMap<String, usize> {
        (self.history)()
    }

    pub(crate) fn clear_candidate_state(mut self) {
        self.suggestions.set(Vec::new());
        self.popup.set(None);
        self.composition.set(None);
        self.shadow_debug.set(None);
        self.segmented_session.set(None);
        self.segmented_refine_mode.set(false);
        self.active_token.set(String::new());
        self.recommended_indices.set(Vec::new());
        self.selection_started.set(false);
        self.selected.set(0);
    }

    pub(crate) fn clear_candidate_state_and_picker(mut self) {
        self.clear_candidate_state();
        self.number_pick_mode.set(false);
    }
}

impl SegmentedSession {
    pub(crate) fn focused_candidates(&self) -> Vec<String> {
        self.segments
            .get(self.focused)
            .map(|segment| segment.candidates.clone())
            .unwrap_or_default()
    }

    pub(crate) fn focused_selected(&self) -> usize {
        self.segments
            .get(self.focused)
            .map(|segment| segment.selected)
            .unwrap_or(0)
    }

    pub(crate) fn composed_text(&self) -> String {
        self.segments
            .iter()
            .map(SegmentedChoice::selected_text)
            .collect::<String>()
    }
}

fn slice_chars(input: &str, range: std::ops::Range<usize>) -> String {
    input
        .chars()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .collect()
}

fn char_len(input: &str) -> usize {
    input.chars().count()
}

pub(crate) async fn update_candidates(value: String, mut state: EditorSignals) {
    if !state.roman_enabled() {
        state.clear_candidate_state_and_picker();
        return;
    }

    let live_text = state.text;
    let caret = current_editor_caret().await.unwrap_or_else(|| value.chars().count());
    if live_text() != value {
        return;
    }

    let bounds = Transliterator::token_bounds(&value, caret, false);
    let token = slice_chars(&value, bounds.clone());
    if token.trim().is_empty() {
        state.clear_candidate_state_and_picker();
        return;
    }

    if !state.engine_ready() {
        state.clear_candidate_state_and_picker();
        state.active_token.set(token);
        return;
    }

    let history_snapshot = state.history();
    let legacy = engine(DecoderMode::Legacy);
    let legacy_items = legacy.suggest(&token, &history_snapshot);
    if live_text() != value {
        return;
    }
    let items = if state.decoder_mode() == DecoderMode::Shadow && token.chars().count() >= 3 {
        let observation = engine(DecoderMode::Shadow).shadow_observation(&token, &history_snapshot);
        if live_text() != value {
            return;
        }
        let next_segmented = build_segmented_session(&observation, &token, &history_snapshot);
        let visible = choose_visible_suggestions(
            &legacy_items,
            &observation,
            next_segmented.as_ref(),
            state.segmented_refine_mode(),
        );
        state.shadow_debug.set(Some(observation));
        state.segmented_session.set(next_segmented);
        state.segmented_refine_mode.set(false);
        visible
    } else {
        state.shadow_debug.set(None);
        state.segmented_session.set(None);
        state.segmented_refine_mode.set(false);
        legacy_items
    };
    let exact_keys = legacy
        .exact_match_targets(&token)
        .into_iter()
        .map(|item| normalized_suggestion_key(&item))
        .collect::<HashSet<_>>();
    let recommended_indices = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| exact_keys.contains(&normalized_suggestion_key(item)).then_some(index))
        .collect::<Vec<_>>();
    let preserve_selection = state.active_token() == token && !state.suggestions().is_empty();
    let popup_position = if items.is_empty() {
        None
    } else {
        suggestion_popup_position(caret).await
    };
    if live_text() != value {
        return;
    }
    let composition_mark = candidate_composition_mark(bounds.start, &token).await;
    if live_text() != value {
        return;
    }
    state.popup.set(popup_position);
    state.composition.set(composition_mark);
    state.active_token.set(token.clone());
    state.recommended_indices.set(recommended_indices);
    if !preserve_selection {
        state.number_pick_mode.set(false);
        state.selection_started.set(false);
        state.selected.set(0);
    } else if items.is_empty() {
        state.number_pick_mode.set(false);
        state.selection_started.set(false);
        state.selected.set(0);
    } else if state.selected() >= items.len() {
        state.selected.set(items.len().saturating_sub(1));
    }
    state.suggestions.set(items);
}

fn default_popup_position() -> SuggestionPopup {
    SuggestionPopup {
        left: FALLBACK_POPUP_LEFT,
        top: FALLBACK_POPUP_TOP,
    }
}

async fn suggestion_popup_position(caret: usize) -> Option<SuggestionPopup> {
    Some(
        editor_popup_position(caret)
            .await
            .unwrap_or_else(default_popup_position),
    )
}

async fn candidate_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    editor_composition_mark(start, token).await
}

fn choose_visible_suggestions(
    legacy_items: &[String],
    observation: &ShadowObservation,
    segmented_session: Option<&SegmentedSession>,
    segmented_refine_mode: bool,
) -> Vec<String> {
    if segmented_refine_mode {
        if let Some(session) = segmented_session {
            return normalize_visible_suggestions(session.focused_candidates());
        }
    }
    if !observation.wfst_top5.is_empty() {
        normalize_visible_suggestions(merge_suggestion_lists(&observation.wfst_top5, legacy_items, 10))
    } else if let Some(session) = segmented_session {
        normalize_visible_suggestions(session.focused_candidates())
    } else {
        normalize_visible_suggestions(legacy_items.to_vec())
    }
}

fn normalized_suggestion_key(item: &str) -> String {
    item.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn connect_khmer_display(item: &str) -> String {
    let parts = item.split_whitespace().collect::<Vec<_>>();
    if parts.len() <= 1 {
        return item.to_owned();
    }
    if parts.iter().all(|part| part.chars().any(is_khmer_char)) {
        parts.concat()
    } else {
        item.to_owned()
    }
}

fn is_khmer_char(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch) || ('\u{19e0}'..='\u{19ff}').contains(&ch)
}

fn normalize_visible_suggestions(items: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for item in items {
        let display = connect_khmer_display(&item);
        let key = normalized_suggestion_key(&display);
        if seen.insert(key) {
            normalized.push(display);
        }
    }

    normalized
}

fn merge_suggestion_lists(primary: &[String], fallback: &[String], limit: usize) -> Vec<String> {
    let mut merged = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for item in primary.iter().chain(fallback.iter()) {
        let key = normalized_suggestion_key(item);
        if seen.insert(key) {
            merged.push(item.clone());
            if merged.len() >= limit {
                break;
            }
        }
    }

    merged
}

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
        state.selected.set(session.focused_selected());
        state.selection_started.set(true);
        state.segmented_session.set(Some(session));
        return true;
    }
    session.focused = next;
    state.segmented_refine_mode.set(true);
    state.suggestions.set(session.focused_candidates());
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
    state.selected.set(candidate_index);
    state.selection_started.set(true);
    state.segmented_session.set(Some(session));
    true
}

pub(crate) async fn commit_segmented_selection(typed_space: bool, mut state: EditorSignals) {
    let Some(session) = state.segmented_session() else {
        return;
    };
    let choice = session.composed_text();
    if choice.is_empty() {
        return;
    }
    let current_text = state.text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = state.history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    state.history.set(next_history);

    save_editor_text(&applied.text);
    state.text.set(applied.text);
    state.clear_candidate_state();
    state.pending_caret.set(Some(applied.caret));
}

fn build_segmented_session(
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

fn reflow_segmented_session_from_selection(
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

    let focused = if focused + 1 < segments.len() {
        focused + 1
    } else {
        focused.min(segments.len().saturating_sub(1))
    };
    Some(SegmentedSession {
        raw_input: session.raw_input.clone(),
        segments,
        focused,
    })
}

pub(crate) async fn commit_selection(typed_space: bool, mut state: EditorSignals) {
    let items = state.suggestions();
    if items.is_empty() {
        return;
    }
    let Some(choice) = items.get(state.selected()).cloned() else {
        return;
    };
    let current_text = state.text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = state.history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    state.history.set(next_history);

    save_editor_text(&applied.text);
    state.text.set(applied.text);
    state.clear_candidate_state();
    state.pending_caret.set(Some(applied.caret));
}

pub(crate) async fn refresh_popup_position(mut popup: Signal<Option<SuggestionPopup>>) {
    let Some(caret) = current_editor_caret().await else {
        popup.set(None);
        return;
    };
    popup.set(suggestion_popup_position(caret).await);
}

pub(crate) fn popup_style(popup: Option<SuggestionPopup>) -> String {
    let Some(popup) = popup else {
        return "display:none;".to_owned();
    };
    format!("left:{:.1}px; top:{:.1}px;", popup.left, popup.top)
}

pub(crate) fn composition_style(mark: &CompositionMark, selection_started: bool) -> String {
    let top = mark.top + mark.height - 3.0;
    let opacity = if selection_started { 0.75 } else { 1.0 };
    format!(
        "left:{:.1}px; top:{:.1}px; width:{:.1}px; opacity:{:.2};",
        mark.left, top, mark.width, opacity
    )
}

pub(crate) fn composition_preview_style(mark: &CompositionMark, font_size: usize) -> String {
    format!(
        "left:{:.1}px; top:{:.1}px; width:{:.1}px; height:{:.1}px; font-size:{}px;",
        mark.left, mark.top, mark.width, mark.height, font_size
    )
}

pub(crate) fn segmented_composition_preview_style(mark: &CompositionMark, font_size: usize) -> String {
    format!(
        "left:{:.1}px; top:{:.1}px; min-width:{:.1}px; min-height:{:.1}px; font-size:{}px;",
        mark.left, mark.top, mark.width, mark.height, font_size
    )
}

pub(crate) fn segmented_preview_parts(session: &SegmentedSession) -> (String, String, String) {
    let mut before = String::new();
    let mut focused = String::new();
    let mut after = String::new();

    for (index, segment) in session.segments.iter().enumerate() {
        let text = segment.selected_text();
        if index < session.focused {
            before.push_str(&text);
        } else if index == session.focused {
            focused = text;
        } else {
            after.push_str(&text);
        }
    }

    (before, focused, after)
}

pub(crate) fn shortcut_index(key: &str) -> Option<usize> {
    match key {
        "1" => Some(0),
        "2" => Some(1),
        "3" => Some(2),
        "4" => Some(3),
        "5" => Some(4),
        _ => None,
    }
}

pub(crate) fn shortcut_label(index: usize) -> String {
    ((index % crate::VISIBLE_SUGGESTIONS) + 1).to_string()
}

pub(crate) fn visible_page_start(selected: usize, total: usize) -> usize {
    if total <= crate::VISIBLE_SUGGESTIONS {
        0
    } else {
        (selected / crate::VISIBLE_SUGGESTIONS) * crate::VISIBLE_SUGGESTIONS
    }
}

pub(crate) fn should_exit_number_pick(key: &str) -> bool {
    matches!(key, "Backspace" | "Delete" | "ArrowLeft" | "ArrowRight" | "Escape") || key.chars().count() == 1
}

pub(crate) fn is_space_key(key: &str) -> bool {
    matches!(key, " " | "Space" | "Spacebar")
}

#[cfg(test)]
mod tests {
    use roman_lookup::{DecodeSegment, DecoderMode, ShadowMismatch, ShadowObservation};

    use super::{
        build_segmented_session, char_len, choose_visible_suggestions, connect_khmer_display,
        reflow_segmented_session_from_selection, slice_chars, SegmentedChoice, SegmentedSession,
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
}
