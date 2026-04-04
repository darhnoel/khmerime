use std::collections::HashMap;

use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation, Transliterator};

use crate::ui::platform::{current_editor_caret, editor_composition_mark, editor_popup_position};
use crate::ui::storage::{save_editor_text, save_history};
use crate::{engine, CompositionMark, SuggestionPopup, FALLBACK_POPUP_LEFT, FALLBACK_POPUP_TOP};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SegmentedChoice {
    pub input: String,
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
    pub segments: Vec<SegmentedChoice>,
    pub focused: usize,
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

pub(crate) async fn update_candidates(
    value: String,
    live_text: Signal<String>,
    roman_enabled: Signal<bool>,
    decoder_mode: DecoderMode,
    engine_ready: Signal<bool>,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut shadow_debug: Signal<Option<ShadowObservation>>,
    mut segmented_session: Signal<Option<SegmentedSession>>,
    mut segmented_refine_mode: Signal<bool>,
    mut active_token: Signal<String>,
    mut number_pick_mode: Signal<bool>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    history: Signal<HashMap<String, usize>>,
) {
    if !roman_enabled() {
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        segmented_session.set(None);
        segmented_refine_mode.set(false);
        active_token.set(String::new());
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    let caret = current_editor_caret().await.unwrap_or_else(|| value.chars().count());
    if live_text() != value {
        return;
    }

    let bounds = Transliterator::token_bounds(&value, caret, false);
    let token = slice_chars(&value, bounds.clone());
    if token.trim().is_empty() {
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        segmented_session.set(None);
        segmented_refine_mode.set(false);
        active_token.set(String::new());
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    if !engine_ready() {
        suggestions.set(Vec::new());
        popup.set(None);
        composition.set(None);
        shadow_debug.set(None);
        segmented_session.set(None);
        segmented_refine_mode.set(false);
        active_token.set(token);
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
        return;
    }

    let history_snapshot = history();
    let legacy = engine(DecoderMode::Legacy);
    let legacy_items = legacy.suggest(&token, &history_snapshot);
    if live_text() != value {
        return;
    }
    let items = if decoder_mode == DecoderMode::Shadow {
        let observation = engine(DecoderMode::Shadow).shadow_observation(&token, &history_snapshot);
        if live_text() != value {
            return;
        }
        let next_segmented = build_segmented_session(&observation, &history_snapshot);
        let visible = choose_visible_suggestions(
            &legacy_items,
            &observation,
            next_segmented.as_ref(),
            segmented_refine_mode(),
        );
        shadow_debug.set(Some(observation));
        segmented_session.set(next_segmented);
        segmented_refine_mode.set(false);
        visible
    } else {
        shadow_debug.set(None);
        segmented_session.set(None);
        segmented_refine_mode.set(false);
        legacy_items
    };
    let preserve_selection = active_token() == token && !suggestions().is_empty();
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
    popup.set(popup_position);
    composition.set(composition_mark);
    active_token.set(token.clone());
    if !preserve_selection {
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
    } else if items.is_empty() {
        number_pick_mode.set(false);
        selection_started.set(false);
        selected.set(0);
    } else if selected() >= items.len() {
        selected.set(items.len().saturating_sub(1));
    }
    suggestions.set(items);
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
            return session.focused_candidates();
        }
    }
    if !observation.wfst_top5.is_empty() {
        merge_suggestion_lists(&observation.wfst_top5, legacy_items, 10)
    } else if let Some(session) = segmented_session {
        session.focused_candidates()
    } else {
        legacy_items.to_vec()
    }
}

fn normalized_suggestion_key(item: &str) -> String {
    item.chars().filter(|ch| !ch.is_whitespace()).collect()
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

pub(crate) fn move_segment_focus(
    delta: isize,
    mut segmented_session: Signal<Option<SegmentedSession>>,
    mut segmented_refine_mode: Signal<bool>,
    mut suggestions: Signal<Vec<String>>,
    mut selected: Signal<usize>,
    mut selection_started: Signal<bool>,
) -> bool {
    let Some(mut session) = segmented_session() else {
        return false;
    };
    if session.segments.len() <= 1 {
        return false;
    }
    let len = session.segments.len() as isize;
    let next = (session.focused as isize + delta).clamp(0, len - 1) as usize;
    if next == session.focused {
        if segmented_refine_mode() {
            return false;
        }
        segmented_refine_mode.set(true);
        suggestions.set(session.focused_candidates());
        selected.set(session.focused_selected());
        selection_started.set(true);
        segmented_session.set(Some(session));
        return true;
    }
    session.focused = next;
    segmented_refine_mode.set(true);
    suggestions.set(session.focused_candidates());
    selected.set(session.focused_selected());
    selection_started.set(true);
    segmented_session.set(Some(session));
    true
}

pub(crate) fn select_segment_candidate(
    candidate_index: usize,
    mut segmented_session: Signal<Option<SegmentedSession>>,
    mut segmented_refine_mode: Signal<bool>,
    mut suggestions: Signal<Vec<String>>,
    mut selected: Signal<usize>,
    mut selection_started: Signal<bool>,
) -> bool {
    let Some(mut session) = segmented_session() else {
        return false;
    };
    let Some(segment) = session.segments.get_mut(session.focused) else {
        return false;
    };
    if candidate_index >= segment.candidates.len() {
        return false;
    }
    segment.selected = candidate_index;
    segmented_refine_mode.set(true);
    suggestions.set(segment.candidates.clone());
    selected.set(candidate_index);
    selection_started.set(true);
    segmented_session.set(Some(session));
    true
}

pub(crate) async fn commit_segmented_selection(
    typed_space: bool,
    mut text: Signal<String>,
    mut segmented_session: Signal<Option<SegmentedSession>>,
    mut segmented_refine_mode: Signal<bool>,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut shadow_debug: Signal<Option<ShadowObservation>>,
    mut active_token: Signal<String>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    mut pending_caret: Signal<Option<usize>>,
    mut history: Signal<HashMap<String, usize>>,
) {
    let Some(session) = segmented_session() else {
        return;
    };
    let choice = session.composed_text();
    if choice.is_empty() {
        return;
    }
    let current_text = text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    history.set(next_history);

    save_editor_text(&applied.text);
    text.set(applied.text);
    suggestions.set(Vec::new());
    popup.set(None);
    composition.set(None);
    shadow_debug.set(None);
    segmented_session.set(None);
    segmented_refine_mode.set(false);
    active_token.set(String::new());
    selection_started.set(false);
    selected.set(0);
    pending_caret.set(Some(applied.caret));
}

fn build_segmented_session(
    observation: &ShadowObservation,
    history: &HashMap<String, usize>,
) -> Option<SegmentedSession> {
    let pairs = observation
        .wfst_top_segments
        .iter()
        .filter_map(|segment| segment.split_once("=>"))
        .map(|(input, output)| (input.to_owned(), output.to_owned()))
        .collect::<Vec<_>>();

    if pairs.len() < 2 {
        return None;
    }

    let legacy = engine(DecoderMode::Legacy);
    let segments = pairs
        .into_iter()
        .map(|(input, output)| {
            let mut candidates = legacy.suggest(&input, history);
            if let Some(position) = candidates.iter().position(|candidate| candidate == &output) {
                if position != 0 {
                    let preferred = candidates.remove(position);
                    candidates.insert(0, preferred);
                }
            } else {
                candidates.insert(0, output);
            }
            candidates.truncate(10);
            SegmentedChoice {
                input,
                candidates,
                selected: 0,
            }
        })
        .collect::<Vec<_>>();

    Some(SegmentedSession { segments, focused: 0 })
}

pub(crate) async fn commit_selection(
    typed_space: bool,
    mut text: Signal<String>,
    mut suggestions: Signal<Vec<String>>,
    mut popup: Signal<Option<SuggestionPopup>>,
    mut composition: Signal<Option<CompositionMark>>,
    mut active_token: Signal<String>,
    mut selection_started: Signal<bool>,
    mut selected: Signal<usize>,
    mut pending_caret: Signal<Option<usize>>,
    mut history: Signal<HashMap<String, usize>>,
) {
    let items = suggestions();
    if items.is_empty() {
        return;
    }
    let Some(choice) = items.get(selected()).cloned() else {
        return;
    };
    let current_text = text();
    let caret = current_editor_caret()
        .await
        .unwrap_or_else(|| current_text.chars().count());
    let applied = Transliterator::apply_suggestion(&current_text, caret, &choice, typed_space);

    let mut next_history = history();
    Transliterator::learn(&mut next_history, &choice);
    save_history(&next_history);
    history.set(next_history);

    save_editor_text(&applied.text);
    text.set(applied.text);
    suggestions.set(Vec::new());
    popup.set(None);
    composition.set(None);
    active_token.set(String::new());
    selection_started.set(false);
    selected.set(0);
    pending_caret.set(Some(applied.caret));
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
    use roman_lookup::{DecoderMode, ShadowMismatch, ShadowObservation};

    use super::{choose_visible_suggestions, SegmentedChoice, SegmentedSession};

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

    #[test]
    fn uses_segment_candidates_in_refine_mode() {
        let legacy = vec!["ខ្ញុំ ទៅ".to_owned()];
        let observation = sample_observation();
        assert_eq!(
            choose_visible_suggestions(
                &legacy,
                &observation,
                Some(&SegmentedSession {
                    segments: vec![
                        SegmentedChoice {
                            input: "khnhom".to_owned(),
                            candidates: vec!["ខ្ញុំ".to_owned()],
                            selected: 0,
                        },
                        SegmentedChoice {
                            input: "tov".to_owned(),
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
    fn merges_wfst_and_legacy_suggestions_when_available() {
        let legacy = vec!["ខ្ញុំ ទៅ".to_owned(), "ខ្ញមទៅ".to_owned()];
        let observation = sample_observation();
        assert_eq!(
            choose_visible_suggestions(&legacy, &observation, None, false),
            vec!["ខ្ញុំទៅ".to_owned(), "ខ្ញមទៅ".to_owned()]
        );
    }

    #[test]
    fn falls_back_to_legacy_suggestions_when_wfst_has_no_candidates() {
        let legacy = vec!["ខ្ញុំ ទៅ".to_owned()];
        let mut observation = sample_observation();
        observation.wfst_failure = Some("timeout".to_owned());
        observation.wfst_top5.clear();
        assert_eq!(choose_visible_suggestions(&legacy, &observation, None, false), legacy);
    }
}
