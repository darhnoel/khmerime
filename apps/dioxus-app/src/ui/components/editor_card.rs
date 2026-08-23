use dioxus::html::Modifiers;
use dioxus::prelude::*;
use roman_lookup::ManualComposeKind;

use crate::ui::editor::{
    click_candidate, commit_active_selection, composition_preview_style, composition_style, enter_segment_edit,
    exit_segment_edit, is_space_key, move_segment_focus, popup_style, select_segment_candidate, set_manual_kind_filter,
    shortcut_index, shortcut_label, should_exit_number_pick, skip_manual_roman_char, undo_manual_step,
    update_candidates, visible_page_start, CandidateLevel, CandidateMode, EditorSignals, InputMode,
};
use crate::ui::platform::move_editor_caret;
use crate::ui::storage::{save_editor_text, save_enabled};
use crate::{EDITOR_ID, VISIBLE_SUGGESTIONS};

#[cfg(test)]
mod tests {
    use super::space_cycle_enabled;
    use crate::ui::editor::CandidateMode;

    #[test]
    fn space_cycle_is_disabled_for_next_word_mode() {
        assert!(!space_cycle_enabled(true, CandidateMode::NextWord));
    }

    #[test]
    fn space_cycle_is_enabled_for_transliteration_mode() {
        assert!(space_cycle_enabled(true, CandidateMode::Transliteration));
    }
}

fn roman_hint_label(variants: &[String]) -> String {
    // Show all variants if there are 3 or fewer, otherwise show the first 3 followed by ellipsis.
    format!("{}", variants.join(" / "))
}

fn cycle_live_candidate(delta: isize, mut state: EditorSignals) -> bool {
    let len = if state.segmented_refine_mode() && state.segmented_session().is_some() {
        state
            .segmented_session()
            .map(|session| session.current_candidate_len())
            .unwrap_or(0)
    } else {
        state.suggestions().len()
    };
    if len == 0 {
        return false;
    }

    let next = if !state.selection_started() {
        if delta < 0 {
            len.saturating_sub(1)
        } else {
            0
        }
    } else if delta < 0 {
        (state.selected() + len - 1) % len
    } else {
        (state.selected() + 1) % len
    };

    let changed = if state.segmented_refine_mode() && state.segmented_session().is_some() {
        select_segment_candidate(next, state)
    } else {
        state.selected.set(next);
        state.selection_started.set(true);
        true
    };
    if changed {
        state.number_pick_mode.set(false);
    }
    changed
}

fn handle_horizontal_arrow(delta: isize, has_live_suggestions: bool, mut state: EditorSignals) -> bool {
    if state.candidate_level() == CandidateLevel::Phrase {
        // Phrase mode reserves horizontal arrows so they cannot move the
        // document caret into the active Roman composition (macOS ADR-0004).
        return true;
    }
    if state.candidate_level() == CandidateLevel::Segment {
        let _ = move_segment_focus(delta, state);
        state.number_pick_mode.set(false);
        // Consume even at the first/last segment; never turn the same arrow
        // into candidate cycling or leak it into the textarea.
        return true;
    }

    if has_live_suggestions && (state.selection_started() || state.number_pick_mode()) {
        return cycle_live_candidate(delta, state);
    }

    false
}

fn space_cycle_enabled(has_live_suggestions: bool, candidate_mode: CandidateMode) -> bool {
    has_live_suggestions && candidate_mode == CandidateMode::Transliteration
}

fn apply_shortcut_selection(
    key: &str,
    modifiers: Modifiers,
    live_suggestion_len: usize,
    mut state: EditorSignals,
) -> bool {
    if modifiers.contains(Modifiers::CONTROL)
        || modifiers.contains(Modifiers::ALT)
        || modifiers.contains(Modifiers::META)
    {
        return false;
    }

    let Some(offset) = shortcut_index(key) else {
        return false;
    };

    let page_start = visible_page_start(state.selected(), live_suggestion_len);
    let index = page_start + offset;
    if index >= live_suggestion_len {
        return false;
    }

    if state.segmented_refine_mode() && state.segmented_session().is_some() {
        select_segment_candidate(index, state);
    } else {
        state.selected.set(index);
        state.selection_started.set(true);
    }
    state.number_pick_mode.set(true);
    true
}

#[component]
pub(crate) fn EditorCard(state: EditorSignals, font_size: Signal<usize>) -> Element {
    let text_value = state.text();
    let suggestions = state.suggestions();
    let suggestion_total = suggestions.len();
    let page_start = visible_page_start(state.selected(), suggestion_total);
    let page_count = suggestion_total.saturating_add(VISIBLE_SUGGESTIONS - 1) / VISIBLE_SUGGESTIONS;
    let page_number = if page_count == 0 {
        0
    } else {
        page_start / VISIBLE_SUGGESTIONS + 1
    };
    let candidate_level = state.candidate_level();
    let phrase_candidates = state.phrase_candidates();
    let segment_session = if candidate_level == CandidateLevel::Segment {
        state.segmented_session()
    } else {
        None
    };
    let recommended_indices = state.recommended_indices();
    let roman_variant_hints = state.roman_variant_hints();
    let has_suggestions = !suggestions.is_empty();
    // Next-word predictions get their own docked bar below the editor (ADR:
    // docked prediction bar). They are pointer-only and never float at the caret.
    let is_next_word = state.candidate_mode() == CandidateMode::NextWord;
    let show_candidate_list = has_suggestions && !is_next_word;
    let next_word_predictions = if is_next_word { suggestions.clone() } else { Vec::new() };
    let manual_state = if state.input_mode() == InputMode::ManualCharacterTyping {
        state.manual_typing_state()
    } else {
        None
    };
    let manual_inline_preview = manual_state.as_ref().and_then(|manual| {
        if !manual.composed_text.is_empty() {
            Some(manual.composed_text.clone())
        } else if state.selection_started() {
            suggestions.get(state.selected()).cloned()
        } else {
            None
        }
    });
    let manual_consonant_count = manual_state
        .as_ref()
        .map(|manual| {
            manual
                .candidates
                .iter()
                .filter(|candidate| candidate.kind == ManualComposeKind::BaseConsonant)
                .count()
        })
        .unwrap_or(0);
    let manual_vowel_count = manual_state
        .as_ref()
        .map(|manual| {
            manual
                .candidates
                .iter()
                .filter(|candidate| candidate.kind == ManualComposeKind::Vowel)
                .count()
        })
        .unwrap_or(0);
    let manual_subscript_count = manual_state
        .as_ref()
        .map(|manual| {
            manual
                .candidates
                .iter()
                .filter(|candidate| candidate.kind == ManualComposeKind::Subscript)
                .count()
        })
        .unwrap_or(0);
    let manual_can_undo = manual_state
        .as_ref()
        .map(|manual| !manual.checkpoints.is_empty())
        .unwrap_or(false);
    rsx! {
        div { class: "editor-card",
            div { class: "editor-wrap",
                textarea {
                    id: EDITOR_ID,
                    "data-testid": "editor-input",
                    class: if state.composition().is_some() { "editor editor-composing" } else { "editor" },
                    style: "font-size: {font_size()}px;",
                    value: "{text_value}",
                    placeholder: "ចាប់ផ្ដើមសរសេរនៅទីនេះ…",
                    spellcheck: "false",
                    autocomplete: "off",
                    autocorrect: "off",
                    oninput: move |event| {
                        let value = event.value();
                        let current_text = state.text();
                        let live_suggestions = state.suggestions();
                        let manual_cycle_mode_active = state.input_mode() == InputMode::ManualCharacterTyping
                            && state.manual_typing_state().is_some()
                            && !live_suggestions.is_empty()
                            && (state.number_pick_mode() || state.selection_started());
                        if manual_cycle_mode_active && value != current_text {
                            // Guard manual cycle/edit mode from accidental printable text mutation.
                            save_editor_text(&current_text);
                            state.text.set(current_text);
                            return;
                        }

                        save_editor_text(&value);
                        state.text.set(value.clone());
                        state.manual_save_request.set(None);
                        // Start fresh after text changes so the next ArrowDown selects the first
                        // candidate for the current token instead of continuing stale selection.
                        state.number_pick_mode.set(false);
                        state.selection_started.set(false);
                        state.selected.set(0);
                        spawn(update_candidates(value, state));
                    },
                    onkeydown: move |event| {
                        let key = event.key().to_string();
                        let modifiers = event.modifiers();

                        if modifiers.contains(Modifiers::ALT)
                            && modifiers.contains(Modifiers::CONTROL)
                            && key.eq_ignore_ascii_case("k")
                        {
                            event.prevent_default();
                            let next = !state.roman_enabled();
                            state.roman_enabled.set(next);
                            save_enabled(next);
                            if !next {
                                state.clear_candidate_state_and_picker();
                            } else {
                                spawn(update_candidates(state.text(), state));
                            }
                            return;
                        }

                        if !state.roman_enabled() {
                            return;
                        }

                        let live_suggestions = state.suggestions();
                        // Next-word predictions are pointer-only (tap the docked bar):
                        // they must not respond to Tab/Enter/Space, so Enter stays free
                        // for newlines. Only transliteration candidates are keyboard-driven.
                        let has_live_suggestions = !live_suggestions.is_empty()
                            && state.candidate_mode() != CandidateMode::NextWord;
                        let can_cycle_with_space = space_cycle_enabled(has_live_suggestions, state.candidate_mode());
                        let live_suggestion_len = live_suggestions.len();
                        let manual_cycle_mode_active = state.input_mode() == InputMode::ManualCharacterTyping
                            && state.manual_typing_state().is_some()
                            && has_live_suggestions
                            && (state.number_pick_mode() || state.selection_started());
                        let selection_lock_active = has_live_suggestions && state.number_pick_mode();

                        match key.as_str() {
                            "ArrowLeft" => {
                                if handle_horizontal_arrow(-1, has_live_suggestions, state) {
                                    event.prevent_default();
                                }
                            }
                            "ArrowRight" => {
                                if handle_horizontal_arrow(1, has_live_suggestions, state) {
                                    event.prevent_default();
                                }
                            }
                            "Tab" if state.input_mode() == InputMode::ManualCharacterTyping => {
                                event.prevent_default();
                                let Some(manual) = state.manual_typing_state() else {
                                    return;
                                };
                                let ordered = [
                                    ManualComposeKind::BaseConsonant,
                                    ManualComposeKind::Vowel,
                                    ManualComposeKind::Subscript,
                                ];
                                let current_index = ordered
                                    .iter()
                                    .position(|kind| *kind == manual.kind_filter)
                                    .unwrap_or(0);
                                for step in 1..=ordered.len() {
                                    let kind = ordered[(current_index + step) % ordered.len()];
                                    if manual.candidates.iter().any(|candidate| candidate.kind == kind) {
                                        let _ = set_manual_kind_filter(kind, state);
                                        break;
                                    }
                                }
                                state.number_pick_mode.set(false);
                            }
                            key
                                if manual_cycle_mode_active
                                    && key.eq_ignore_ascii_case("s")
                                    && !modifiers.contains(Modifiers::CONTROL)
                                    && !modifiers.contains(Modifiers::ALT)
                                    && !modifiers.contains(Modifiers::META) =>
                            {
                                event.prevent_default();
                                if skip_manual_roman_char(state) {
                                    state.number_pick_mode.set(true);
                                    state.selection_started.set(true);
                                }
                            }
                            key
                                if manual_cycle_mode_active
                                    && key.eq_ignore_ascii_case("u")
                                    && !modifiers.contains(Modifiers::CONTROL)
                                    && !modifiers.contains(Modifiers::ALT)
                                    && !modifiers.contains(Modifiers::META) =>
                            {
                                event.prevent_default();
                                if undo_manual_step(state) {
                                    state.number_pick_mode.set(true);
                                    state.selection_started.set(true);
                                }
                            }
                            "Tab" if candidate_level == CandidateLevel::Phrase => {
                                event.prevent_default();
                                let _ = enter_segment_edit(state.selected(), state);
                            }
                            "Tab" if candidate_level == CandidateLevel::Segment => {
                                event.prevent_default();
                                let _ = exit_segment_edit(state);
                            }
                            "Tab" if has_live_suggestions => {
                                event.prevent_default();
                                let len = live_suggestion_len;
                                let next = if !state.selection_started() {
                                    0
                                } else {
                                    (state.selected() + 1) % len
                                };
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            "ArrowDown" if has_live_suggestions => {
                                event.prevent_default();
                                if event.is_auto_repeating() {
                                    return;
                                }
                                if state.segmented_refine_mode() {
                                    let Some(session) = state.segmented_session() else {
                                        return;
                                    };

                                    let len = session.current_candidate_len();
                                    if len == 0 {
                                        return;
                                    }

                                    let next = if !state.selection_started() {
                                        0
                                    } else {
                                        (state.selected() + 1) % len
                                    };

                                    select_segment_candidate(next, state);
                                    state.selection_started.set(true);
                                } else {
                                    let len = live_suggestion_len;
                                    if len == 0 {
                                        return;
                                    }

                                    let next = if !state.selection_started() {
                                        0
                                    } else {
                                        (state.selected() + 1) % len
                                    };

                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }

                                state.number_pick_mode.set(false);
                                // if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                //     let next = if !state.selection_started() { 0 } else { (state.selected() + 1) % len };
                                //     select_segment_candidate(next, state);
                                // } else {
                                //     if !state.selection_started() {
                                //         state.selected.set(0);
                                //     } else {
                                //         state.selected.set((state.selected() + 1) % len);
                                //     }
                                //     state.selection_started.set(true);
                                // }
                                // state.number_pick_mode.set(false);
                            }
                            "ArrowUp" if has_live_suggestions => {
                                if event.is_auto_repeating() {
                                    return;
                                }
                                event.prevent_default();
                                let len = live_suggestion_len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    let next = if !state.selection_started() {
                                        len.saturating_sub(1)
                                    } else {
                                        (state.selected() + len - 1) % len
                                    };
                                    select_segment_candidate(next, state);
                                } else {
                                    if !state.selection_started() {
                                        state.selected.set(len.saturating_sub(1));
                                    } else {
                                        state.selected.set((state.selected() + len - 1) % len);
                                    }
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            key
                                if is_space_key(key)
                                    && modifiers.contains(Modifiers::SHIFT)
                                    && can_cycle_with_space =>
                            {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            key if is_space_key(key) && can_cycle_with_space && !state.selection_started() => {
                                event.prevent_default();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(0, state);
                                } else {
                                    state.selected.set(0);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            key if is_space_key(key) && can_cycle_with_space => {
                                event.prevent_default();
                                let len = live_suggestion_len;
                                let next = (state.selected() + 1) % len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            "Enter" if has_live_suggestions => {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            "Enter" if state.input_mode() == InputMode::ManualCharacterTyping => {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            key if has_live_suggestions
                                && apply_shortcut_selection(key, modifiers, live_suggestion_len, state) =>
                            {
                                event.prevent_default();
                                if candidate_level == CandidateLevel::Phrase {
                                    spawn(click_candidate(state.selected(), state));
                                }
                            }
                            key if selection_lock_active && has_live_suggestions => {
                                if should_exit_number_pick(key) {
                                    state.number_pick_mode.set(false);
                                    state.selection_started.set(false);
                                } else if key.chars().count() == 1
                                    && !modifiers.contains(Modifiers::CONTROL)
                                    && !modifiers.contains(Modifiers::ALT)
                                    && !modifiers.contains(Modifiers::META)
                                {
                                    // Keep selection lock: printable keys should not edit text while cycling.
                                    event.prevent_default();
                                }
                            }
                            _ => {}
                        }
                    },
                }
                // Caret popup: transliteration candidates only. Next-word
                // predictions render in the docked bar below (not here).
                if show_candidate_list {
                    div {
                        class: if state.input_mode() == InputMode::ManualCharacterTyping {
                            "suggestion-popup suggestion-popup-compact suggestion-popup-manual"
                        } else if candidate_level == CandidateLevel::Flat {
                            "suggestion-popup suggestion-popup-compact"
                        } else if candidate_level == CandidateLevel::Segment {
                            "suggestion-popup suggestion-popup-segment"
                        } else {
                            "suggestion-popup"
                        },
                        "data-testid": "suggestion-popup",
                        "data-candidate-level": match candidate_level {
                            CandidateLevel::Flat => "flat",
                            CandidateLevel::Phrase => "phrase",
                            CandidateLevel::Segment => "segment",
                        },
                        style: popup_style(state.popup()),
                        if let Some(manual) = &manual_state {
                            if !manual.composed_text.is_empty() || manual.consumed > 0 {
                                div { class: "manual-candidate-context", "data-testid": "manual-progress",
                                    div { class: "manual-progress-item manual-progress-built",
                                        span { class: "manual-progress-label", "Built" }
                                        strong { "{manual.composed_text}" }
                                    }
                                    div { class: "manual-progress-item manual-progress-remaining",
                                        span { class: "manual-progress-label", "Remaining" }
                                        strong {
                                            if manual.remaining_roman().is_empty() { "—" }
                                            else { "{manual.remaining_roman()}" }
                                        }
                                    }
                                    div { class: "manual-progress-next",
                                        span { class: "manual-progress-label", "Next" }
                                        span { "{manual.expected_kind.label()}" }
                                    }
                                }
                            }
                        }
                        if let Some(session) = &segment_session {
                            div { class: "candidate-context", "data-testid": "segment-edit-header",
                                button {
                                    class: "candidate-back",
                                    aria_label: "Return to phrase candidates",
                                    title: "Phrase candidates",
                                    onclick: move |_| { let _ = exit_segment_edit(state); },
                                    "‹"
                                }
                                div { class: "candidate-segments",
                                    for (segment_index, segment) in session.segments.iter().enumerate() {
                                        button {
                                            key: "segment-{segment_index}-{segment.input}",
                                            class: if segment_index == session.focused {
                                                "candidate-segment active"
                                            } else {
                                                "candidate-segment"
                                            },
                                            onclick: move |_| {
                                                let focused = state.segmented_session().map(|live| live.focused).unwrap_or(0);
                                                let delta = segment_index as isize - focused as isize;
                                                if delta != 0 { let _ = move_segment_focus(delta, state); }
                                            },
                                            span { class: "candidate-segment-output", "{segment.selected_text()}" }
                                            span { class: "candidate-segment-input", "{segment.input}" }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "candidate-track candidate-track-popup",
                            ul { class: "suggestion-list candidate-list",
                                for (index, item) in suggestions.iter()
                                    .enumerate()
                                    .skip(page_start)
                                    .take(VISIBLE_SUGGESTIONS) {
                                    li {
                                        key: "popup-{index}-{item}",
                                        class: if index == state.selected()
                                            && (state.selection_started() || candidate_level != CandidateLevel::Flat) {
                                            "suggestion active"
                                        } else {
                                            "suggestion"
                                        },
                                        button {
                                            class: "candidate-choice",
                                            onclick: move |_| {
                                                spawn(click_candidate(index, state));
                                            },
                                            span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                            span { class: "suggestion-main",
                                                span { class: "suggestion-word", "{item}" }
                                                if candidate_level == CandidateLevel::Flat {
                                                    if let Some(variants) = roman_variant_hints.get(&index) {
                                                        span { class: "suggestion-roman-hint", "{roman_hint_label(variants)}" }
                                                    } else {
                                                        span { class: "suggestion-roman-hint", "(derived)"}
                                                    }
                                                }
                                            }
                                            if candidate_level == CandidateLevel::Flat && recommended_indices.contains(&index) {
                                                span { class: "suggestion-recommended", "គួរប្រើ" }
                                            }
                                        }
                                        if candidate_level == CandidateLevel::Phrase
                                            && phrase_candidates.get(index).map(|phrase| phrase.segments.len() >= 2).unwrap_or(false) {
                                            button {
                                                class: "candidate-edit",
                                                aria_label: "Edit phrase segments",
                                                title: "Edit phrase segments",
                                                onclick: move |_| { let _ = enter_segment_edit(index, state); },
                                                "✎"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if page_count > 1 {
                            div { class: "candidate-page", "data-testid": "candidate-page", "{page_number}/{page_count}" }
                        }
                    }
                }
                // Docked next-word prediction bar: labeled strip under the editor,
                // always in the same place, tap-only (Enter stays free for newlines).
                if is_next_word && !next_word_predictions.is_empty() {
                    div { class: "next-word-dock", "data-testid": "next-word-dock",
                        span { class: "next-word-dock-label",
                            svg { class: "next-word-dock-arrow", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2", stroke_linecap: "round",
                                path { d: "M5 12h14" } path { d: "m12 5 7 7-7 7" }
                            }
                            "បន្ទាប់"
                        }
                        div { class: "next-word-dock-chips",
                            for (index, item) in next_word_predictions.iter().enumerate().take(VISIBLE_SUGGESTIONS) {
                                button {
                                    key: "nextword-{index}-{item}",
                                    class: if index == 0 { "next-word-chip next-word-chip-top" } else { "next-word-chip" },
                                    onclick: move |_| { spawn(click_candidate(index, state)); },
                                    span { class: "kh", "{item}" }
                                }
                            }
                        }
                        span { class: "next-word-dock-hint", "ប៉ះដើម្បីបញ្ចូល" }
                    }
                }
                div {
                    class: if show_candidate_list { "candidate-bar" } else { "candidate-bar candidate-bar-empty" },
                    div { class: "candidate-track candidate-track-mobile",
                        if show_candidate_list {
                            ul { class: "suggestion-list candidate-list",
                                for (index, item) in suggestions.iter()
                                    .enumerate()
                                    .skip(page_start)
                                    .take(VISIBLE_SUGGESTIONS) {
                                    li {
                                        key: "mobile-{index}-{item}",
                                        class: if state.selection_started() && index == state.selected() { "suggestion active" } else { "suggestion" },
                                        button {
                                            onclick: move |_| {
                                                spawn(click_candidate(index, state));
                                            },
                                            span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                            span { class: "suggestion-main",
                                                span { class: "suggestion-word", "{item}" }
                                                if let Some(variants) = roman_variant_hints.get(&index) {
                                                    span { class: "suggestion-roman-hint", "{roman_hint_label(variants)}" }
                                                } else {
                                                    span { class: "suggestion-roman-hint", "(derived)"}
                                                }
                                            }
                                            if recommended_indices.contains(&index) {
                                                span { class: "suggestion-recommended", "គួរប្រើ" }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "candidate-empty", aria_hidden: "true",
                                span { class: "segment-placeholder-chip segment-placeholder-chip-1" }
                                span { class: "segment-placeholder-chip segment-placeholder-chip-2" }
                                span { class: "segment-placeholder-chip segment-placeholder-chip-3" }
                            }
                        }
                    }
                    div { class: "candidate-footer",
                        if let Some(manual) = &manual_state {
                            div { class: "manual-kind-switch",
                                button {
                                    class: if manual.kind_filter == ManualComposeKind::BaseConsonant {
                                        "manual-kind-tab active"
                                    } else {
                                        "manual-kind-tab"
                                    },
                                    disabled: manual_consonant_count == 0,
                                    onclick: move |_| {
                                        let _ = set_manual_kind_filter(ManualComposeKind::BaseConsonant, state);
                                    },
                                    "Consonant ({manual_consonant_count})"
                                }
                                button {
                                    class: if manual.kind_filter == ManualComposeKind::Vowel {
                                        "manual-kind-tab active"
                                    } else {
                                        "manual-kind-tab"
                                    },
                                    disabled: manual_vowel_count == 0,
                                    onclick: move |_| {
                                        let _ = set_manual_kind_filter(ManualComposeKind::Vowel, state);
                                    },
                                    "Vowel ({manual_vowel_count})"
                                }
                                button {
                                    class: if manual.kind_filter == ManualComposeKind::Subscript {
                                        "manual-kind-tab active"
                                    } else {
                                        "manual-kind-tab"
                                    },
                                    disabled: manual_subscript_count == 0,
                                    onclick: move |_| {
                                        let _ = set_manual_kind_filter(ManualComposeKind::Subscript, state);
                                    },
                                    "Subscript ({manual_subscript_count})"
                                }
                                button {
                                    class: "manual-kind-tab",
                                    disabled: manual.remaining_roman().is_empty(),
                                    onclick: move |_| {
                                        let _ = skip_manual_roman_char(state);
                                    },
                                    "Skip (S)"
                                }
                                button {
                                    class: "manual-kind-tab",
                                    disabled: !manual_can_undo,
                                    onclick: move |_| {
                                        let _ = undo_manual_step(state);
                                    },
                                    "Undo (U)"
                                }
                            }
                        }
                        div { class: "candidate-hints desktop-candidate-hints", "data-testid": "candidate-hints",
                            if is_next_word {
                                span { class: "candidate-hint",
                                    span { class: "editor-tip-text", "ប៉ះពាក្យដើម្បីបញ្ចូល" }
                                    span { class: "editor-tip-sep", "·" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "ចុះបន្ទាត់" }
                                }
                            } else if state.input_mode() == InputMode::ManualCharacterTyping {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "ប្ដូរប្រភេទ" }
                                    span { class: "keycap", "S" }
                                    span { class: "editor-tip-text", "រំលង" }
                                    span { class: "keycap", "U" }
                                    span { class: "editor-tip-text", "ថយក្រោយ" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "បញ្ចប់" }
                                }
                            } else if candidate_level == CandidateLevel::Phrase {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Space / ↑↓" }
                                    span { class: "editor-tip-text", "ប្ដូរឃ្លា" }
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "កែពាក្យ" }
                                    span { class: "keycap", "1–5 / Enter" }
                                    span { class: "editor-tip-text", "បញ្ចូល" }
                                }
                            } else if candidate_level == CandidateLevel::Segment {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "← →" }
                                    span { class: "editor-tip-text", "ប្ដូរពាក្យ" }
                                    span { class: "keycap", "Space / ↑↓" }
                                    span { class: "editor-tip-text", "ប្ដូរជម្រើស" }
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "ត្រឡប់" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "បញ្ចូលឃ្លា" }
                                }
                            } else if show_candidate_list {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Space" }
                                    span { class: "editor-tip-text", "ប្ដូរជម្រើស" }
                                    span { class: "keycap", "1–5" }
                                    span { class: "editor-tip-text", "ជ្រើស" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "បញ្ចូល" }
                                }
                            } else {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Ctrl+Alt+K" }
                                    span { class: "editor-tip-text", "បើក/បិទការបម្លែង" }
                                    span { class: "editor-tip-sep", "·" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "ចុះបន្ទាត់" }
                                }
                            }
                        }
                        div { class: "mobile-candidate-footer",
                            div { class: "mobile-caret-controls",
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-caret-left",
                                    aria_label: "Arrow-left behavior",
                                    onclick: move |_| {
                                        if !handle_horizontal_arrow(-1, show_candidate_list, state) {
                                            spawn(async move {
                                                let _ = move_editor_caret(-1).await;
                                            });
                                        }
                                    },
                                    "←"
                                }
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-caret-right",
                                    aria_label: "Arrow-right behavior",
                                    onclick: move |_| {
                                        if !handle_horizontal_arrow(1, show_candidate_list, state) {
                                            spawn(async move {
                                                let _ = move_editor_caret(1).await;
                                            });
                                        }
                                    },
                                    "→"
                                }
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-select-up",
                                    aria_label: "Select previous suggestion",
                                    disabled: !show_candidate_list,
                                    onclick: move |_| {
                                        let _ = cycle_live_candidate(-1, state);
                                    },
                                    "↑"
                                }
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-select-down",
                                    aria_label: "Select next suggestion",
                                    disabled: !show_candidate_list,
                                    onclick: move |_| {
                                        let _ = cycle_live_candidate(1, state);
                                    },
                                    "↓"
                                }
                            }
                            div { class: "mobile-candidate-hints", "data-testid": "mobile-candidate-hints",
                                if state.suggestion_loading() {
                                    span { class: "candidate-hint-loading", "កំពុងរៀបចំ…" }
                                }
                                if is_next_word {
                                    span { class: "editor-tip-text", "ប៉ះដើម្បីបញ្ចូល · Enter ចុះបន្ទាត់" }
                                } else if state.input_mode() == InputMode::ManualCharacterTyping {
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "ប្ដូរ" }
                                    span { class: "keycap", "S" }
                                    span { class: "editor-tip-text", "រំលង" }
                                    span { class: "keycap", "U" }
                                    span { class: "editor-tip-text", "ថយក្រោយ" }
                                } else if candidate_level == CandidateLevel::Phrase {
                                    span { class: "editor-tip-text", "ប៉ះឃ្លាដើម្បីបញ្ចូល · ប៉ះ ✎ ដើម្បីកែពាក្យ" }
                                } else if candidate_level == CandidateLevel::Segment {
                                    span { class: "editor-tip-text", "ប៉ះពាក្យដើម្បីប្ដូរ · ប៉ះ ‹ ដើម្បីត្រឡប់" }
                                } else if show_candidate_list {
                                    span { class: "keycap", "↑↓" }
                                    span { class: "editor-tip-text", "ជ្រើស" }
                                    span { class: "keycap", "Space" }
                                    span { class: "editor-tip-text", "ប្ដូរ" }
                                    span { class: "keycap", "1–5" }
                                    span { class: "editor-tip-text", "បញ្ចូល" }
                                } else {
                                    span { class: "editor-tip-text", "ប៉ះផ្ទៃសរសេរ ហើយចាប់ផ្ដើមវាយ" }
                                }
                            }
                        }
                    }
                }
                if let Some(mark) = state.composition() {
                    if state.input_mode() == InputMode::ManualCharacterTyping {
                        if let Some(preview) = manual_inline_preview.clone() {
                            div {
                                class: "composition-preview",
                                style: composition_preview_style(&mark, font_size()),
                                span { class: "composition-preview-text", "{preview}" }
                                span { class: "composition-caret", aria_hidden: "true" }
                            }
                        } else {
                            div {
                                class: "composition-mark",
                                style: composition_style(&mark, false),
                            }
                        }
                    } else {
                        div {
                            class: "composition-mark",
                            style: composition_style(&mark, false),
                        }
                    }
                }
            }
        }
    }
}
