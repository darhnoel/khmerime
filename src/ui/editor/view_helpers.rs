use dioxus::prelude::*;

use crate::ui::platform::{current_editor_caret, editor_composition_mark, editor_popup_position};
use crate::{CompositionMark, SuggestionPopup, FALLBACK_POPUP_LEFT, FALLBACK_POPUP_TOP};

use super::SegmentedSession;

fn default_popup_position() -> SuggestionPopup {
    SuggestionPopup {
        left: FALLBACK_POPUP_LEFT,
        top: FALLBACK_POPUP_TOP,
    }
}

pub(super) async fn suggestion_popup_position(caret: usize) -> Option<SuggestionPopup> {
    Some(
        editor_popup_position(caret)
            .await
            .unwrap_or_else(default_popup_position),
    )
}

pub(super) async fn candidate_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    editor_composition_mark(start, token).await
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
    matches!(key, "Backspace" | "Delete" | "ArrowLeft" | "ArrowRight" | "Escape")
}

pub(crate) fn is_space_key(key: &str) -> bool {
    matches!(key, " " | "Space" | "Spacebar")
}
